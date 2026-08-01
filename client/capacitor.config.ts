import type { CapacitorConfig } from '@capacitor/cli';

// Same identifier the desktop Tauri bundle uses (desktop/src-tauri/tauri.conf.json)
// so all three distributions (web, desktop, Android) read as one product family.
const config: CapacitorConfig = {
  appId: 'com.atabeylers.anatoliasim',
  appName: 'Anatolia Sim',
  webDir: 'dist',
  // Do NOT add server.allowNavigation here -- this is a known Capacitor
  // Android bug (ionic-team/capacitor#4164, #5455, #7454): any hostname
  // listed there breaks the native plugin bridge app-wide on Android (every
  // custom plugin -- LocalServerPlugin, ApkUpdaterPlugin, FileOpenerPlugin --
  // starts throwing "plugin is not implemented on android"), not just for
  // navigation to that host. NativeModeGate.tsx's "Cloud" choice used to
  // need this (it did a real window.location.href to CLOUD_API_URL, which
  // Capacitor's WebViewClient hands off to the system browser without an
  // allowNavigation entry) but no longer navigates at all -- it stays on
  // this bundled origin and points axios at the cloud API instead, the same
  // pattern "Local" already used. DashboardPage.tsx's "Devam Et" (resume a
  // cloud sim from local mode) still does a real cross-origin navigation and
  // will still fall back to the system browser on Android without this --
  // an accepted, narrower trade-off against breaking every native plugin.
};

export default config;
