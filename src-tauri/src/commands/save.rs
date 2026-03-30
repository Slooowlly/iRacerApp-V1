use chrono::Local;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::config::app_config::{AppConfig, SaveMeta};
use crate::db::connection::Database;

// ── Structs públicas ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct FlushResult {
    pub last_saved: String,
}

#[derive(Debug, serde::Serialize)]
pub struct BackupInfo {
    /// Número da temporada que o backup representa.
    pub season_number: u32,
    /// Nome do arquivo (ex.: "temporada_003.db").
    pub file_name: String,
    /// Caminho completo (para uso interno / debug).
    pub file_path: String,
    /// Tamanho em KB.
    pub size_kb: u64,
    /// Data de modificação do arquivo (ISO 8601).
    pub modified_at: String,
}

// ── flush_save ────────────────────────────────────────────────────────────────
//
// Consolida o estado atual do save sem regravar dados de jogo.
// 1. Verifica que o career_dir existe
// 2. Abre o banco e faz WAL checkpoint
// 3. Atualiza meta.json com last_saved = agora
// 4. Retorna o timestamp para o frontend limpar isDirty
//
#[tauri::command]
pub async fn flush_save(app: AppHandle, career_id: String) -> Result<FlushResult, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let career_number = parse_career_number(&career_id)?;

    let career_dir = config.career_dir(career_number);
    if !career_dir.exists() {
        return Err(format!("Save não encontrado: {career_id}"));
    }

    let db_path = config.career_db_path(career_number);
    let db = Database::open_existing(&db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;
    checkpoint_wal(&db)?;

    let meta_path = config.career_meta_path(career_number);
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    update_meta_timestamps(&meta_path, |meta| {
        meta.last_saved = Some(now.clone());
    })?;

    Ok(FlushResult { last_saved: now })
}

// ── create_season_backup (Tauri command) ──────────────────────────────────────
//
// Cria um backup canônico de fim de temporada.
// Convenção de nome: backups/temporada_NNN.db
// Política de retenção: 1 arquivo por temporada — sobrescreve se já existe.
//
#[tauri::command]
pub async fn create_season_backup(
    app: AppHandle,
    career_id: String,
    season_number: u32,
) -> Result<BackupInfo, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let career_number = parse_career_number(&career_id)?;
    let career_dir = config.career_dir(career_number);

    if !career_dir.exists() {
        return Err(format!("Save não encontrado: {career_id}"));
    }

    let db_path = config.career_db_path(career_number);
    let meta_path = config.career_meta_path(career_number);

    let info = backup_season_internal(&db_path, &career_dir, season_number, &meta_path)?;
    Ok(info)
}

// ── list_backups ──────────────────────────────────────────────────────────────
//
// Lista todos os backups disponíveis para uma carreira.
// Lê career_NNN/backups/ e retorna metadados de cada snapshot.
//
#[tauri::command]
pub async fn list_backups(app: AppHandle, career_id: String) -> Result<Vec<BackupInfo>, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let career_number = parse_career_number(&career_id)?;
    let career_dir = config.career_dir(career_number);

    if !career_dir.exists() {
        return Err(format!("Save não encontrado: {career_id}"));
    }

    let backups_dir = career_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = scan_backups_dir(&backups_dir);
    backups.sort_by(|a, b| a.season_number.cmp(&b.season_number));
    Ok(backups)
}

// ── restore_backup ────────────────────────────────────────────────────────────
//
// Restaura um backup de temporada específico.
// Política de segurança: salva o career.db atual como career.db.bak antes de restaurar.
//
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    career_id: String,
    season_number: u32,
) -> Result<(), String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let career_number = parse_career_number(&career_id)?;
    let career_dir = config.career_dir(career_number);

    if !career_dir.exists() {
        return Err(format!("Save não encontrado: {career_id}"));
    }

    let backup_path = career_dir
        .join("backups")
        .join(season_backup_filename(season_number));

    if !backup_path.exists() {
        return Err(format!(
            "Backup da temporada {} não encontrado.",
            season_number
        ));
    }

    let db_path = config.career_db_path(career_number);

    // 1. WAL checkpoint no banco atual antes de tocar os arquivos
    if db_path.exists() {
        let db = Database::open_existing(&db_path)
            .map_err(|e| format!("Falha ao abrir banco atual: {e}"))?;
        checkpoint_wal(&db)?;
        drop(db);

        // 2. Salvar cópia de segurança do banco atual
        let safety = career_dir.join("career.db.bak");
        std::fs::copy(&db_path, &safety)
            .map_err(|e| format!("Falha ao criar cópia de segurança do banco atual: {e}"))?;

        // 3. Remover WAL e SHM órfãos que possam confundir o SQLite após cópia
        let _ = std::fs::remove_file(career_dir.join("career.db-wal"));
        let _ = std::fs::remove_file(career_dir.join("career.db-shm"));
    }

    // 4. Copiar o backup escolhido para career.db
    std::fs::copy(&backup_path, &db_path).map_err(|e| format!("Falha ao restaurar backup: {e}"))?;

    Ok(())
}

