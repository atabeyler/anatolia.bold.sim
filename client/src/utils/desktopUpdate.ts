// Desktop's Tauri updater. App.tsx wires up the install callback and event
// listener once at startup (and runs an initial check); SettingsOverlay's
// "check for updates" button reuses this same check so a user who dismissed
// the auto-check banner, or started the app before a new release existed,
// isn't stuck without a way to look again.
export function isTauriDesktop(): boolean {
  return typeof window !== 'undefined' && !!((window as any).__TAURI_IPC__ || (window as any).__TAURI__);
}

export interface DesktopUpdateInfo {
  version?: string;
}

export async function checkForDesktopUpdate(): Promise<DesktopUpdateInfo | null> {
  if (!isTauriDesktop()) return null;
  try {
    const { checkUpdate } = await import('@tauri-apps/api/updater');
    const { shouldUpdate, manifest } = await checkUpdate();
    return shouldUpdate ? { version: manifest?.version } : null;
  } catch (err) {
    console.warn('[updater] unavailable:', err);
    return null;
  }
}

// Returns to dist-chooser's Cloud/Local selection screen from anywhere
// further into the app -- e.g. LoginPage's own "back to selection" link,
// mirroring the one web visitors already get from BrowserModeGate.
// dist-chooser is a separate static page entirely outside this SPA (see
// tauri.conf.json's distDir), served from a Tauri-internal asset origin
// whose exact URL differs by platform (https://tauri.localhost on Windows,
// tauri://localhost elsewhere, per dist-chooser/index.html's own comment) --
// relaunching the whole app process is the one version/platform-independent
// way back to it, since every cold launch already starts there. Stops an
// already-started local server first (see main.rs's stop_local_server) so
// relaunch() -- which restarts the process without ever going through the
// window's own CloseRequested cleanup -- doesn't leave it running as an
// orphan still holding port 3001 out from under the relaunched app.
export async function returnToDesktopChooser(): Promise<void> {
  if (!isTauriDesktop()) return;
  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    await invoke('stop_local_server');
  } catch {
    // Cloud was chosen (nothing running to stop) or the command is
    // otherwise unavailable -- either way, still proceed to relaunch.
  }
  const { relaunch } = await import('@tauri-apps/api/process');
  await relaunch();
}
