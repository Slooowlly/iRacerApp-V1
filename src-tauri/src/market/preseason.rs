use std::path::Path;

use chrono::Local;
use rand::Rng;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::db::queries::contracts as contract_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::teams as team_queries;
use crate::generators::ids::{next_id, IdType};
use crate::market::pipeline::run_market;
use crate::market::proposals::{MarketProposal, ProposalStatus};
use crate::models::contract::Contract;
use crate::models::driver::Driver;
use crate::models::enums::{ContractStatus, DriverStatus, TeamRole};
use crate::models::team::HierarchyStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreSeasonPhase {
    ContractExpiry,
    Transfers,
    PlayerProposals,
    RookiePlacement,
    Finalization,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreSeasonState {
    pub season_number: i32,
    pub current_week: i32,
    pub total_weeks: i32,
    pub phase: PreSeasonPhase,
    pub is_complete: bool,
    pub player_has_pending_proposals: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketEventType {
    ContractExpired,
    ContractRenewed,
    TransferCompleted,
    TransferRejected,
    RookieSigned,
    PlayerProposalReceived,
    HierarchyUpdated,
    PreSeasonComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEvent {
    pub event_type: MarketEventType,
    pub headline: String,
    pub description: String,
    pub driver_id: Option<String>,
    pub driver_name: Option<String>,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub from_team: Option<String>,
    pub to_team: Option<String>,
    pub categoria: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekResult {
    pub week_number: i32,
    pub phase: PreSeasonPhase,
    pub events: Vec<MarketEvent>,
    pub is_last_week: bool,
    pub player_proposals: Vec<MarketProposal>,
    pub remaining_vacancies: i32,
    pub next_phase: PreSeasonPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreSeasonPlan {
    pub state: PreSeasonState,
    pub planned_events: Vec<PlannedEvent>,
    pub executed_weeks: Vec<WeekResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedEvent {
    pub week: i32,
    pub event: PendingAction,
    pub executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingAction {
    ExpireContract {
        contract_id: String,
        driver_id: String,
        driver_name: String,
        team_id: String,
        team_name: String,
    },
    RenewContract {
        driver_id: String,
        driver_name: String,
        team_id: String,
        team_name: String,
        new_salary: f64,
        new_duration: i32,
        new_role: String,
    },
    Transfer {
        driver_id: String,
        driver_name: String,
        from_team_id: Option<String>,
        from_team_name: Option<String>,
        to_team_id: String,
        to_team_name: String,
        salary: f64,
        duration: i32,
        role: String,
    },
    PlayerProposal {
        proposal: MarketProposal,
    },
    PlaceRookie {
        driver: Driver,
        team_id: String,
        team_name: String,
        salary: f64,
        duration: i32,
        role: String,
    },
    UpdateHierarchy {
        team_id: String,
        team_name: String,
        n1_id: Option<String>,
        n1_name: String,
        n2_id: Option<String>,
        n2_name: String,
    },
}

pub fn initialize_preseason(
    conn: &Connection,
    season_number: i32,
    rng: &mut impl Rng,
) -> Result<PreSeasonPlan, String> {
    let season_id = get_season_id_by_number(conn, season_number)?
        .ok_or_else(|| format!("Temporada {season_number} nao encontrada"))?;
    reset_market_state(conn, &season_id, &PreSeasonPhase::ContractExpiry)?;

    let temp_db_path = clone_connection_to_temp(conn)?;
    let temp_conn = Connection::open(&temp_db_path)
        .map_err(|e| format!("Falha ao abrir clone temporario do banco: {e}"))?;

    let original_contracts = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao carregar contratos atuais: {e}"))?;
    let original_teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao carregar equipes: {e}"))?;
    let _original_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar pilotos atuais: {e}"))?;
    let market_report = run_market(&temp_conn, season_number, rng)
        .map_err(|e| format!("Falha ao simular mercado para o plano: {e}"))?;
    let temp_teams = team_queries::get_all_teams(&temp_conn)
        .map_err(|e| format!("Falha ao carregar equipes do clone: {e}"))?;
    let temp_drivers = driver_queries::get_all_drivers(&temp_conn)
        .map_err(|e| format!("Falha ao carregar pilotos do clone: {e}"))?;

    let original_contracts_by_driver = original_contracts
        .iter()
        .cloned()
        .map(|contract| (contract.piloto_id.clone(), contract))
        .collect::<std::collections::HashMap<_, _>>();
    let original_teams_by_id = original_teams
        .iter()
        .cloned()
        .map(|team| (team.id.clone(), team))
        .collect::<std::collections::HashMap<_, _>>();
    let temp_drivers_by_id = temp_drivers
        .iter()
        .cloned()
        .map(|driver| (driver.id.clone(), driver))
        .collect::<std::collections::HashMap<_, _>>();

    let mut planned_events = build_expiry_events(&temp_conn, &original_contracts)?;
    planned_events.extend(build_renewal_events(
        &temp_conn,
        &market_report.new_signings,
    )?);
    planned_events.extend(build_transfer_events(
        &temp_conn,
        &market_report.new_signings,
        &original_contracts_by_driver,
    )?);

    let mut current_week = planned_events
        .iter()
        .map(|event| event.week)
        .max()
        .unwrap_or(2)
        + 1;
    if !market_report.player_proposals.is_empty() {
        for proposal in market_report.player_proposals.iter().cloned() {
            planned_events.push(PlannedEvent {
                week: current_week,
                event: PendingAction::PlayerProposal { proposal },
                executed: false,
            });
        }
        current_week += 1;
    }

    planned_events.extend(build_rookie_events(
        &temp_conn,
        &market_report.new_signings,
        &temp_drivers_by_id,
        current_week,
    )?);
    current_week += 1;
    planned_events.extend(build_hierarchy_events(
        &temp_teams,
        &original_teams_by_id,
        &temp_drivers_by_id,
        current_week,
    ));

    let total_weeks = current_week.max(3);
    cleanup_temp_db(&temp_db_path);

    Ok(PreSeasonPlan {
        state: PreSeasonState {
            season_number,
            current_week: 1,
            total_weeks,
            phase: phase_for_week(1, &planned_events),
            is_complete: false,
            player_has_pending_proposals: market_report
                .player_proposals
                .iter()
                .any(|proposal| proposal.status == ProposalStatus::Pendente),
        },
        planned_events,
        executed_weeks: Vec::new(),
    })
}

pub fn advance_week(conn: &Connection, plan: &mut PreSeasonPlan) -> Result<WeekResult, String> {
    if plan.state.is_complete {
        return Err("Pre-temporada ja esta completa".to_string());
    }

    let week = plan.state.current_week;
    let season_id = get_season_id_by_number(conn, plan.state.season_number)?
        .ok_or_else(|| format!("Temporada {} nao encontrada", plan.state.season_number))?;
    let phase = phase_for_week(week, &plan.planned_events);
    let indices: Vec<usize> = plan
        .planned_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            if event.week == week && !event.executed {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    let mut events = Vec::new();
    let mut player_proposals = Vec::new();
    for index in indices {
        let action = plan.planned_events[index].event.clone();
        execute_action(
            conn,
            &season_id,
            plan.state.season_number,
            &action,
            &mut events,
            &mut player_proposals,
        )?;
        plan.planned_events[index].executed = true;
    }

    sync_team_slots_from_active_contracts(conn)?;
    let remaining_vacancies = count_remaining_vacancies(conn)?;

    let is_last_week = week >= plan.state.total_weeks;
    if is_last_week {
        plan.state.current_week = plan.state.total_weeks + 1;
        plan.state.phase = PreSeasonPhase::Complete;
        plan.state.is_complete = true;
        events.push(MarketEvent {
            event_type: MarketEventType::PreSeasonComplete,
            headline: "Pre-temporada encerrada".to_string(),
            description: "O mercado de transferencias foi finalizado.".to_string(),
            driver_id: None,
            driver_name: None,
            team_id: None,
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        });
        update_market_state(conn, &season_id, "Fechado", &PreSeasonPhase::Complete, true)?;
    } else {
        plan.state.current_week += 1;
        plan.state.phase = phase_for_week(plan.state.current_week, &plan.planned_events);
        update_market_state(conn, &season_id, "Aberto", &plan.state.phase, false)?;
    }

    let next_phase = if plan.state.is_complete {
        PreSeasonPhase::Complete
    } else {
        phase_for_week(plan.state.current_week, &plan.planned_events)
    };
    let result = WeekResult {
        week_number: week,
        phase,
        events,
        is_last_week,
        player_proposals,
        remaining_vacancies,
        next_phase,
    };
    plan.executed_weeks.push(result.clone());
    Ok(result)
}

pub fn save_preseason_plan(save_path: &Path, plan: &PreSeasonPlan) -> Result<(), String> {
    std::fs::create_dir_all(save_path)
        .map_err(|e| format!("Falha ao criar diretorio da pre-temporada: {e}"))?;
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| format!("Falha ao serializar plano da pre-temporada: {e}"))?;
    std::fs::write(preseason_plan_path(save_path), json)
        .map_err(|e| format!("Falha ao salvar plano da pre-temporada: {e}"))
}

pub fn load_preseason_plan(save_path: &Path) -> Result<Option<PreSeasonPlan>, String> {
    let path = preseason_plan_path(save_path);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Falha ao ler plano da pre-temporada: {e}"))?;
    let plan = serde_json::from_str(&content)
        .map_err(|e| format!("Falha ao parsear plano da pre-temporada: {e}"))?;
    Ok(Some(plan))
}

pub fn delete_preseason_plan(save_path: &Path) -> Result<(), String> {
    let path = preseason_plan_path(save_path);
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(path).map_err(|e| format!("Falha ao apagar plano da pre-temporada: {e}"))
}

fn preseason_plan_path(save_path: &Path) -> std::path::PathBuf {
    save_path.join("preseason_plan.json")
}

fn phase_for_week(week: i32, planned_events: &[PlannedEvent]) -> PreSeasonPhase {
    let mut phases = planned_events
        .iter()
        .filter(|event| event.week == week)
        .map(|event| phase_for_action(&event.event))
        .collect::<Vec<_>>();
    phases.sort_by_key(phase_order);
    phases
        .into_iter()
        .next()
        .or_else(|| {
            planned_events
                .iter()
                .filter(|event| event.week > week)
                .map(|event| phase_for_action(&event.event))
                .min_by_key(phase_order)
        })
        .unwrap_or(PreSeasonPhase::Complete)
}

fn phase_for_action(action: &PendingAction) -> PreSeasonPhase {
    match action {
        PendingAction::ExpireContract { .. } | PendingAction::RenewContract { .. } => {
            PreSeasonPhase::ContractExpiry
        }
        PendingAction::Transfer { .. } => PreSeasonPhase::Transfers,
        PendingAction::PlayerProposal { .. } => PreSeasonPhase::PlayerProposals,
        PendingAction::PlaceRookie { .. } => PreSeasonPhase::RookiePlacement,
        PendingAction::UpdateHierarchy { .. } => PreSeasonPhase::Finalization,
    }
}

fn phase_order(phase: &PreSeasonPhase) -> i32 {
    match phase {
        PreSeasonPhase::ContractExpiry => 1,
        PreSeasonPhase::Transfers => 2,
        PreSeasonPhase::PlayerProposals => 3,
        PreSeasonPhase::RookiePlacement => 4,
        PreSeasonPhase::Finalization => 5,
        PreSeasonPhase::Complete => 6,
    }
}

fn build_expiry_events(
    temp_conn: &Connection,
    original_contracts: &[Contract],
) -> Result<Vec<PlannedEvent>, String> {
    let expiring: Vec<_> = original_contracts
        .iter()
        .filter_map(|contract| {
            let temp_contract = contract_queries::get_contract_by_id(temp_conn, &contract.id)
                .ok()
                .flatten()?;
            if temp_contract.status == ContractStatus::Ativo {
                return None;
            }
            Some(PlannedEvent {
                week: 1,
                event: PendingAction::ExpireContract {
                    contract_id: contract.id.clone(),
                    driver_id: contract.piloto_id.clone(),
                    driver_name: contract.piloto_nome.clone(),
                    team_id: contract.equipe_id.clone(),
                    team_name: contract.equipe_nome.clone(),
                },
                executed: false,
            })
        })
        .collect();

    let split = expiring.len().div_ceil(2).max(1);
    Ok(expiring
        .into_iter()
        .enumerate()
        .map(|(index, mut event)| {
            event.week = if index < split { 1 } else { 2 };
            event
        })
        .collect())
}

fn build_renewal_events(
    temp_conn: &Connection,
    signings: &[crate::market::proposals::SigningInfo],
) -> Result<Vec<PlannedEvent>, String> {
    let renewals: Vec<_> = signings
        .iter()
        .filter(|signing| signing.tipo == "renovacao")
        .cloned()
        .collect();
    let split = renewals.len().div_ceil(2).max(1);
    let mut events = Vec::new();
    for (index, signing) in renewals.into_iter().enumerate() {
        let contract =
            contract_queries::get_active_contract_for_pilot(temp_conn, &signing.driver_id)
                .map_err(|e| format!("Falha ao buscar renovacao planejada: {e}"))?
                .ok_or_else(|| {
                    format!(
                        "Contrato renovado de '{}' nao encontrado",
                        signing.driver_id
                    )
                })?;
        events.push(PlannedEvent {
            week: if index < split { 1 } else { 2 },
            event: PendingAction::RenewContract {
                driver_id: signing.driver_id,
                driver_name: signing.driver_name,
                team_id: signing.team_id,
                team_name: signing.team_name,
                new_salary: contract.salario_anual,
                new_duration: contract.duracao_anos,
                new_role: contract.papel.as_str().to_string(),
            },
            executed: false,
        });
    }
    Ok(events)
}

fn build_transfer_events(
    temp_conn: &Connection,
    signings: &[crate::market::proposals::SigningInfo],
    original_contracts_by_driver: &std::collections::HashMap<String, Contract>,
) -> Result<Vec<PlannedEvent>, String> {
    let mut events = Vec::new();
    for (index, signing) in signings
        .iter()
        .filter(|signing| signing.tipo == "transferencia")
        .cloned()
        .enumerate()
    {
        let contract =
            contract_queries::get_active_contract_for_pilot(temp_conn, &signing.driver_id)
                .map_err(|e| format!("Falha ao buscar transferencia planejada: {e}"))?
                .ok_or_else(|| format!("Contrato de '{}' nao encontrado", signing.driver_id))?;
        let previous_team = original_contracts_by_driver.get(&signing.driver_id);
        events.push(PlannedEvent {
            week: 3 + (index / 3).min(2) as i32,
            event: PendingAction::Transfer {
                driver_id: signing.driver_id,
                driver_name: signing.driver_name,
                from_team_id: previous_team.map(|contract| contract.equipe_id.clone()),
                from_team_name: previous_team.map(|contract| contract.equipe_nome.clone()),
                to_team_id: signing.team_id,
                to_team_name: signing.team_name,
                salary: contract.salario_anual,
                duration: contract.duracao_anos,
                role: contract.papel.as_str().to_string(),
            },
            executed: false,
        });
    }
    Ok(events)
}

fn build_rookie_events(
    temp_conn: &Connection,
    signings: &[crate::market::proposals::SigningInfo],
    temp_drivers_by_id: &std::collections::HashMap<String, Driver>,
    week: i32,
) -> Result<Vec<PlannedEvent>, String> {
    let mut events = Vec::new();
    for signing in signings.iter().filter(|signing| signing.tipo == "rookie") {
        let contract =
            contract_queries::get_active_contract_for_pilot(temp_conn, &signing.driver_id)
                .map_err(|e| format!("Falha ao buscar rookie planejado: {e}"))?
                .ok_or_else(|| {
                    format!("Contrato de rookie '{}' nao encontrado", signing.driver_id)
                })?;
        let driver = temp_drivers_by_id
            .get(&signing.driver_id)
            .cloned()
            .ok_or_else(|| format!("Rookie '{}' nao encontrado no clone", signing.driver_id))?;
        events.push(PlannedEvent {
            week,
            event: PendingAction::PlaceRookie {
                driver,
                team_id: signing.team_id.clone(),
                team_name: signing.team_name.clone(),
                salary: contract.salario_anual,
                duration: contract.duracao_anos,
                role: contract.papel.as_str().to_string(),
            },
            executed: false,
        });
    }
    Ok(events)
}

fn build_hierarchy_events(
    temp_teams: &[crate::models::team::Team],
    original_teams_by_id: &std::collections::HashMap<String, crate::models::team::Team>,
    temp_drivers_by_id: &std::collections::HashMap<String, Driver>,
    week: i32,
) -> Vec<PlannedEvent> {
    let mut events = Vec::new();
    for team in temp_teams {
        let changed = original_teams_by_id.get(&team.id).is_none_or(|current| {
            current.piloto_1_id != team.piloto_1_id
                || current.piloto_2_id != team.piloto_2_id
                || current.hierarquia_n1_id != team.hierarquia_n1_id
                || current.hierarquia_n2_id != team.hierarquia_n2_id
        });
        if !changed {
            continue;
        }
        let n1_name = team
            .piloto_1_id
            .as_ref()
            .and_then(|id| temp_drivers_by_id.get(id))
            .map(|driver| driver.nome.clone())
            .unwrap_or_else(|| "Sem piloto".to_string());
        let n2_name = team
            .piloto_2_id
            .as_ref()
            .and_then(|id| temp_drivers_by_id.get(id))
            .map(|driver| driver.nome.clone())
            .unwrap_or_else(|| "Sem piloto".to_string());
        events.push(PlannedEvent {
            week,
            event: PendingAction::UpdateHierarchy {
                team_id: team.id.clone(),
                team_name: team.nome.clone(),
                n1_id: team.piloto_1_id.clone(),
                n1_name,
                n2_id: team.piloto_2_id.clone(),
                n2_name,
            },
            executed: false,
        });
    }
    events
}

fn get_season_id_by_number(
    conn: &Connection,
    season_number: i32,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT id FROM seasons WHERE numero = ?1 LIMIT 1",
        rusqlite::params![season_number],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| format!("Falha ao buscar temporada {season_number}: {e}"))
}

fn reset_market_state(
    conn: &Connection,
    season_id: &str,
    phase: &PreSeasonPhase,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM market_proposals WHERE temporada_id = ?1",
        rusqlite::params![season_id],
    )
    .map_err(|e| format!("Falha ao limpar propostas antigas da pre-temporada: {e}"))?;
    conn.execute(
        "DELETE FROM market WHERE temporada_id = ?1",
        rusqlite::params![season_id],
    )
    .map_err(|e| format!("Falha ao limpar estado antigo do mercado: {e}"))?;
    conn.execute(
        "INSERT INTO market (temporada_id, status, fase, inicio, fim)
         VALUES (?1, 'Aberto', ?2, ?3, '')",
        rusqlite::params![season_id, phase_label(phase), timestamp_now()],
    )
    .map_err(|e| format!("Falha ao inicializar estado do mercado: {e}"))?;
    Ok(())
}

fn update_market_state(
    conn: &Connection,
    season_id: &str,
    status: &str,
    phase: &PreSeasonPhase,
    completed: bool,
) -> Result<(), String> {
    let end_value = if completed {
        timestamp_now()
    } else {
        String::new()
    };
    conn.execute(
        "UPDATE market
         SET status = ?1, fase = ?2, fim = CASE WHEN ?3 = '' THEN fim ELSE ?3 END
         WHERE temporada_id = ?4",
        rusqlite::params![status, phase_label(phase), end_value, season_id],
    )
    .map_err(|e| format!("Falha ao atualizar estado do mercado: {e}"))?;
    Ok(())
}

fn phase_label(phase: &PreSeasonPhase) -> &'static str {
    match phase {
        PreSeasonPhase::ContractExpiry => "ContractExpiry",
        PreSeasonPhase::Transfers => "Transfers",
        PreSeasonPhase::PlayerProposals => "PlayerProposals",
        PreSeasonPhase::RookiePlacement => "RookiePlacement",
        PreSeasonPhase::Finalization => "Finalization",
        PreSeasonPhase::Complete => "Complete",
    }
}

fn timestamp_now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn clone_connection_to_temp(conn: &Connection) -> Result<std::path::PathBuf, String> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Falha ao checkpointar banco antes do clone: {e}"))?;
    let temp_path = std::env::temp_dir().join(format!(
        "iracerapp_preseason_clone_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Falha ao gerar timestamp do clone: {e}"))?
            .as_nanos()
    ));
    let escaped = temp_path
        .to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{escaped}';"))
        .map_err(|e| format!("Falha ao clonar banco para planejamento da pre-temporada: {e}"))?;
    Ok(temp_path)
}

fn cleanup_temp_db(path: &Path) {
    let _ = std::fs::remove_file(path);
    let wal = format!("{}-wal", path.to_string_lossy());
    let shm = format!("{}-shm", path.to_string_lossy());
    let _ = std::fs::remove_file(wal);
    let _ = std::fs::remove_file(shm);
}

fn execute_action(
    conn: &Connection,
    season_id: &str,
    season_number: i32,
    action: &PendingAction,
    events: &mut Vec<MarketEvent>,
    player_proposals: &mut Vec<MarketProposal>,
) -> Result<(), String> {
    match action {
        PendingAction::ExpireContract {
            contract_id,
            driver_id,
            driver_name,
            team_id,
            team_name,
        } => {
            let status = driver_queries::get_driver(conn, driver_id)
                .map(|driver| {
                    if driver.status == DriverStatus::Aposentado {
                        ContractStatus::Rescindido
                    } else {
                        ContractStatus::Expirado
                    }
                })
                .unwrap_or(ContractStatus::Expirado);
            contract_queries::update_contract_status(conn, contract_id, &status)
                .map_err(|e| format!("Falha ao encerrar contrato '{}': {e}", contract_id))?;
            clear_driver_from_team(conn, team_id, driver_id)?;
            events.push(MarketEvent {
                event_type: MarketEventType::ContractExpired,
                headline: format!("{driver_name} deixa {team_name}"),
                description: format!("O vinculo de {driver_name} com {team_name} foi encerrado."),
                driver_id: Some(driver_id.clone()),
                driver_name: Some(driver_name.clone()),
                team_id: Some(team_id.clone()),
                team_name: Some(team_name.clone()),
                from_team: Some(team_name.clone()),
                to_team: None,
                categoria: team_queries::get_team_by_id(conn, team_id)
                    .ok()
                    .flatten()
                    .map(|team| team.categoria),
            });
        }
        PendingAction::RenewContract {
            driver_id,
            driver_name,
            team_id,
            team_name,
            new_salary,
            new_duration,
            new_role,
        } => {
            let team = team_queries::get_team_by_id(conn, team_id)
                .map_err(|e| format!("Falha ao buscar equipe da renovacao: {e}"))?
                .ok_or_else(|| format!("Equipe '{}' nao encontrada", team_id))?;
            let contract = Contract::new(
                next_id(conn, IdType::Contract)
                    .map_err(|e| format!("Falha ao gerar ID da renovacao: {e}"))?,
                driver_id.clone(),
                driver_name.clone(),
                team_id.clone(),
                team_name.clone(),
                season_number,
                *new_duration,
                *new_salary,
                TeamRole::from_str(new_role),
                team.categoria.clone(),
            );
            contract_queries::insert_contract(conn, &contract)
                .map_err(|e| format!("Falha ao inserir renovacao '{}': {e}", driver_id))?;
            events.push(MarketEvent {
                event_type: MarketEventType::ContractRenewed,
                headline: format!("{driver_name} renova com {team_name}"),
                description: format!(
                    "{driver_name} renovou com {team_name} por {} ano(s).",
                    new_duration
                ),
                driver_id: Some(driver_id.clone()),
                driver_name: Some(driver_name.clone()),
                team_id: Some(team_id.clone()),
                team_name: Some(team_name.clone()),
                from_team: Some(team_name.clone()),
                to_team: Some(team_name.clone()),
                categoria: Some(team.categoria),
            });
        }
        PendingAction::Transfer {
            driver_id,
            driver_name,
            from_team_name,
            to_team_id,
            to_team_name,
            salary,
            duration,
            role,
            ..
        } => {
            sign_driver_to_team(
                conn,
                driver_id,
                driver_name,
                to_team_id,
                season_number,
                *salary,
                *duration,
                role,
            )?;
            events.push(MarketEvent {
                event_type: MarketEventType::TransferCompleted,
                headline: format!("{driver_name} assina com {to_team_name}"),
                description: format!(
                    "{driver_name} deixa {} e assina com {to_team_name}.",
                    from_team_name
                        .clone()
                        .unwrap_or_else(|| "o mercado livre".to_string())
                ),
                driver_id: Some(driver_id.clone()),
                driver_name: Some(driver_name.clone()),
                team_id: Some(to_team_id.clone()),
                team_name: Some(to_team_name.clone()),
                from_team: from_team_name.clone(),
                to_team: Some(to_team_name.clone()),
                categoria: team_queries::get_team_by_id(conn, to_team_id)
                    .ok()
                    .flatten()
                    .map(|team| team.categoria),
            });
        }
        PendingAction::PlayerProposal { proposal } => {
            persist_player_proposal(conn, season_id, proposal)?;
            player_proposals.push(proposal.clone());
            events.push(MarketEvent {
                event_type: MarketEventType::PlayerProposalReceived,
                headline: format!("Voce recebeu uma proposta de {}", proposal.equipe_nome),
                description: format!(
                    "{} oferece {} por {} ano(s).",
                    proposal.equipe_nome,
                    proposal.papel.as_str(),
                    proposal.duracao_anos
                ),
                driver_id: Some(proposal.piloto_id.clone()),
                driver_name: Some(proposal.piloto_nome.clone()),
                team_id: Some(proposal.equipe_id.clone()),
                team_name: Some(proposal.equipe_nome.clone()),
                from_team: None,
                to_team: Some(proposal.equipe_nome.clone()),
                categoria: Some(proposal.categoria.clone()),
            });
        }
        PendingAction::PlaceRookie {
            driver,
            team_id,
            team_name,
            salary,
            duration,
            role,
        } => {
            ensure_driver_exists(conn, driver)?;
            sign_driver_to_team(
                conn,
                &driver.id,
                &driver.nome,
                team_id,
                season_number,
                *salary,
                *duration,
                role,
            )?;
            events.push(MarketEvent {
                event_type: MarketEventType::RookieSigned,
                headline: format!("Rookie {} assina com {team_name}", driver.nome),
                description: format!("{} e o novo piloto da {team_name}.", driver.nome),
                driver_id: Some(driver.id.clone()),
                driver_name: Some(driver.nome.clone()),
                team_id: Some(team_id.clone()),
                team_name: Some(team_name.clone()),
                from_team: None,
                to_team: Some(team_name.clone()),
                categoria: team_queries::get_team_by_id(conn, team_id)
                    .ok()
                    .flatten()
                    .map(|team| team.categoria),
            });
        }
        PendingAction::UpdateHierarchy {
            team_id,
            team_name,
            n1_id,
            n1_name,
            n2_id,
            n2_name,
        } => {
            team_queries::update_team_pilots(conn, team_id, n1_id.as_deref(), n2_id.as_deref())
                .map_err(|e| format!("Falha ao atualizar pilotos da equipe '{}': {e}", team_id))?;
            team_queries::update_team_hierarchy(
                conn,
                team_id,
                n1_id.as_deref(),
                n2_id.as_deref(),
                HierarchyStatus::Estavel.as_str(),
                0.0,
            )
            .map_err(|e| format!("Falha ao atualizar hierarquia da equipe '{}': {e}", team_id))?;
            events.push(MarketEvent {
                event_type: MarketEventType::HierarchyUpdated,
                headline: format!("{team_name}: {n1_name} e N1, {n2_name} e N2"),
                description: format!("{team_name} definiu sua hierarquia para a temporada."),
                driver_id: None,
                driver_name: None,
                team_id: Some(team_id.clone()),
                team_name: Some(team_name.clone()),
                from_team: None,
                to_team: None,
                categoria: team_queries::get_team_by_id(conn, team_id)
                    .ok()
                    .flatten()
                    .map(|team| team.categoria),
            });
        }
    }
    Ok(())
}

fn persist_player_proposal(
    conn: &Connection,
    season_id: &str,
    proposal: &MarketProposal,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO market_proposals (
            id, temporada_id, equipe_id, piloto_id, papel, salario, status, motivo_recusa, criado_em
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &proposal.id,
            season_id,
            &proposal.equipe_id,
            &proposal.piloto_id,
            proposal.papel.as_str(),
            proposal.salario_oferecido,
            proposal.status.as_str(),
            proposal.motivo_recusa.clone(),
            timestamp_now(),
        ],
    )
    .map_err(|e| format!("Falha ao persistir proposta do jogador: {e}"))?;
    Ok(())
}

