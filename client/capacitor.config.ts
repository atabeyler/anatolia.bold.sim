import type { CapacitorConfig } from '@capacitor/cli';

// Same identifier the desktop Tauri bundle uses (desktop/src-tauri/tauri.conf.json)
// so all three distributions (web, desktop, Android) read as one product family.
const config: CapacitorConfig = {
  appId: 'com.atabeylers.anatoliasim',
  appName: 'Anatolia Sim',
  webDir: 'dist',
  // NativeModeGate.tsx's "Cloud" choice does a real window.location.href
  // navigation to CLOUD_API_URL (anatolia-bold-sim.fly.dev) -- without this
  // host in allowNavigation, Capacitor's WebViewClient treats it as an
  // external link and hands it off to the system browser instead of loading
  // it in-app, which is exactly what "Cloud" is supposed to do here.
  server: {
    allowNavigation: ['anatolia-bold-sim.fly.dev'],
  },
};

export default config;
