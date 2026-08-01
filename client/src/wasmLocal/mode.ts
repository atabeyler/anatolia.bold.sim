// Browser-only "Local (WASM)" mode: the simulation itself runs in this tab
// (via a Web Worker wrapping sim-wasm) with no simulation server at all --
// but the account/login flow is completely unchanged, exactly like
// Android/Desktop's own "Yerel" mode (see NativeModeGate.tsx / the desktop
// dist-chooser): sign in for real, only the simulation data stays local.
// Chosen via BrowserModeGate.tsx's Cloud/Local gate, shown once per browser.
// Activating this flips axios's default adapter (see apiAdapter.ts) so every
// existing page/panel's own `/api/simulations/...` calls keep working
// completely unchanged, serviced from this tab's own in-memory + IndexedDB
// state instead of a network request.
let active = false;

export function isWasmLocalModeActive(): boolean {
  return active;
}

export function activateWasmLocalMode(): void {
  active = true;
}

// Exposed for tests -- not currently wired to any UI action.
export function deactivateWasmLocalMode(): void {
  active = false;
}