// ── Função pura interna — chamada de career.rs e do command Tauri ─────────────
//
// Cria o snapshot canônico da temporada:
//   1. WAL checkpoint (garante .db completo)
//   2. Cria pasta backups/ se necessário
//   3. Copia career.db → backups/temporada_NNN.db (sobrescreve se existe)
//   4. Atualiza last_backup e last_saved no meta.json
//
// Essa função é síncrona e não tem dependência de Tauri/AppHandle —
// pode ser invocada livremente de qualquer parte do backend.
//
pub(crate) fn backup_season_internal(
    db_path: &Path,
    career_dir: &Path,
    season_number: u32,
    meta_path: &Path,
) -> Result<BackupInfo, String> {
    // 1. Abrir banco
    let db = Database::open_existing(db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;

    // 2. Garantir pasta backups/
    let backups_dir = career_dir.join("backups");
    std::fs::create_dir_all(&backups_dir)
        .map_err(|e| format!("Falha ao criar diretório de backups: {e}"))?;

    // 3. Destino: backups/temporada_NNN.db (sobrescreve — retenção 1 por temporada)
    // Database::backup() faz WAL checkpoint(TRUNCATE) + cópia atômica.
    let filename = season_backup_filename(season_number);
    let dest = backups_dir.join(&filename);
    db.backup(&dest)
        .map_err(|e| format!("Falha ao criar backup: {e}"))?;

    // 4. Atualizar meta.json
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    update_meta_timestamps(meta_path, |meta| {
        meta.last_backup = Some(now.clone());
        meta.last_saved = Some(now.clone());
    })?;

    // 5. Coletar metadados do arquivo criado para o caller
    let info = file_backup_info(&dest, season_number, &filename);
    Ok(info)
}

// ── Helpers privados ──────────────────────────────────────────────────────────

/// Convenção de nome de backup: temporada_NNN.db
fn season_backup_filename(season_number: u32) -> String {
    format!("temporada_{:03}.db", season_number)
}

/// Faz WAL checkpoint no banco aberto.
fn checkpoint_wal(db: &Database) -> Result<(), String> {
    db.conn
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Falha no WAL checkpoint: {e}"))
}

/// Lê meta.json, aplica uma mutação via closure, regrava.
fn update_meta_timestamps<F>(meta_path: &Path, mutate: F) -> Result<(), String>
where
    F: FnOnce(&mut SaveMeta),
{
    let content =
        std::fs::read_to_string(meta_path).map_err(|e| format!("Falha ao ler meta.json: {e}"))?;
    let mut meta: SaveMeta =
        serde_json::from_str(&content).map_err(|e| format!("Falha ao parsear meta.json: {e}"))?;
    mutate(&mut meta);
    let updated = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Falha ao serializar meta.json: {e}"))?;
    std::fs::write(meta_path, updated).map_err(|e| format!("Falha ao gravar meta.json: {e}"))
}

/// Parseia número da carreira de "career_001" ou "001" ou "1".
pub(crate) fn parse_career_number(career_id: &str) -> Result<u32, String> {
    let s = career_id.trim_start_matches("career_");
    s.parse::<u32>()
        .map_err(|_| format!("career_id inválido: '{career_id}'"))
}

/// Varre backups_dir e retorna metadados de cada temporada_NNN.db encontrado.
fn scan_backups_dir(backups_dir: &Path) -> Vec<BackupInfo> {
    let Ok(entries) = std::fs::read_dir(backups_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let season_number = parse_backup_filename(&name)?;
            let path = entry.path();
            Some(file_backup_info(&path, season_number, &name))
        })
        .collect()
}

/// Tenta extrair o número de temporada de "temporada_NNN.db" ou "season_NNN.db" (legado).
fn parse_backup_filename(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".db")?;
    // suporte a "temporada_NNN" (atual) e "season_NNN" (legado passo anterior)
    let digits = stem
        .strip_prefix("temporada_")
        .or_else(|| stem.strip_prefix("season_"))?;
    digits.parse::<u32>().ok()
}

/// Constrói um BackupInfo a partir do caminho do arquivo.
fn file_backup_info(path: &PathBuf, season_number: u32, file_name: &str) -> BackupInfo {
    let metadata = std::fs::metadata(path);
    let size_kb = metadata.as_ref().map(|m| m.len() / 1024).unwrap_or(0);
    let modified_at = metadata
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                // Converter epoch segundos → ISO 8601 simples
                let secs = d.as_secs();
                let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
                    .unwrap_or_default()
                    .with_timezone(&Local);
                dt.format("%Y-%m-%dT%H:%M:%S").to_string()
            })
        })
        .unwrap_or_default();

    BackupInfo {
        season_number,
        file_name: file_name.to_string(),
        file_path: path.to_string_lossy().to_string(),
        size_kb,
        modified_at,
    }
}
