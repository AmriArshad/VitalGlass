// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // On NVIDIA under Wayland, WebKitGTK's DMA-BUF renderer crashes with
    // "Error 71 (Protocol error) dispatching to Wayland display" because of
    // the driver's explicit-sync support. Turning explicit sync off for this
    // process fixes the crash and keeps GPU rendering (60 fps scrolling).
    // The older workaround, WEBKIT_DISABLE_DMABUF_RENDERER=1, also works but
    // copies every frame through the CPU (about 42 fps, visible stutter), so
    // it's left as a manual fallback; see the README's troubleshooting.
    #[cfg(target_os = "linux")]
    if std::path::Path::new("/proc/driver/nvidia/version").exists()
        && std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none()
    {
        // SAFETY: called before Tauri starts any threads. WebKit's web
        // process inherits the variable when it's spawned later.
        unsafe { std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1") };
    }

    vitalglass_lib::run();
}
