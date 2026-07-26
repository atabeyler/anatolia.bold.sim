// Android counterpart to desktop's "Yerel"/"Bulut" chooser
// (desktop/src-tauri/dist-chooser/index.html). Only meaningful inside the
// Capacitor-wrapped native app; on the web and inside desktop's own Tauri
// webview this module is inert (isNativeAndroidApp() is false there, so
// every other export here is a no-op / passthrough).
import { Capacitor, registerPlugin } from '@capacitor/core';

interface LocalServerPlugin {
  start(): Promise<void>;
  stop(): Promise<void>;
  isRunning(): Promise<{ running: boolean }>;
}

const LocalServer = registerPlugin<LocalServerPlugin>('LocalServer');

export const LOCAL_SERVER_URL = 'http://127.0.0.1:3001';

// Set once per app session, right after the user picks "Yerel" in the
// chooser and the local sim-server subprocess is confirmed up -- read by
// cloud.ts's isLocalOrigin() and useSimWebSocket's connection URL, exactly
// like desktop's own 127.0.0.1-hostname check does for its local sidecar.
let yerelModeActive = false;

export function isNativeAndroidApp(): boolean {
  return Capacitor.isNativePlatform() && Capacitor.getPlatform() === 'android';
}

export function isYerelModeActive(): boolean {
  return yerelModeActive;
}

export async function startLocalServerAndActivate(): Promise<void> {
  await LocalServer.start();
  yerelModeActive = true;
}

// Installing an app update while its own local sim-server subprocess (a
// native binary this very process spawned and still owns, see
// LocalServerPlugin.java) is still alive is one more thing standing between
// PackageManager and a clean swap of the APK backing all of it -- stop it
// first so an in-progress "Yerel" session doesn't compound the same
// self-update fragility ApkUpdaterPlugin's finishAffinity() addresses on the
// activity side. A no-op if the user is in "Bulut" mode (never started).
export async function stopLocalServerIfRunning(): Promise<void> {
  if (!yerelModeActive) return;
  await LocalServer.stop();
  yerelModeActive = false;
}

// Returns to the Cloud/Local chooser (NativeModeGate) from anywhere further
// into the app -- e.g. LoginPage's own "back to selection" link, mirroring
// the one BrowserModeGate's web visitors already get. NativeModeGate's
// "past the chooser" state (its `ready` flag) lives only in that
// component's own memory -- nothing persists it, which is also why every
// cold app launch already re-shows the chooser -- so a full reload is what
// actually gets back there, the same mechanism a plain page navigation
// would use on the web. Stops an already-started local server first so it
// isn't left running as an orphan under the reloaded session's nose.
export async function returnToChooser(): Promise<void> {
  await stopLocalServerIfRunning();
  window.location.href = '/';
}
