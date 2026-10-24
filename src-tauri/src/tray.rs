//! System tray icon. Closing the window hides it to the tray, so the
//! 15-minute sync keeps running; Quit in the tray menu (or Ctrl+Q in the
//! window) exits for real.
//!
//! On Linux the tray is a StatusNotifierItem (libayatana-appindicator),
//! which only supports a menu, not click events, so everything is in the
//! menu.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show VitalGlass", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &sync, &separator, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("VitalGlass")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "sync" => {
                crate::sync::start(app.clone());
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Bring the main window back from the tray (or from behind other windows).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Hide instead of closing, so the app keeps syncing in the tray.
pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// `quit` command: lets the frontend's Ctrl+Q exit the app.
#[tauri::command]
pub fn quit(window: WebviewWindow) {
    window.app_handle().exit(0);
}
