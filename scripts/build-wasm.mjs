// Orchestrates `npm run build:wasm` as a real script instead of one long
// shell one-liner, for two reasons:
//
// 1. Portability -- this runs on Linux (Render, GitHub Actions Ubuntu jobs)
//    *and* Windows (Desktop Release's windows-latest runner), and setting
//    per-command environment variables (see below) has no single syntax
//    that works in both bash and PowerShell/cmd.exe. Node's child_process
//    `env` option is identical on every platform.
//
// 2. Render's build container ships a pre-installed *stable* Rust toolchain
//    whose CARGO_HOME (`/usr/local/cargo`) is read-only -- fine for the
//    existing stable toolchain and for `cargo install wasm-bindgen-cli`
//    (which no-ops once that exact version is already installed, so it
//    never actually needs to write there), but installing a brand-new
//    toolchain (this crate's pinned nightly -- see
//    rust/sim-wasm/rust-toolchain.toml) also makes rustup run its own
//    self-update preflight check: it creates a throwaway `updtest*` temp
//    dir in CARGO_HOME/bin just to confirm that's writable, and --
//    confirmed by reading rustup's own source (self_update_permitted in
//    src/cli/self_update.rs) -- only handles a `PermissionDenied` (EACCES)
//    result gracefully; a read-only filesystem surfaces as `EROFS`, which
//    isn't caught by that match arm and propagates as a hard failure
//    instead of just skipping self-update. This broke a real Render
//    deploy (production!) the moment sim-wasm started needing a second,
//    nightly toolchain.
//
//    `RUSTUP_AUTO_SELF_UPDATE` is *not* a real rustup env var (confirmed by
//    grepping rustup's own source -- it doesn't exist anywhere) -- that was
//    a wrong first guess at the right knob, and shipping it changed
//    nothing, breaking the same Render deploy a second time. The setting
//    (`auto_self_update`) is only ever read from the on-disk settings.toml
//    (SelfUpdateMode::from_cfg) -- *except* for one specific, directly
//    verified escape hatch checked first, before the settings file is even
//    read: `if process.var("CI").is_ok() && process.var("RUSTUP_CI").is_err()
//    { return Disable }`. GitHub Actions' own CI jobs already set `CI=true`
//    themselves, which is exactly why they never hit this -- Render's build
//    container doesn't. Setting `CI=true` here makes that same behavior
//    explicit and host-independent rather than an accident of which
//    environment happens to define it.
import { execFileSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const simWasmDir = resolve(repoRoot, 'rust/sim-wasm');
const pkgDir = resolve(repoRoot, 'client/src/wasmLocal/pkg');

const NIGHTLY = 'nightly-2026-07-18';
const noSelfUpdateEnv = { ...process.env, CI: 'true' };
delete noSelfUpdateEnv.RUSTUP_CI;

function run(cmd, args, options = {}) {
  console.log(`$ ${cmd} ${args.join(' ')}`);
  execFileSync(cmd, args, { stdio: 'inherit', ...options });
}

// Like run(), but captures output instead of streaming it live, so a catch
// block can inspect *why* it failed -- `stdio: 'inherit'` (what run() uses)
// never populates `error.stderr`. Explicit `stdio: ['ignore', 'pipe',
// 'pipe']` rather than just setting `encoding` and leaving `stdio`
// unspecified: some Node versions still tee a piped-but-encoded stream
// straight to the parent's own stderr in addition to capturing it, which
// silently double-printed everything here otherwise. Prints what it
// captured itself, exactly once, so CI logs don't lose visibility either
// way.
function runCaptured(cmd, args, options = {}) {
  console.log(`$ ${cmd} ${args.join(' ')}`);
  try {
    const output = execFileSync(cmd, args, { encoding: 'utf-8', stdio: ['ignore', 'pipe', 'pipe'], ...options });
    process.stdout.write(output);
    return output;
  } catch (err) {
    if (err.stdout) process.stdout.write(err.stdout);
    if (err.stderr) process.stderr.write(err.stderr);
    throw err;
  }
}

run('rustup', ['toolchain', 'install', NIGHTLY, '--profile', 'minimal', '--component', 'rust-src'], { env: noSelfUpdateEnv });
run('rustup', ['target', 'add', 'wasm32-unknown-unknown', '--toolchain', NIGHTLY], { env: noSelfUpdateEnv });

// A CI cache (Desktop Release's Swatinem/rust-cache -- unlike test.yml/
// android-release.yml's own dedicated wasm-bindgen-cli cache step, which
// keeps ~/.cargo/bin *and* .crates.toml/.crates2.json in lockstep) can
// restore ~/.cargo/bin's binaries without cargo's own install-tracking
// metadata agreeing they're there, so `cargo install` refuses with "binary
// `X` already exists in destination" instead of the normal (successful)
// "already installed, ignoring". The suggested fix (--force) would make
// cargo actually rewrite those binary files, risking reintroducing exactly
// the Render read-only-CARGO_HOME/bin problem this script exists to avoid
// (see the big comment above) -- but there's no need to write anything at
// all here: this error *means* the binaries are already on disk (that's
// the whole reason it fired), so this just confirms the one this build
// actually needs still runs, without touching the filesystem further.
try {
  runCaptured('cargo', ['install', 'wasm-bindgen-cli', '--version', '0.2.126', '--locked']);
} catch (err) {
  const output = String(err.stderr ?? '') + String(err.stdout ?? '');
  if (!/already exists in destination/.test(output)) throw err;
  console.log('wasm-bindgen-cli binary already present on disk (stale cache metadata) -- verifying it still works instead of reinstalling.');
  run('wasm-bindgen', ['--version']);
}

run('rustup', ['run', NIGHTLY, 'cargo', 'build', '--release', '--target', 'wasm32-unknown-unknown', '-Z', 'build-std=panic_abort,std'], {
  cwd: simWasmDir,
  env: noSelfUpdateEnv,
});
run('wasm-bindgen', [
  '--target', 'web',
  '--out-dir', pkgDir,
  '--out-name', 'sim_wasm',
  resolve(simWasmDir, 'target/wasm32-unknown-unknown/release/sim_wasm.wasm'),
]);
run('node', [resolve(__dirname, 'write-wasm-pkg-json.mjs')]);
