// Runs as tauri.conf.json's `beforeBuildCommand`, right before `cargo tauri
// build` bundles resources. Tauri v1 resolves each `bundle.resources` glob
// relative to tauri.conf.json's directory (desktop/src-tauri), but any `..`
// path component gets rewritten to the literal folder name `_up_` when it
// copies files into the bundle (see tauri-utils' `resource_relpath`) --
// e.g. "../../rust/target/release/sim-server.exe" lands at
// "_up_/_up_/rust/target/release/sim-server.exe" inside the installed app,
// not at "rust/target/release/sim-server.exe" like `resolve_resource()` in
// main.rs expects. That mismatch is why launching "Yerel" (local) mode
// failed with "Rust server binary not found in resources".
//
// Staging the build outputs into a plain subfolder of src-tauri first lets
// tauri.conf.json reference them with no `..` at all, so the bundled path
// matches exactly what main.rs looks up.
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..');
const resourcesDir = join(__dirname, 'src-tauri', 'resources');

const serverBinary = join(
  repoRoot, 'rust', 'target', 'release',
  process.platform === 'win32' ? 'sim-server.exe' : 'sim-server',
);
if (!existsSync(serverBinary)) {
  throw new Error(`sim-server binary not found at ${serverBinary} — run the root "build" script first.`);
}

const clientDist = join(repoRoot, 'client', 'dist');
if (!existsSync(clientDist)) {
  throw new Error(`client/dist not found at ${clientDist} — run the root "build" script first.`);
}

rmSync(resourcesDir, { recursive: true, force: true });
mkdirSync(resourcesDir, { recursive: true });

cpSync(serverBinary, join(resourcesDir, process.platform === 'win32' ? 'sim-server.exe' : 'sim-server'));
// Preserved as "client/dist" (not flattened) because sim-server's own
// static-file lookup (rust/sim-server/src/main.rs) searches for a
// "client/dist" directory next to itself.
cpSync(clientDist, join(resourcesDir, 'client', 'dist'), { recursive: true });

console.log(`[stage-resources] staged sim-server + client/dist into ${resourcesDir}`);
