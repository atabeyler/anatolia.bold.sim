package com.atabeylers.anatoliasim;

import android.util.Log;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.InetSocketAddress;
import java.net.Socket;

// Android counterpart to desktop/src-tauri/src/main.rs's start_local_server:
// spawns the same sim-server binary (cross-compiled for arm64-v8a and bundled
// as a "native library" -- Android only ever extracts jniLibs/*.so with
// execute permission, so the plain ELF binary is just renamed to satisfy
// that, it is not really a shared object) so "Yerel" mode reuses the exact
// same REST API the client already speaks to on the web and on desktop,
// with no client-side data-layer rewrite. Runs on this device's own CPU,
// only while the app is in the foreground (no foreground service), which
// matches the product's actual requirement -- see AGENTS.md.
@CapacitorPlugin(name = "LocalServer")
public class LocalServerPlugin extends Plugin {
    private static final String TAG = "LocalServerPlugin";
    private static final int PORT = 3001;
    private static final int WAIT_ATTEMPTS = 120;
    private static final long WAIT_INTERVAL_MS = 250;

    private Process process;

    @PluginMethod
    public void start(PluginCall call) {
        synchronized (this) {
            if (process != null && process.isAlive()) {
                call.resolve();
                return;
            }
        }

        new Thread(() -> {
            try {
                String binaryPath = getContext().getApplicationInfo().nativeLibraryDir + "/libsimserver.so";
                File binary = new File(binaryPath);
                if (!binary.exists()) {
                    call.reject("sim-server binary not found at " + binaryPath);
                    return;
                }

                File dataDir = new File(getContext().getFilesDir(), "sim-data");
                if (!dataDir.exists() && !dataDir.mkdirs()) {
                    call.reject("Could not create local data directory");
                    return;
                }

                ProcessBuilder builder = new ProcessBuilder(binaryPath);
                builder.environment().put("PORT", String.valueOf(PORT));
                builder.environment().put("NODE_ENV", "production");
                builder.environment().put("SIM_DATA_DIR", dataDir.getAbsolutePath());
                builder.redirectErrorStream(true);
                builder.directory(dataDir);

                Process started = builder.start();
                synchronized (this) {
                    process = started;
                }
                drainOutput(started);

                if (!waitForServer()) {
                    call.reject("Yerel sunucu zamanında yanıt vermedi");
                    return;
                }
                call.resolve();
            } catch (IOException err) {
                Log.e(TAG, "Failed to start local server", err);
                call.reject("Yerel sunucu başlatılamadı: " + err.getMessage());
            }
        }).start();
    }

    @PluginMethod
    public void stop(PluginCall call) {
        synchronized (this) {
            if (process != null) {
                process.destroy();
                process = null;
            }
        }
        call.resolve();
    }

    @PluginMethod
    public void isRunning(PluginCall call) {
        JSObject result = new JSObject();
        synchronized (this) {
            result.put("running", process != null && process.isAlive());
        }
        call.resolve(result);
    }

    @Override
    protected void handleOnDestroy() {
        synchronized (this) {
            if (process != null) {
                process.destroy();
                process = null;
            }
        }
        super.handleOnDestroy();
    }

    private boolean waitForServer() {
        for (int i = 0; i < WAIT_ATTEMPTS; i++) {
            try (Socket socket = new Socket()) {
                socket.connect(new InetSocketAddress("127.0.0.1", PORT), 250);
                return true;
            } catch (IOException ignored) {
                try {
                    Thread.sleep(WAIT_INTERVAL_MS);
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return false;
                }
            }
        }
        return false;
    }

    // sim-server's stdout/stderr must be drained on Android -- an
    // unconsumed pipe fills its OS buffer and blocks the child process,
    // unlike a desktop console which just keeps scrolling.
    private void drainOutput(Process started) {
        new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(new InputStreamReader(started.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    Log.d(TAG, line);
                }
            } catch (IOException ignored) {
                // process ended or was destroyed -- nothing left to drain
            }
        }).start();
    }
}
