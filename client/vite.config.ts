import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { resolve, dirname } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const { version } = JSON.parse(readFileSync(resolve(__dirname, '../package.json'), 'utf-8'));

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(version),
  },
  plugins: [react()],
  server: {
    port: 5173,
    // Matches rust/sim-server/src/main.rs's own cross_origin_isolation_headers
    // middleware exactly (including require-corp over credentialless -- see
    // its doc comment) -- without these, `npm run dev`/`vite preview` can't
    // exercise WASM-local's multi-threaded path at all (no SharedArrayBuffer,
    // so initThreadPool silently falls back to 1 thread) even though
    // production would have it.
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
    proxy: {
      '/api': 'http://localhost:3001',
      '/ws': { target: 'ws://localhost:3001', ws: true },
    },
  },
  preview: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  build: {
    outDir: 'dist',
    rollupOptions: { output: { manualChunks: { three: ['three','@react-three/fiber','@react-three/drei'], react: ['react','react-dom','react-router-dom'] } } },
  },
  // wasm-bindgen-rayon's generated code spawns its own nested Worker from
  // *inside* worker.ts (see rust/sim-wasm's own Cargo.toml comment) to build
  // its thread pool -- that nested worker import is itself split into a
  // separate chunk, which Vite's default worker build format ('iife') can't
  // represent (IIFE can't be code-split). Only surfaces in a real production
  // `vite build`, not `vite dev`'s unbundled dev-server serving -- confirmed
  // the hard way when this shipped without it and broke CI's client build.
  worker: {
    format: 'es',
  },
});
