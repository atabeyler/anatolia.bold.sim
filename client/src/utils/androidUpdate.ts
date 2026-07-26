import { App } from '@capacitor/app';
import { registerPlugin } from '@capacitor/core';
import { isNativeAndroidApp, stopLocalServerIfRunning } from './nativeMode';
import { CLOUD_API_URL } from './cloud';

// Android's counterpart to desktop's Tauri updater (App.tsx's checkUpdate/
// installUpdate). There's no store here to do this for us -- distribution
// is a sideloaded APK -- and no silent background install either: Android
// always requires the user to explicitly tap through its own package
// installer for an app from outside Google Play. So this only ever gets
// as far as "here's a newer version, tap to download it"; the rest is the
// OS's normal APK-install flow, same as it would be for any manually
// downloaded APK.
//
// Goes through our own server (releases.rs's android_latest/android_asset)
// instead of hitting api.github.com directly, like an earlier version of
// this did -- that only worked because atabeyler/anatolia-sim was a public
// repo; our server holds the one token needed to keep reading releases once
// it isn't. It also sidesteps the CORS trap that broke the original direct
// github.com asset-download URL (redirects to a release-assets.
// githubusercontent.com URL with no CORS headers, so a WebView fetch()
// couldn't read the response) -- this server's own /android/asset/:id
// response carries whatever CORS headers this app's other API responses do.
const RELEASES_API_URL = `${CLOUD_API_URL}/api/updates/android/latest`;

export interface AndroidUpdateInfo {
  version: string;
  url: string;
}

interface ProxiedRelease {
  version: string;
  download_url: string;
}

// Mirrors android-release.yml's own x.y.z -> x*10000 + y*100 + z scheme,
// computed client-side now instead of read from a separately-fetched
// manifest -- see this file's top-of-file comment for why that manifest
// fetch was the actual bug.
function versionToCode(version: string): number | null {
  const parts = version.split('.').map(Number);
  if (parts.length !== 3 || parts.some(n => !Number.isFinite(n))) return null;
  const [x, y, z] = parts;
  return x * 10000 + y * 100 + z;
}

export async function checkForAndroidUpdate(): Promise<AndroidUpdateInfo | null> {
  const result = await checkForAndroidUpdateDetailed();
  return result.status === 'found' ? result.info : null;
}

export type AndroidUpdateCheckResult =
  | { status: 'found'; info: AndroidUpdateInfo }
  | { status: 'up-to-date' }
  | { status: 'error'; reason: string };

// Same check as checkForAndroidUpdate(), but never collapses a genuine
// failure (network error, GitHub unreachable, malformed response) into the
// same "up to date" result a real up-to-date check produces -- that silent
// collapse is exactly what made this look like "the update checker is
// broken" reports impossible to tell apart from "you're actually current"
// ones without a remote debugger attached. Settings' manual check button
// uses this to show the real reason instead of a misleading "✓ güncel".
export async function checkForAndroidUpdateDetailed(): Promise<AndroidUpdateCheckResult> {
  if (!isNativeAndroidApp()) return { status: 'error', reason: 'not-android' };

  let releaseRes: Response;
  try {
    releaseRes = await fetch(RELEASES_API_URL, { cache: 'no-store' });
  } catch (err) {
    return { status: 'error', reason: `fetch-failed: ${err instanceof Error ? err.message : String(err)}` };
  }
  if (!releaseRes.ok) {
    return { status: 'error', reason: `http-${releaseRes.status}` };
  }

  let release: ProxiedRelease;
  try {
    release = await releaseRes.json();
  } catch (err) {
    return { status: 'error', reason: `bad-json: ${err instanceof Error ? err.message : String(err)}` };
  }

  const latestCode = versionToCode(release.version);
  if (latestCode === null) {
    return { status: 'error', reason: `bad-tag: ${release.version}` };
  }

  let info: { build: string };
  try {
    info = await App.getInfo();
  } catch (err) {
    return { status: 'error', reason: `app-info-failed: ${err instanceof Error ? err.message : String(err)}` };
  }

  const installedCode = parseInt(info.build, 10);
  if (!Number.isFinite(installedCode)) {
    return { status: 'error', reason: `bad-installed-code: ${info.build}` };
  }
  if (latestCode <= installedCode) {
    return { status: 'up-to-date' };
  }
  return { status: 'found', info: { version: release.version, url: `${CLOUD_API_URL}${release.download_url}` } };
}

interface ApkUpdaterPlugin {
  install(options: { url: string; filename: string }): Promise<void>;
  addListener(eventName: 'downloadProgress', cb: (data: { percent: number }) => void): Promise<{ remove: () => void }>;
}

const ApkUpdater = registerPlugin<ApkUpdaterPlugin>('ApkUpdater');

export type InstallAndroidUpdateResult = 'ok' | 'permission-required' | 'error';

// Downloads the APK into the app's own cache dir and hands it straight to
// Android's native package installer (see ApkUpdaterPlugin.java) instead of
// opening an in-app browser pointed at the GitHub download page -- keeps the
// whole flow inside the app until Android's own "install this app?" system
// dialog, the closest a sideloaded APK can get to desktop's silent
// download-and-relaunch updater.
export async function installAndroidUpdate(
  update: AndroidUpdateInfo,
  onProgress?: (percent: number) => void,
): Promise<InstallAndroidUpdateResult> {
  let listener: { remove: () => void } | undefined;
  try {
    if (onProgress) {
      listener = await ApkUpdater.addListener('downloadProgress', (data) => onProgress(data.percent));
    }
    // See nativeMode.ts's own comment: don't leave a live subprocess of the
    // very app about to be replaced running through the install.
    await stopLocalServerIfRunning();
    await ApkUpdater.install({ url: update.url, filename: `anatolia-sim-${update.version}.apk` });
    return 'ok';
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return message.includes('install-permission-required') ? 'permission-required' : 'error';
  } finally {
    listener?.remove();
  }
}
