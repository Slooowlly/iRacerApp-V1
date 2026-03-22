use serde::{Deserialize, Serialize};

pub mod generator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: String,
    pub tipo: NewsType,
    pub icone: String,
    pub titulo: String,
    pub texto: String,
    pub rodada: Option<i32>,
    pub semana_pretemporada: Option<i32>,
    pub temporada: i32,
    pub categoria_id: Option<String>,
    pub categoria_nome: Option<String>,
    pub importancia: NewsImportance,
    pub timestamp: i64,
    pub driver_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NewsType {
    Corrida,
    Incidente,
    Mercado,
    Promocao,
    Rebaixamento,
    Aposentadoria,
    Rookies,
    Hierarquia,
    Milestone,
    Lesao,
    Evolucao,
    PreTemporada,
    Rivalidade,
}

impl NewsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NewsType::Corrida => "Corrida",
            NewsType::Incidente => "Incidente",
            NewsType::Mercado => "Mercado",
            NewsType::Promocao => "Promocao",
            NewsType::Rebaixamento => "Rebaixamento",
            NewsType::Aposentadoria => "Aposentadoria",
            NewsType::Rookies => "Rookies",
            NewsType::Hierarquia => "Hierarquia",
            NewsType::Milestone => "Milestone",
            NewsType::Lesao => "Lesao",
            NewsType::Evolucao => "Evolucao",
            NewsType::PreTemporada => "PreTemporada",
            NewsType::Rivalidade => "Rivalidade",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "Incidente" => Self::Incidente,
            "Mercado" => Self::Mercado,
            "Promocao" => Self::Promocao,
            "Rebaixamento" => Self::Rebaixamento,
            "Aposentadoria" => Self::Aposentadoria,
            "Rookies" => Self::Rookies,
            "Hierarquia" => Self::Hierarquia,
            "Milestone" => Self::Milestone,
            "Lesao" => Self::Lesao,
            "Evolucao" => Self::Evolucao,
            "PreTemporada" => Self::PreTemporada,
            "Rivalidade" => Self::Rivalidade,
            _ => Self::Corrida,
        }
    }

    pub fn icone(&self) -> &'static str {
        match self {
            NewsType::Corrida => "🏆",
            NewsType::Incidente => "💥",
            NewsType::Mercado => "📋",
            NewsType::Promocao => "⬆️",
            NewsType::Rebaixamento => "⬇️",
            NewsType::Aposentadoria => "👴",
            NewsType::Rookies => "🎓",
            NewsType::Hierarquia => "⚡",
            NewsType::Milestone => "🏅",
            NewsType::Lesao => "🏥",
            NewsType::Evolucao => "📈",
            NewsType::PreTemporada => "📰",
            NewsType::Rivalidade => "⚔️",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NewsImportance {
    Baixa,
    Media,
    Alta,
    Destaque,
}

impl NewsImportance {
    pub fn as_str(&self) -> &'static str {
        match self {
            NewsImportance::Baixa => "Baixa",
            NewsImportance::Media => "Media",
            NewsImportance::Alta => "Alta",
            NewsImportance::Destaque => "Destaque",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "Baixa" => Self::Baixa,
            "Alta" => Self::Alta,
            "Destaque" => Self::Destaque,
            _ => Self::Media,
        }
    }
}
