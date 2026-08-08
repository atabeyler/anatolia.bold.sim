use serde_json::{json, Value};
use sim_core::{Individual, SimulationState};
use sqlx::{
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    FromRow,
    PgPool,
    QueryBuilder,
    SqlitePool,
};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;
use std::sync::Arc;
use std::time::Duration;

use crate::ratelimit::RateLimiter;
use crate::runtime::RuntimeManager;

#[derive(Clone)]
pub enum DbBackend {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

#[derive(Clone)]
pub struct AppState {
    pub backend: DbBackend,
    pub runtime: Arc<RuntimeManager>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub async fn new() -> Result<Self, sqlx::Error> {
        // Prefer a managed Postgres database (DATABASE_URL) when configured --
        // it persists independently of the web service's container/machine,
        // unlike the SQLite fallback below which lives on ephemeral local
        // storage and is wiped on every deploy.
        //
        // When DATABASE_URL *is* set, Postgres is the only acceptable backend --
        // silently falling back to a fresh, empty SQLite database here used to
        // mean the whole service would boot "successfully" and look healthy
        // while serving from a database with no users/simulations in it at all
        // (this is exactly what caused a real admin login lockout: nothing in
        // the logs but a warn!, nothing in the UI, just every login failing).
        // Fail startup instead so a Postgres outage is a visible, loud deploy
        // failure rather than a silent, confusing data loss.
        let backend = match std::env::var("DATABASE_URL") {
            Ok(database_url) if !database_url.trim().is_empty() => {
                DbBackend::Postgres(Self::connect_postgres(&database_url).await?)
            }
            // RENDER_EXTERNAL_URL is only ever set by Render itself (see
            // spawn_self_ping in main.rs) -- a reliable signal that this
            // process is a real browser-facing web deploy, not the desktop
            // sidecar or a local dev run. The browser client must never end
            // up talking to a throwaway SQLite database (accounts now only
            // exist in Postgres, see auth::is_local_backend), so treat a
            // missing DATABASE_URL here the same as an unreachable one: a
            // loud startup failure instead of a silently empty fallback DB.
            _ if std::env::var("RENDER_EXTERNAL_URL").is_ok() => {
                panic!("DATABASE_URL is required on the web deploy (RENDER_EXTERNAL_URL is set) -- refusing to fall back to a throwaway SQLite database");
            }
            _ => Self::sqlite_backend().await?,
        };

        migrate(&backend).await?;
        let runtime = Arc::new(RuntimeManager::new());
        // Re-enabled: the O(n^2) group-member scans that were the actual
        // root cause of the crash loop (not this function itself) are fixed.
        resume_running_simulations(&backend, &runtime).await?;
        Ok(Self { backend, runtime, rate_limiter: Arc::new(RateLimiter::new()) })
    }

    async fn connect_postgres(database_url: &str) -> Result<PgPool, sqlx::Error> {
        let mut last_err: Option<sqlx::Error> = None;

        for attempt in 0..3 {
            match tokio::time::timeout(
                Duration::from_secs(20),
                PgPoolOptions::new()
                    // Was 8. resume_running_simulations spins up a runtime_loop
                    // for every simulation still marked "running" on every server
                    // boot, and each loop's per-batch iteration holds up to 2
                    // connections concurrently (state load + genealogy delta) plus
                    // sequential save/upsert calls -- with more than a couple of
                    // simulations left running at once, plus ordinary HTTP/websocket
                    // traffic sharing the same pool, 8 was tight enough to produce
                    // real "pool timed out" failures and stalls (see runtime.rs's
                    // error logging) that looked like the simulation itself freezing,
                    // regardless of that simulation's own age/history.
                    .max_connections(20)
                    .acquire_timeout(Duration::from_secs(20))
                    // migrate() below used to be the only place that ran `SET
                    // search_path TO antsim, public` -- but that's a per-session
                    // setting, and it only ever applied to whichever single
                    // connection sqlx happened to hand that one query. Every
                    // other connection in this pool (there can be up to
                    // max_connections of them, opened lazily as load requires)
                    // kept Postgres's default search_path, which does NOT
                    // include the antsim schema. Two logically-identical
                    // queries could then resolve `users`/`simulations` to two
                    // different physical tables depending on which pooled
                    // connection happened to service each one -- e.g. a login
                    // finding the antsim.users row that created it while the
                    // admin panel's list, landing on a different connection,
                    // only sees whatever ended up in public.users. after_connect
                    // runs this on every connection the pool ever opens, so
                    // every query -- no matter which physical connection
                    // services it -- resolves against the same schema.
                    .after_connect(|conn, _meta| {
                        Box::pin(async move {
                            sqlx::Executor::execute(conn, "SET search_path TO antsim, public").await?;
                            Ok(())
                        })
                    })
                    .connect(database_url),
            )
            .await
            {
                Ok(Ok(pool)) => return Ok(pool),
                Ok(Err(err)) => last_err = Some(err),
                Err(_) => {
                    last_err = Some(sqlx::Error::PoolTimedOut);
                }
            }

            if attempt < 2 {
                tokio::time::sleep(Duration::from_secs(2 * (attempt + 1))).await;
            }
        }

        Err(last_err.unwrap_or(sqlx::Error::PoolTimedOut))
    }

    // The tick loop (runtime.rs) does a save plus an individuals upsert on
    // every single batch -- as often as once a second at low speed. SQLite's
    // default rollback-journal mode fsyncs the whole database file on every
    // commit, and flash storage on a phone can make that fsync take hundreds
    // of milliseconds (this is what a Performance panel screenshot with
    // Kaydet/Upsert times of 84ms/682ms on Android "Yerel" mode turned out to
    // be -- the tick loop was correctly honoring a much higher selected
    // speed, but couldn't keep up because of storage latency, not because of
    // any throttling bug). WAL mode only fsyncs at checkpoints instead of
    // every write, and NORMAL synchronous is the durability level WAL is
    // designed to pair with (a page can be lost on an OS crash between
    // checkpoints, but not corrupted -- acceptable for a single-device local
    // simulation that isn't the system of record the way the cloud Postgres
    // deployment is).
    fn sqlite_connect_options(url: &str) -> Result<SqliteConnectOptions, sqlx::Error> {
        Ok(SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5)))
    }

    // SIM_DATA_DIR points the SQLite file at a persistent-disk mount in
    // production if one is attached; otherwise falls back to the repo-relative
    // path used for local development. Only reached when DATABASE_URL is unset.
    async fn sqlite_backend() -> Result<DbBackend, sqlx::Error> {
        let data_dir = std::env::var("SIM_DATA_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).join("rust")
        });
        let db_path = data_dir.join("sim.db");
        let normalized = db_path.to_string_lossy().replace('\\', "/");
        let sqlite_url = format!("sqlite:///{}?mode=rwc", normalized);
        let pool = SqlitePoolOptions::new().max_connections(8).connect_with(Self::sqlite_connect_options(&sqlite_url)?).await?;
        Ok(DbBackend::Sqlite(pool))
    }
}

// Surfaced on /api/health for visibility without digging through deploy
// logs. Startup now fails outright if DATABASE_URL is set but Postgres is
// unreachable (see AppState::new above), so by the time this is callable
// "sqlite" only ever means DATABASE_URL was never configured at all (local/
// desktop mode) -- never a silent, unrequested fallback from a Postgres
// outage.
pub fn backend_name(backend: &DbBackend) -> &'static str {
    match backend {
        DbBackend::Postgres(_) => "postgres",
        DbBackend::Sqlite(_) => "sqlite",
    }
}

fn as_pg(backend: &DbBackend) -> Option<&PgPool> {
    match backend {
        DbBackend::Postgres(pool) => Some(pool),
        _ => None,
    }
}

fn as_sqlite(backend: &DbBackend) -> Option<&SqlitePool> {
    match backend {
        DbBackend::Sqlite(pool) => Some(pool),
        _ => None,
    }
}

