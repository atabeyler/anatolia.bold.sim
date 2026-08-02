// Client-only rebuild of GET /:id/report (routes.rs's get_report), operating
// directly on the full in-browser SimulationState -- which, unlike the
// server's population endpoint, already holds every field this needs (no
// separate "give me the report-shaped projection" round trip is possible
// here, so this reads raw individual/psychology/social fields directly).
import { engine } from './engineClient';
import type { StoredCheckpoint } from './db';

type AnyRecord = Record<string, unknown>;

function ageYears(ind: AnyRecord, currentDay: number): number {
  const deathDay = ind.death_day as number | null;
  const day = ind.is_dead ? (deathDay ?? currentDay) : currentDay;
  return Math.max((day as number) - (ind.birth_day as number), 0) / 365;
}

function displayName(ind: AnyRecord): string {
  const phenotype = ind.phenotype as AnyRecord | undefined;
  const extra = ind.extra as AnyRecord | undefined;
  return (phenotype?.name as string) ?? (extra?.name as string) ?? 'Unnamed';
}

// Mirrors routes.rs's pascal_to_snake -- DeathCause's Rust Debug format is
// PascalCase ("OldAge"), some paths already write lowercase ("infection").
function toSnakeCase(s: string): string {
  let out = '';
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch >= 'A' && ch <= 'Z') {
      if (i > 0) out += '_';
      out += ch.toLowerCase();
    } else {
      out += ch;
    }
  }
  return out;
}

const AGE_BANDS: Array<[string, (age: number) => boolean]> = [
  ['infant_0_1', (a) => a < 1],
  ['child_1_15', (a) => a >= 1 && a < 15],
  ['adolescent_15_25', (a) => a >= 15 && a < 25],
  ['adult_25_65', (a) => a >= 25 && a < 65],
  ['elder_65_plus', (a) => a >= 65],
];

