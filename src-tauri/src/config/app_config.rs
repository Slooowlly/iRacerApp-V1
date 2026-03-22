use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── SaveMeta — espelha career_NNN/meta.json ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMeta {
    pub career_number: u32,
    pub player_name: String,
    pub current_season: u32,
    pub current_year: u32,
    pub created_at: String,
    pub last_played: String,
    /// Última consolidação explícita do save (flush_save).
    #[serde(default)]
    pub last_saved: Option<String>,
    /// Último backup criado (create_season_backup).
    #[serde(default)]
    pub last_backup: Option<String>,
    #[serde(default)]
    pub team_name: Option<String>,
    pub category: String,
    pub difficulty: String,
    #[serde(default)]
    pub total_races: i32,
}

// ── AppConfig — espelha config.json ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub last_career: Option<u32>,
    pub language: String,
    pub autosave_enabled: bool,
    
    // Window state
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,

    // iRacing Paths
    pub airosters_path: Option<PathBuf>,
    pub aiseasons_path: Option<PathBuf>,

    /// Diretório base do app (AppData/Local/iracing-career-simulator).
    /// Não persiste no JSON — preenchido em tempo de execução.
    #[serde(skip)]
    pub base_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            version: "1.0.0".to_string(),
            last_career: None,
            language: "pt-BR".to_string(),
            autosave_enabled: true,
            window_width: 1280,
            window_height: 720,
            window_maximized: false,
            airosters_path: None,
            aiseasons_path: None,
            base_dir: PathBuf::new(),
        }
    }
}

impl AppConfig {
    // ── Carregar ou criar padrão ──────────────────────────────────────────────

    pub fn load_or_default(base_dir: &Path) -> Self {
        let path = base_dir.join("config.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<AppConfig>(&content) {
                Ok(mut cfg) => {
                    cfg.base_dir = base_dir.to_path_buf();
                    return cfg;
                }
                Err(e) => {
                    eprintln!("[config] config.json corrompido: {e}. Fazendo backup e usando configuração padrão.");
                    let backup = path.with_extension("json.bak");
                    let _ = std::fs::copy(&path, &backup);
                }
            }
        }
        let mut cfg = AppConfig::default();
        cfg.base_dir = base_dir.to_path_buf();
        cfg
    }

    /// Persistir config.json no disco.
    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.base_dir)
            .map_err(|e| format!("Falha ao criar diretório base: {e}"))?;
        let path = self.base_dir.join("config.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar config: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Falha ao gravar config.json: {e}"))
    }

    // ── Helpers de caminho ────────────────────────────────────────────────────

    pub fn saves_dir(&self) -> PathBuf {
        self.base_dir.join("saves")
    }

    pub fn career_dir(&self, career_number: u32) -> PathBuf {
        self.saves_dir()
            .join(format!("career_{:03}", career_number))
    }

    pub fn career_db_path(&self, career_number: u32) -> PathBuf {
        self.career_dir(career_number).join("career.db")
    }

    pub fn career_meta_path(&self, career_number: u32) -> PathBuf {
        self.career_dir(career_number).join("meta.json")
    }

    /// Retorna o próximo número de carreira disponível (max existente + 1).
    pub fn next_career_number(&self) -> u32 {
        let saves = self.saves_dir();
        if !saves.exists() {
            return 1;
        }
        let max = std::fs::read_dir(&saves)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name();
                        let s = name.to_string_lossy();
                        if s.starts_with("career_") {
                            s[7..].parse::<u32>().ok()
                        } else {
                            None
                        }
                    })
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        max + 1
    }

    /// Lista todos os saves existentes lendo cada meta.json.
    pub fn list_saves(&self) -> Vec<SaveMeta> {
        let saves = self.saves_dir();
        if !saves.exists() {
            return Vec::new();
        }
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&saves) {
            for entry in entries.filter_map(|e| e.ok()) {
                let meta_path = entry.path().join("meta.json");
                if let Ok(content) = std::fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<SaveMeta>(&content) {
                        result.push(meta);
                    }
                }
            }
        }
        result.sort_by(|a, b| b.last_played.cmp(&a.last_played));
        result
    }
}