pub async fn migrate(backend: &DbBackend) -> Result<(), sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
            .execute(pool)
            .await?;
        sqlx::query("CREATE SCHEMA IF NOT EXISTS antsim")
            .execute(pool)
            .await?;
        sqlx::query("SET search_path TO antsim, public")
            .execute(pool)
            .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_code VARCHAR(20) UNIQUE,
                username VARCHAR(50) UNIQUE,
                first_name VARCHAR(100) NOT NULL DEFAULT '',
                last_name VARCHAR(100) NOT NULL DEFAULT '',
                tc_no VARCHAR(11) UNIQUE,
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                role VARCHAR(20) DEFAULT 'pending',
                is_approved BOOLEAN DEFAULT false,
                is_banned BOOLEAN DEFAULT false,
                ban_reason TEXT,
                email_verified BOOLEAN DEFAULT false,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        // The table may already exist from before this column set was
        // finalized (e.g. the pre-migration Node.js backend's schema on the
        // same managed Postgres instance) -- CREATE TABLE IF NOT EXISTS is a
        // no-op in that case, so patch in anything still missing.
        sqlx::query(
            r#"
            ALTER TABLE users
                ADD COLUMN IF NOT EXISTS user_code VARCHAR(20),
                ADD COLUMN IF NOT EXISTS username VARCHAR(50),
                ADD COLUMN IF NOT EXISTS first_name VARCHAR(100) DEFAULT '',
                ADD COLUMN IF NOT EXISTS last_name VARCHAR(100) DEFAULT '',
                ADD COLUMN IF NOT EXISTS tc_no VARCHAR(11),
                ADD COLUMN IF NOT EXISTS email VARCHAR(255),
                ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255) DEFAULT '',
                ADD COLUMN IF NOT EXISTS role VARCHAR(20) DEFAULT 'pending',
                ADD COLUMN IF NOT EXISTS is_approved BOOLEAN DEFAULT false,
                ADD COLUMN IF NOT EXISTS is_banned BOOLEAN DEFAULT false,
                ADD COLUMN IF NOT EXISTS ban_reason TEXT,
                ADD COLUMN IF NOT EXISTS email_verified BOOLEAN DEFAULT false,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW(),
                ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW(),
                -- Last-used "create simulation" wizard field values (name/
                -- lat/lon/founder params), as a JSON blob the client just
                -- round-trips opaquely. Account-scoped rather than
                -- localStorage: iOS Safari's ITP caps script-writable
                -- storage to 7 days of no top-level visit and silently
                -- wipes it, which localStorage alone can't survive on any
                -- device -- this does, and also means the same defaults
                -- follow you across devices/browsers instead of being
                -- per-browser.
                ADD COLUMN IF NOT EXISTS wizard_defaults TEXT
            "#,
        )
        .execute(pool)
        .await?;
        // A pre-existing legacy table can have these columns already, but
        // NOT NULL with no default (or a different default) -- our INSERT
        // statements only set the columns they need and rely on the schema
        // default for the rest, so every default has to actually be in
        // place or those inserts fail with "null value ... violates
        // not-null constraint" the moment a legacy row's column is hit.
        sqlx::query(
            r#"
            ALTER TABLE users
                ALTER COLUMN first_name SET DEFAULT '',
                ALTER COLUMN last_name SET DEFAULT '',
                ALTER COLUMN role SET DEFAULT 'pending',
                ALTER COLUMN is_approved SET DEFAULT false,
                ALTER COLUMN is_banned SET DEFAULT false,
                ALTER COLUMN email_verified SET DEFAULT false,
                ALTER COLUMN created_at SET DEFAULT NOW(),
                ALTER COLUMN updated_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            UPDATE users SET
                first_name = COALESCE(first_name, ''),
                last_name = COALESCE(last_name, ''),
                role = COALESCE(role, 'pending'),
                is_approved = COALESCE(is_approved, false),
                is_banned = COALESCE(is_banned, false),
                email_verified = COALESCE(email_verified, false),
                created_at = COALESCE(created_at, NOW()),
                updated_at = COALESCE(updated_at, NOW())
            WHERE first_name IS NULL OR last_name IS NULL OR role IS NULL OR is_approved IS NULL
               OR is_banned IS NULL OR email_verified IS NULL OR created_at IS NULL OR updated_at IS NULL
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS simulations (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID,
                name VARCHAR(255) NOT NULL,
                status VARCHAR(20) DEFAULT 'paused',
                current_day INTEGER DEFAULT 0,
                current_year INTEGER DEFAULT 0,
                start_latitude DOUBLE PRECISION NOT NULL DEFAULT 0,
                start_longitude DOUBLE PRECISION NOT NULL DEFAULT 0,
                speed_multiplier INTEGER DEFAULT 1,
                founder_1 JSONB NOT NULL DEFAULT '{}'::jsonb,
                founder_2 JSONB NOT NULL DEFAULT '{}'::jsonb,
                world_state JSONB DEFAULT '{}'::jsonb,
                settings JSONB DEFAULT '{}'::jsonb,
                state_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                population_count INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE simulations
                ADD COLUMN IF NOT EXISTS user_id UUID,
                ADD COLUMN IF NOT EXISTS name VARCHAR(255) DEFAULT 'Untitled Simulation',
                ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'paused',
                ADD COLUMN IF NOT EXISTS current_day INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS current_year INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS start_latitude DOUBLE PRECISION DEFAULT 0,
                ADD COLUMN IF NOT EXISTS start_longitude DOUBLE PRECISION DEFAULT 0,
                ADD COLUMN IF NOT EXISTS speed_multiplier INTEGER DEFAULT 1,
                ADD COLUMN IF NOT EXISTS founder_1 JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS founder_2 JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS world_state JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS settings JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS state_json JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS population_count INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW(),
                ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE simulations
                ALTER COLUMN name SET DEFAULT 'Untitled Simulation',
                ALTER COLUMN status SET DEFAULT 'paused',
                ALTER COLUMN current_day SET DEFAULT 0,
                ALTER COLUMN current_year SET DEFAULT 0,
                ALTER COLUMN start_latitude SET DEFAULT 0,
                ALTER COLUMN start_longitude SET DEFAULT 0,
                ALTER COLUMN speed_multiplier SET DEFAULT 1,
                ALTER COLUMN founder_1 SET DEFAULT '{}'::jsonb,
                ALTER COLUMN founder_2 SET DEFAULT '{}'::jsonb,
                ALTER COLUMN world_state SET DEFAULT '{}'::jsonb,
                ALTER COLUMN settings SET DEFAULT '{}'::jsonb,
                ALTER COLUMN state_json SET DEFAULT '{}'::jsonb,
                ALTER COLUMN population_count SET DEFAULT 0,
                ALTER COLUMN created_at SET DEFAULT NOW(),
                ALTER COLUMN updated_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        // SET DEFAULT only affects future inserts -- rows that were already
        // sitting in a pre-existing legacy table (or written during earlier,
        // still-broken deploys of this app) can have real NULLs stored in
        // columns our SimulationRow/CheckpointRow structs decode as
        // non-optional, which fails every read of that row. Backfill those
        // in place, once, at startup.
        sqlx::query(
            r#"
            UPDATE simulations SET
                name = COALESCE(name, 'Untitled Simulation'),
                status = COALESCE(status, 'paused'),
                current_day = COALESCE(current_day, 0),
                current_year = COALESCE(current_year, 0),
                start_latitude = COALESCE(start_latitude, 0),
                start_longitude = COALESCE(start_longitude, 0),
                speed_multiplier = COALESCE(speed_multiplier, 1),
                founder_1 = COALESCE(founder_1, '{}'::jsonb),
                founder_2 = COALESCE(founder_2, '{}'::jsonb),
                world_state = COALESCE(world_state, '{}'::jsonb),
                settings = COALESCE(settings, '{}'::jsonb),
                state_json = COALESCE(state_json, '{}'::jsonb),
                population_count = COALESCE(population_count, 0),
                created_at = COALESCE(created_at, NOW()),
                updated_at = COALESCE(updated_at, NOW())
            WHERE name IS NULL OR status IS NULL OR current_day IS NULL OR current_year IS NULL
               OR start_latitude IS NULL OR start_longitude IS NULL OR speed_multiplier IS NULL
               OR founder_1 IS NULL OR founder_2 IS NULL OR world_state IS NULL OR settings IS NULL
               OR state_json IS NULL OR population_count IS NULL OR created_at IS NULL OR updated_at IS NULL
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS individuals (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                simulation_id UUID NOT NULL,
                birth_day INTEGER NOT NULL,
                death_day INTEGER,
                alive BOOLEAN DEFAULT true,
                is_dead BOOLEAN DEFAULT false,
                sex VARCHAR(10) NOT NULL DEFAULT 'unknown',
                x DOUBLE PRECISION NOT NULL DEFAULT 0,
                y DOUBLE PRECISION NOT NULL DEFAULT 0,
                genome JSONB NOT NULL DEFAULT '{}'::jsonb,
                phenotype JSONB NOT NULL DEFAULT '{}'::jsonb,
                epigenome JSONB DEFAULT '{}'::jsonb,
                health JSONB DEFAULT '{}'::jsonb,
                mind JSONB DEFAULT '{}'::jsonb,
                social JSONB DEFAULT '{}'::jsonb,
                skills JSONB DEFAULT '[]'::jsonb,
                beliefs JSONB DEFAULT '[]'::jsonb,
                language JSONB DEFAULT '{}'::jsonb,
                memory JSONB DEFAULT '{}'::jsonb,
                parent_1_id UUID,
                parent_2_id UUID,
                death_cause VARCHAR(50),
                is_founder BOOLEAN DEFAULT false,
                home_x DOUBLE PRECISION,
                home_y DOUBLE PRECISION,
                group_id VARCHAR(100),
                inbreeding_coeff DOUBLE PRECISION DEFAULT 0,
                psychology JSONB DEFAULT '{}'::jsonb,
                inventory JSONB DEFAULT '{}'::jsonb,
                known_techs JSONB DEFAULT '[]'::jsonb,
                data_json JSONB,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE individuals
                ADD COLUMN IF NOT EXISTS simulation_id UUID,
                ADD COLUMN IF NOT EXISTS birth_day INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS death_day INTEGER,
                ADD COLUMN IF NOT EXISTS alive BOOLEAN DEFAULT true,
                ADD COLUMN IF NOT EXISTS is_dead BOOLEAN DEFAULT false,
                ADD COLUMN IF NOT EXISTS sex VARCHAR(10) DEFAULT 'unknown',
                ADD COLUMN IF NOT EXISTS x DOUBLE PRECISION DEFAULT 0,
                ADD COLUMN IF NOT EXISTS y DOUBLE PRECISION DEFAULT 0,
                ADD COLUMN IF NOT EXISTS genome JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS phenotype JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS epigenome JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS health JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS mind JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS social JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS skills JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS beliefs JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS language JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS memory JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS parent_1_id UUID,
                ADD COLUMN IF NOT EXISTS parent_2_id UUID,
                ADD COLUMN IF NOT EXISTS death_cause VARCHAR(50),
                ADD COLUMN IF NOT EXISTS is_founder BOOLEAN DEFAULT false,
                ADD COLUMN IF NOT EXISTS home_x DOUBLE PRECISION,
                ADD COLUMN IF NOT EXISTS home_y DOUBLE PRECISION,
                ADD COLUMN IF NOT EXISTS group_id VARCHAR(100),
                ADD COLUMN IF NOT EXISTS inbreeding_coeff DOUBLE PRECISION DEFAULT 0,
                ADD COLUMN IF NOT EXISTS psychology JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS inventory JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS known_techs JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS data_json JSONB,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW(),
                ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE individuals
                ALTER COLUMN birth_day SET DEFAULT 0,
                ALTER COLUMN alive SET DEFAULT true,
                ALTER COLUMN is_dead SET DEFAULT false,
                ALTER COLUMN sex SET DEFAULT 'unknown',
                ALTER COLUMN x SET DEFAULT 0,
                ALTER COLUMN y SET DEFAULT 0,
                ALTER COLUMN genome SET DEFAULT '{}'::jsonb,
                ALTER COLUMN phenotype SET DEFAULT '{}'::jsonb,
                ALTER COLUMN epigenome SET DEFAULT '{}'::jsonb,
                ALTER COLUMN health SET DEFAULT '{}'::jsonb,
                ALTER COLUMN mind SET DEFAULT '{}'::jsonb,
                ALTER COLUMN social SET DEFAULT '{}'::jsonb,
                ALTER COLUMN skills SET DEFAULT '[]'::jsonb,
                ALTER COLUMN beliefs SET DEFAULT '[]'::jsonb,
                ALTER COLUMN language SET DEFAULT '{}'::jsonb,
                ALTER COLUMN memory SET DEFAULT '{}'::jsonb,
                ALTER COLUMN is_founder SET DEFAULT false,
                ALTER COLUMN inbreeding_coeff SET DEFAULT 0,
                ALTER COLUMN psychology SET DEFAULT '{}'::jsonb,
                ALTER COLUMN inventory SET DEFAULT '{}'::jsonb,
                ALTER COLUMN known_techs SET DEFAULT '[]'::jsonb,
                ALTER COLUMN created_at SET DEFAULT NOW(),
                ALTER COLUMN updated_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        // Every `individuals` query filters by simulation_id (upsert_individuals,
        // load_individual_payloads, load_genealogy_index, the bounded tick-loop
        // load) -- without this, each one is a sequential scan across every
        // individual ever born in every simulation on the whole server, not just
        // this one, and that only gets worse as more simulations accumulate.
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_individuals_simulation_id ON individuals(simulation_id)")
            .execute(pool)
            .await?;
        // death_day/parent_1_id/parent_2_id/inbreeding_coeff have existed as
        // dedicated columns on this table for a while but were never actually
        // populated by upsert_individuals until now -- backfill them from the
        // data_json each row already carries so load_genealogy_index and the
        // bounded tick-loop load work correctly for simulations that were
        // already in progress before this migration shipped, not just new
        // ones. Guarded so it's only real work the first time it runs; every
        // boot after that, the WHERE clause matches nothing.
        sqlx::query(
            r#"
            UPDATE individuals SET
                death_day = COALESCE(death_day, (data_json->>'death_day')::integer),
                parent_1_id = COALESCE(parent_1_id, (data_json->>'parent_1_id')::uuid),
                parent_2_id = COALESCE(parent_2_id, (data_json->>'parent_2_id')::uuid),
                inbreeding_coeff = GREATEST(inbreeding_coeff, COALESCE((data_json->>'inbreeding_coeff')::double precision, 0))
            WHERE data_json IS NOT NULL
              AND (
                (death_day IS NULL AND data_json->>'death_day' IS NOT NULL) OR
                (parent_1_id IS NULL AND data_json->>'parent_1_id' IS NOT NULL) OR
                (parent_2_id IS NULL AND data_json->>'parent_2_id' IS NOT NULL) OR
                (inbreeding_coeff = 0 AND COALESCE((data_json->>'inbreeding_coeff')::double precision, 0) > 0)
              )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS checkpoints (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                simulation_id UUID NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                population_count INTEGER NOT NULL,
                population_snapshot JSONB NOT NULL DEFAULT '[]'::jsonb,
                world_state JSONB NOT NULL DEFAULT '{}'::jsonb,
                tech_state JSONB NOT NULL DEFAULT '[]'::jsonb,
                belief_state JSONB NOT NULL DEFAULT '[]'::jsonb,
                art_state JSONB NOT NULL DEFAULT '[]'::jsonb,
                groups JSONB NOT NULL DEFAULT '[]'::jsonb,
                stats JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE checkpoints
                ADD COLUMN IF NOT EXISTS simulation_id UUID,
                ADD COLUMN IF NOT EXISTS sim_day INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS sim_year INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS population_count INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS population_snapshot JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS world_state JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS tech_state JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS belief_state JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS art_state JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS groups JSONB DEFAULT '[]'::jsonb,
                ADD COLUMN IF NOT EXISTS stats JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE checkpoints
                ALTER COLUMN sim_day SET DEFAULT 0,
                ALTER COLUMN sim_year SET DEFAULT 0,
                ALTER COLUMN population_count SET DEFAULT 0,
                ALTER COLUMN population_snapshot SET DEFAULT '[]'::jsonb,
                ALTER COLUMN world_state SET DEFAULT '{}'::jsonb,
                ALTER COLUMN tech_state SET DEFAULT '[]'::jsonb,
                ALTER COLUMN belief_state SET DEFAULT '[]'::jsonb,
                ALTER COLUMN art_state SET DEFAULT '[]'::jsonb,
                ALTER COLUMN groups SET DEFAULT '[]'::jsonb,
                ALTER COLUMN stats SET DEFAULT '{}'::jsonb,
                ALTER COLUMN created_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            UPDATE checkpoints SET
                sim_day = COALESCE(sim_day, 0),
                sim_year = COALESCE(sim_year, 0),
                population_count = COALESCE(population_count, 0),
                population_snapshot = COALESCE(population_snapshot, '[]'::jsonb),
                world_state = COALESCE(world_state, '{}'::jsonb),
                tech_state = COALESCE(tech_state, '[]'::jsonb),
                belief_state = COALESCE(belief_state, '[]'::jsonb),
                art_state = COALESCE(art_state, '[]'::jsonb),
                groups = COALESCE(groups, '[]'::jsonb),
                stats = COALESCE(stats, '{}'::jsonb),
                created_at = COALESCE(created_at, NOW())
            WHERE sim_day IS NULL OR sim_year IS NULL OR population_count IS NULL
               OR population_snapshot IS NULL OR world_state IS NULL OR tech_state IS NULL
               OR belief_state IS NULL OR art_state IS NULL OR groups IS NULL OR stats IS NULL
               OR created_at IS NULL
            "#,
        )
        .execute(pool)
        .await?;
        // Backs the desktop app's "Yerel" (local) mode: while a local
        // simulation runs, the local sim-server periodically pushes a
        // lightweight snapshot up here (see live_sync_tick/live_sync in
        // routes.rs) so it's watchable from any browser (WatchPage.tsx) and
        // listed on the cloud dashboard's "Canlı Simülasyonlar", without the
        // local device ever exposing raw DB credentials -- the push goes
        // through this server's own authenticated HTTP API, not a direct
        // Postgres connection from the user's machine. One row per
        // (user_id, simulation_id): each push overwrites the last, this is
        // a live view, not a history.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS live_snapshots (
                user_id UUID NOT NULL,
                simulation_id UUID NOT NULL,
                simulation_name TEXT,
                current_day INTEGER NOT NULL DEFAULT 0,
                current_year INTEGER NOT NULL DEFAULT 0,
                population_count INTEGER NOT NULL DEFAULT 0,
                agents_snapshot JSONB NOT NULL DEFAULT '[]'::jsonb,
                stats JSONB NOT NULL DEFAULT '{}'::jsonb,
                groups JSONB NOT NULL DEFAULT '[]'::jsonb,
                is_running BOOLEAN NOT NULL DEFAULT true,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (user_id, simulation_id)
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS god_interventions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                simulation_id UUID NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                type VARCHAR(50) NOT NULL,
                params JSONB NOT NULL,
                affected_individuals INTEGER DEFAULT 0,
                deaths INTEGER DEFAULT 0,
                user_note TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE god_interventions
                ADD COLUMN IF NOT EXISTS simulation_id UUID,
                ADD COLUMN IF NOT EXISTS sim_day INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS sim_year INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS type VARCHAR(50) DEFAULT '',
                ADD COLUMN IF NOT EXISTS params JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS affected_individuals INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS deaths INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS user_note TEXT,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE god_interventions
                ALTER COLUMN sim_day SET DEFAULT 0,
                ALTER COLUMN sim_year SET DEFAULT 0,
                ALTER COLUMN type SET DEFAULT '',
                ALTER COLUMN params SET DEFAULT '{}'::jsonb,
                ALTER COLUMN affected_individuals SET DEFAULT 0,
                ALTER COLUMN deaths SET DEFAULT 0,
                ALTER COLUMN created_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS simulation_events (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                simulation_id UUID NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                event_type VARCHAR(100) NOT NULL,
                description TEXT,
                data JSONB DEFAULT '{}'::jsonb,
                importance INTEGER DEFAULT 1,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE simulation_events
                ADD COLUMN IF NOT EXISTS simulation_id UUID,
                ADD COLUMN IF NOT EXISTS sim_day INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS sim_year INTEGER DEFAULT 0,
                ADD COLUMN IF NOT EXISTS event_type VARCHAR(100) DEFAULT '',
                ADD COLUMN IF NOT EXISTS description TEXT,
                ADD COLUMN IF NOT EXISTS data JSONB DEFAULT '{}'::jsonb,
                ADD COLUMN IF NOT EXISTS importance INTEGER DEFAULT 1,
                ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            ALTER TABLE simulation_events
                ALTER COLUMN sim_day SET DEFAULT 0,
                ALTER COLUMN sim_year SET DEFAULT 0,
                ALTER COLUMN event_type SET DEFAULT '',
                ALTER COLUMN data SET DEFAULT '{}'::jsonb,
                ALTER COLUMN importance SET DEFAULT 1,
                ALTER COLUMN created_at SET DEFAULT NOW()
            "#,
        )
        .execute(pool)
        .await?;
        return Ok(());
    }

    if let Some(pool) = as_sqlite(backend) {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                user_code TEXT UNIQUE,
                username TEXT UNIQUE,
                first_name TEXT NOT NULL DEFAULT '',
                last_name TEXT NOT NULL DEFAULT '',
                tc_no TEXT UNIQUE,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'pending',
                is_approved INTEGER NOT NULL DEFAULT 0,
                is_banned INTEGER NOT NULL DEFAULT 0,
                ban_reason TEXT,
                email_verified INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                wizard_defaults TEXT
            );
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS simulations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                current_day INTEGER NOT NULL DEFAULT 0,
                current_year INTEGER NOT NULL DEFAULT 0,
                population_count INTEGER NOT NULL DEFAULT 0,
                state_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(pool)
        .await?;

        // Give SQLite (desktop/Android "Yerel" mode) the same dedicated
        // speed_multiplier column Postgres already has instead of leaving it
        // to live solely inside state_json. JSON-only meant every periodic
        // tick-loop save (save_tick_progress) -- which writes a fresh
        // state_json every batch but is documented as never supposed to
        // touch speed, see that function's own comment -- clobbered a
        // just-picked speed back to whatever stale value the loop had
        // loaded a batch earlier, since the JSON blob was speed's only home
        // on this backend. That race is between the tick loop's own save
        // and a user's speed-change click, which happens on literally every
        // attempt to change speed, not rarely -- so a real column, exactly
        // like Postgres, is needed rather than accepting the race.
        let has_speed_column = sqlx::query("SELECT 1 FROM pragma_table_info('simulations') WHERE name = 'speed_multiplier'")
            .fetch_optional(pool)
            .await?
            .is_some();
        if !has_speed_column {
            sqlx::query("ALTER TABLE simulations ADD COLUMN speed_multiplier INTEGER NOT NULL DEFAULT 1")
                .execute(pool)
                .await?;
            sqlx::query("UPDATE simulations SET speed_multiplier = COALESCE(json_extract(state_json, '$.speed_multiplier'), 1)")
                .execute(pool)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS individuals (
                id TEXT PRIMARY KEY,
                simulation_id TEXT NOT NULL,
                birth_day INTEGER NOT NULL,
                death_day INTEGER,
                alive INTEGER NOT NULL DEFAULT 1,
                is_dead INTEGER NOT NULL DEFAULT 0,
                parent_1_id TEXT,
                parent_2_id TEXT,
                inbreeding_coeff REAL NOT NULL DEFAULT 0,
                data_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_individuals_simulation_id ON individuals(simulation_id)")
            .execute(pool)
            .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                simulation_id TEXT NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                population_count INTEGER NOT NULL,
                population_snapshot TEXT NOT NULL,
                world_state TEXT NOT NULL,
                tech_state TEXT NOT NULL,
                belief_state TEXT NOT NULL,
                art_state TEXT NOT NULL,
                groups TEXT NOT NULL,
                stats TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS god_interventions (
                id TEXT PRIMARY KEY,
                simulation_id TEXT NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                type TEXT NOT NULL,
                params TEXT NOT NULL,
                affected_individuals INTEGER NOT NULL DEFAULT 0,
                deaths INTEGER NOT NULL DEFAULT 0,
                user_note TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS simulation_events (
                id TEXT PRIMARY KEY,
                simulation_id TEXT NOT NULL,
                sim_day INTEGER NOT NULL,
                sim_year INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT,
                data TEXT NOT NULL DEFAULT '{}',
                importance INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

// A simulation's tick loop lives only in this process's in-memory
// RuntimeManager (runtime.rs) -- it has no persistent record of which
// simulations it was ticking before a restart. Every deploy (or crash)
// kills that loop without ever getting a chance to touch the DB, so
// a fresh boot otherwise inherits rows stuck reading "running" with nothing
// actually advancing them: the dashboard's "Canlı İzle" list keeps listing
// them as live, watching one 404s since no session exists for it, and the
// owner sees it as unexpectedly "stuck" (has to hit start again to get it
// moving). An earlier version of this function marked every such row
// "paused" instead -- simpler, but wrong: a machine restart (deploys,
// crashes) would routinely stop a simulation the owner never asked to pause,
// forcing a manual restart every time. Restarting each one's tick loop
// here instead means an in-progress simulation survives a restart exactly
// like it survived every tick before it -- no click required, and nothing
// left claiming to be live that isn't.
async fn resume_running_simulations(backend: &DbBackend, runtime: &Arc<RuntimeManager>) -> Result<(), sqlx::Error> {
    let running_ids: Vec<String> = if let Some(pool) = as_pg(backend) {
        sqlx::query_scalar("SELECT id::text FROM simulations WHERE status = 'running'")
            .fetch_all(pool)
            .await?
    } else if let Some(pool) = as_sqlite(backend) {
        sqlx::query_scalar("SELECT id FROM simulations WHERE status = 'running'")
            .fetch_all(pool)
            .await?
    } else {
        Vec::new()
    };

    for sim_id in running_ids {
        runtime.start(backend.clone(), sim_id).await;
    }

    Ok(())
}

#[derive(Debug, FromRow)]
pub struct SimulationRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub current_day: i64,
    pub current_year: i64,
    pub population_count: i64,
    pub state_json: Value,
    pub updated_at: String,
    // Both backends have a dedicated column for this now (see migrate()'s
    // SQLite ALTER TABLE and update_simulation_fields) -- row_to_state
    // always prefers it over whatever's embedded in state_json.
    pub speed_multiplier: Option<i32>,
}

#[derive(Debug, FromRow)]
pub struct CheckpointRow {
    pub id: String,
    pub simulation_id: String,
    pub sim_day: i64,
    pub sim_year: i64,
    pub population_count: i64,
    pub population_snapshot: Value,
    pub world_state: Value,
    pub tech_state: Value,
    pub belief_state: Value,
    pub art_state: Value,
    pub groups: Value,
    pub stats: Value,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: String,
    pub user_code: Option<String>,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub tc_no: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub role: Option<String>,
    pub is_approved: i64,
    pub is_banned: i64,
    pub ban_reason: Option<String>,
    pub email_verified: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Just the current_day scalar, without pulling state_json over the wire.
/// Callers that only need it (e.g. the population/deceased list, which now
/// reads individuals from their own table) would otherwise force a full
/// fetch+deserialize of a JSON blob that only grows with total-ever-born.
pub async fn load_current_day(backend: &DbBackend, id: &str) -> Result<Option<i32>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_scalar::<_, i64>("SELECT current_day::int8 FROM simulations WHERE id = $1::uuid")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map(|v| v.map(|d| d as i32));
    }
    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_scalar::<_, i64>("SELECT current_day FROM simulations WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map(|v| v.map(|d| d as i32));
    }
    Ok(None)
}

pub async fn load_simulation(backend: &DbBackend, id: &str) -> Result<Option<SimulationRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, SimulationRow>(
            r#"
            SELECT id::text AS id, name, status, current_day::int8 AS current_day, current_year::int8 AS current_year, population_count::int8 AS population_count, state_json, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS."000Z"') AS updated_at, speed_multiplier
            FROM simulations
            WHERE id = $1::uuid
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, SimulationRow>(
            r#"
            SELECT id, name, status, current_day, current_year, population_count, state_json, strftime('%Y-%m-%dT%H:%M:%S.000Z', updated_at) AS updated_at, speed_multiplier
            FROM simulations
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await;
    }

    Ok(None)
}

// Every field of `SimulationState` (sim-core/src/state.rs) except `individuals`,
// borrowed rather than cloned. Used to build the `state_json` written to the
// `simulations` row on every save -- `individuals` is replaced with an empty
// slice so it is never visited by serde, not just discarded afterwards. That
// matters because `save_tick_progress` runs on every single tick-loop batch,
// and re-serializing every individual ever born (dead ones included -- the
// vec only grows) into `state_json` on every batch was pure write
// amplification: the dedicated `individuals` table (kept current by
// `upsert_individuals`, which already skips unchanged long-dead individuals)
// is the sole durable store for per-individual data; `load_full_state` below
// reconstructs `SimulationState.individuals` from it whenever a caller needs
// the full population in memory.
//
// Keep this struct's field list in sync with `SimulationState`'s -- the
// `state_struct_fields_are_mirrored_in_persistence_dto` scan test at the
// bottom of this file fails loudly if a field is added to one and not the
// other, so a forgotten field here doesn't silently vanish from state_json.
#[derive(serde::Serialize)]
struct StateForPersistence<'a> {
    id: &'a Option<String>,
    name: &'a Option<String>,
    user_id: &'a Option<String>,
    start_latitude: &'a Option<f64>,
    start_longitude: &'a Option<f64>,
    current_day: i32,
    current_year: i32,
    status: &'a Option<String>,
    speed_multiplier: Option<i32>,
    world_state: &'a sim_core::WorldState,
    individuals: &'a [Individual],
    founder_1: &'a Option<Value>,
    founder_2: &'a Option<Value>,
    discovered_techs: &'a [String],
    discovered_beliefs: &'a [String],
    belief_labels: &'a std::collections::HashMap<String, String>,
    civilization_name: &'a Option<String>,
    discovered_arts: &'a [String],
    astronomy_knowledge: &'a [String],
    celestial_observations: &'a [String],
    groups: &'a [Value],
    settlements: &'a [Value],
    pending_births: &'a [Individual],
    events: &'a [Value],
    milestones: &'a [String],
    total_ever_born: i32,
    total_ever_died: i32,
    #[serde(flatten)]
    extra: &'a serde_json::Map<String, Value>,
}

fn state_json_for_db(state: &SimulationState) -> Value {
    serde_json::to_value(StateForPersistence {
        id: &state.id,
        name: &state.name,
        user_id: &state.user_id,
        start_latitude: &state.start_latitude,
        start_longitude: &state.start_longitude,
        current_day: state.current_day,
        current_year: state.current_year,
        status: &state.status,
        speed_multiplier: state.speed_multiplier,
        world_state: &state.world_state,
        individuals: &[],
        founder_1: &state.founder_1,
        founder_2: &state.founder_2,
        discovered_techs: &state.discovered_techs,
        discovered_beliefs: &state.discovered_beliefs,
        belief_labels: &state.belief_labels,
        civilization_name: &state.civilization_name,
        discovered_arts: &state.discovered_arts,
        astronomy_knowledge: &state.astronomy_knowledge,
        celestial_observations: &state.celestial_observations,
        groups: &state.groups,
        settlements: &state.settlements,
        pending_births: &state.pending_births,
        events: &state.events,
        milestones: &state.milestones,
        total_ever_born: state.total_ever_born,
        total_ever_died: state.total_ever_died,
        extra: &state.extra,
    })
    .unwrap_or_else(|_| json!({}))
}

pub async fn save_state(backend: &DbBackend, state: &SimulationState) -> Result<(), sqlx::Error> {
    let id = state
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let name = state.name.clone().unwrap_or_else(|| "Untitled Simulation".to_string());
    let state_json = state_json_for_db(state);
    // Unlike save_existing_state/save_tick_progress, individuals.len() here
    // is correct as-is: save_state only ever runs at simulation creation
    // (new founders) or import (a full uploaded snapshot) -- both callers
    // pass a state whose individuals list is by definition the complete,
    // unbounded set, so there's no total_ever_born to fall back on yet
    // (row_to_state sets it from this very column on the next load).
    let population_count = state.individuals.len() as i64;

    if let Some(pool) = as_pg(backend) {
        sqlx::query(
            r#"
            INSERT INTO simulations (id, name, status, current_day, current_year, speed_multiplier, population_count, state_json, updated_at)
            VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                current_day = excluded.current_day,
                current_year = excluded.current_year,
                speed_multiplier = excluded.speed_multiplier,
                population_count = excluded.population_count,
                state_json = excluded.state_json,
                updated_at = NOW()
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(state.status.clone().unwrap_or_else(|| "running".to_string()))
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(state.speed_multiplier.unwrap_or(1))
        .bind(population_count)
        .bind(state_json)
        .execute(pool)
        .await?;
        return Ok(());
    }

    if let Some(pool) = as_sqlite(backend) {
        sqlx::query(
            r#"
            INSERT INTO simulations (id, name, status, current_day, current_year, speed_multiplier, population_count, state_json, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                current_day = excluded.current_day,
                current_year = excluded.current_year,
                speed_multiplier = excluded.speed_multiplier,
                population_count = excluded.population_count,
                state_json = excluded.state_json,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(state.status.clone().unwrap_or_else(|| "running".to_string()))
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(state.speed_multiplier.unwrap_or(1))
        .bind(population_count)
        .bind(state_json_for_db(state).to_string())
        .execute(pool)
        .await?;
    }

    Ok(())
}

// Like save_state, but a plain UPDATE rather than an upsert -- it never
// creates a new row. Used everywhere except the two places that legitimately
// create a brand-new simulation (create_simulation, import_simulation): the
// tick loop's periodic save, manual start/pause/speed changes, and god-mode
// snapshots all operate on a simulation that's supposed to already exist.
// save_state's INSERT ... ON CONFLICT DO UPDATE could silently resurrect a
// simulation that was deleted moments earlier -- RuntimeManager::terminate's
// docs describe exactly this race (a tick still saving when the row is
// deleted) and close most of the window by awaiting the loop's exit first,
// but that guard only works if this process still has that session in its
// map; a tick loop resumed by a *different* boot (resume_running_simulations
// racing a delete right around a machine restart) has no such guard. This
// closes the gap structurally instead: if the row's gone, the UPDATE
// affects zero rows and the caller can stop, full stop, with no path that
// ever re-inserts it.
pub async fn save_existing_state(backend: &DbBackend, state: &SimulationState) -> Result<bool, sqlx::Error> {
    let Some(id) = state.id.clone() else { return Ok(false) };
    let name = state.name.clone().unwrap_or_else(|| "Untitled Simulation".to_string());
    let state_json = state_json_for_db(state);
    // Not state.individuals.len() -- that's only every-individual-ever-born
    // when the caller loaded via load_full_state (unbounded); the tick loop's
    // bounded load makes it strictly smaller than total-ever-born, and
    // total_ever_born is the one counter both paths keep correct (see
    // SimulationState's own doc comment on the field).
    let population_count = state.total_ever_born as i64;

    if let Some(pool) = as_pg(backend) {
        let affected = sqlx::query(
            r#"
            UPDATE simulations SET
                name = $2,
                status = $3,
                current_day = $4,
                current_year = $5,
                speed_multiplier = $6,
                population_count = $7,
                state_json = $8,
                updated_at = NOW()
            WHERE id = $1::uuid
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(state.status.clone().unwrap_or_else(|| "running".to_string()))
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(state.speed_multiplier.unwrap_or(1))
        .bind(population_count)
        .bind(state_json)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    if let Some(pool) = as_sqlite(backend) {
        let affected = sqlx::query(
            r#"
            UPDATE simulations SET
                name = ?,
                status = ?,
                current_day = ?,
                current_year = ?,
                population_count = ?,
                state_json = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(state.status.clone().unwrap_or_else(|| "running".to_string()))
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(population_count)
        .bind(state_json_for_db(state).to_string())
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    Ok(false)
}

// Same UPDATE as save_existing_state, minus the `status` *and*
// `speed_multiplier` columns -- used only by the tick loop's own periodic
// save (runtime.rs). That loop only ever reaches this save after already
// confirming its in-memory `status` is "running" (see runtime_loop's status
// check), and it only ever *reads* speed_multiplier (to size batch_size/
// target_delay_ms), never changes it -- so writing either back is at best a
// same-value no-op and at worst actively wrong: a pause/terminate/speed
// request that lands in the DB *during* an in-flight batch (now genuinely
// concurrent with it thanks to spawn_blocking freeing the async runtime to
// service that request mid-batch) would get silently clobbered back to the
// stale pre-batch value the moment that batch's save runs -- making a user's
// pause/terminate click, or a freshly-picked speed, appear to do nothing (or
// silently revert a moment later). Both columns are exclusively owned by
// update_simulation_fields (start/pause/terminate/set speed) and this save
// must never step on either.
pub async fn save_tick_progress(backend: &DbBackend, state: &SimulationState) -> Result<bool, sqlx::Error> {
    let Some(id) = state.id.clone() else { return Ok(false) };
    let name = state.name.clone().unwrap_or_else(|| "Untitled Simulation".to_string());
    let state_json = state_json_for_db(state);
    // See save_existing_state's comment -- this runs on every tick-loop
    // batch, precisely the path whose state.individuals is bounded.
    let population_count = state.total_ever_born as i64;

    if let Some(pool) = as_pg(backend) {
        let affected = sqlx::query(
            r#"
            UPDATE simulations SET
                name = $2,
                current_day = $3,
                current_year = $4,
                population_count = $5,
                state_json = $6,
                updated_at = NOW()
            WHERE id = $1::uuid
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(population_count)
        .bind(state_json)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    if let Some(pool) = as_sqlite(backend) {
        let affected = sqlx::query(
            r#"
            UPDATE simulations SET
                name = ?,
                current_day = ?,
                current_year = ?,
                population_count = ?,
                state_json = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(state.current_day)
        .bind(state.current_year)
        .bind(population_count)
        .bind(state_json_for_db(state).to_string())
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    Ok(false)
}

// On Postgres (the shared cloud deployment) this deliberately skips
// load_simulation/row_to_state/save_state -- a full read-modify-write of
// the whole state_json blob. A simulation's tick loop (runtime.rs) is
// concurrently doing its own read-modify-write of that same blob every
// batch, and a request handler racing it with a full rewrite can clobber
// whichever side saves second -- e.g. a "pause" that read the state a
// moment before a tick advanced it, then saves afterward, silently
// rewinds current_day back to what it read. That raciness is what made
// the simulation clock look like it randomly jumped forward and back.
// Targeting only the status/speed_multiplier columns makes this immune to
// that: row_to_state always overrides `status`/`current_day`/`current_year`
// from their dedicated columns rather than trusting state_json's copies
// (see below), so nothing here needs to touch state_json at all.
pub async fn update_simulation_fields(
    backend: &DbBackend,
    id: &str,
    status: Option<&str>,
    speed_multiplier: Option<i32>,
) -> Result<bool, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let affected = sqlx::query(
            "UPDATE simulations SET \
                status = COALESCE($2, status), \
                speed_multiplier = COALESCE($3, speed_multiplier), \
                updated_at = NOW() \
             WHERE id = $1::uuid",
        )
        .bind(id)
        .bind(status)
        .bind(speed_multiplier)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    if let Some(pool) = as_sqlite(backend) {
        // Both status and speed_multiplier are dedicated columns here now
        // (see migrate()'s SQLite ALTER TABLE), so this can take the same
        // race-free path as Postgres instead of a full read-modify-write of
        // state_json -- which used to be how a speed change got applied on
        // this backend, and which the tick loop's own periodic state_json
        // save (save_tick_progress) would silently clobber back to a stale
        // value moments later, since state_json was speed's only home.
        let affected = sqlx::query(
            "UPDATE simulations SET \
                status = COALESCE(?, status), \
                speed_multiplier = COALESCE(?, speed_multiplier), \
                updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(status)
        .bind(speed_multiplier)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
        return Ok(affected > 0);
    }

    Ok(false)
}

pub async fn delete_simulation(backend: &DbBackend, id: &str) -> Result<bool, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let mut tx = pool.begin().await?;
        let _ = sqlx::query("DELETE FROM individuals WHERE simulation_id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let _ = sqlx::query("DELETE FROM checkpoints WHERE simulation_id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // A legacy (pre-migration) database may have FK constraints from
        // these two tables back to simulations.id that our own CREATE TABLE
        // doesn't declare -- deleting the simulation without clearing these
        // first can fail the transaction with a foreign-key violation.
        let _ = sqlx::query("DELETE FROM god_interventions WHERE simulation_id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let _ = sqlx::query("DELETE FROM simulation_events WHERE simulation_id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // A live_snapshots row for a locally-run simulation never has a
        // matching simulations row at all (see live_sync in routes.rs) --
        // this is the only cleanup path such a row has, so it must run
        // even when the DELETE below affects zero simulations rows.
        let _ = sqlx::query("DELETE FROM live_snapshots WHERE simulation_id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let affected = sqlx::query("DELETE FROM simulations WHERE id = $1::uuid")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        return Ok(affected > 0);
    }

    if let Some(pool) = as_sqlite(backend) {
        let mut tx = pool.begin().await?;
        let _ = sqlx::query("DELETE FROM individuals WHERE simulation_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let _ = sqlx::query("DELETE FROM checkpoints WHERE simulation_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let affected = sqlx::query("DELETE FROM simulations WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        return Ok(affected > 0);
    }

    Ok(false)
}

/// Upserts one user's live view of one (necessarily local) simulation --
/// see live_snapshots' own doc comment in migrate(). No-op on the SQLite
/// backend: a local sim-server only ever sends these, it never receives
/// them (there's no local counterpart table to write into).
#[allow(clippy::too_many_arguments)]
pub async fn upsert_live_snapshot(
    backend: &DbBackend,
    user_id: &str,
    simulation_id: &str,
    simulation_name: &str,
    current_day: i32,
    current_year: i32,
    population_count: i32,
    agents_snapshot: &Value,
    stats: &Value,
    groups: &Value,
    is_running: bool,
) -> Result<(), sqlx::Error> {
    let Some(pool) = as_pg(backend) else { return Ok(()) };
    sqlx::query(
        r#"
        INSERT INTO live_snapshots
            (user_id, simulation_id, simulation_name, current_day, current_year,
             population_count, agents_snapshot, stats, groups, is_running, updated_at)
        VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        ON CONFLICT (user_id, simulation_id) DO UPDATE SET
            simulation_name = excluded.simulation_name,
            current_day = excluded.current_day,
            current_year = excluded.current_year,
            population_count = excluded.population_count,
            agents_snapshot = excluded.agents_snapshot,
            stats = excluded.stats,
            groups = excluded.groups,
            is_running = excluded.is_running,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(simulation_id)
    .bind(simulation_name)
    .bind(current_day)
    .bind(current_year)
    .bind(population_count)
    .bind(agents_snapshot)
    .bind(stats)
    .bind(groups)
    .bind(is_running)
    .execute(pool)
    .await?;
    Ok(())
}

/// Full snapshot for one simulation (WatchPage.tsx's data source), across
/// any owner -- matches list_live's existing no-ownership-filter precedent
/// for "any currently-running row is watchable by ID". `None` on the
/// SQLite backend (a local server has nowhere to read one back from).
pub async fn load_live_snapshot(backend: &DbBackend, simulation_id: &str) -> Result<Option<Value>, sqlx::Error> {
    let Some(pool) = as_pg(backend) else { return Ok(None) };
    let row = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT jsonb_build_object(
            'simulation_id', simulation_id::text,
            'simulation_name', simulation_name,
            'current_day', current_day,
            'current_year', current_year,
            'population_count', population_count,
            'agents_snapshot', agents_snapshot,
            'stats', stats,
            'groups', groups,
            'is_running', is_running,
            'updated_at', updated_at::text
        )
        FROM live_snapshots
        WHERE simulation_id = $1::uuid
        "#,
    )
    .bind(simulation_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lightweight rows for the cloud dashboard's "Canlı Simülasyonlar" list --
/// merged with list_live's own same-backend "running" rows in routes.rs.
/// Excludes anything not pushed to in the last 2 minutes (a local app closed
/// without a final push -- e.g. the window was killed rather than closed
/// normally -- would otherwise linger here forever, since nothing else ever
/// deletes a stale row).
pub async fn list_live_snapshots(backend: &DbBackend) -> Result<Vec<Value>, sqlx::Error> {
    let Some(pool) = as_pg(backend) else { return Ok(Vec::new()) };
    let rows = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT jsonb_build_object(
            'simulation_id', simulation_id::text,
            'simulation_name', simulation_name,
            'current_day', current_day,
            'current_year', current_year,
            'population_count', population_count,
            'updated_at', updated_at::text
        )
        FROM live_snapshots
        WHERE is_running = true AND updated_at > NOW() - INTERVAL '2 minutes'
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Ported from the old Node backend's GET /:id/db-status (never migrated to
/// Rust at all -- the route didn't exist here, so the client's Performance
/// panel "DATABASE STATUS" section always 404'd, silently, on every
/// platform). Node's version queried per-domain tables (technologies,
/// belief_systems, language_records, individual_conversations,
/// publications) that only ever existed in its own older schema; Rust
/// consolidated all of that into fields on the state JSON blob instead, so
/// counts for those come from the caller's already-loaded SimulationState
/// rather than a query here.
///
/// `events` used to be one more query here too (`SELECT COUNT(*) FROM
/// simulation_events`), always returning 0 for every simulation ever
/// created: that table is only ever read from (this count) and cleaned up
/// (admin::cleanup_simulation_data, terminate_simulation) -- nothing
/// anywhere ever runs an INSERT into it, since events live in the state
/// blob's own `events` array instead (get_events already reads from there,
/// not this table). Removed the dead query; the caller now reports
/// `sim.events.len()` from its own already-loaded state, exactly like
/// technologies/beliefs/groups below already did.
#[derive(Default)]
pub struct DbStatusCounts {
    pub individuals_total: i64,
    pub individuals_alive: i64,
    pub checkpoints: i64,
    pub db_size_bytes: Option<i64>,
}

pub async fn db_status_counts(backend: &DbBackend, sim_id: &str) -> Result<DbStatusCounts, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let (individuals_total, individuals_alive) =
            sqlx::query_as::<_, (i64, i64)>("SELECT COUNT(*), COUNT(*) FILTER (WHERE alive = true) FROM individuals WHERE simulation_id = $1::uuid")
                .bind(sim_id)
                .fetch_one(pool)
                .await?;
        let checkpoints = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM checkpoints WHERE simulation_id = $1::uuid").bind(sim_id).fetch_one(pool).await?;
        // PGlite (the old Node local mode's embedded Postgres) didn't
        // support pg_database_size -- kept as best-effort here too, though
        // a real managed Postgres instance always supports it.
        let db_size_bytes = sqlx::query_scalar::<_, i64>("SELECT pg_database_size(current_database())").fetch_one(pool).await.ok();
        return Ok(DbStatusCounts { individuals_total, individuals_alive, checkpoints, db_size_bytes });
    }

    if let Some(pool) = as_sqlite(backend) {
        let individuals_total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM individuals WHERE simulation_id = ?").bind(sim_id).fetch_one(pool).await?;
        let individuals_alive = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM individuals WHERE simulation_id = ? AND alive = 1").bind(sim_id).fetch_one(pool).await?;
        let checkpoints = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM checkpoints WHERE simulation_id = ?").bind(sim_id).fetch_one(pool).await?;
        // SQLite has no server-wide "database size" the way Postgres does
        // (each simulation shares one on-disk file with every other local
        // simulation) -- left None, which the client already renders as no
        // size badge rather than a misleading 0.
        return Ok(DbStatusCounts { individuals_total, individuals_alive, checkpoints, db_size_bytes: None });
    }

    Ok(DbStatusCounts::default())
}

pub async fn system_counts(backend: &DbBackend) -> Result<(i64, i64), sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let sims = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM simulations WHERE status = 'running'")
            .fetch_one(pool)
            .await?;
        let pop = sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(population_count), 0) FROM simulations")
            .fetch_one(pool)
            .await?;
        return Ok((sims, pop));
    }

    if let Some(pool) = as_sqlite(backend) {
        let sims = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM simulations WHERE status = 'running'")
            .fetch_one(pool)
            .await?;
        let pop = sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(population_count), 0) FROM simulations")
            .fetch_one(pool)
            .await?;
        return Ok((sims, pop));
    }

    Ok((0, 0))
}

pub async fn list_simulations(backend: &DbBackend) -> Result<Vec<SimulationRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, SimulationRow>(
            r#"
            SELECT id::text AS id, name, status, current_day::int8 AS current_day, current_year::int8 AS current_year, population_count::int8 AS population_count, state_json, to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS."000Z"') AS updated_at, speed_multiplier
            FROM simulations
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, SimulationRow>(
            r#"
            SELECT id, name, status, current_day, current_year, population_count, state_json, strftime('%Y-%m-%dT%H:%M:%S.000Z', updated_at) AS updated_at, speed_multiplier
            FROM simulations
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(pool)
        .await;
    }

    Ok(vec![])
}

pub fn row_to_state(row: &SimulationRow) -> SimulationState {
    let mut state: SimulationState = serde_json::from_value(row.state_json.clone()).unwrap_or_default();
    if state.id.is_none() {
        state.id = Some(row.id.clone());
    }
    if state.name.is_none() {
        state.name = Some(row.name.clone());
    }
    state.status = Some(row.status.clone());
    state.current_day = row.current_day as i32;
    state.current_year = row.current_year as i32;
    // Both backends hold this in a dedicated column that
    // update_simulation_fields can update without touching state_json (see
    // there for why); prefer it over the JSON copy so a running tick loop
    // picks up a speed change on its very next iteration instead of only
    // ever seeing whatever value state_json had at the start of the batch.
    if let Some(speed) = row.speed_multiplier {
        state.speed_multiplier = Some(speed);
    }
    // Dedicated-column-is-truth, same reasoning as speed_multiplier above:
    // state_json's own total_ever_born can be stale (or, for a simulation
    // saved before this field existed, simply absent/0), but this column is
    // written by save_state/save_existing_state/save_tick_progress on every
    // save, so it's always current.
    state.total_ever_born = row.population_count as i32;
    state.extra.insert("_population_count".to_string(), json!(row.population_count));
    state
}

/// Like `row_to_state`, but also reconstructs the full `individuals` vec from
/// the dedicated `individuals` table -- state_json no longer carries them (see
/// `state_json_for_db`), so this is the way to get a `SimulationState` with
/// *every* individual ever born in memory. Used by the cold/infrequent paths
/// that genuinely need full history (ws.rs's once-a-second broadcast,
/// routes.rs's report/export/event endpoints, god.rs's interventions). The
/// tick loop itself (runtime.rs) uses `load_bounded_tick_state_no_genealogy`
/// plus an incrementally-cached `load_genealogy_index` instead -- loading
/// everyone ever born, every single batch, doesn't scale with total-ever-born
/// the way this function's callers can afford.
pub async fn load_full_state(backend: &DbBackend, id: &str) -> Result<Option<SimulationState>, sqlx::Error> {
    // The simulations row and the individuals table are independent reads --
    // running them concurrently (two connections out of the same pool)
    // instead of sequentially roughly halves this function's DB-round-trip
    // latency, which matters here specifically: ws.rs awaits this once a
    // second per open connection, and its own doc comment on why that
    // matters (smooth day-counter progress) applies here too.
    let (row_result, payloads_result) = tokio::join!(load_simulation(backend, id), load_individual_payloads(backend, id, None, None));
    let Some(row) = row_result? else { return Ok(None) };
    let mut state = row_to_state(&row);
    state.individuals = payloads_result?.into_iter().filter_map(|p| serde_json::from_value(p).ok()).collect();
    // Unlike the bounded loader, every individual this simulation has ever
    // had is already present in `state.individuals` here, so total_ever_died
    // (see its own doc comment on state.rs) can be derived directly instead
    // of needing a separate COUNT query.
    state.total_ever_died = state.individuals.iter().filter(|i| i.is_dead || !i.alive).count() as i32;
    Ok(Some(state))
}

/// The `individuals` table's parent_1_id/parent_2_id/inbreeding_coeff for
/// *everyone ever born* in a simulation (or, with `since_birth_day`, only
/// those born on or after a given day) -- three small dedicated columns, no
/// JSONB payload -- assembled into the index `compute_inbreeding_coefficient`'s
/// ancestor traversal needs (see `biology::genome::GenealogyIndex`'s own doc
/// comment for why it can't just use whatever's in `state.individuals` once
/// that's bounded).
///
/// `since_birth_day: None` loads everyone (the historical, always-correct
/// behavior `load_full_state` and tests rely on). `Some(day)` loads
/// only the delta born on or after `day` -- entries are immutable once
/// written (a birth's parent ids and inbreeding_coeff never change), so a
/// caller merging deltas into a persisted cache (see runtime.rs's
/// `genealogy_cache`) is always correct, never stale. This is what makes an
/// unbounded "refetch everyone ever born" query, which used to run on every
/// single tick-loop batch and grew slower forever as a simulation aged, safe
/// to replace with "fetch only what's new since last time".
pub async fn load_genealogy_index(backend: &DbBackend, simulation_id: &str, since_birth_day: Option<i32>) -> Result<sim_core::GenealogyIndex, sqlx::Error> {
    let mut index = sim_core::GenealogyIndex::new();
    if let Some(pool) = as_pg(backend) {
        let rows: Vec<(Uuid, Option<Uuid>, Option<Uuid>, f64)> = match since_birth_day {
            Some(cutoff) => {
                sqlx::query_as("SELECT id, parent_1_id, parent_2_id, inbreeding_coeff FROM individuals WHERE simulation_id = $1::uuid AND birth_day >= $2")
                    .bind(simulation_id)
                    .bind(cutoff)
                    .fetch_all(pool)
                    .await?
            }
            None => {
                sqlx::query_as("SELECT id, parent_1_id, parent_2_id, inbreeding_coeff FROM individuals WHERE simulation_id = $1::uuid")
                    .bind(simulation_id)
                    .fetch_all(pool)
                    .await?
            }
        };
        for (id, parent_1, parent_2, inbreeding_coeff) in rows {
            index.insert(
                id.to_string(),
                sim_core::GenealogyEntry { parent_1_id: parent_1.map(|u| u.to_string()), parent_2_id: parent_2.map(|u| u.to_string()), inbreeding_coeff },
            );
        }
        return Ok(index);
    }

    if let Some(pool) = as_sqlite(backend) {
        let rows: Vec<(String, Option<String>, Option<String>, f64)> = match since_birth_day {
            Some(cutoff) => {
                sqlx::query_as("SELECT id, parent_1_id, parent_2_id, inbreeding_coeff FROM individuals WHERE simulation_id = ? AND birth_day >= ?")
                    .bind(simulation_id)
                    .bind(cutoff)
                    .fetch_all(pool)
                    .await?
            }
            None => {
                sqlx::query_as("SELECT id, parent_1_id, parent_2_id, inbreeding_coeff FROM individuals WHERE simulation_id = ?")
                    .bind(simulation_id)
                    .fetch_all(pool)
                    .await?
            }
        };
        for (id, parent_1_id, parent_2_id, inbreeding_coeff) in rows {
            index.insert(id, sim_core::GenealogyEntry { parent_1_id, parent_2_id, inbreeding_coeff });
        }
        return Ok(index);
    }

    Ok(index)
}

/// The tick loop's per-batch load (runtime.rs): like `load_full_state`, but
/// `individuals` only carries the alive-plus-recently-dead window instead of
/// everyone ever born. `state.genealogy` is left empty here -- runtime.rs
/// maintains its own incrementally-updated genealogy cache across iterations
/// instead (see `load_genealogy_index`'s `since_birth_day`), rather than
/// re-fetching everyone ever born via `load_genealogy_index(..., None)` on
/// every single batch. The cutoff mirrors `strip_dead_individual_if_due`'s
/// own grace window exactly: tick.rs already establishes that nothing reads
/// a dead individual's data once they're past `DEAD_FIELD_STRIP_GRACE_DAYS`,
/// so dropping them from the active in-memory set at that same point loses
/// nothing the tick loop itself would ever look at again. A missing
/// death_day (not yet backfilled -- see the migration in `migrate()`, or a
/// death-day bookkeeping gap) is treated as "keep it in memory", same
/// fail-safe direction `upsert_individuals`'s own grace window already takes.
pub async fn load_bounded_tick_state_no_genealogy(backend: &DbBackend, id: &str) -> Result<Option<SimulationState>, sqlx::Error> {
    let Some(row) = load_simulation(backend, id).await? else { return Ok(None) };
    let mut state = row_to_state(&row);
    let cutoff_day = state.current_day - sim_core::DEAD_FIELD_STRIP_GRACE_DAYS;
    let (payloads, dead_count) = tokio::join!(load_individual_payloads_bounded(backend, id, cutoff_day), count_dead_individuals(backend, id));
    state.individuals = payloads?.into_iter().filter_map(|p| serde_json::from_value(p).ok()).collect();
    // total_ever_died must reflect *every* death this simulation has ever
    // had, not just the ones still within the strip grace window above --
    // see its own doc comment on state.rs. A cheap COUNT query alongside
    // the bounded payload fetch keeps this correct without loading every
    // dead individual's full JSON payload just to count them.
    state.total_ever_died = dead_count?.max(state.total_ever_died as i64) as i32;
    Ok(Some(state))
}

/// The true, unbounded count of individuals this simulation has ever marked
/// dead/not-alive -- used to seed `total_ever_died` (see its own doc
/// comment on state.rs) from a state-loading path that otherwise only ever
/// sees a bounded slice of `individuals`. Mirrors `load_individual_payloads`'s
/// own `(alive = false OR is_dead = true)` predicate.
async fn count_dead_individuals(backend: &DbBackend, simulation_id: &str) -> Result<i64, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM individuals WHERE simulation_id = $1::uuid AND (alive = false OR is_dead = true)")
            .bind(simulation_id)
            .fetch_one(pool)
            .await;
    }
    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM individuals WHERE simulation_id = ? AND (alive = 0 OR is_dead = 1)")
            .bind(simulation_id)
            .fetch_one(pool)
            .await;
    }
    Ok(0)
}

/// Bounded counterpart to `load_individual_payloads`: excludes only rows
/// that are unambiguously past the strip grace window (dead, with a
/// `death_day` dedicated column set, and `current_day - death_day >=
/// cutoff`'s complement) -- see `load_bounded_tick_state_no_genealogy` for
/// why that's the correct line to cut at.
///
/// Returns the raw `data_json` column value, not an `Individual` -- the
/// genome inside is still in its slim on-the-wire form (see
/// `serialize_slim_genome`/`deserialize_hydrated_genome` in sim-core's
/// state.rs) until something actually deserializes it into `Individual`,
/// which is what runs `hydrate_genome_metadata` and fills back in
/// chromosome/expression_type/hemizygous status. The current sole caller
/// does exactly that (`serde_json::from_value::<Individual>` right below).
/// Do not serve this `Value` to a client directly without going through
/// `Individual` first -- it would ship incomplete genome metadata.
async fn load_individual_payloads_bounded(backend: &DbBackend, simulation_id: &str, cutoff_day: i32) -> Result<Vec<Value>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT data_json FROM individuals WHERE simulation_id = $1::uuid \
             AND NOT (is_dead = true AND death_day IS NOT NULL AND death_day < $2) \
             ORDER BY created_at ASC",
        )
        .bind(simulation_id)
        .bind(cutoff_day)
        .fetch_all(pool)
        .await?;
        return Ok(rows);
    }

    if let Some(pool) = as_sqlite(backend) {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT data_json FROM individuals WHERE simulation_id = ? \
             AND NOT (is_dead = 1 AND death_day IS NOT NULL AND death_day < ?) \
             ORDER BY created_at ASC",
        )
        .bind(simulation_id)
        .bind(cutoff_day)
        .fetch_all(pool)
        .await?;
        return Ok(rows.into_iter().filter_map(|json| serde_json::from_str::<Value>(&json).ok()).collect());
    }

    Ok(vec![])
}

// Multi-row upserts (one round trip per chunk instead of one per individual)
// -- with a population in the hundreds this used to mean hundreds of
// sequential network round trips to the DB on every single tick batch,
// which dominated the actual wall-clock time far more than the intended
// speed-multiplier delay did. That's what made higher speed settings (e.g.
// 20x) barely faster in practice than 1x once population grew.
const PG_UPSERT_CHUNK: usize = 200;
const SQLITE_UPSERT_CHUNK: usize = 100; // SQLite's default bound-parameter limit is tighter than Postgres's.

// Once dead, an individual is never touched again by the tick loop (every
// pass that mutates individuals skips `is_dead`/`!alive` ones), so their
// data_json is fully static a few days after death -- give death-day
// bookkeeping (which isn't perfectly precise at every call site, e.g. a
// direct god-mode kill) a grace window rather than requiring an exact
// day match.
//
// This MUST be >= runtime.rs's MAX_BATCH_SIZE. `state.current_day` here is
// the day *after* a whole batch (up to MAX_BATCH_SIZE days) has already run
// -- an individual who dies on the batch's first day is only ever
// considered for upsert once, at the end of that same batch, by which time
// `state.current_day - death_day` can already be as large as
// MAX_BATCH_SIZE. A grace window smaller than that (this used to be a flat
// 3) meant most deaths were silently skipped by every future upsert and
// never actually reached the `individuals` table as `is_dead = true` --
// undercounting `stats.deaths` and `population?alive=false` against the
// event log's own (correctly persisted, via a separate path) death count.
// +7 keeps one extra real retry window beyond the guaranteed same-batch
// upsert, matching this constant's original "a few days of grace" intent.
const DEAD_UPSERT_GRACE_DAYS: i32 = crate::runtime::MAX_BATCH_SIZE as i32 + 7;

// Precomputed, backend-agnostic form of one individual's upsert row --
// everything push_pg_upsert/the SQLite loop below need, minus the actual
// serde_json::to_value(individual) call, so that call can happen off the
// async runtime (see upsert_individuals's own comment on why).
#[derive(Clone)]
struct PreparedUpsertRow {
    id: String,
    birth_day: i32,
    death_day: Option<i32>,
    alive: bool,
    is_dead: bool,
    parent_1_id: Option<String>,
    parent_2_id: Option<String>,
    inbreeding_coeff: f64,
    payload: Value,
}

// Split into a pure signal-matching core (unit-testable without a real
// sqlx::Error, which is awkward to construct synthetically) and a thin
// wrapper that pulls the code/message out of whatever the driver returned.
// Postgres reports a clean SQLSTATE (23503); SQLite's error code for this
// isn't as cleanly exposed through sqlx, so message-text matching is the
// portable fallback for both.
fn is_foreign_key_violation_signal(code: Option<&str>, message: &str) -> bool {
    code == Some("23503") || message.to_lowercase().contains("foreign key")
}

/// Recognizes a foreign-key violation regardless of backend -- the one error
/// upsert_individuals has a real, safe fallback for (see
/// sanitize_dangling_parents below). Any other error is a genuine failure
/// the caller should still see and log as before.
fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    match err.as_database_error() {
        Some(db_err) => is_foreign_key_violation_signal(db_err.code().as_deref(), db_err.message()),
        None => false,
    }
}

/// Last-resort recovery from the one scenario upsert_individuals' transitive
/// parent expansion can't fix: a referenced parent that's not resolvable
/// anywhere in this process's memory (e.g. already-existing corruption from
/// before that fix, or a process restart during a pause that forgot an
/// ancestor before they were ever persisted). Nulls out any
/// parent_1_id/parent_2_id that isn't part of *this same batch* --
/// guaranteed safe against the FK, at the cost of losing that one genealogy
/// edge -- rather than let the whole batch (everyone else's perfectly fine
/// data included) fail to save, forever, every batch.
fn sanitize_dangling_parents(rows: &[PreparedUpsertRow], to_upsert_ids: &HashSet<String>) -> Vec<PreparedUpsertRow> {
    rows.iter()
        .cloned()
        .map(|mut row| {
            row.parent_1_id = row.parent_1_id.filter(|p| to_upsert_ids.contains(p));
            row.parent_2_id = row.parent_2_id.filter(|p| to_upsert_ids.contains(p));
            row
        })
        .collect()
}

fn push_pg_upsert<'a>(qb: &mut QueryBuilder<'a, sqlx::Postgres>, chunk: &'a [PreparedUpsertRow], simulation_id: Uuid) {
    qb.push_values(chunk, |mut b, row| {
        let ind_id = Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::new_v4());
        // These three mirror data_json's own parent_1_id/parent_2_id/
        // inbreeding_coeff -- kept as dedicated columns too so
        // load_genealogy_index (db.rs) can read everyone-ever-born's
        // ancestry without parsing a single JSONB blob per row.
        let parent_1: Option<Uuid> = row.parent_1_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        let parent_2: Option<Uuid> = row.parent_2_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        b.push_bind(ind_id)
            .push_bind(simulation_id)
            .push_bind(row.birth_day)
            .push_bind(row.death_day)
            .push_bind(row.alive)
            .push_bind(row.is_dead)
            .push_bind(parent_1)
            .push_bind(parent_2)
            .push_bind(row.inbreeding_coeff)
            .push_bind(row.payload.clone());
    });
    qb.push(
        " ON CONFLICT (id) DO UPDATE SET \
            simulation_id = excluded.simulation_id, \
            birth_day = excluded.birth_day, \
            death_day = excluded.death_day, \
            alive = excluded.alive, \
            is_dead = excluded.is_dead, \
            parent_1_id = excluded.parent_1_id, \
            parent_2_id = excluded.parent_2_id, \
            inbreeding_coeff = excluded.inbreeding_coeff, \
            data_json = excluded.data_json, \
            updated_at = NOW()",
    );
}

/// `include_ancestors` controls the expensive transitive-parent walk
/// documented below: pass `true` only when its guarantee is actually about
/// to be relied on (the one-off call sites -- import, a single manual tick,
/// a God Mode intervention, tests -- and runtime.rs's recurring batch loop
/// right when upload is paused or hasn't yet done its one post-(re)start
/// resync). Every other recurring-batch call already has that guarantee
/// satisfied by an earlier `true` pass -- see runtime.rs's `full_resync_needed`
/// -- so `false` there skips the walk entirely and upserts only the cheap,
/// bounded `eligible` set (alive + recently-dead), which is the overwhelming
/// majority of this function's real-world cost on a long-running simulation.
pub async fn upsert_individuals(backend: &DbBackend, state: &SimulationState, include_ancestors: bool) -> Result<(), sqlx::Error> {
    // Re-upserting every dead individual's full data_json on every single
    // batch forever, long after they actually stopped changing, is pure
    // write amplification that only grows as a simulation ages
    // (total-ever-born only ever grows). A missing death_day is treated as
    // "always upsert" -- safer than silently never persisting someone whose
    // death wasn't dated.
    //
    // DEAD_UPSERT_GRACE_DAYS (107 days) was chosen to always exceed a single
    // batch's own span (MAX_BATCH_SIZE, 100 days), so this filter used to be
    // safe to apply on its own: an individual was guaranteed at least one
    // upsert opportunity while eligible, since upserts ran every batch. That
    // guarantee breaks once upload can be *paused* for far longer than one
    // batch (see runtime.rs's should_flush_upload) -- an individual can now
    // be born, die, and age out of this window entirely between two
    // upserts, without ever being written to the DB even once. If they're
    // someone else's parent, that someone's parent_1_id/parent_2_id would
    // then point at a row that was never inserted, violating the
    // individuals table's own foreign keys and failing this whole upsert
    // forever, every batch, since nothing here ever un-sticks it on its own.
    //
    // The fix (when `include_ancestors` is true): transitively pull in every
    // referenced parent regardless of their own grace-day eligibility.
    // `state.individuals` never drops anyone once loaded (see tick.rs's
    // strip_dead_individual_if_due, which only clears their heavy fields
    // after they're long dead, never removes the row), so any ancestor
    // still resolvable in this process's memory -- which covers every case
    // where upload has been paused but the process itself hasn't restarted
    // since -- gets included here. Skipped when `include_ancestors` is
    // false: every ancestor still referenced by someone eligible was
    // already correctly persisted by an earlier `true` pass, so walking the
    // whole lineage back to the founders again here would just re-write
    // rows that haven't changed since -- on a simulation with many
    // generations, that's most of total-ever-born, every single batch.
    let eligible: Vec<&Individual> = state
        .individuals
        .iter()
        .filter(|i| i.alive || i.death_day.is_none_or(|d| state.current_day - d <= DEAD_UPSERT_GRACE_DAYS))
        .collect();
    if eligible.is_empty() {
        return Ok(());
    }
    let to_upsert: Vec<Individual> = if include_ancestors {
        let by_id: HashMap<&str, &Individual> = state.individuals.iter().map(|i| (i.id.as_str(), i)).collect();
        let mut to_upsert_ids: HashSet<String> = HashSet::new();
        let mut to_upsert: Vec<Individual> = Vec::new();
        let mut queue: Vec<&Individual> = eligible;
        let mut idx = 0;
        while idx < queue.len() {
            let individual = queue[idx];
            idx += 1;
            if !to_upsert_ids.insert(individual.id.clone()) {
                continue;
            }
            for parent_id in [individual.parent_1_id.as_deref(), individual.parent_2_id.as_deref()].into_iter().flatten() {
                if !to_upsert_ids.contains(parent_id) {
                    if let Some(parent) = by_id.get(parent_id) {
                        queue.push(parent);
                    }
                }
            }
            to_upsert.push(individual.clone());
        }
        to_upsert
    } else {
        eligible.into_iter().cloned().collect()
    };
    if to_upsert.is_empty() {
        return Ok(());
    }
    let simulation_id_str = state.id.clone().unwrap_or_default();
    let simulation_id = Uuid::parse_str(&simulation_id_str).unwrap_or_else(|_| Uuid::nil());

    // serde_json::to_value(individual) walks the individual's entire genome/
    // epigenome/mind/psychology/language.vocabulary/inventory/memory/etc --
    // real CPU work that scales with population and with how much state
    // each individual has accumulated over their life. Doing that inline
    // here, on whichever tokio async worker thread happens to be running
    // this task, has no .await yield point and blocks that worker for the
    // whole batch -- exactly the problem runtime.rs's own spawn_blocking
    // comment on advance_one_day already describes (this process only has a
    // couple of these worker threads; one stuck here starves everything
    // else scheduled on it, including /api/health and this same
    // simulation's own WebSocket ticks). Moving the serialization into
    // spawn_blocking frees the async runtime immediately; running it
    // through rayon's par_iter inside that blocking closure spreads the
    // work itself across every available core (see main.rs's
    // configure_rayon_thread_pool) instead of doing 100s of individuals
    // one at a time.
    let rows: Vec<PreparedUpsertRow> = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        to_upsert
            .par_iter()
            .map(|individual| PreparedUpsertRow {
                id: individual.id.clone(),
                birth_day: individual.birth_day,
                death_day: individual.death_day,
                alive: individual.alive,
                is_dead: individual.is_dead,
                parent_1_id: individual.parent_1_id.clone(),
                parent_2_id: individual.parent_2_id.clone(),
                inbreeding_coeff: individual.inbreeding_coeff.unwrap_or(0.0),
                payload: serde_json::to_value(individual).unwrap_or_else(|_| serde_json::json!({})),
            })
            .collect()
    })
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return Ok(());
    }
    // Used only by sanitize_dangling_parents' foreign-key-violation retry
    // path below (both backends) -- rebuilt from `rows` rather than kept
    // from the ancestor-walk branch above, since that branch-local set
    // isn't in scope when `include_ancestors` was false.
    let to_upsert_ids: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();

    if let Some(pool) = as_pg(backend) {
        // The common case (population under PG_UPSERT_CHUNK, i.e. almost
        // always) needs only one INSERT statement, which is already atomic
        // on its own -- wrapping it in an explicit transaction would add two
        // pure-overhead network round trips (BEGIN + COMMIT) to every batch
        // for no correctness benefit. Only reach for a real transaction when
        // more than one statement actually needs to commit together.
        if rows.len() <= PG_UPSERT_CHUNK {
            let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                "INSERT INTO individuals (id, simulation_id, birth_day, death_day, alive, is_dead, parent_1_id, parent_2_id, inbreeding_coeff, data_json) ",
            );
            push_pg_upsert(&mut qb, &rows, simulation_id);
            if let Err(err) = qb.build().execute(pool).await {
                if !is_foreign_key_violation(&err) {
                    return Err(err);
                }
                tracing::warn!(simulation_id = %simulation_id_str, "upsert_individuals hit a foreign key violation, retrying with unresolved parent references nulled out");
                let sanitized = sanitize_dangling_parents(&rows, &to_upsert_ids);
                let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                    "INSERT INTO individuals (id, simulation_id, birth_day, death_day, alive, is_dead, parent_1_id, parent_2_id, inbreeding_coeff, data_json) ",
                );
                push_pg_upsert(&mut qb, &sanitized, simulation_id);
                qb.build().execute(pool).await?;
            }
            return Ok(());
        }

        let mut tx = pool.begin().await?;
        let mut fk_violation = false;
        for chunk in rows.chunks(PG_UPSERT_CHUNK) {
            let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                "INSERT INTO individuals (id, simulation_id, birth_day, death_day, alive, is_dead, parent_1_id, parent_2_id, inbreeding_coeff, data_json) ",
            );
            push_pg_upsert(&mut qb, chunk, simulation_id);
            if let Err(err) = qb.build().execute(&mut *tx).await {
                if !is_foreign_key_violation(&err) {
                    return Err(err);
                }
                fk_violation = true;
                break;
            }
        }
        if fk_violation {
            // Postgres aborts the whole transaction after any statement
            // error -- roll back and retry the entire batch from scratch
            // with sanitized parent references rather than try to resume a
            // poisoned transaction.
            tx.rollback().await?;
            tracing::warn!(simulation_id = %simulation_id_str, "upsert_individuals hit a foreign key violation, retrying with unresolved parent references nulled out");
            let sanitized = sanitize_dangling_parents(&rows, &to_upsert_ids);
            let mut retry_tx = pool.begin().await?;
            for chunk in sanitized.chunks(PG_UPSERT_CHUNK) {
                let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                    "INSERT INTO individuals (id, simulation_id, birth_day, death_day, alive, is_dead, parent_1_id, parent_2_id, inbreeding_coeff, data_json) ",
                );
                push_pg_upsert(&mut qb, chunk, simulation_id);
                qb.build().execute(&mut *retry_tx).await?;
            }
            retry_tx.commit().await?;
        } else {
            tx.commit().await?;
        }
        return Ok(());
    }

    if let Some(pool) = as_sqlite(backend) {
        async fn run_sqlite_batch(pool: &SqlitePool, rows: &[PreparedUpsertRow], simulation_id_str: &str) -> Result<(), sqlx::Error> {
            let mut tx = pool.begin().await?;
            for chunk in rows.chunks(SQLITE_UPSERT_CHUNK) {
                let mut qb: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new(
                    "INSERT INTO individuals (id, simulation_id, birth_day, death_day, alive, is_dead, parent_1_id, parent_2_id, inbreeding_coeff, data_json) ",
                );
                qb.push_values(chunk, |mut b, row| {
                    b.push_bind(row.id.clone())
                        .push_bind(simulation_id_str.to_string())
                        .push_bind(row.birth_day)
                        .push_bind(row.death_day)
                        .push_bind(row.alive as i32)
                        .push_bind(row.is_dead as i32)
                        .push_bind(row.parent_1_id.clone())
                        .push_bind(row.parent_2_id.clone())
                        .push_bind(row.inbreeding_coeff)
                        .push_bind(row.payload.to_string());
                });
                qb.push(
                    " ON CONFLICT(id) DO UPDATE SET \
                        simulation_id = excluded.simulation_id, \
                        birth_day = excluded.birth_day, \
                        death_day = excluded.death_day, \
                        alive = excluded.alive, \
                        is_dead = excluded.is_dead, \
                        parent_1_id = excluded.parent_1_id, \
                        parent_2_id = excluded.parent_2_id, \
                        inbreeding_coeff = excluded.inbreeding_coeff, \
                        data_json = excluded.data_json, \
                        updated_at = CURRENT_TIMESTAMP",
                );
                qb.build().execute(&mut *tx).await?;
            }
            tx.commit().await
        }

        if let Err(err) = run_sqlite_batch(pool, &rows, &simulation_id_str).await {
            if !is_foreign_key_violation(&err) {
                return Err(err);
            }
            tracing::warn!(simulation_id = %simulation_id_str, "upsert_individuals hit a foreign key violation, retrying with unresolved parent references nulled out");
            let sanitized = sanitize_dangling_parents(&rows, &to_upsert_ids);
            run_sqlite_batch(pool, &sanitized, &simulation_id_str).await?;
        }
    }

    Ok(())
}