fn ensure_driver_exists(conn: &Connection, driver: &Driver) -> Result<(), String> {
    if driver_queries::get_driver(conn, &driver.id).is_ok() {
        return Ok(());
    }
    driver_queries::insert_driver(conn, driver)
        .map_err(|e| format!("Falha ao inserir rookie planejado '{}': {e}", driver.id))
}

fn sign_driver_to_team(
    conn: &Connection,
    driver_id: &str,
    driver_name: &str,
    team_id: &str,
    season_number: i32,
    salary: f64,
    duration: i32,
    role: &str,
) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao buscar equipe '{}' para assinatura: {e}", team_id))?
        .ok_or_else(|| format!("Equipe '{}' nao encontrada", team_id))?;
    let driver = driver_queries::get_driver(conn, driver_id).map_err(|e| {
        format!(
            "Falha ao buscar piloto '{}' para assinatura: {e}",
            driver_id
        )
    })?;
    let contract = Contract::new(
        next_id(conn, IdType::Contract)
            .map_err(|e| format!("Falha ao gerar ID de contrato: {e}"))?,
        driver_id.to_string(),
        driver_name.to_string(),
        team_id.to_string(),
        team.nome.clone(),
        season_number,
        duration,
        salary,
        TeamRole::from_str(role),
        team.categoria.clone(),
    );
    contract_queries::insert_contract(conn, &contract)
        .map_err(|e| format!("Falha ao assinar contrato de '{}': {e}", driver_id))?;

    let mut updated_driver = driver;
    updated_driver.categoria_atual = Some(team.categoria.clone());
    driver_queries::update_driver(conn, &updated_driver)
        .map_err(|e| format!("Falha ao atualizar piloto contratado '{}': {e}", driver_id))?;
    Ok(())
}

