pub mod career;
pub mod career_commands;
pub mod career_detail;
pub mod career_types;
pub mod config;
pub mod export;
pub mod market;
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