/// Loads individual payloads directly from the `individuals` table -- not by
/// deserializing the simulation's state_json blob -- so the population and
/// deceased-list endpoints (polled every few seconds by the client) don't pay
/// for a JSON blob whose size only grows with total-ever-born rather than
/// current population. `alive_filter` matches the same semantics as the
/// client's `?alive=` query param: `Some(true)` = alive and not dead,
/// `Some(false)` = not alive or dead, `None` = everyone.
pub async fn load_individual_payloads(
    backend: &DbBackend,
    simulation_id: &str,
    alive_filter: Option<bool>,
    limit: Option<i64>,
) -> Result<Vec<Value>, sqlx::Error> {
    let limit = limit.unwrap_or(i64::MAX);
    if let Some(pool) = as_pg(backend) {
        let base = "SELECT data_json FROM individuals WHERE simulation_id = $1::uuid";
        let query = match alive_filter {
            Some(true) => format!("{base} AND alive = true AND is_dead = false ORDER BY created_at ASC LIMIT $2"),
            Some(false) => format!("{base} AND (alive = false OR is_dead = true) ORDER BY created_at ASC LIMIT $2"),
            None => format!("{base} ORDER BY created_at ASC LIMIT $2"),
        };
        let rows = sqlx::query_scalar::<_, Value>(&query).bind(simulation_id).bind(limit).fetch_all(pool).await?;
        return Ok(rows);
    }

    if let Some(pool) = as_sqlite(backend) {
        let base = "SELECT data_json FROM individuals WHERE simulation_id = ?";
        let query = match alive_filter {
            Some(true) => format!("{base} AND alive = 1 AND is_dead = 0 ORDER BY created_at ASC LIMIT ?"),
            Some(false) => format!("{base} AND (alive = 0 OR is_dead = 1) ORDER BY created_at ASC LIMIT ?"),
            None => format!("{base} ORDER BY created_at ASC LIMIT ?"),
        };
        let rows = sqlx::query_scalar::<_, String>(&query).bind(simulation_id).bind(limit).fetch_all(pool).await?;
        return Ok(rows
            .into_iter()
            .filter_map(|json| serde_json::from_str::<Value>(&json).ok())
            .collect());
    }

    Ok(vec![])
}

