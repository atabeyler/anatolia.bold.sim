import { isNativeAndroidApp } from './nativeMode';

// Accounts only ever live in the cloud (Postgres) database now. The
// desktop app's "Yerel" mode runs a local sim-server (SQLite, no users
// table of its own) purely for simulation compute/storage, and Android's
// native Yerel mode does the same. Whenever either of those local hosts is
// active, auth and other cloud-owned calls must be aimed at the cloud
// explicitly; when the page is already being served by the cloud, a
// relative path already resolves there.
function shouldUseCloudApi(): boolean {
  return isLocalOrigin() || isNativeAndroidApp();
}
export const CLOUD_API_URL = 'https://anatolia-sim.onrender.com';

export function isLocalOrigin(): boolean {
  if (typeof window === 'undefined') return false;
  return window.location.hostname === '127.0.0.1' || window.location.hostname === 'localhost';
}

// For endpoints that must always go through the cloud (auth), regardless of
// which server is currently hosting the page.
export function authUrl(path: string): string {
  return shouldUseCloudApi() ? `${CLOUD_API_URL}${path}` : path;
}

// Generic helper for cloud-owned endpoints that should not be routed to the
// local sidecar when the desktop app is running in Yerel mode.
export function cloudUrl(path: string): string {
  return shouldUseCloudApi() ? `${CLOUD_API_URL}${path}` : path;
}