fn clear_driver_from_team(conn: &Connection, team_id: &str, driver_id: &str) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao buscar equipe para liberar piloto: {e}"))?
        .ok_or_else(|| format!("Equipe '{}' nao encontrada", team_id))?;
    let piloto_1 = if team.piloto_1_id.as_deref() == Some(driver_id) {
        None
    } else {
        team.piloto_1_id.as_deref()
    };
    let piloto_2 = if team.piloto_2_id.as_deref() == Some(driver_id) {
        None
    } else {
        team.piloto_2_id.as_deref()
    };
    team_queries::update_team_pilots(conn, team_id, piloto_1, piloto_2)
        .map_err(|e| format!("Falha ao remover piloto da equipe '{}': {e}", team_id))
}

fn sync_team_slots_from_active_contracts(conn: &Connection) -> Result<(), String> {
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao carregar equipes: {e}"))?;
    let drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar pilotos: {e}"))?;
    let drivers_by_id = drivers
        .into_iter()
        .map(|driver| (driver.id.clone(), driver))
        .collect::<std::collections::HashMap<_, _>>();
    let active_contracts = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao carregar contratos ativos: {e}"))?;
    let mut contracts_by_team = std::collections::HashMap::<String, Vec<Contract>>::new();

    for contract in active_contracts {
        let Some(driver) = drivers_by_id.get(&contract.piloto_id) else {
            continue;
        };
        if driver.status == DriverStatus::Aposentado {
            contract_queries::update_contract_status(
                conn,
                &contract.id,
                &ContractStatus::Rescindido,
            )
            .map_err(|e| {
                format!(
                    "Falha ao rescindir contrato invalido '{}': {e}",
                    contract.id
                )
            })?;
            continue;
        }
        contracts_by_team
            .entry(contract.equipe_id.clone())
            .or_default()
            .push(contract);
    }

    for team in teams {
        let mut contracts = contracts_by_team.remove(&team.id).unwrap_or_default();
        contracts.sort_by(|a, b| {
            let skill_a = drivers_by_id
                .get(&a.piloto_id)
                .map(|driver| driver.atributos.skill)
                .unwrap_or(0.0);
            let skill_b = drivers_by_id
                .get(&b.piloto_id)
                .map(|driver| driver.atributos.skill)
                .unwrap_or(0.0);
            skill_b.total_cmp(&skill_a)
        });
        let piloto_1 = contracts
            .first()
            .map(|contract| contract.piloto_id.as_str());
        let piloto_2 = contracts.get(1).map(|contract| contract.piloto_id.as_str());
        team_queries::update_team_pilots(conn, &team.id, piloto_1, piloto_2).map_err(|e| {
            format!(
                "Falha ao sincronizar pilotos da equipe '{}': {e}",
                team.nome
            )
        })?;
    }

    Ok(())
}