/// Single-individual counterpart to `load_individual_payloads`, for the
/// individual-detail endpoint -- same rationale, avoids the state_json blob.
pub async fn load_individual_payload(backend: &DbBackend, simulation_id: &str, individual_id: &str) -> Result<Option<Value>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_scalar::<_, Value>("SELECT data_json FROM individuals WHERE simulation_id = $1::uuid AND id = $2::uuid")
            .bind(simulation_id)
            .bind(individual_id)
            .fetch_optional(pool)
            .await;
    }
    if let Some(pool) = as_sqlite(backend) {
        let row = sqlx::query_scalar::<_, String>("SELECT data_json FROM individuals WHERE simulation_id = ? AND id = ?")
            .bind(simulation_id)
            .bind(individual_id)
            .fetch_optional(pool)
            .await?;
        return Ok(row.and_then(|json| serde_json::from_str::<Value>(&json).ok()));
    }
    Ok(None)
}

/// Cheapest possible query for "what's the most recent auto-checkpoint's
/// sim_day" -- runtime.rs's periodic auto-checkpointing needs only this one
/// integer to decide whether enough sim-days have elapsed since the last
/// checkpoint, and `list_checkpoints` would pull every checkpoint's full
/// population_snapshot blob just to read the first row's sim_day.
pub async fn latest_checkpoint_day(backend: &DbBackend, simulation_id: &str) -> Result<Option<i32>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MAX(sim_day) FROM checkpoints WHERE simulation_id = $1::uuid",
        )
        .bind(simulation_id)
        .fetch_one(pool)
        .await
        .map(|v| v.map(|d| d as i32));
    }
    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MAX(sim_day) FROM checkpoints WHERE simulation_id = ?",
        )
        .bind(simulation_id)
        .fetch_one(pool)
        .await
        .map(|v| v.map(|d| d as i32));
    }
    Ok(None)
}

