use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::Manager;

struct ResizeThrottle(Mutex<Instant>);

// ── Módulos do sistema ──
mod calendar;
mod commands;
mod config;
mod constants;
mod db;
mod evolution;
mod export;
mod generators;
mod hierarchy;
mod market;
mod models;
mod news;
mod promotion;
mod rivalry;
mod simulation;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let base_dir = app.path().app_data_dir()
                .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;
            let config = config::app_config::AppConfig::load_or_default(&base_dir);
            
            if let Some(window) = app.get_webview_window("main") {
                // Aplicar tamanho
                let _ = window.set_size(tauri::LogicalSize::new(config.window_width, config.window_height));

                // Maximizar se necessário
                if config.window_maximized {
                    let _ = window.maximize();
                }
            }

            // Estado para throttle de resize (evita I/O excessivo)
            app.manage(ResizeThrottle(Mutex::new(Instant::now())));

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Resized(size) = event {
                let app = window.app_handle();

                // Throttle: ignora eventos intermediários, salva no máximo a cada 500ms
                let throttle = app.state::<ResizeThrottle>();
                let mut last = throttle.0.lock().unwrap();
                let now = Instant::now();
                if now.duration_since(*last) < Duration::from_millis(500) {
                    return;
                }
                *last = now;
                drop(last);

                let base_dir = app.path().app_data_dir().unwrap();
                let mut config = config::app_config::AppConfig::load_or_default(&base_dir);

                let is_maximized = window.is_maximized().unwrap_or(false);
                config.window_maximized = is_maximized;

                if !is_maximized {
                    let logical_size = size.to_logical(window.scale_factor().unwrap_or(1.0));
                    config.window_width = logical_size.width;
                    config.window_height = logical_size.height;
                }

                let _ = config.save();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::config::get_config,
            commands::config::update_config,
            commands::career_commands::create_career,
            commands::career_commands::load_career,
            commands::career_commands::advance_season,
            commands::career_commands::advance_market_week,
            commands::career_commands::get_preseason_state,
            commands::career_commands::finalize_preseason,
            commands::career_commands::get_player_proposals,
            commands::career_commands::respond_to_proposal,
            commands::career_commands::get_news,
            commands::career_commands::delete_career,
            commands::career_commands::list_saves,
            commands::career_commands::get_drivers_by_category,
            commands::career_commands::get_teams_standings,
            commands::career_commands::get_race_results_by_category,
            commands::career_commands::get_previous_champions,
            commands::career_commands::get_calendar_for_category,
            commands::career_commands::get_driver,
            commands::career_commands::get_driver_detail,
            commands::race::simulate_race_weekend,
            commands::window::minimize_window,
            commands::window::toggle_maximize_window,
            commands::window::close_window,
            commands::window::get_window_maximized,
            commands::window::toggle_fullscreen_window,
            commands::window::get_window_fullscreen,
            commands::save::flush_save,
            commands::save::create_season_backup,
            commands::save::list_backups,
            commands::save::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
