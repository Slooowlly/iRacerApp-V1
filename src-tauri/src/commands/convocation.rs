use tauri::{AppHandle, Manager};

use crate::config::app_config::AppConfig;
use crate::convocation::{
    advance_to_convocation_window as adv_fn, encerrar_bloco_especial as encerrar_fn,
    iniciar_bloco_especial as iniciar_fn, run_convocation_window as run_fn,
    run_pos_especial as pos_fn, ConvocationResult, PosEspecialResult,
};
use crate::db::connection::Database;

/// BlocoRegular → JanelaConvocacao.
#[tauri::command]
pub fn advance_to_convocation_window(career_id: String, app: AppHandle) -> Result<(), String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.saves_dir().join(&career_id).join("career.db");
    let db = Database::open_existing(&db_path).map_err(|e| e.to_string())?;
    adv_fn(&db.conn).map_err(|e| e.to_string())
}

/// Monta os grids das categorias especiais (permanece em JanelaConvocacao).
#[tauri::command]
pub fn run_convocation_window(
    career_id: String,
    app: AppHandle,
) -> Result<ConvocationResult, String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.saves_dir().join(&career_id).join("career.db");
    let db = Database::open_existing(&db_path).map_err(|e| e.to_string())?;
    run_fn(&db.conn).map_err(|e| e.to_string())
}

/// JanelaConvocacao → BlocoEspecial.
#[tauri::command]
pub fn iniciar_bloco_especial(career_id: String, app: AppHandle) -> Result<(), String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.saves_dir().join(&career_id).join("career.db");
    let db = Database::open_existing(&db_path).map_err(|e| e.to_string())?;
    iniciar_fn(&db.conn).map_err(|e| e.to_string())
}

/// BlocoEspecial → PosEspecial (fim esportivo das corridas especiais).
#[tauri::command]
pub fn encerrar_bloco_especial(career_id: String, app: AppHandle) -> Result<(), String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.saves_dir().join(&career_id).join("career.db");
    let db = Database::open_existing(&db_path).map_err(|e| e.to_string())?;
    encerrar_fn(&db.conn).map_err(|e| e.to_string())
}

/// Desmontagem do bloco especial: expira contratos, limpa lineups, gera notícias.
/// Permanece em PosEspecial após execução.
#[tauri::command]
pub fn run_pos_especial(career_id: String, app: AppHandle) -> Result<PosEspecialResult, String> {
    let base_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.saves_dir().join(&career_id).join("career.db");
    let db = Database::open_existing(&db_path).map_err(|e| e.to_string())?;
    pos_fn(&db.conn).map_err(|e| e.to_string())
}