pub async fn list_checkpoints(backend: &DbBackend, simulation_id: &str) -> Result<Vec<CheckpointRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, CheckpointRow>(
            r#"
            SELECT
                id::text AS id,
                simulation_id::text AS simulation_id,
                sim_day::int8 AS sim_day,
                sim_year::int8 AS sim_year,
                population_count::int8 AS population_count,
                population_snapshot,
                world_state,
                tech_state,
                belief_state,
                art_state,
                groups,
                stats,
                created_at::text AS created_at
            FROM checkpoints
            WHERE simulation_id = $1::uuid
            ORDER BY sim_day DESC, created_at DESC
            "#,
        )
        .bind(simulation_id)
        .fetch_all(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, CheckpointRow>(
            r#"
            SELECT
                id,
                simulation_id,
                sim_day,
                sim_year,
                population_count,
                population_snapshot,
                world_state,
                tech_state,
                belief_state,
                art_state,
                groups,
                stats,
                created_at
            FROM checkpoints
            WHERE simulation_id = ?
            ORDER BY sim_day DESC, created_at DESC
            "#,
        )
        .bind(simulation_id)
        .fetch_all(pool)
        .await;
    }

    Ok(vec![])
}

pub async fn load_checkpoint(
    backend: &DbBackend,
    checkpoint_id: &str,
    simulation_id: &str,
) -> Result<Option<CheckpointRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, CheckpointRow>(
            r#"
            SELECT
                id::text AS id,
                simulation_id::text AS simulation_id,
                sim_day::int8 AS sim_day,
                sim_year::int8 AS sim_year,
                population_count::int8 AS population_count,
                population_snapshot,
                world_state,
                tech_state,
                belief_state,
                art_state,
                groups,
                stats,
                created_at::text AS created_at
            FROM checkpoints
            WHERE id = $1::uuid AND simulation_id = $2::uuid
            "#,
        )
        .bind(checkpoint_id)
        .bind(simulation_id)
        .fetch_optional(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, CheckpointRow>(
            r#"
            SELECT
                id,
                simulation_id,
                sim_day,
                sim_year,
                population_count,
                population_snapshot,
                world_state,
                tech_state,
                belief_state,
                art_state,
                groups,
                stats,
                created_at
            FROM checkpoints
            WHERE id = ? AND simulation_id = ?
            "#,
        )
        .bind(checkpoint_id)
        .bind(simulation_id)
        .fetch_optional(pool)
        .await;
    }

    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_checkpoint(
    backend: &DbBackend,
    checkpoint_id: &str,
    simulation_id: &str,
    sim_day: i32,
    sim_year: i32,
    population_count: i64,
    population_snapshot: Value,
    world_state: Value,
    tech_state: Value,
    belief_state: Value,
    art_state: Value,
    groups: Value,
    stats: Value,
) -> Result<(), sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        sqlx::query(
            r#"
            INSERT INTO checkpoints (
                id, simulation_id, sim_day, sim_year, population_count,
                population_snapshot, world_state, tech_state, belief_state,
                art_state, groups, stats
            ) VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(checkpoint_id)
        .bind(simulation_id)
        .bind(sim_day)
        .bind(sim_year)
        .bind(population_count)
        .bind(population_snapshot)
        .bind(world_state)
        .bind(tech_state)
        .bind(belief_state)
        .bind(art_state)
        .bind(groups)
        .bind(stats)
        .execute(pool)
        .await?;
        return Ok(());
    }

    if let Some(pool) = as_sqlite(backend) {
        sqlx::query(
            r#"
            INSERT INTO checkpoints (
                id, simulation_id, sim_day, sim_year, population_count,
                population_snapshot, world_state, tech_state, belief_state,
                art_state, groups, stats
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(checkpoint_id)
        .bind(simulation_id)
        .bind(sim_day)
        .bind(sim_year)
        .bind(population_count)
        .bind(population_snapshot.to_string())
        .bind(world_state.to_string())
        .bind(tech_state.to_string())
        .bind(belief_state.to_string())
        .bind(art_state.to_string())
        .bind(groups.to_string())
        .bind(stats.to_string())
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn load_user_by_code(backend: &DbBackend, user_code: &str) -> Result<Option<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM users
            WHERE upper(user_code) = upper($1)
            "#,
        )
        .bind(user_code)
        .fetch_optional(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            FROM users
            WHERE upper(user_code) = upper(?)
            "#,
        )
        .bind(user_code)
        .fetch_optional(pool)
        .await;
    }

    Ok(None)
}

