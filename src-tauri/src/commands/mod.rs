pub mod calendar;
pub mod career;
pub mod career_commands;
pub mod career_detail;
pub mod career_types;
pub mod config;
pub mod convocation;
pub mod export;
pub mod market;
pub mod news_editorial;
pub mod news_helpers;
pub mod news_tab;
pub mod race;
pub mod race_history;
pub mod save;
pub mod season;
pub mod window;

/// Comando de teste — será removido depois
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Olá, {}! Backend Rust conectado.", name)
}
