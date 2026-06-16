// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's DMABUF renderer triggers a Wayland protocol crash
    // ("Gdk Error 71") on many Linux/Mesa/Nvidia setups. Disabling it before
    // the webview starts avoids the crash. Honor an explicit override if set.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    newt_todo_desktop_lib::run()
}
