//! Tauri backend: registers app state, the commands the frontend calls
//! with `invoke()`, and the background sync timer.

mod account;
mod dashboard;
mod desktop;
mod state;
mod sync;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            sync::spawn_timer(app.handle().clone());
            Ok(())
        })
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            account::account_status,
            account::sign_in,
            account::sign_out,
            dashboard::dashboard,
            desktop::desktop_theme,
            sync::sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VitalGlass");
}
