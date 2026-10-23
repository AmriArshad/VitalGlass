//! Tauri backend: registers app state and the commands the frontend calls
//! with `invoke()`.

mod account;

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
            Ok(())
        })
        .manage(account::AppState::default())
        .invoke_handler(tauri::generate_handler![
            account::account_status,
            account::sign_in,
            account::sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VitalGlass");
}