pub async fn load_user_by_id(backend: &DbBackend, id: &str) -> Result<Option<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM users
            WHERE id = $1::uuid
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await;
    }

    Ok(None)
}

/// Last-used "create simulation" wizard values, an opaque JSON blob the
/// client owns the shape of (see SimCreationWizard.tsx) -- account-scoped
/// so it survives iOS Safari's ITP-driven localStorage eviction and
/// follows the user across devices, instead of the old per-browser
/// localStorage-only version.
pub async fn get_wizard_defaults(backend: &DbBackend, user_id: &str) -> Result<Option<String>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_scalar::<_, Option<String>>("SELECT wizard_defaults FROM users WHERE id = $1::uuid").bind(user_id).fetch_optional(pool).await.map(|v| v.flatten());
    }
    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_scalar::<_, Option<String>>("SELECT wizard_defaults FROM users WHERE id = ?").bind(user_id).fetch_optional(pool).await.map(|v| v.flatten());
    }
    Ok(None)
}

pub async fn set_wizard_defaults(backend: &DbBackend, user_id: &str, value: &str) -> Result<(), sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        sqlx::query("UPDATE users SET wizard_defaults = $2 WHERE id = $1::uuid").bind(user_id).bind(value).execute(pool).await?;
        return Ok(());
    }
    if let Some(pool) = as_sqlite(backend) {
        sqlx::query("UPDATE users SET wizard_defaults = ? WHERE id = ?").bind(value).bind(user_id).execute(pool).await?;
        return Ok(());
    }
    Ok(())
}