fn count_remaining_vacancies(conn: &Connection) -> Result<i32, String> {
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao contar vagas: {e}"))?;
    Ok(teams
        .iter()
        .map(|team| {
            let mut open = 0;
            if team.piloto_1_id.is_none() {
                open += 1;
            }
            if team.piloto_2_id.is_none() {
                open += 1;
            }
            open
        })
        .sum())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::{params, Connection};

    use super::*;
    use crate::constants::teams::get_team_templates;
    use crate::db::migrations;
    use crate::db::queries::contracts as contract_queries;
    use crate::db::queries::drivers as driver_queries;
    use crate::db::queries::seasons as season_queries;
    use crate::db::queries::teams as team_queries;
    use crate::models::contract::Contract;
    use crate::models::driver::Driver;
    use crate::models::enums::{DriverStatus, TeamRole};
    use crate::models::season::Season;

    #[test]
    fn test_initialize_preseason_creates_plan() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(500);

        let plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        assert_eq!(plan.state.season_number, 2);
        assert_eq!(plan.state.current_week, 1);
        assert!(plan.state.total_weeks >= 3);
        assert!(!plan.planned_events.is_empty());
    }

    #[test]
    fn test_initialize_preseason_has_all_phases() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(501);

        let plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");
        let phases: Vec<_> = (1..=plan.state.total_weeks)
            .map(|week| phase_for_week(week, &plan.planned_events))
            .collect();

        assert!(phases.contains(&PreSeasonPhase::ContractExpiry));
        assert!(phases.contains(&PreSeasonPhase::Transfers));
        assert!(phases.contains(&PreSeasonPhase::PlayerProposals));
        assert!(phases.contains(&PreSeasonPhase::RookiePlacement));
        assert!(phases.contains(&PreSeasonPhase::Finalization));
    }

    #[test]
    fn test_plan_total_weeks_reasonable() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(502);

        let plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        assert!((3..=12).contains(&plan.state.total_weeks));
    }

    #[test]
    fn test_advance_week_executes_events() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(503);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        let result = advance_week(&conn, &mut plan).expect("week should advance");

        assert_eq!(result.week_number, 1);
        assert!(!result.events.is_empty());
        assert!(plan.planned_events.iter().any(|event| event.executed));
    }

    #[test]
    fn test_advance_week_increments_week() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(504);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        advance_week(&conn, &mut plan).expect("week should advance");

        assert_eq!(plan.state.current_week, 2);
    }

    #[test]
    fn test_advance_week_phase_transitions() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(505);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        let mut seen_player_phase = false;
        while !plan.state.is_complete {
            let result = advance_week(&conn, &mut plan).expect("week should advance");
            if result.next_phase == PreSeasonPhase::PlayerProposals
                || result.phase == PreSeasonPhase::PlayerProposals
            {
                seen_player_phase = true;
            }
        }

        assert!(seen_player_phase);
        assert_eq!(plan.state.phase, PreSeasonPhase::Complete);
    }

    #[test]
    fn test_contract_expiry_week() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(506);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        let week_one_expiries: Vec<String> = plan
            .planned_events
            .iter()
            .filter_map(|event| match &event.event {
                PendingAction::ExpireContract { contract_id, .. } if event.week == 1 => {
                    Some(contract_id.clone())
                }
                _ => None,
            })
            .collect();
        assert!(!week_one_expiries.is_empty());

        let result = advance_week(&conn, &mut plan).expect("week should advance");
        assert_eq!(result.phase, PreSeasonPhase::ContractExpiry);
        assert!(result
            .events
            .iter()
            .any(|event| event.event_type == MarketEventType::ContractExpired));

        for contract_id in week_one_expiries {
            let status: String = conn
                .query_row(
                    "SELECT status FROM contracts WHERE id = ?1",
                    [&contract_id],
                    |row| row.get(0),
                )
                .expect("contract status");
            assert_ne!(status, "Ativo");
        }
    }

    #[test]
    fn test_renewal_week() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(507);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        while !plan.state.is_complete {
            let result = advance_week(&conn, &mut plan).expect("week should advance");
            if result
                .events
                .iter()
                .any(|event| event.event_type == MarketEventType::ContractRenewed)
            {
                break;
            }
        }

        let renewed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contracts WHERE piloto_id = 'P001' AND temporada_inicio = 2 AND status = 'Ativo'",
                [],
                |row| row.get(0),
            )
            .expect("renewed count");
        assert!(renewed >= 1);
    }

    #[test]
    fn test_transfer_week() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(508);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        while !plan.state.is_complete {
            let result = advance_week(&conn, &mut plan).expect("week should advance");
            if result
                .events
                .iter()
                .any(|event| event.event_type == MarketEventType::TransferCompleted)
            {
                break;
            }
        }

        let team = team_queries::get_team_by_id(&conn, "T002")
            .expect("team query")
            .expect("team");
        assert!(team.piloto_1_id.is_some() || team.piloto_2_id.is_some());
    }

    #[test]
    fn test_rookie_placement_week() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(509);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        while !plan.state.is_complete {
            let result = advance_week(&conn, &mut plan).expect("week should advance");
            if result
                .events
                .iter()
                .any(|event| event.event_type == MarketEventType::RookieSigned)
            {
                break;
            }
        }

        let rookie_contracts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contracts WHERE temporada_inicio = 2 AND status = 'Ativo'",
                [],
                |row| row.get(0),
            )
            .expect("rookie contracts");
        assert!(rookie_contracts >= 2);
    }

    #[test]
    fn test_all_teams_filled_after_complete() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(510);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        while !plan.state.is_complete {
            advance_week(&conn, &mut plan).expect("week should advance");
        }

        let teams = team_queries::get_all_teams(&conn).expect("teams");
        assert!(teams
            .iter()
            .all(|team| team.piloto_1_id.is_some() && team.piloto_2_id.is_some()));
    }

    #[test]
    fn test_plan_persistence() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(511);
        let plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");
        let temp_dir = unique_test_dir("preseason_persistence");
        fs::create_dir_all(&temp_dir).expect("temp dir");

        save_preseason_plan(&temp_dir, &plan).expect("plan should save");
        let loaded = load_preseason_plan(&temp_dir)
            .expect("plan should load")
            .expect("plan should exist");

        assert_eq!(loaded.state.season_number, plan.state.season_number);
        assert_eq!(loaded.state.total_weeks, plan.state.total_weeks);
        assert_eq!(loaded.planned_events.len(), plan.planned_events.len());

        delete_preseason_plan(&temp_dir).expect("delete plan");
        assert!(load_preseason_plan(&temp_dir)
            .expect("load after delete")
            .is_none());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_cannot_advance_after_complete() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(512);
        let mut plan = initialize_preseason(&conn, 2, &mut rng).expect("plan should be created");

        while !plan.state.is_complete {
            advance_week(&conn, &mut plan).expect("week should advance");
        }

        let error = advance_week(&conn, &mut plan).expect_err("should reject after complete");
        assert!(error.contains("completa"));
    }

    fn setup_market_fixture() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        let previous = Season::new("S001".to_string(), 1, 2024);
        let next = Season::new("S002".to_string(), 2, 2025);
        season_queries::insert_season(&conn, &previous).expect("previous season");
        season_queries::finalize_season(&conn, &previous.id).expect("finalize previous");
        season_queries::insert_season(&conn, &next).expect("next season");

        let mut team_rng = StdRng::seed_from_u64(200);
        let team_a = sample_team("gt4", "T001", &mut team_rng);
        let team_b = sample_team("gt4", "T002", &mut team_rng);
        team_queries::insert_team(&conn, &team_a).expect("team a");
        team_queries::insert_team(&conn, &team_b).expect("team b");

        let driver_a = sample_driver("P001", "Piloto A", Some("gt4"), 78.0, DriverStatus::Ativo);
        let driver_b = sample_driver("P002", "Piloto B", Some("gt4"), 66.0, DriverStatus::Ativo);
        let driver_c = sample_driver(
            "P003",
            "Piloto C",
            Some("gt4"),
            62.0,
            DriverStatus::Aposentado,
        );
        let driver_d = sample_driver("P004", "Piloto D", Some("gt4"), 74.0, DriverStatus::Ativo);
        let driver_e = sample_driver("P005", "Piloto E", None, 59.0, DriverStatus::Ativo);
        let driver_f = sample_driver("P006", "Piloto F", Some("gt3"), 76.0, DriverStatus::Ativo);
        let mut player = sample_driver("P007", "Jogador", Some("gt4"), 72.0, DriverStatus::Ativo);
        player.is_jogador = true;
        for driver in [
            &driver_a, &driver_b, &driver_c, &driver_d, &driver_e, &driver_f, &player,
        ] {
            driver_queries::insert_driver(&conn, driver).expect("insert driver");
        }

        let contract_a = Contract::new(
            "C001".to_string(),
            driver_a.id.clone(),
            driver_a.nome.clone(),
            team_a.id.clone(),
            team_a.nome.clone(),
            1,
            1,
            140_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        let contract_b = Contract::new(
            "C002".to_string(),
            driver_b.id.clone(),
            driver_b.nome.clone(),
            team_a.id.clone(),
            team_a.nome.clone(),
            1,
            1,
            95_000.0,
            TeamRole::Numero2,
            "gt4".to_string(),
        );
        let contract_c = Contract::new(
            "C003".to_string(),
            driver_c.id.clone(),
            driver_c.nome.clone(),
            team_b.id.clone(),
            team_b.nome.clone(),
            1,
            2,
            85_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        let contract_d = Contract::new(
            "C004".to_string(),
            player.id.clone(),
            player.nome.clone(),
            team_b.id.clone(),
            team_b.nome.clone(),
            1,
            1,
            90_000.0,
            TeamRole::Numero2,
            "gt4".to_string(),
        );
        contract_queries::insert_contract(&conn, &contract_a).expect("contract a");
        contract_queries::insert_contract(&conn, &contract_b).expect("contract b");
        contract_queries::insert_contract(&conn, &contract_c).expect("contract c");
        contract_queries::insert_contract(&conn, &contract_d).expect("contract d");

        team_queries::update_team_pilots(&conn, &team_a.id, Some(&driver_a.id), Some(&driver_b.id))
            .expect("team a pilots");
        team_queries::update_team_pilots(&conn, &team_b.id, Some(&driver_c.id), Some(&player.id))
            .expect("team b pilots");

        insert_standing(
            &conn,
            &previous.id,
            &driver_a.id,
            &team_a.id,
            "gt4",
            1,
            120.0,
            3,
            2,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_b.id,
            &team_a.id,
            "gt4",
            4,
            72.0,
            1,
            1,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_c.id,
            &team_b.id,
            "gt4",
            6,
            40.0,
            0,
            0,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_d.id,
            &team_b.id,
            "gt4",
            2,
            96.0,
            2,
            1,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_f.id,
            &team_a.id,
            "gt3",
            3,
            88.0,
            1,
            2,
        );
        insert_standing(
            &conn,
            &previous.id,
            &player.id,
            &team_b.id,
            "gt4",
            5,
            60.0,
            0,
            0,
        );

        // Licenças — necessárias para que o filtro de mercado não bloqueie os pilotos.
        // gt4 exige nível 2, gt3 exige nível 3.
        for (piloto_id, nivel) in [
            ("P001", 2), ("P002", 2), ("P003", 2), ("P004", 2),
            ("P005", 0), ("P006", 3), ("P007", 2),
        ] {
            conn.execute(
                "INSERT INTO licenses (piloto_id, nivel, categoria_origem, data_obtencao, temporadas_na_categoria)
                 VALUES (?1, ?2, 'gt4', '2024', 3)",
                params![piloto_id, nivel.to_string()],
            )
            .expect("insert license");
        }

        conn.execute(
            "UPDATE meta SET value = '5' WHERE key = 'next_contract_id'",
            [],
        )
        .expect("contract counter");
        conn.execute(
            "UPDATE meta SET value = '8' WHERE key = 'next_driver_id'",
            [],
        )
        .expect("driver counter");

        conn
    }

    fn sample_team(category: &str, id: &str, rng: &mut StdRng) -> crate::models::team::Team {
        let template = get_team_templates(category)[0];
        crate::models::team::Team::from_template_with_rng(
            template,
            category,
            id.to_string(),
            2025,
            rng,
        )
    }

    fn sample_driver(
        id: &str,
        name: &str,
        category: Option<&str>,
        skill: f64,
        status: DriverStatus,
    ) -> Driver {
        let mut driver = Driver::new(
            id.to_string(),
            name.to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            24,
            2020,
        );
        driver.categoria_atual = category.map(str::to_string);
        driver.status = status;
        driver.atributos.skill = skill;
        driver.atributos.consistencia = 68.0;
        driver.stats_temporada.vitorias = 1;
        driver.stats_temporada.poles = 1;
        driver.stats_carreira.titulos = 1;
        driver
    }

    fn insert_standing(
        conn: &Connection,
        season_id: &str,
        driver_id: &str,
        team_id: &str,
        category: &str,
        position: i32,
        points: f64,
        wins: i32,
        poles: i32,
    ) {
        conn.execute(
            "INSERT INTO standings (
                temporada_id, piloto_id, equipe_id, categoria, posicao, pontos, vitorias, podios, poles, corridas
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![season_id, driver_id, team_id, category, position, points, wins, wins + 1, poles, 8],
        )
        .expect("insert standing");
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("iracerapp_{label}_{nanos}"))
    }
}
