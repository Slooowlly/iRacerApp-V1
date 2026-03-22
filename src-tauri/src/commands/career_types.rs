use serde::{Deserialize, Serialize};

use crate::commands::race_history::{RoundResult, TrophyInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCareerInput {
    pub player_name: String,
    pub player_nationality: String,
    pub player_age: Option<i32>,
    pub category: String,
    pub team_index: usize,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCareerResult {
    pub success: bool,
    pub career_id: String,
    pub save_path: String,
    pub player_id: String,
    pub player_team_id: String,
    pub player_team_name: String,
    pub season_id: String,
    pub total_drivers: usize,
    pub total_teams: usize,
    pub total_races: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub career_id: String,
    pub player_name: String,
    pub category: String,
    pub category_name: String,
    pub season: i32,
    pub year: i32,
    pub difficulty: String,
    pub created: String,
    pub last_played: String,
    pub total_races: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerData {
    pub career_id: String,
    pub save_path: String,
    pub difficulty: String,
    pub player: DriverSummary,
    pub player_team: TeamSummary,
    pub season: SeasonSummary,
    pub next_race: Option<RaceSummary>,
    pub total_drivers: usize,
    pub total_teams: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverSummary {
    pub id: String,
    pub nome: String,
    pub nacionalidade: String,
    pub idade: i32,
    pub skill: u8,
    pub equipe_id: Option<String>,
    pub equipe_nome: Option<String>,
    pub equipe_nome_curto: Option<String>,
    pub equipe_cor: String,
    pub is_jogador: bool,
    pub pontos: i32,
    pub vitorias: i32,
    pub podios: i32,
    pub posicao_campeonato: i32,
    pub results: Vec<Option<RoundResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSummary {
    pub id: String,
    pub nome: String,
    pub nome_curto: String,
    pub cor_primaria: String,
    pub cor_secundaria: String,
    pub categoria: String,
    pub car_performance: f64,
    pub confiabilidade: f64,
    pub budget: f64,
    pub piloto_1_id: Option<String>,
    pub piloto_1_nome: Option<String>,
    pub piloto_2_id: Option<String>,
    pub piloto_2_nome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonSummary {
    pub id: String,
    pub numero: i32,
    pub ano: i32,
    pub rodada_atual: i32,
    pub total_rodadas: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceSummary {
    pub id: String,
    pub rodada: i32,
    pub track_name: String,
    pub clima: String,
    pub duracao_corrida_min: i32,
    pub status: String,
    pub temperatura: f64,
    pub horario: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverDetail {
    pub id: String,
    pub nome: String,
    pub nacionalidade: String,
    pub idade: i32,
    pub genero: String,
    pub is_jogador: bool,
    pub status: String,
    pub equipe_id: Option<String>,
    pub equipe_nome: Option<String>,
    pub equipe_cor_primaria: Option<String>,
    pub equipe_cor_secundaria: Option<String>,
    pub papel: Option<String>,
    pub personalidade_primaria: Option<PersonalityInfo>,
    pub personalidade_secundaria: Option<PersonalityInfo>,
    pub motivacao: u8,
    pub tags: Vec<TagInfo>,
    pub stats_temporada: StatsBlock,
    pub stats_carreira: StatsBlock,
    pub contrato: Option<ContractDetail>,
    pub perfil: DriverProfileBlock,
    pub competitivo: DriverCompetitiveBlock,
    pub performance: DriverPerformanceBlock,
    pub forma: DriverFormBlock,
    pub trajetoria: DriverCareerPathBlock,
    pub contrato_mercado: DriverContractMarketBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relacionamentos: Option<DriverRelationshipsBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reputacao: Option<DriverReputationBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saude: Option<DriverHealthBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityInfo {
    pub tipo: String,
    pub emoji: String,
    pub descricao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub attribute_name: String,
    pub tag_text: String,
    pub level: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsBlock {
    pub corridas: i32,
    pub pontos: i32,
    pub vitorias: i32,
    pub podios: i32,
    pub poles: i32,
    pub melhor_resultado: i32,
    pub dnfs: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDetail {
    pub equipe_nome: String,
    pub papel: String,
    pub salario_anual: f64,
    pub temporada_inicio: i32,
    pub temporada_fim: i32,
    pub anos_restantes: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverProfileBlock {
    pub nome: String,
    pub bandeira: String,
    pub nacionalidade: String,
    pub idade: i32,
    pub genero: String,
    pub status: String,
    pub is_jogador: bool,
    pub equipe_nome: Option<String>,
    pub papel: Option<String>,
    pub licenca: DriverLicenseInfo,
    pub badges: Vec<DriverBadge>,
    pub equipe_cor_primaria: Option<String>,
    pub equipe_cor_secundaria: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLicenseInfo {
    pub nivel: String,
    pub sigla: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverBadge {
    pub label: String,
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCompetitiveBlock {
    pub personalidade_primaria: Option<PersonalityInfo>,
    pub personalidade_secundaria: Option<PersonalityInfo>,
    pub motivacao: u8,
    pub qualidades: Vec<TagInfo>,
    pub defeitos: Vec<TagInfo>,
    pub neutro: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverPerformanceBlock {
    pub temporada: PerformanceStatsBlock,
    pub carreira: PerformanceStatsBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStatsBlock {
    pub vitorias: i32,
    pub podios: i32,
    pub top_10: Option<i32>,
    pub fora_top_10: Option<i32>,
    pub poles: i32,
    pub voltas_rapidas: Option<i32>,
    pub hat_tricks: Option<i32>,
    pub corridas: i32,
    pub dnfs: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverFormBlock {
    pub ultimas_5: Vec<FormResultEntry>,
    pub media_chegada: Option<f64>,
    pub tendencia: String,
    pub momento: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormResultEntry {
    pub rodada: i32,
    pub chegada: Option<i32>,
    pub dnf: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCareerPathBlock {
    pub ano_estreia: i32,
    pub equipe_estreia: Option<String>,
    pub categoria_atual: Option<String>,
    pub temporadas_na_categoria: i32,
    pub corridas_na_categoria: i32,
    pub titulos: i32,
    pub foi_campeao: bool,
    pub marcos: Vec<CareerMilestone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerMilestone {
    pub tipo: String,
    pub titulo: String,
    pub descricao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverContractMarketBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contrato: Option<ContractDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mercado: Option<DriverMarketBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverMarketBlock {
    pub valor_mercado: Option<f64>,
    pub salario_estimado: Option<f64>,
    pub chance_transferencia: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverRelationshipsBlock {
    pub rival_principal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverReputationBlock {
    pub popularidade: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverHealthBlock {
    pub saude_geral: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStanding {
    pub posicao: i32,
    pub id: String,
    pub nome: String,
    pub nome_curto: String,
    pub cor_primaria: String,
    pub pontos: i32,
    pub vitorias: i32,
    pub piloto_1_nome: Option<String>,
    pub piloto_2_nome: Option<String>,
    pub trofeus: Vec<TrophyInfo>,
}

#[derive(Debug, Serialize)]
pub struct VerifyDatabaseResponse {
    pub career_number: u32,
    pub db_path: String,
    pub table_count: i64,
    pub status: String,
}
