//! Tauri backend: registers app state, the commands the frontend calls
//! with `invoke()`, the background sync timer and the tray icon.

mod account;
mod dashboard;
mod desktop;
mod state;
mod sync;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch (e.g. from the app menu while we're in
        // the tray) just shows the existing window and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            sync::spawn_timer(app.handle().clone());
            tray::setup(app.handle())?;
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            account::account_status,
            account::sign_in,
            account::sign_out,
            dashboard::dashboard,
            desktop::desktop_theme,
            sync::sync_now,
            tray::quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VitalGlass");
}
