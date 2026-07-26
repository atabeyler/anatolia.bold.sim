// Accounts only ever live in the cloud (Postgres) database now -- the
// desktop app's "Yerel" mode runs a local sim-server (SQLite, no users
// table of its own) purely for simulation compute/storage. Whenever the
// page is being served by that local sidecar (127.0.0.1), auth and
// cross-device simulation-listing calls must be aimed at the cloud
// explicitly; when the page is already being served by the cloud, a
// relative path already resolves there.
export const CLOUD_API_URL = 'https://anatolia-sim.onrender.com';

export function isLocalOrigin(): boolean {
  if (typeof window === 'undefined') return false;
  return window.location.hostname === '127.0.0.1' || window.location.hostname === 'localhost';
}

// For endpoints that must always go through the cloud (auth), regardless of
// which server is currently hosting the page.
export function authUrl(path: string): string {
  return isLocalOrigin() ? `${CLOUD_API_URL}${path}` : path;
}
