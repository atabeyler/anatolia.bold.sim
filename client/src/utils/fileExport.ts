import { Filesystem, Directory, Encoding } from '@capacitor/filesystem';
import { Share } from '@capacitor/share';
import { registerPlugin } from '@capacitor/core';
import { isNativeAndroidApp } from './nativeMode';

interface FileOpenerPlugin {
  open(options: { path: string; mimeType: string }): Promise<void>;
}
const FileOpener = registerPlugin<FileOpenerPlugin>('FileOpener');

export interface SavedFile {
  uri: string;
  mimeType: string;
  filename: string;
}

// Writes the file to disk -- Android: the app's own cache dir via
// Filesystem; web/desktop: an in-memory Blob URL -- without doing anything
// with it yet. Share and Open are separate, explicit follow-up actions (see
// below) so the user picks what happens to a generated file instead of one
// button silently doing both, and Android WebViews don't reliably surface a
// blob-URL `<a download>` click to the user the way a real browser tab
// does: the old single-step version of this reported success with no
// Downloads-folder entry and nothing in the notification shade -- nowhere a
// user would think to look.
export async function saveFile(filename: string, mimeType: string, data: string, isBase64: boolean): Promise<SavedFile> {
  if (isNativeAndroidApp()) {
    const { uri } = await Filesystem.writeFile({
      path: filename,
      data,
      directory: Directory.Cache,
      ...(isBase64 ? {} : { encoding: Encoding.UTF8 }),
    });
    return { uri, mimeType, filename };
  }
  const blob = isBase64
    ? new Blob([Uint8Array.from(atob(data), c => c.charCodeAt(0))], { type: mimeType })
    : new Blob([data], { type: mimeType });
  return { uri: URL.createObjectURL(blob), mimeType, filename };
}

// Hands the file to another app to send somewhere -- mail, chat, cloud
// storage, ... -- via the OS share sheet.
export async function shareFile(file: SavedFile): Promise<void> {
  if (isNativeAndroidApp()) {
    await Share.share({ title: file.filename, files: [file.uri], dialogTitle: file.filename });
    return;
  }
  // Actually sharing a file (not just a link) via the Web Share API needs
  // File objects (Level 2, inconsistent desktop support) rather than a Blob
  // URL -- the reliable behavior everywhere else is a direct download.
  const a = document.createElement('a');
  a.href = file.uri;
  a.download = file.filename;
  a.click();
}

// Opens the file in place with whatever app the OS considers its default
// viewer -- distinct from handing it to another app to send elsewhere.
export async function openFile(file: SavedFile): Promise<void> {
  if (isNativeAndroidApp()) {
    await FileOpener.open({ path: file.uri, mimeType: file.mimeType });
    return;
  }
  window.open(file.uri, '_blank');
}
