// wasm-bindgen's raw CLI output (unlike wasm-pack's full packaging) never
// writes a package.json into client/src/wasmLocal/pkg -- but
// wasm-bindgen-rayon's generated workerHelpers.js snippet does
// `import('../../..')` to reach the main module from its own nested-worker
// file, which is a *directory* import that only resolves (in both Vite's
// dev-server resolver and a real browser's module loader once bundled)
// if that directory has a package.json pointing at the actual entry file.
// Without this, cross_origin_isolation_headers' whole point -- a real
// multi-threaded WASM-local build -- silently 500s the moment a worker
// tries to spin up its thread pool. Regenerated on every `npm run
// build:wasm` since pkg/ itself is gitignored and rebuilt fresh each time.
import { writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = resolve(__dirname, '../client/src/wasmLocal/pkg');

writeFileSync(
  resolve(pkgDir, 'package.json'),
  JSON.stringify({ type: 'module', main: 'sim_wasm.js', module: 'sim_wasm.js' }, null, 2) + '\n',
);