pub async fn list_users(backend: &DbBackend) -> Result<Vec<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await;
    }

    if let Some(pool) = as_sqlite(backend) {
        return sqlx::query_as::<_, UserRow>(
            r#"
            SELECT
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await;
    }

    Ok(vec![])
}

#[allow(clippy::too_many_arguments)]
pub async fn create_or_update_user(
    backend: &DbBackend,
    user_code: &str,
    email: &str,
    first_name: &str,
    last_name: &str,
    tc_no: &str,
    password_hash: &str,
    role: &str,
    is_approved: bool,
) -> Result<Option<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (user_code, username, first_name, last_name, tc_no, email, password_hash, role, is_approved)
            VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_code) DO UPDATE SET
                username = EXCLUDED.username,
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                tc_no = EXCLUDED.tc_no,
                email = EXCLUDED.email,
                password_hash = EXCLUDED.password_hash,
                role = EXCLUDED.role,
                is_approved = EXCLUDED.is_approved,
                is_banned = false,
                ban_reason = NULL,
                updated_at = NOW()
            RETURNING
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(user_code)
        .bind(first_name)
        .bind(last_name)
        .bind(tc_no)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .bind(is_approved)
        .fetch_one(pool)
        .await?;
        return Ok(Some(row));
    }

    if let Some(pool) = as_sqlite(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (id, user_code, username, first_name, last_name, tc_no, email, password_hash, role, is_approved, is_banned, ban_reason, email_verified)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, 0)
            ON CONFLICT(user_code) DO UPDATE SET
                username = excluded.username,
                first_name = excluded.first_name,
                last_name = excluded.last_name,
                tc_no = excluded.tc_no,
                email = excluded.email,
                password_hash = excluded.password_hash,
                role = excluded.role,
                is_approved = excluded.is_approved,
                is_banned = 0,
                ban_reason = NULL,
                updated_at = CURRENT_TIMESTAMP
            RETURNING
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_code)
        .bind(user_code)
        .bind(first_name)
        .bind(last_name)
        .bind(tc_no)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .bind(i64::from(is_approved))
        .fetch_one(pool)
        .await?;
        return Ok(Some(row));
    }

    Ok(None)
}