export async function buildReport(state: AnyRecord, checkpoints: StoredCheckpoint[]): Promise<AnyRecord> {
  const stateJson = JSON.stringify(state);
  const currentStats = (await engine.getStats(stateJson)) as AnyRecord;
  const events = (await engine.getEvents(stateJson)) as AnyRecord[];
  const individuals = (state.individuals as AnyRecord[]) ?? [];
  const currentDay = (state.current_day as number) ?? 0;

  const deadIndividuals = individuals.filter((i) => i.is_dead || !i.alive);
  const deathTotal = deadIndividuals.length;
  const avgAgeAtDeath = deathTotal > 0 ? deadIndividuals.reduce((sum, i) => sum + ageYears(i, currentDay), 0) / deathTotal : null;

  const byCause: Record<string, number> = {};
  for (const ind of deadIndividuals) {
    const extra = (ind.extra as AnyRecord) ?? {};
    const cause = (extra.death_cause as string) ?? 'unknown';
    const key = toSnakeCase(typeof cause === 'string' ? cause : 'unknown');
    byCause[key] = (byCause[key] ?? 0) + 1;
  }
  const byAgeGroup: Record<string, number> = {};
  for (const ind of deadIndividuals) {
    const age = ageYears(ind, currentDay);
    const band = AGE_BANDS.find(([, test]) => test(age));
    if (band) byAgeGroup[band[0]] = (byAgeGroup[band[0]] ?? 0) + 1;
  }
  const leadingCause = Object.entries(byCause).sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;

  const totalEver = Math.max(individuals.length, 1);
  const infantDeaths = byAgeGroup.infant_0_1 ?? 0;
  const childDeaths = byAgeGroup.child_1_15 ?? 0;

  // engine.getEvents() runs every raw event through to_client_event, which
  // nests everything but event_type/sim_day/sim_year/importance/description
  // under `data` -- the server's own /report route instead rebuilds each
  // migration into a flat { year, day, distance_km, reason, from, to, ... }
  // record (see routes.rs's migration_history) before ReportPanel.tsx ever
  // sees it. Filtering to raw to_client_event shapes here (as before) left
  // every column ReportPanel.tsx reads (e.year, e.distance_km, e.reason, ...)
  // undefined, rendering as an all-dash Migration History table in WASM-
  // Local mode specifically.
  const migrationHistory = events
    .filter((e) => e.event_type === 'migration')
    .map((e) => {
      const data = (e.data as AnyRecord) ?? {};
      return {
        year: e.sim_year,
        day: e.sim_day,
        distance_km: data.distance_km ?? null,
        reason: data.reason ?? null,
        from: data.from ?? null,
        to: data.to ?? null,
        food_abundance: data.food_abundance ?? null,
        water_abundance: data.water_abundance ?? null,
        season: data.season ?? null,
      };
    });
  const totalMigrationDistance = migrationHistory.reduce((sum, e) => sum + ((e.distance_km as number) ?? 0), 0);

  const populationHistory = checkpoints.map((c) => ({ year: c.sim_year, day: c.sim_day, population: c.population_count, ...c.stats }));
  const peak = populationHistory.reduce<AnyRecord | null>((best, cp) => ((cp.population as number) > ((best?.population as number) ?? -1) ? cp : best), null);

  const beliefLabels = (state.belief_labels as Record<string, string>) ?? {};
  const technologyTimeline = ((state.discovered_techs as string[]) ?? []).map((name) => ({ name, year: state.current_year, day: state.current_day }));
  const beliefTimeline = ((state.discovered_beliefs as string[]) ?? []).map((code) => ({
    name: beliefLabels[code] ?? null,
    code,
    year: state.current_year,
    day: state.current_day,
  }));
  const artTimeline = ((state.discovered_arts as string[]) ?? []).map((name) => ({ name, year: state.current_year, day: state.current_day, type: 'art' }));

  const notableEvents = events.filter((e) => e.importance === 'medium' || e.importance === 'high');

  const nameById = new Map<string, string>();
  for (const ind of individuals) nameById.set(ind.id as string, displayName(ind));

  const reportIndividuals = individuals.map((ind) => {
    const age = ageYears(ind, currentDay);
    const psychology = (ind.psychology as AnyRecord) ?? {};
    const relationships = Object.entries((psychology.relationships as Record<string, number>) ?? {})
      .sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]))
      .slice(0, 5)
      .map(([otherId, bond]) => ({ id: otherId, name: nameById.get(otherId) ?? 'Unnamed', bond: Math.round(bond * 100) / 100 }));
    const extra = (ind.extra as AnyRecord) ?? {};
    const phenotype = (ind.phenotype as AnyRecord) ?? {};
    const social = (ind.social as AnyRecord) ?? {};
    const mind = (ind.mind as AnyRecord) ?? {};
    const mindExtra = (mind.extra as AnyRecord) ?? {};
    const deathDay = ind.death_day as number | null;
    return {
      id: ind.id,
      name: (phenotype.name as string) ?? 'Unnamed',
      sex: ind.sex,
      is_founder: ind.is_founder,
      birth_year: Math.floor((ind.birth_day as number) / 365),
      death_year: deathDay != null ? Math.floor(deathDay / 365) : null,
      age_at_death: deathDay != null ? Math.max((deathDay - (ind.birth_day as number)) / 365, 0) : null,
      death_cause: extra.death_cause ?? null,
      is_dead: ind.is_dead || !ind.alive,
      intelligence: Math.round(((phenotype.fluid_intelligence as number) ?? 0) * 100) / 100,
      age_years: Math.round(age * 10) / 10,
      mental_state: psychology.mental_state,
      wellbeing: Math.round(((psychology.wellbeing as number) ?? 0) * 100) / 100,
      theory_of_mind: psychology.theory_of_mind,
      reputation: Math.round(((social.reputation as number) ?? 0) * 100) / 100,
      role: extra.group_role ?? null,
      relationships,
      inner_thought_log: mindExtra.inner_thought_log ?? [],
      // Mirrors routes.rs's own get_report addition -- this individual's own
      // live hormone state (see AGENTS.md's Hormones section).
      hormones: ind.hormones ?? {},
    };
  });

  return {
    // ReportPanel.tsx reads name/start_latitude/start_longitude/biome off
    // `r.simulation` (matching routes.rs's own /report shape) -- this mode's
    // buildReport() never emitted that object at all, so the report's cover
    // page always showed the "?°, ?°" / "?" biome placeholders regardless of
    // what the simulation was actually started with.
    simulation: {
      id: state.id,
      name: state.name ?? null,
      status: state.status ?? null,
      start_latitude: state.start_latitude ?? null,
      start_longitude: state.start_longitude ?? null,
      biome: (state.world_state as AnyRecord)?.biome ?? null,
      current_year: state.current_year,
      current_day: state.current_day,
    },
    summary: {
      civilization_name: state.civilization_name,
      total_years: state.current_year,
      total_days: state.current_day,
      start_coordinates: { latitude: state.start_latitude, longitude: state.start_longitude },
      biome: (state.world_state as AnyRecord)?.biome,
      total_individuals_ever: individuals.length,
      peak_population: peak?.population ?? 0,
      peak_population_year: peak?.year ?? null,
      current_population: currentStats.population,
      technologies_discovered: technologyTimeline.length,
      technology_list: state.discovered_techs ?? [],
      beliefs_formed: beliefTimeline.length,
      belief_list: beliefTimeline.map((b) => b.name ?? `#${b.code.replace('belief_', '')}`),
      art_forms: artTimeline.length,
      language_stage: currentStats.max_language_stage,
      language_stage_name: 'unknown',
      vocabulary_size: currentStats.word_count,
      total_deaths: deathTotal,
      avg_age_at_death_years: avgAgeAtDeath != null ? Math.round(avgAgeAtDeath * 10) / 10 : null,
      infant_mortality_rate: Math.round((infantDeaths / totalEver) * 1000) / 1000,
      child_mortality_rate: Math.round((childDeaths / totalEver) * 1000) / 1000,
      leading_cause_of_death: leadingCause,
      migration_events: migrationHistory.length,
      total_migration_distance_km: Math.round(totalMigrationDistance * 10) / 10,
      epidemic_count: notableEvents.filter((e) => e.event_type === 'epidemic_outbreak').length,
      disaster_count: notableEvents.filter((e) => e.event_type === 'disaster').length,
      final_happiness_index: currentStats.happiness_index,
      final_gini: currentStats.gini,
      final_qol_index: currentStats.qol_index,
      report_generated_at: new Date().toISOString(),
    },
    current_stats: currentStats,
    population_history: populationHistory,
    technology_timeline: technologyTimeline,
    belief_timeline: beliefTimeline,
    art_timeline: artTimeline,
    migration_history: migrationHistory,
    death_statistics: {
      total: deathTotal,
      avg_age_at_death: avgAgeAtDeath != null ? Math.round(avgAgeAtDeath * 10) / 10 : null,
      by_cause: byCause,
      by_age_group: byAgeGroup,
    },
    individuals: reportIndividuals,
    notable_events: notableEvents,
    all_events: events,
  };
}
