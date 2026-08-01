// Cross-platform dispatcher for the `rust:build`/`rust:test`/`rust:check`/
// `rust:serve` npm scripts. These used to invoke `rust\scripts\build.cmd`
// (etc.) directly with a hardcoded Windows-style backslash path -- a plain
// `.cmd` batch file, unrunnable outside `cmd.exe`/PowerShell -- so every one
// of those scripts silently failed on Linux/macOS with no explanation
// (README works around this by telling every developer, Windows included,
// to just run `cargo run -p sim-server` directly instead, but the broken
// npm scripts stayed in package.json regardless).
//
// On Windows the `.cmd` scripts still do real, necessary work beyond just
// running cargo -- they source Visual Studio Build Tools' vcvars64.bat first
// to put the MSVC linker on PATH, which a plain `cargo` invocation from an
// arbitrary shell can't assume is already set up. So this dispatcher keeps
// deferring to them on win32, and only substitutes a direct `cargo`
// invocation (no MSVC bootstrap needed) on every other platform.
import { execFileSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const rustDir = resolve(repoRoot, 'rust');

const ACTIONS = {
  build: ['build'],
  check: ['check'],
  test: ['test'],
  serve: ['run', '-p', 'sim-server'],
};

const action = process.argv[2];
const cargoArgs = ACTIONS[action];
if (!cargoArgs) {
  console.error(`Usage: node scripts/rust-cmd.mjs <${Object.keys(ACTIONS).join('|')}>`);
  process.exit(1);
}

if (process.platform === 'win32') {
  execFileSync(resolve(rustDir, 'scripts', `${action}.cmd`), { stdio: 'inherit', shell: true });
} else {
  execFileSync('cargo', [...cargoArgs, '--manifest-path', resolve(rustDir, 'Cargo.toml')], { stdio: 'inherit' });
}
