#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// The app opens on a small local chooser page (dist-chooser/index.html) that
// asks the user "Yerel" (local, offline, this machine only) or "Bulut"
// (cloud, synced across every device pointed at the same server). Cloud mode
// just navigates the window to the production URL. Local mode spawns this
// machine's own sim-server via the command below, then navigates to it --
// the two modes never run at once, and a local simulation is never visible
// from another device since it never touches the shared database.

use std::{
    net::TcpStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::Duration,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::Manager;

// CREATE_NO_WINDOW: sim-server.exe is a plain console binary (it also runs
// headless on Render), so spawning it normally from this GUI-subsystem app
// pops up a separate black console window showing its logs. This flag tells
// Windows not to allocate one for the child process.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct LocalServerState(Mutex<Option<Child>>);

fn wait_for_server(port: u16, attempts: usize) -> bool {
    for _ in 0..attempts {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

fn resolve_server_binary(app: &tauri::AppHandle) -> Option<PathBuf> {
    // Must match the destination path staged by desktop/stage-resources.mjs
    // and declared in tauri.conf.json's bundle.resources -- NOT the original
    // build location under rust/target/release. Tauri's bundler rewrites any
    // ".." path component in a resource glob to a literal "_up_" folder when
    // copying it into the installed app, so a resource declared with ".."
    // (as this used to be) never lands where a plain, non-escaping
    // `resolve_resource()` lookup like this one expects it.
    let rel = if cfg!(target_os = "windows") {
        "resources/sim-server.exe"
    } else {
        "resources/sim-server"
    };
    app.path_resolver().resolve_resource(rel)
}

fn resolve_server_cwd(binary: &PathBuf) -> Option<PathBuf> {
    let cwd = binary.parent()?.to_path_buf();
    if cwd.is_dir() {
        Some(cwd)
    } else {
        None
    }
}

#[tauri::command]
async fn start_local_server(app: tauri::AppHandle, state: tauri::State<'_, LocalServerState>) -> Result<(), String> {
    {
        let mut slot = state.0.lock().map_err(|err| err.to_string())?;
        if slot.is_some() {
            return Ok(()); // already running from an earlier choice this session
        }

        let binary = resolve_server_binary(&app).ok_or_else(|| "Rust server binary not found in resources".to_string())?;
        // sim-server expects to run from the directory that contains its
        // bundled static assets. The binary is copied into that same resource
        // folder, so using its parent directory is the safest way to keep the
        // executable and the files it serves together.
        let cwd = resolve_server_cwd(&binary)
            .or_else(|| app.path_resolver().resource_dir())
            .ok_or_else(|| "Server working directory not found".to_string())?;

        // sim.db must live somewhere the process can always write to --
        // the resource dir can be a read-only Program Files install --
        // so point it at Tauri's per-user app data directory instead.
        let data_dir = app.path_resolver().app_data_dir().ok_or_else(|| "App data directory not found".to_string())?;
        std::fs::create_dir_all(&data_dir).map_err(|err| err.to_string())?;

        let mut cmd = Command::new(binary);
        cmd.current_dir(cwd)
            .env("PORT", "3001")
            .env("NODE_ENV", "production")
            .env("SIM_DATA_DIR", &data_dir);

        // In a release Windows build this process itself runs windowless
        // (windows_subsystem = "windows" above), so it has no console stdio
        // handles for a child to inherit -- passing them along anyway plus
        // CREATE_NO_WINDOW risks the child panicking on a write to a broken
        // handle. Discard its stdio and suppress the console Windows would
        // otherwise auto-allocate for this console-subsystem child. In a dev
        // build there's a real terminal attached, so keep inheriting there.
        #[cfg(all(target_os = "windows", not(debug_assertions)))]
        cmd.stdout(Stdio::null()).stderr(Stdio::null()).creation_flags(CREATE_NO_WINDOW);
        #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let child = cmd.spawn().map_err(|err| err.to_string())?;
        *slot = Some(child);
    }

    if !wait_for_server(3001, 120) {
        return Err("Yerel sunucu 30 saniye içinde yanıt vermedi".to_string());
    }
    Ok(())
}

fn kill_local_server(state: &LocalServerState) {
    if let Ok(mut slot) = state.0.lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
        }
    }
}

// Lets the "back to Cloud/Local selection" button (LoginPage.tsx, desktop
// only) tear down an already-started local sim-server *before* relaunching
// the app to show dist-chooser again -- without this, relaunch() restarts
// the whole process without ever going through on_window_event's
// CloseRequested cleanup below, so the old server would be left running as
// an orphan (and could still be holding port 3001 by the time the freshly
// relaunched app's own start_local_server tries to bind it). A no-op if
// Cloud was chosen instead (nothing was ever started).
#[tauri::command]
fn stop_local_server(state: tauri::State<'_, LocalServerState>) {
    kill_local_server(&state);
}

fn main() {
    tauri::Builder::default()
        .manage(LocalServerState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![start_local_server, stop_local_server])
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
                if let Some(state) = event.window().try_state::<LocalServerState>() {
                    kill_local_server(&state);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
