use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::models::enums::SeasonStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Season {
    pub id: String,
    pub numero: i32,
    pub ano: i32,
    pub status: SeasonStatus,
    pub rodada_atual: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl Season {
    pub fn new(id: String, numero: i32, ano: i32) -> Self {
        let now = current_timestamp();
        Self {
            id,
            numero,
            ano,
            status: SeasonStatus::EmAndamento,
            rodada_atual: 1,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn avancar_rodada(&mut self) {
        self.rodada_atual += 1;
        self.updated_at = current_timestamp();
    }

    pub fn finalizar(&mut self) {
        self.status = SeasonStatus::Finalizada;
        self.updated_at = current_timestamp();
    }

    pub fn is_ativa(&self) -> bool {
        self.status == SeasonStatus::EmAndamento
    }
}

fn current_timestamp() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_new() {
        let season = Season::new("S001".to_string(), 1, 2024);
        assert_eq!(season.id, "S001");
        assert_eq!(season.numero, 1);
        assert_eq!(season.ano, 2024);
        assert_eq!(season.status, SeasonStatus::EmAndamento);
        assert_eq!(season.rodada_atual, 1);
    }

    #[test]
    fn test_season_avancar_rodada() {
        let mut season = Season::new("S001".to_string(), 1, 2024);
        season.avancar_rodada();
        assert_eq!(season.rodada_atual, 2);
    }

    #[test]
    fn test_season_finalizar() {
        let mut season = Season::new("S001".to_string(), 1, 2024);
        season.finalizar();
        assert_eq!(season.status, SeasonStatus::Finalizada);
    }

    #[test]
    fn test_season_is_ativa() {
        let mut season = Season::new("S001".to_string(), 1, 2024);
        assert!(season.is_ativa());
        season.finalizar();
        assert!(!season.is_ativa());
    }
}
