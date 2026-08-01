import type { CapacitorConfig } from '@capacitor/cli';

// Same identifier the desktop Tauri bundle uses (desktop/src-tauri/tauri.conf.json)
// so all three distributions (web, desktop, Android) read as one product family.
const config: CapacitorConfig = {
  appId: 'com.atabeylers.anatoliasim',
  appName: 'Anatolia Sim',
  webDir: 'dist',
};

export default config;