/// Distinct from `create_or_update_user`: that function's `ON CONFLICT
/// (user_code) DO UPDATE` is an upsert, which would silently overwrite an
/// existing account's password/role if reused here for an admin's "create
/// user" action. This is a plain INSERT that surfaces a unique-constraint
/// error to the caller instead, and (unlike registration) takes an actual
/// `username` value rather than always mirroring `user_code` into it, since
/// an admin-created account's nickname is a distinct, optional field here.
#[allow(clippy::too_many_arguments)]
pub async fn admin_create_user(
    backend: &DbBackend,
    user_code: &str,
    username: Option<&str>,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (user_code, username, first_name, last_name, tc_no, email, password_hash, role, is_approved)
            VALUES ($1, $2, '', '', NULL, $3, $4, $5, true)
            RETURNING
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(user_code)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .fetch_one(pool)
        .await?;
        return Ok(Some(row));
    }

    if let Some(pool) = as_sqlite(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (id, user_code, username, first_name, last_name, tc_no, email, password_hash, role, is_approved, is_banned, ban_reason, email_verified)
            VALUES (?, ?, ?, '', '', NULL, ?, ?, ?, 1, 0, NULL, 0)
            RETURNING
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_code)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .fetch_one(pool)
        .await?;
        return Ok(Some(row));
    }

    Ok(None)
}

pub async fn update_user_flag(
    backend: &DbBackend,
    id: &str,
    approved: Option<bool>,
    banned: Option<bool>,
    ban_reason: Option<&str>,
    role: Option<&str>,
) -> Result<Option<UserRow>, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET
                is_approved = COALESCE($2, is_approved),
                is_banned = COALESCE($3, is_banned),
                ban_reason = COALESCE($4, ban_reason),
                role = COALESCE($5, role),
                updated_at = NOW()
            WHERE id = $1::uuid
            RETURNING
                id::text AS id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                CASE WHEN is_approved THEN 1 ELSE 0 END::int8 AS is_approved,
                CASE WHEN is_banned THEN 1 ELSE 0 END::int8 AS is_banned,
                ban_reason,
                CASE WHEN email_verified THEN 1 ELSE 0 END::int8 AS email_verified,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(id)
        .bind(approved)
        .bind(banned)
        .bind(ban_reason)
        .bind(role)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    if let Some(pool) = as_sqlite(backend) {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET
                is_approved = COALESCE(?, is_approved),
                is_banned = COALESCE(?, is_banned),
                ban_reason = COALESCE(?, ban_reason),
                role = COALESCE(?, role),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            RETURNING
                id,
                user_code,
                username,
                first_name,
                last_name,
                tc_no,
                email,
                password_hash,
                role,
                is_approved,
                is_banned,
                ban_reason,
                email_verified,
                created_at,
                updated_at
            "#,
        )
        .bind(approved.map(i64::from))
        .bind(banned.map(i64::from))
        .bind(ban_reason)
        .bind(role)
        .bind(id)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    Ok(None)
}

pub async fn delete_user(backend: &DbBackend, id: &str) -> Result<bool, sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        return Ok(sqlx::query("DELETE FROM users WHERE id = $1::uuid")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0);
    }

    if let Some(pool) = as_sqlite(backend) {
        return Ok(sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0);
    }

    Ok(false)
}

pub async fn cleanup_simulation_data(backend: &DbBackend) -> Result<(u64, u64, u64), sqlx::Error> {
    if let Some(pool) = as_pg(backend) {
        let checkpoints = sqlx::query("DELETE FROM checkpoints").execute(pool).await?.rows_affected();
        let events = sqlx::query(
            "DELETE FROM simulation_events WHERE id IN (SELECT id FROM (SELECT id, ROW_NUMBER() OVER (PARTITION BY simulation_id ORDER BY sim_day DESC) AS rn FROM simulation_events) t WHERE rn > 200)"
        ).execute(pool).await?.rows_affected();
        let dead = sqlx::query("DELETE FROM individuals WHERE alive = false").execute(pool).await?.rows_affected();
        return Ok((checkpoints, events, dead));
    }

    if let Some(pool) = as_sqlite(backend) {
        let checkpoints = sqlx::query("DELETE FROM checkpoints").execute(pool).await?.rows_affected();
        let events = sqlx::query("DELETE FROM simulation_events WHERE id IN (SELECT id FROM (SELECT id, ROW_NUMBER() OVER (PARTITION BY simulation_id ORDER BY sim_day DESC) AS rn FROM simulation_events) WHERE rn > 200)")
            .execute(pool).await?.rows_affected();
        let dead = sqlx::query("DELETE FROM individuals WHERE alive = 0").execute(pool).await?.rows_affected();
        return Ok((checkpoints, events, dead));
    }

    Ok((0, 0, 0))
}

#[cfg(test)]
mod state_persistence_dto_tests {
    use std::fs;

    fn field_names_in_struct(source: &str, struct_signature_marker: &str) -> Vec<String> {
        let start = source.find(struct_signature_marker).expect("struct signature not found");
        let body_start = source[start..].find('{').expect("struct body not found") + start + 1;
        let mut depth = 1;
        let mut end = body_start;
        for (i, c) in source[body_start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = body_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        source[body_start..end]
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
            .filter_map(|l| l.split(':').next())
            .map(|name| name.trim())
            .map(|name| name.strip_prefix("pub ").unwrap_or(name).trim().to_string())
            .filter(|name| !name.is_empty())
            .collect()
    }

    // Guards the `state_json_for_db` DTO above: if `SimulationState` (sim-core)
    // gains a field that isn't added to `StateForPersistence` (db.rs) too, that
    // field would silently stop being written to state_json instead of failing
    // loudly. `individuals` is one deliberate, permanent exception -- that's
    // the whole point of this DTO. `genealogy` and `disabled_engines` are the
    // others: both `#[serde(skip)]` on `SimulationState` itself (never
    // round-trip through JSON at all -- genealogy's sole source is db.rs's
    // `load_genealogy_index`, disabled_engines' is runtime.rs's own session-
    // scoped toggle set, refreshed into it fresh every tick loop iteration),
    // so neither can ever appear in `StateForPersistence`'s output either.
    #[test]
    fn state_struct_fields_are_mirrored_in_persistence_dto() {
        let state_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../sim-core/src/state.rs")).expect("read sim-core/src/state.rs");
        let db_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/db.rs")).expect("read src/db.rs");

        let state_fields = field_names_in_struct(&state_rs, "pub struct SimulationState {");
        let dto_fields = field_names_in_struct(&db_rs, "struct StateForPersistence<'a> {");

        let missing: Vec<&String> = state_fields
            .iter()
            .filter(|f| !matches!(f.as_str(), "individuals" | "genealogy" | "disabled_engines") && !dto_fields.contains(f))
            .collect();
        assert!(missing.is_empty(), "SimulationState field(s) missing from StateForPersistence in db.rs -- state_json would silently drop them: {missing:?}");
    }
}

#[cfg(test)]
mod upsert_fk_recovery_tests {
    use super::*;

    // ── is_foreign_key_violation_signal() ───────────────────────────────

    #[test]
    fn postgres_sqlstate_23503_is_recognized_regardless_of_message_text() {
        assert!(is_foreign_key_violation_signal(Some("23503"), "anything at all"));
    }

    #[test]
    fn a_message_mentioning_foreign_key_is_recognized_without_a_code() {
        assert!(is_foreign_key_violation_signal(None, "FOREIGN KEY constraint failed"));
        assert!(is_foreign_key_violation_signal(None, "violates foreign key constraint \"individuals_parent_2_id_fkey\""));
    }

    #[test]
    fn an_unrelated_error_is_not_mistaken_for_a_foreign_key_violation() {
        assert!(!is_foreign_key_violation_signal(Some("23505"), "duplicate key value violates unique constraint"));
        assert!(!is_foreign_key_violation_signal(None, "connection reset by peer"));
    }

    // ── sanitize_dangling_parents() ──────────────────────────────────────

    fn prepared_row(id: &str, parent_1: Option<&str>, parent_2: Option<&str>) -> PreparedUpsertRow {
        PreparedUpsertRow {
            id: id.to_string(),
            birth_day: 0,
            death_day: None,
            alive: true,
            is_dead: false,
            parent_1_id: parent_1.map(str::to_string),
            parent_2_id: parent_2.map(str::to_string),
            inbreeding_coeff: 0.0,
            payload: json!({}),
        }
    }

    #[test]
    fn a_parent_reference_included_in_the_same_batch_survives_sanitization() {
        let rows = vec![prepared_row("parent", None, None), prepared_row("child", Some("parent"), None)];
        let ids: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();
        let sanitized = sanitize_dangling_parents(&rows, &ids);
        let child = sanitized.iter().find(|r| r.id == "child").expect("child present");
        assert_eq!(child.parent_1_id.as_deref(), Some("parent"), "a parent included in the same batch must not be nulled out");
    }

    #[test]
    fn a_parent_reference_outside_the_batch_is_nulled_out() {
        let rows = vec![prepared_row("child", Some("ghost-parent"), Some("another-ghost"))];
        let ids: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();
        let sanitized = sanitize_dangling_parents(&rows, &ids);
        let child = &sanitized[0];
        assert_eq!(child.parent_1_id, None, "a parent not present in this batch must be nulled out, not left dangling");
        assert_eq!(child.parent_2_id, None);
    }

    #[test]
    fn sanitizing_never_drops_the_row_itself_only_the_dangling_references() {
        let rows = vec![prepared_row("orphan", Some("nowhere"), None)];
        let ids: HashSet<String> = rows.iter().map(|r| r.id.clone()).collect();
        let sanitized = sanitize_dangling_parents(&rows, &ids);
        assert_eq!(sanitized.len(), 1, "the row itself must still be upserted, only its dangling parent reference is dropped");
        assert_eq!(sanitized[0].id, "orphan");
    }
}
