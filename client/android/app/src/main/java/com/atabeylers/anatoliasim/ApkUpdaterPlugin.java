package com.atabeylers.anatoliasim;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;
import androidx.core.content.FileProvider;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

// Android counterpart to desktop's Tauri updater (App.tsx's checkUpdate/
// installUpdate): downloads the release APK straight into the app's own
// cache dir and hands it to Android's native package installer, instead of
// opening an in-app browser pointed at a github.com/githubusercontent.com
// download page. The flow stays inside the app until Android's own "install
// this app?" system dialog -- the closest a sideloaded APK can get to
// desktop's silent download-and-relaunch.
@CapacitorPlugin(name = "ApkUpdater")
public class ApkUpdaterPlugin extends Plugin {
    private static final String TAG = "ApkUpdaterPlugin";

    @PluginMethod
    public void install(PluginCall call) {
        String url = call.getString("url");
        String filename = call.getString("filename", "update.apk");
        if (url == null) {
            call.reject("Missing url");
            return;
        }

        // Android O+ gates "install from this source" as a per-app grant
        // separate from the manifest permission -- without it, the install
        // intent below would just land on Android's own "blocked" screen
        // with no way back into our flow. Send the user straight to the one
        // settings screen that grants it for this app and let the caller
        // ask them to retry, rather than firing an intent guaranteed to be
        // rejected.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
                && !getContext().getPackageManager().canRequestPackageInstalls()) {
            Intent settingsIntent = new Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:" + getContext().getPackageName()));
            settingsIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            getContext().startActivity(settingsIntent);
            call.reject("install-permission-required");
            return;
        }

        new Thread(() -> downloadAndInstall(call, url, filename)).start();
    }

    private void downloadAndInstall(PluginCall call, String urlString, String filename) {
        try {
            File dir = new File(getContext().getCacheDir(), "updates");
            if (!dir.exists() && !dir.mkdirs()) {
                call.reject("Could not create download directory");
                return;
            }
            File target = new File(dir, filename);

            HttpURLConnection conn = (HttpURLConnection) new URL(urlString).openConnection();
            conn.setInstanceFollowRedirects(true);
            conn.connect();
            int status = conn.getResponseCode();
            if (status != HttpURLConnection.HTTP_OK) {
                call.reject("Download failed: HTTP " + status);
                return;
            }

            long total = conn.getContentLengthLong();
            long downloaded = 0;
            long lastNotifiedAt = 0;

            try (InputStream in = conn.getInputStream(); OutputStream out = new FileOutputStream(target)) {
                byte[] buffer = new byte[8192];
                int read;
                while ((read = in.read(buffer)) != -1) {
                    out.write(buffer, 0, read);
                    downloaded += read;
                    if (total > 0 && downloaded - lastNotifiedAt > total / 50) {
                        lastNotifiedAt = downloaded;
                        JSObject progress = new JSObject();
                        progress.put("percent", (int) (downloaded * 100 / total));
                        notifyListeners("downloadProgress", progress);
                    }
                }
            }

            Uri contentUri = FileProvider.getUriForFile(
                    getContext(), getContext().getPackageName() + ".fileprovider", target);
            Intent install = new Intent(Intent.ACTION_VIEW);
            install.setDataAndType(contentUri, "application/vnd.android.package-archive");
            install.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_GRANT_READ_URI_PERMISSION);
            getContext().startActivity(install);
            call.resolve();

            // The installer runs in its own task (FLAG_ACTIVITY_NEW_TASK above),
            // but this app's own process/activity staying alive while
            // PackageManager tries to replace the very APK backing it is a
            // known way for the system installer to stall indefinitely on
            // "Yükleniyor..." (reported: works fine when the same APK is
            // downloaded and installed manually outside the app, but hangs
            // when triggered from inside a running instance of the app being
            // replaced). Get out of the way immediately so the swap has a
            // clean shot -- finishAffinity() only tears down our own task,
            // never the independent installer task just launched above.
            Activity activity = getActivity();
            if (activity != null) {
                activity.runOnUiThread(activity::finishAffinity);
            }
        } catch (Exception err) {
            Log.e(TAG, "Update download/install failed", err);
            call.reject("İndirme başarısız: " + err.getMessage());
        }
    }
}
