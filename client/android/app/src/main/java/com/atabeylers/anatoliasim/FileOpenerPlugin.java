package com.atabeylers.anatoliasim;

import android.content.Intent;
import android.net.Uri;
import androidx.core.content.FileProvider;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.File;

// Opens a file already written to disk (e.g. by @capacitor/filesystem) with
// whatever app Android considers the default viewer for its MIME type --
// Intent.ACTION_VIEW through the same FileProvider-URI pattern
// ApkUpdaterPlugin already uses for the "install this APK" prompt,
// generalized to any file/MIME type. Distinct from @capacitor/share's
// ACTION_SEND: that hands a file to another app to *send* somewhere (mail,
// chat, ...); this asks Android to *display* it in place -- a separate
// "Aç" action from "Paylaş".
@CapacitorPlugin(name = "FileOpener")
public class FileOpenerPlugin extends Plugin {
    @PluginMethod
    public void open(PluginCall call) {
        String path = call.getString("path");
        String mimeType = call.getString("mimeType", "*/*");
        if (path == null) {
            call.reject("Missing path");
            return;
        }
        try {
            // Filesystem.writeFile returns a file:// URI string, not a bare
            // path -- Uri.parse(...).getPath() strips the scheme back down
            // to the filesystem path FileProvider.getUriForFile needs.
            String filePath = Uri.parse(path).getPath();
            File file = new File(filePath != null ? filePath : path);
            Uri contentUri = FileProvider.getUriForFile(
                    getContext(), getContext().getPackageName() + ".fileprovider", file);
            Intent intent = new Intent(Intent.ACTION_VIEW);
            intent.setDataAndType(contentUri, mimeType);
            intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_GRANT_READ_URI_PERMISSION);
            getContext().startActivity(intent);
            call.resolve();
        } catch (Exception err) {
            call.reject("Dosya açılamadı: " + err.getMessage());
        }
    }
}
