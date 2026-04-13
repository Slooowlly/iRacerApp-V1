use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::calendar::{generate_all_calendars_with_year, CalendarEntry};
use crate::commands::career_detail::build_driver_detail_payload;
use crate::commands::career_types::{
    BriefingPhraseEntry, BriefingPhraseEntryInput, BriefingPhraseHistory, BriefingStorySummary,
    AcceptedSpecialOfferSummary, CareerData, CareerResumeContext, CareerResumeView,
    ContractWarningInfo, CreateCareerResult, DriverDetail, DriverSummary,
    NextRaceBriefingSummary, PrimaryRivalSummary, RaceSummary, SaveInfo, SeasonSummary,
    TeamStanding, TeamSummary, TrackHistorySummary,
    VerifyDatabaseResponse,
};
use crate::commands::race_history::{
    build_driver_histories, empty_previous_champions, ConstructorChampion, DriverRaceHistory,
    PreviousChampions, RoundResult, TrophyInfo,
};
use crate::config::app_config::{AppConfig, SaveMeta};
use crate::constants::{categories, scoring};
use crate::db::connection::Database;
use crate::db::queries::calendar as calendar_queries;
use crate::db::queries::contracts as contract_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::market_proposals as market_proposal_queries;
use crate::db::queries::meta as meta_queries;
use crate::db::queries::news as news_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::standings as standings_queries;
use crate::db::queries::standings::ChampionshipContext;
use crate::db::queries::teams as team_queries;
use crate::event_interest::{
    calculate_expected_event_interest, to_summary, EventInterestContext, EventInterestSummary,
};
use crate::evolution::pipeline::{run_end_of_season, EndOfSeasonResult};
use crate::generators::ids::{next_id, IdType};
use crate::generators::nationality::{format_nationality, get_nationality};
use crate::generators::world::generate_world;
use crate::market::pipeline::fill_all_remaining_vacancies;
use crate::market::preseason::{
    advance_week, delete_preseason_plan, load_preseason_plan, save_preseason_plan, PendingAction,
    PlannedEvent, PreSeasonPlan, PreSeasonState, WeekResult,
};
use crate::market::proposals::{MarketProposal, ProposalStatus};
use crate::models::driver::Driver;
use crate::models::enums::{ContractStatus, DriverStatus, SeasonPhase, TeamRole};
use crate::models::license::{
    driver_has_required_license_for_category, ensure_driver_can_join_category,
    grant_driver_license_for_category_if_needed,
};
use crate::models::season::Season;
use crate::models::team::{Team, TeamHierarchyClimate};
use crate::news::{NewsImportance, NewsItem, NewsType};

pub use crate::commands::career_types::CreateCareerInput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProposalView {
    pub proposal_id: String,
    pub equipe_id: String,
    pub equipe_nome: String,
    pub equipe_cor_primaria: String,
    pub equipe_cor_secundaria: String,
    pub categoria: String,
    pub categoria_nome: String,
    pub categoria_tier: u8,
    pub papel: String,
    pub salario_oferecido: f64,
    pub duracao_anos: i32,
    pub car_performance: f64,
    pub car_performance_rating: u8,
    pub reputacao: f64,
    pub companheiro_nome: Option<String>,
    pub companheiro_skill: Option<u8>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResponse {
    pub success: bool,
    pub action: String,
    pub message: String,
    pub new_team_name: Option<String>,
    pub remaining_proposals: i32,
    pub news_generated: Vec<String>,
}

pub(crate) fn create_career_in_base_dir(
    base_dir: &Path,
    input: CreateCareerInput,
) -> Result<CreateCareerResult, String> {
    validate_create_career_input(&input)?;

    let normalized_name = input.player_name.trim().to_string();
    let normalized_nationality = input.player_nationality.trim().to_lowercase();
    let normalized_category = input.category.trim().to_lowercase();
    let normalized_difficulty = input.difficulty.trim().to_lowercase();
    let normalized_age = input.player_age.unwrap_or(20).clamp(16, 60);
    let nationality_label = format_nationality(&normalized_nationality, "M", "pt-BR");

    let mut config = AppConfig::load_or_default(base_dir);
    let saves_dir = config.saves_dir();
    let career_id = next_career_id(&saves_dir);
    let career_number = career_number_from_id(&career_id)
        .ok_or_else(|| format!("Falha ao interpretar career_id '{career_id}'"))?;
    let career_dir = saves_dir.join(&career_id);
    let db_path = career_dir.join("career.db");
    let meta_path = career_dir.join("meta.json");

    std::fs::create_dir_all(&career_dir)
        .map_err(|e| format!("Falha ao criar diretorio da carreira: {e}"))?;

    let creation_result = (|| -> Result<CreateCareerResult, String> {
        let mut db = Database::create_new(&db_path)
            .map_err(|e| format!("Falha ao criar banco da carreira: {e}"))?;

        let world = generate_world(
            &normalized_name,
            &nationality_label,
            normalized_age,
            &normalized_category,
            input.team_index,
            &normalized_difficulty,
        )?;

        let season_id = next_id(&db.conn, IdType::Season)
            .map_err(|e| format!("Falha ao gerar ID da temporada: {e}"))?;
        let season = Season::new(season_id.clone(), 1, 2024);
        let calendars =
            generate_all_calendars_with_year(&season_id, season.ano, &mut rand::thread_rng())?;
        let total_races = count_total_races(&calendars);
        let all_calendar_entries: Vec<CalendarEntry> = calendars
            .values()
            .flat_map(|entries| entries.iter().cloned())
            .collect();

        db.transaction(|tx| {
            for driver in &world.drivers {
                driver_queries::insert_driver(tx, driver)?;
            }

            team_queries::insert_teams(tx, &world.teams)?;
            contract_queries::insert_contracts(tx, &world.contracts)?;
            for contract in &world.contracts {
                grant_driver_license_for_category_if_needed(
                    tx,
                    &contract.piloto_id,
                    &contract.categoria,
                )
                .map_err(crate::db::connection::DbError::Migration)?;
            }
            season_queries::insert_season(tx, &season)?;
            calendar_queries::insert_calendar_entries(tx, &all_calendar_entries)?;
            sync_meta_counters(
                tx,
                world.drivers.len(),
                world.teams.len(),
                world.contracts.len(),
                1,
                total_races,
            )?;
            Ok(())
        })
        .map_err(|e| format!("Falha ao persistir dados da carreira: {e}"))?;

        let player_team = world
            .teams
            .iter()
            .find(|team| team.id == world.player_team_id)
            .ok_or_else(|| "Equipe do jogador nao encontrada apos gerar o mundo".to_string())?;

        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let meta = serde_json::json!({
            "version": 1,
            "career_number": career_number,
            "player_name": normalized_name,
            "current_season": 1,
            "current_year": 2024,
            "created_at": now,
            "last_played": now,
            "team_name": player_team.nome,
            "category": normalized_category,
            "difficulty": normalized_difficulty,
            "total_races": total_races as i32,
        });

        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Falha ao serializar meta.json: {e}"))?;
        std::fs::write(&meta_path, meta_json)
            .map_err(|e| format!("Falha ao gravar meta.json: {e}"))?;

        config.last_career = Some(career_number);
        config
            .save()
            .map_err(|e| format!("Falha ao salvar config do app: {e}"))?;

        Ok(CreateCareerResult {
            success: true,
            career_id,
            save_path: career_dir.to_string_lossy().to_string(),
            player_id: world.player.id,
            player_team_id: player_team.id.clone(),
            player_team_name: player_team.nome.clone(),
            season_id,
            total_drivers: world.drivers.len(),
            total_teams: world.teams.len(),
            total_races,
            message: "Carreira criada com sucesso".to_string(),
        })
    })();

    if creation_result.is_err() && career_dir.exists() {
        let _ = std::fs::remove_dir_all(&career_dir);
    }

    creation_result
}

pub(crate) fn load_career_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<CareerData, String> {
    let career_number =
        career_number_from_id(career_id).ok_or_else(|| "ID de carreira invalido.".to_string())?;
    let mut config = AppConfig::load_or_default(base_dir);
    let (db, career_dir, mut meta) = open_career_resources(base_dir, career_id)?;
    let meta_path = career_dir.join("meta.json");
    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar piloto do jogador: {e}"))?;
    let player_team = find_player_team(&db.conn, &player.id, active_season.fase)?;
    let next_race = if let Some(ref team) = player_team {
        calendar_queries::get_next_race(&db.conn, &active_season.id, &team.categoria)
            .map_err(|e| format!("Falha ao carregar proxima corrida: {e}"))?
    } else {
        None
    };

    let total_drivers = driver_queries::count_drivers(&db.conn)
        .map_err(|e| format!("Falha ao contar pilotos: {e}"))? as usize;
    let total_teams =
        count_rows(&db.conn, "teams").map_err(|e| format!("Falha ao contar equipes: {e}"))?;
    let total_rodadas = if let Some(ref team) = player_team {
        count_calendar_entries(&db.conn, &active_season.id, &team.categoria)
            .map_err(|e| format!("Falha ao contar corridas da temporada: {e}"))?
    } else {
        0
    };

    // Calcular interesse esperado da próxima corrida (fallback silencioso se falhar).
    // Usa race.categoria como fonte semântica do campeonato do evento.
    let event_interest_summary: Option<EventInterestSummary> = next_race.as_ref().map(|race| {
        let champ = standings_queries::get_championship_context(&db.conn, &race.categoria)
            .unwrap_or(ChampionshipContext {
                player_position: 0,
                gap_to_leader: 0,
            });
        let remaining = total_rodadas - race.rodada;
        let is_title_decider =
            remaining <= 2 && champ.gap_to_leader <= 50 && champ.player_position > 0;
        let ctx = EventInterestContext {
            categoria: race.categoria.clone(),
            season_phase: race.season_phase,
            rodada: race.rodada,
            total_rodadas,
            week_of_year: race.week_of_year,
            track_id: race.track_id as i32,
            track_name: race.track_name.clone(),
            is_player_event: true,
            player_championship_position: if champ.player_position > 0 {
                Some(champ.player_position)
            } else {
                None
            },
            player_media: Some(player.atributos.midia as f32),
            championship_gap_to_leader: if champ.gap_to_leader > 0 || champ.player_position == 1 {
                Some(champ.gap_to_leader)
            } else {
                None
            },
            is_title_decider_candidate: is_title_decider,
            thematic_slot: race.thematic_slot,
        };
        let result = calculate_expected_event_interest(&ctx);
        to_summary(&result)
    });

    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    meta.last_played = now.clone();
    write_save_meta(&meta_path, &meta)?;
    config.last_career = Some(career_number);
    config
        .save()
        .map_err(|e| format!("Falha ao atualizar config do app: {e}"))?;

    let team_summary = player_team
        .as_ref()
        .map(|team| {
            build_team_summary(&db.conn, team)
                .map_err(|e| format!("Falha ao montar resumo da equipe: {e}"))
        })
        .transpose()?;
    let accepted_special_offer = build_accepted_special_offer_summary(&db.conn, &player)?;
    let next_race_summary = next_race.as_ref().map(|race| RaceSummary {
        id: race.id.clone(),
        rodada: race.rodada,
        track_name: race.track_name.clone(),
        clima: race.clima.as_str().to_string(),
        duracao_corrida_min: race.duracao_corrida_min,
        status: race.status.as_str().to_string(),
        temperatura: race.temperatura,
        horario: race.horario.clone(),
        week_of_year: race.week_of_year,
        season_phase: race.season_phase.as_str().to_string(),
        display_date: race.display_date.clone(),
        event_interest: event_interest_summary.clone(),
    });
    let next_race_briefing_summary = next_race.as_ref().map(|race| {
        build_next_race_briefing_summary(&db.conn, &player.id, active_season.numero, race)
            .unwrap_or_else(|_error| empty_next_race_briefing_summary())
    });
    let resume_context = read_resume_context(&career_dir)?;

    Ok(CareerData {
        career_id: career_id.to_string(),
        save_path: career_dir.to_string_lossy().to_string(),
        difficulty: meta.difficulty.clone(),
        player: DriverSummary {
            id: player.id.clone(),
            nome: player.nome.clone(),
            nacionalidade: player.nacionalidade.clone(),
            idade: player.idade as i32,
            skill: player.atributos.skill.round().clamp(0.0, 100.0) as u8,
            categoria_especial_ativa: player.categoria_especial_ativa.clone(),
            equipe_id: player_team.as_ref().map(|t| t.id.clone()),
            equipe_nome: player_team.as_ref().map(|t| t.nome.clone()),
            equipe_nome_curto: player_team.as_ref().map(|t| t.nome_curto.clone()),
            equipe_cor: player_team
                .as_ref()
                .map(|t| t.cor_primaria.clone())
                .unwrap_or_default(),
            is_jogador: player.is_jogador,
            pontos: player.stats_temporada.pontos.round() as i32,
            vitorias: player.stats_temporada.vitorias as i32,
            podios: player.stats_temporada.podios as i32,
            posicao_campeonato: 0,
            results: Vec::new(),
        },
        player_team: team_summary,
        season: SeasonSummary {
            id: active_season.id.clone(),
            numero: active_season.numero,
            ano: active_season.ano,
            rodada_atual: active_season.rodada_atual,
            total_rodadas,
            status: active_season.status.as_str().to_string(),
            fase: active_season.fase.as_str().to_string(),
        },
        accepted_special_offer,
        next_race: next_race_summary,
        next_race_briefing: next_race_briefing_summary,
        total_drivers,
        total_teams,
        resume_context,
    })
}

pub(crate) fn delete_career_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<String, String> {
    let career_number =
        career_number_from_id(career_id).ok_or_else(|| "ID de carreira invalido.".to_string())?;
    let mut config = AppConfig::load_or_default(base_dir);
    let career_dir = config.saves_dir().join(career_id);

    if !career_dir.exists() {
        return Err("Save nao encontrado.".to_string());
    }

    std::fs::remove_dir_all(&career_dir).map_err(|e| format!("Falha ao deletar save: {e}"))?;

    if config.last_career == Some(career_number) {
        config.last_career = None;
        config
            .save()
            .map_err(|e| format!("Falha ao atualizar config do app: {e}"))?;
    }

    Ok(format!("Carreira {career_id} deletada com sucesso."))
}

pub(crate) fn list_saves_in_base_dir(base_dir: &Path) -> Result<Vec<SaveInfo>, String> {
    let config = AppConfig::load_or_default(base_dir);
    Ok(config
        .list_saves()
        .into_iter()
        .map(save_meta_to_info)
        .collect())
}

fn validate_create_career_input(input: &CreateCareerInput) -> Result<(), String> {
    let name = input.player_name.trim();
    let nationality_id = input.player_nationality.trim().to_lowercase();
    let category = input.category.trim().to_lowercase();
    let difficulty = input.difficulty.trim().to_lowercase();
    if name.is_empty() {
        return Err("Informe um nome para o piloto.".to_string());
    }
    if name.chars().count() > 50 {
        return Err("O nome do piloto deve ter no maximo 50 caracteres.".to_string());
    }
    if get_nationality(&nationality_id).is_none() {
        return Err("Selecione uma nacionalidade valida.".to_string());
    }
    if !matches!(category.as_str(), "mazda_rookie" | "toyota_rookie") {
        return Err("A categoria inicial deve ser Mazda Rookie ou Toyota Rookie.".to_string());
    }
    if input.team_index > 5 {
        return Err("A equipe escolhida e invalida para a categoria inicial.".to_string());
    }
    if scoring::get_difficulty_config(&difficulty).is_none() {
        return Err("Selecione uma dificuldade valida.".to_string());
    }
    if let Some(age) = input.player_age {
        if !(16..=60).contains(&age) {
            return Err("A idade do piloto deve ficar entre 16 e 60 anos.".to_string());
        }
    }
    Ok(())
}

fn next_career_id(saves_dir: &Path) -> String {
    if !saves_dir.exists() {
        return "career_001".to_string();
    }

    let next_number = std::fs::read_dir(saves_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.strip_prefix("career_")?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0)
        + 1;

    format!("career_{next_number:03}")
}

fn career_number_from_id(career_id: &str) -> Option<u32> {
    career_id.strip_prefix("career_")?.parse::<u32>().ok()
}

fn count_total_races(calendars: &HashMap<String, Vec<CalendarEntry>>) -> usize {
    calendars.values().map(|entries| entries.len()).sum()
}

fn sync_meta_counters(
    conn: &rusqlite::Connection,
    total_drivers: usize,
    total_teams: usize,
    total_contracts: usize,
    total_seasons: usize,
    total_races: usize,
) -> Result<(), crate::db::connection::DbError> {
    meta_queries::set_meta_value(
        conn,
        "next_driver_id",
        &(total_drivers as u32 + 1).to_string(),
    )?;
    meta_queries::set_meta_value(conn, "next_team_id", &(total_teams as u32 + 1).to_string())?;
    meta_queries::set_meta_value(
        conn,
        "next_contract_id",
        &(total_contracts as u32 + 1).to_string(),
    )?;
    meta_queries::set_meta_value(
        conn,
        "next_season_id",
        &(total_seasons as u32 + 1).to_string(),
    )?;
    meta_queries::set_meta_value(conn, "next_race_id", &(total_races as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "current_season", &total_seasons.to_string())?;
    Ok(())
}

// Internal diagnostic helper kept out of the production Tauri command surface.
#[allow(dead_code)]
pub(crate) fn verify_database(
    app: AppHandle,
    career_number: u32,
) -> Result<VerifyDatabaseResponse, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.career_db_path(career_number);

    let db = Database::open_existing(&db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;

    let table_count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Falha ao contar tabelas: {e}"))?;

    Ok(VerifyDatabaseResponse {
        career_number,
        db_path: db_path.to_string_lossy().to_string(),
        table_count,
        status: "ok".to_string(),
    })
}

// Internal diagnostic helper kept out of the production Tauri command surface.
#[allow(dead_code)]
pub(crate) fn test_create_driver(
    app: AppHandle,
    career_number: u32,
    nome: String,
    nacionalidade: String,
    genero: String,
    category_tier: u32,
    difficulty: String,
) -> Result<Driver, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.career_db_path(career_number);
    let db = Database::open_existing(&db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;

    let id = next_id(&db.conn, IdType::Driver).map_err(|e| format!("Falha ao gerar ID: {e}"))?;

    let mut rng = rand::thread_rng();
    let category_id = match category_tier {
        0 => "mazda_rookie",
        1 => "mazda_amador",
        2 => "bmw_m2",
        3 => "gt4",
        4 => "gt3",
        _ => "endurance",
    };
    let mut existing_names = HashSet::new();
    let mut generated = Driver::generate_for_category(
        category_id,
        category_tier.min(5) as u8,
        &difficulty,
        1,
        &mut existing_names,
        &mut rng,
    );
    let mut driver = generated
        .pop()
        .ok_or_else(|| "Falha ao gerar piloto de teste".to_string())?;
    driver.id = id;
    if !nome.trim().is_empty() {
        driver.nome = nome;
    }
    if !nacionalidade.trim().is_empty() {
        driver.nacionalidade = nacionalidade;
    }
    if !genero.trim().is_empty() {
        driver.genero = genero;
    }

    driver_queries::insert_driver(&db.conn, &driver)
        .map_err(|e| format!("Falha ao inserir piloto: {e}"))?;

    Ok(driver)
}

// Internal diagnostic helper kept out of the production Tauri command surface.
#[allow(dead_code)]
pub(crate) fn test_list_drivers(app: AppHandle, career_number: u32) -> Result<Vec<Driver>, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.career_db_path(career_number);
    let db = Database::open_existing(&db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;

    driver_queries::get_all_drivers(&db.conn).map_err(|e| format!("Falha ao listar pilotos: {e}"))
}

pub(crate) fn get_driver_in_base_dir(
    base_dir: &Path,
    career_number: u32,
    driver_id: &str,
) -> Result<Driver, String> {
    let config = AppConfig::load_or_default(&base_dir);
    let db_path = config.career_db_path(career_number);
    let db = Database::open_existing(&db_path).map_err(|e| format!("Falha ao abrir banco: {e}"))?;

    driver_queries::get_driver(&db.conn, driver_id)
        .map_err(|e| format!("Falha ao buscar piloto: {e}"))
}

pub(crate) fn advance_season_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<EndOfSeasonResult, String> {
    let career_number =
        career_number_from_id(career_id).ok_or_else(|| "ID de carreira invalido.".to_string())?;
    let mut config = AppConfig::load_or_default(base_dir);
    let (mut db, career_dir, mut meta) = open_career_resources(base_dir, career_id)?;
    let meta_path = career_dir.join("meta.json");
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

    // Bloqueia avanço se o bloco especial ainda não foi encerrado formalmente.
    // O ciclo correto é: BlocoEspecial → encerrar_bloco_especial → PosEspecial
    //                    → run_pos_especial → advance_season.
    match season.fase {
        SeasonPhase::JanelaConvocacao | SeasonPhase::BlocoEspecial => {
            return Err(format!(
                "Nao e possivel avançar a temporada na fase '{}'. Encerre o bloco especial primeiro.",
                season.fase
            ));
        }
        SeasonPhase::BlocoRegular | SeasonPhase::PosEspecial => {} // permitido
    }

    let pending_races = calendar_queries::get_pending_races(&db.conn, &season.id)
        .map_err(|e| format!("Falha ao verificar corridas pendentes: {e}"))?;
    if !pending_races.is_empty() {
        let mut pending_categories: Vec<String> = pending_races
            .iter()
            .map(|race| race.categoria.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        pending_categories.sort();
        return Err(format!(
            "Ainda existem {} corridas pendentes na temporada {} ({})",
            pending_races.len(),
            season.numero,
            pending_categories.join(", ")
        ));
    }

    // Backup canônico de fim de temporada — antes de qualquer mutação da próxima.
    // Falha aqui bloqueia o pipeline: melhor abortar do que avançar sem rede de segurança.
    let db_path = career_dir.join("career.db");
    crate::commands::save::backup_season_internal(
        &db_path,
        &career_dir,
        season.numero as u32,
        &meta_path,
    )
    .map_err(|e| format!("Falha ao criar backup de fim de temporada: {e}"))?;

    let result = run_end_of_season(&mut db.conn, &season, &career_dir)?;
    warn_if_noncritical(
        persist_end_of_season_news(&db.conn, &result, season.numero),
        "Falha ao persistir noticias de fim de temporada",
    );
    let total_races = count_season_calendar_entries(&db.conn, &result.new_season_id)
        .map_err(|e| format!("Falha ao contar corridas da nova temporada: {e}"))?;
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    meta.current_season = (season.numero + 1).max(1) as u32;
    meta.current_year = result.new_year.max(0) as u32;
    meta.last_played = now;
    meta.total_races = total_races;
    warn_if_noncritical(
        write_save_meta(&meta_path, &meta),
        "Falha ao atualizar meta.json apos avancar temporada",
    );

    config.last_career = Some(career_number);
    warn_if_noncritical(
        config
            .save()
            .map_err(|e| format!("Falha ao atualizar config do app: {e}")),
        "Falha ao atualizar config do app apos avancar temporada",
    );

    warn_if_noncritical(
        write_resume_context(
            &career_dir,
            &CareerResumeContext {
                active_view: CareerResumeView::EndOfSeason,
                end_of_season_result: Some(result.clone()),
            },
        ),
        "Falha ao persistir resume_context apos avancar temporada",
    );

    Ok(result)
}

/// Simula todas as corridas pendentes da temporada sem participação do jogador,
/// conduzindo a temporada por todas as fases: BlocoRegular → JanelaConvocacao →
/// BlocoEspecial → PosEspecial. Após esta função, advance_season pode ser chamado.
/// Usado quando o jogador está sem equipe e quer pular para a próxima pré-temporada.
pub(crate) fn skip_all_pending_races_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<(), String> {
    let config = AppConfig::load_or_default(base_dir);
    let career_dir = config.saves_dir().join(career_id);
    let db_path = career_dir.join("career.db");
    let mut db = Database::open_existing(&db_path)
        .map_err(|e| format!("Falha ao abrir banco da carreira: {e}"))?;

    // ── Fase 1: BlocoRegular ─────────────────────────────────────────────────
    {
        let season = season_queries::get_active_season(&db.conn)
            .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
            .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

        if season.fase == SeasonPhase::BlocoRegular {
            let pending = calendar_queries::get_pending_races(&db.conn, &season.id)
                .map_err(|e| format!("Falha ao buscar corridas pendentes: {e}"))?;
            for race in &pending {
                crate::commands::race::simulate_category_race(&mut db, race, false)?;
            }
            crate::convocation::advance_to_convocation_window(&db.conn)
                .map_err(|e| format!("Falha ao avancar para janela de convocacao: {e}"))?;
        }
    }

    // ── Fase 2: JanelaConvocacao ─────────────────────────────────────────────
    {
        let season = season_queries::get_active_season(&db.conn)
            .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
            .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

        if season.fase == SeasonPhase::JanelaConvocacao {
            crate::convocation::run_convocation_window(&db.conn)
                .map_err(|e| format!("Falha ao executar janela de convocacao: {e}"))?;
            crate::convocation::iniciar_bloco_especial(&db.conn)
                .map_err(|e| format!("Falha ao iniciar bloco especial: {e}"))?;
        }
    }

    // ── Fase 3: BlocoEspecial ────────────────────────────────────────────────
    {
        let season = season_queries::get_active_season(&db.conn)
            .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
            .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

        if season.fase == SeasonPhase::BlocoEspecial {
            let player = driver_queries::get_player_driver(&db.conn)
                .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
            if player.categoria_especial_ativa.is_some() {
                return Err(
                    "O jogador participa do bloco especial ativo e deve correr essa fase normalmente."
                        .to_string(),
                );
            }

            for category_id in ["production_challenger", "endurance"] {
                let pending = calendar_queries::get_pending_races_for_category(
                    &db.conn,
                    &season.id,
                    category_id,
                )
                .map_err(|e| {
                    format!("Falha ao buscar corridas pendentes de {}: {e}", category_id)
                })?;
                for race in &pending {
                    crate::commands::race::simulate_category_race(&mut db, race, false)?;
                }
            }

            crate::convocation::encerrar_bloco_especial(&db.conn)
                .map_err(|e| format!("Falha ao encerrar bloco especial: {e}"))?;
            crate::convocation::run_pos_especial(&db.conn)
                .map_err(|e| format!("Falha ao executar pos-especial: {e}"))?;
        }
    }

    Ok(())
}

pub(crate) fn advance_market_week_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<WeekResult, String> {
    let _career_number =
        career_number_from_id(career_id).ok_or_else(|| "ID de carreira invalido.".to_string())?;
    let (db, career_dir, mut meta) = open_career_resources(base_dir, career_id)?;
    let meta_path = career_dir.join("meta.json");
    let mut plan = load_preseason_plan(&career_dir)?
        .ok_or_else(|| "Plano da pre-temporada nao encontrado.".to_string())?;
    let tx = db
        .conn
        .unchecked_transaction()
        .map_err(|e| format!("Falha ao iniciar transacao da semana de mercado: {e}"))?;
    let result = advance_week(&tx, &mut plan)?;
    warn_if_noncritical(
        persist_market_week_news(&tx, &plan.state, &result),
        "Falha ao persistir noticias da semana de mercado",
    );
    crate::market::preseason::save_preseason_plan(&career_dir, &plan)?;
    tx.commit()
        .map_err(|e| format!("Falha ao confirmar semana de mercado: {e}"))?;

    meta.last_played = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    warn_if_noncritical(
        write_save_meta(&meta_path, &meta),
        "Falha ao atualizar meta.json apos avancar semana de mercado",
    );
    Ok(result)
}

pub(crate) fn get_preseason_state_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<PreSeasonState, String> {
    let (db, career_dir, _) = open_career_resources(base_dir, career_id)?;
    let mut plan = load_preseason_plan(&career_dir)?
        .ok_or_else(|| "Plano da pre-temporada nao encontrado.".to_string())?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao carregar temporada da pre-temporada: {e}"))?
        .ok_or_else(|| format!("Temporada {} nao encontrada", plan.state.season_number))?;
    if season.numero != plan.state.season_number {
        return Err(format!(
            "Plano de pre-temporada desatualizado para a temporada ativa {}.",
            season.numero
        ));
    }
    crate::market::preseason::refresh_preseason_state_display_date(
        &db.conn,
        &season.id,
        &mut plan.state,
    )?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    plan.state.player_has_team =
        contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
            .map(|c| c.is_some())
            .unwrap_or(false);
    Ok(plan.state)
}

pub(crate) fn get_player_proposals_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<Vec<PlayerProposalView>, String> {
    let (db, _career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao carregar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    let mut proposals =
        market_proposal_queries::get_pending_player_proposals(&db.conn, &season.id, &player.id)
            .map_err(|e| format!("Falha ao buscar propostas pendentes: {e}"))?
            .into_iter()
            .map(|proposal| build_player_proposal_view(&db.conn, &proposal))
            .collect::<Result<Vec<_>, _>>()?;
    proposals.sort_by(|a, b| b.car_performance.total_cmp(&a.car_performance));
    Ok(proposals)
}

pub(crate) fn respond_to_proposal_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    proposal_id: &str,
    accept: bool,
) -> Result<ProposalResponse, String> {
    let (mut db, career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao carregar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let proposal =
        market_proposal_queries::get_market_proposal_by_id(&db.conn, &season.id, proposal_id)
            .map_err(|e| format!("Falha ao carregar proposta: {e}"))?
            .ok_or_else(|| "Proposta nao encontrada.".to_string())?;
    if proposal.piloto_id != player.id {
        return Err("A proposta nao pertence ao jogador.".to_string());
    }
    if proposal.status != ProposalStatus::Pendente {
        return Err("A proposta nao esta mais pendente.".to_string());
    }

    let mut news_items = Vec::new();
    let mut new_team_name = None;
    let action = if accept { "accepted" } else { "rejected" }.to_string();

    if accept {
        let tx = db
            .conn
            .transaction()
            .map_err(|e| format!("Falha ao iniciar transacao de aceite: {e}"))?;
        accept_player_proposal_tx(&tx, &player, &season, &proposal)?;
        tx.commit()
            .map_err(|e| format!("Falha ao confirmar aceite da proposta: {e}"))?;

        warn_if_noncritical(
            reconcile_plan_after_player_accept(&career_dir, &db.conn, &proposal),
            "Falha ao reconciliar plano apos aceite da proposta",
        );
        new_team_name = Some(proposal.equipe_nome.clone());
    } else {
        let tx = db
            .conn
            .transaction()
            .map_err(|e| format!("Falha ao iniciar transacao de recusa: {e}"))?;
        market_proposal_queries::update_proposal_status(
            &tx,
            &proposal.id,
            "Recusada",
            Some("Jogador recusou a proposta"),
        )
        .map_err(|e| format!("Falha ao recusar proposta: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Falha ao confirmar recusa da proposta: {e}"))?;
    }

    let mut remaining =
        market_proposal_queries::count_pending_player_proposals(&db.conn, &season.id, &player.id)
            .map_err(|e| format!("Falha ao contar propostas pendentes: {e}"))?;

    if !accept && remaining == 0 {
        if contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
            .map_err(|e| format!("Falha ao verificar equipe regular do jogador: {e}"))?
            .is_none()
        {
            let emergency = generate_emergency_player_proposals(&db.conn, &player, &season)?;
            if emergency.is_empty() {
                if let Some(team_name) =
                    force_place_player(&db.conn, &player, &season, &mut news_items)?
                {
                    new_team_name = Some(team_name);
                }
            } else {
                remaining = emergency.len() as i32;
            }
        }
    }

    warn_if_noncritical(
        sync_preseason_pending_flag(&career_dir, remaining > 0),
        "Falha ao sincronizar indicador de propostas pendentes",
    );
    let headlines = news_items
        .iter()
        .map(|item| item.titulo.clone())
        .collect::<Vec<_>>();

    let message = if accept {
        format!(
            "Voce assinou com {} como {}!",
            proposal.equipe_nome,
            if proposal.papel == TeamRole::Numero1 {
                "N1"
            } else {
                "N2"
            }
        )
    } else if let Some(team_name) = &new_team_name {
        format!(
            "Voce recusou a proposta de {}. O mercado o alocou em {} para evitar que fique sem equipe.",
            proposal.equipe_nome, team_name
        )
    } else if remaining > 0 {
        format!(
            "Voce recusou a proposta de {}. Novas opcoes emergenciais foram geradas.",
            proposal.equipe_nome
        )
    } else {
        format!("Voce recusou a proposta de {}.", proposal.equipe_nome)
    };

    Ok(ProposalResponse {
        success: true,
        action,
        message,
        new_team_name,
        remaining_proposals: remaining,
        news_generated: headlines,
    })
}

pub(crate) fn get_news_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    season: Option<i32>,
    tipo: Option<&str>,
    limit: Option<i32>,
) -> Result<Vec<NewsItem>, String> {
    let (db, _career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    let max_items = limit.unwrap_or(50).clamp(1, 400);
    let query_limit = if tipo.is_some() { 400 } else { max_items };
    let mut items = match season {
        Some(season_number) => {
            news_queries::get_news_by_season(&db.conn, season_number, query_limit)
                .map_err(|e| format!("Falha ao buscar noticias por temporada: {e}"))?
        }
        None => news_queries::get_recent_news(&db.conn, query_limit)
            .map_err(|e| format!("Falha ao buscar noticias recentes: {e}"))?,
    };

    if let Some(tipo) = tipo {
        let tipo_normalizado = NewsType::from_str_strict(tipo)
            .map_err(|e| format!("Filtro de noticia invalido: {e}"))?;
        items.retain(|item| item.tipo == tipo_normalizado);
    }

    items.truncate(max_items as usize);
    Ok(items)
}

pub(crate) fn finalize_preseason_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<(), String> {
    let (db, career_dir, mut meta) = open_career_resources(base_dir, career_id)?;
    let meta_path = career_dir.join("meta.json");
    let plan = load_preseason_plan(&career_dir)?
        .ok_or_else(|| "Plano da pre-temporada nao encontrado.".to_string())?;
    if !plan.state.is_complete {
        return Err("Pre-temporada ainda nao foi concluida.".to_string());
    }

    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao carregar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    let pending =
        market_proposal_queries::count_pending_player_proposals(&db.conn, &season.id, &player.id)
            .map_err(|e| format!("Falha ao contar propostas pendentes: {e}"))?;
    if pending > 0 {
        return Err(format!(
            "Voce tem {} proposta(s) pendente(s). Resolva antes de iniciar a temporada.",
            pending
        ));
    }

    let mut rng = rand::thread_rng();

    // 1. Invariante: Garantir que todas as equipes regulares tenham lineup completo antes de iniciar
    fill_all_remaining_vacancies(&db.conn, season.numero, &mut rng)
        .map_err(|e| format!("Falha ao preencher vagas remanescentes: {e}"))?;

    // 1b. Invariante: Garantir que N1/N2 de toda equipe regular está alinhado com o lineup final.
    // Normaliza equipes preenchidas por fallback que não passaram pelo UpdateHierarchy do mercado.
    crate::hierarchy::transition::validate_and_normalize_team_hierarchies(&db.conn)?;

    // 2. Limpar artefatos da corrida anterior (cache do dashboard)
    let results_path = career_dir.join("race_results.json");
    if results_path.exists() {
        let _ = std::fs::remove_file(&results_path);
    }

    delete_preseason_plan(&career_dir)?;
    delete_resume_context(&career_dir)?;
    meta.last_played = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    write_save_meta(&meta_path, &meta)?;

    Ok(())
}

pub(crate) fn get_preseason_free_agents_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<Vec<crate::commands::career_types::FreeAgentPreview>, String> {
    let (db, _, _) = open_career_resources(base_dir, career_id)?;
    let raw = contract_queries::get_free_agents_for_preseason(&db.conn)
        .map_err(|e| format!("Falha ao buscar agentes livres: {e}"))?;

    let result = raw
        .into_iter()
        .map(|r| {
            let abbr = r
                .previous_team_name
                .as_deref()
                .map(|name| name.chars().take(3).collect::<String>().to_uppercase());
            let (license_nivel, license_sigla) = match r.max_license_level {
                Some(0) => ("Rookie", "R"),
                Some(1) => ("Amador", "A"),
                Some(2) => ("Pro", "P"),
                Some(3) => ("Super Pro", "SP"),
                Some(4) => ("Elite", "E"),
                Some(_) => ("Super Elite", "SE"),
                None => ("Rookie", "R"),
            };
            crate::commands::career_types::FreeAgentPreview {
                driver_id: r.driver_id,
                driver_name: r.driver_name,
                categoria: r.categoria,
                is_rookie: r.is_rookie,
                previous_team_name: r.previous_team_name,
                previous_team_color: r.previous_team_color,
                previous_team_abbr: abbr,
                seasons_at_last_team: r.seasons_at_last_team,
                total_career_seasons: r.total_career_seasons,
                license_nivel: license_nivel.to_string(),
                license_sigla: license_sigla.to_string(),
            }
        })
        .collect();

    Ok(result)
}

pub(crate) fn get_driver_detail_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    driver_id: &str,
) -> Result<DriverDetail, String> {
    let (db, career_dir, _) = open_career_resources(base_dir, career_id)?;
    let driver = driver_queries::get_driver(&db.conn, driver_id)
        .map_err(|e| format!("Falha ao buscar piloto: {e}"))?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let contract = preferred_active_contract_for_phase(&db.conn, driver_id, season.fase)?;
    let team = resolve_driver_team(&db.conn, driver_id, contract.as_ref())?;
    let role = resolve_driver_role(driver_id, contract.as_ref(), team.as_ref());

    build_driver_detail_payload(
        &db.conn,
        &career_dir,
        &season,
        &driver,
        contract.as_ref(),
        team.as_ref(),
        role,
    )
}

fn read_save_meta(path: &Path) -> Result<SaveMeta, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Falha ao ler meta.json: {e}"))?;
    serde_json::from_str::<SaveMeta>(&content)
        .map_err(|e| format!("Falha ao parsear meta.json: {e}"))
}

fn resume_context_path(career_dir: &Path) -> PathBuf {
    career_dir.join("resume_context.json")
}

fn read_resume_context(career_dir: &Path) -> Result<Option<CareerResumeContext>, String> {
    let path = resume_context_path(career_dir);
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Falha ao ler resume_context.json: {e}"))?;
    let context = serde_json::from_str::<CareerResumeContext>(&content)
        .map_err(|e| format!("Falha ao parsear resume_context.json: {e}"))?;
    normalize_resume_context(career_dir, context)
}

fn normalize_resume_context(
    career_dir: &Path,
    context: CareerResumeContext,
) -> Result<Option<CareerResumeContext>, String> {
    match context.active_view {
        CareerResumeView::Dashboard => Ok(None),
        CareerResumeView::EndOfSeason => {
            if context.end_of_season_result.is_some() {
                Ok(Some(context))
            } else if load_preseason_plan(career_dir)?.is_some() {
                Ok(Some(CareerResumeContext {
                    active_view: CareerResumeView::Preseason,
                    end_of_season_result: None,
                }))
            } else {
                Ok(None)
            }
        }
        CareerResumeView::Preseason => {
            if load_preseason_plan(career_dir)?.is_some() {
                Ok(Some(CareerResumeContext {
                    active_view: CareerResumeView::Preseason,
                    end_of_season_result: None,
                }))
            } else {
                Ok(None)
            }
        }
    }
}

fn write_resume_context(career_dir: &Path, context: &CareerResumeContext) -> Result<(), String> {
    let path = resume_context_path(career_dir);
    let payload = serde_json::to_string_pretty(context)
        .map_err(|e| format!("Falha ao serializar resume_context.json: {e}"))?;
    std::fs::write(&path, payload).map_err(|e| format!("Falha ao gravar resume_context.json: {e}"))
}

fn delete_resume_context(career_dir: &Path) -> Result<(), String> {
    let path = resume_context_path(career_dir);
    if !path.exists() {
        return Ok(());
    }

    std::fs::remove_file(&path).map_err(|e| format!("Falha ao remover resume_context.json: {e}"))
}

pub(crate) fn persist_resume_context_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    active_view: CareerResumeView,
    end_of_season_result: Option<EndOfSeasonResult>,
) -> Result<(), String> {
    let (_db, career_dir, _) = open_career_resources(base_dir, career_id)?;

    match active_view {
        CareerResumeView::Dashboard => delete_resume_context(&career_dir),
        CareerResumeView::EndOfSeason => {
            let Some(result) = end_of_season_result else {
                return Err(
                    "Estado de fim de temporada requer payload para restauracao.".to_string(),
                );
            };

            write_resume_context(
                &career_dir,
                &CareerResumeContext {
                    active_view,
                    end_of_season_result: Some(result),
                },
            )
        }
        CareerResumeView::Preseason => write_resume_context(
            &career_dir,
            &CareerResumeContext {
                active_view,
                end_of_season_result: None,
            },
        ),
    }
}

pub(crate) fn get_briefing_phrase_history_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<BriefingPhraseHistory, String> {
    let (_db, career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    read_briefing_phrase_history(&briefing_phrase_history_path(&career_dir))
}

pub(crate) fn save_briefing_phrase_history_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    season_number: i32,
    entries: Vec<BriefingPhraseEntryInput>,
) -> Result<BriefingPhraseHistory, String> {
    let (_db, career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    let history_path = briefing_phrase_history_path(&career_dir);
    let current = read_briefing_phrase_history(&history_path)?;
    let updated = merge_briefing_phrase_history(current, season_number, entries);
    write_briefing_phrase_history(&history_path, &updated)?;
    Ok(updated)
}

fn briefing_phrase_history_path(career_dir: &Path) -> PathBuf {
    career_dir.join("briefing_phrase_history.json")
}

fn read_briefing_phrase_history(path: &Path) -> Result<BriefingPhraseHistory, String> {
    if !path.exists() {
        return Ok(BriefingPhraseHistory::default());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Falha ao ler briefing_phrase_history.json: {e}"))?;
    serde_json::from_str::<BriefingPhraseHistory>(&content)
        .map_err(|e| format!("Falha ao parsear briefing_phrase_history.json: {e}"))
}

fn write_briefing_phrase_history(
    path: &Path,
    history: &BriefingPhraseHistory,
) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(history)
        .map_err(|e| format!("Falha ao serializar briefing_phrase_history.json: {e}"))?;
    std::fs::write(path, payload)
        .map_err(|e| format!("Falha ao gravar briefing_phrase_history.json: {e}"))
}

fn merge_briefing_phrase_history(
    current: BriefingPhraseHistory,
    season_number: i32,
    entries: Vec<BriefingPhraseEntryInput>,
) -> BriefingPhraseHistory {
    let mut merged_entries = if current.season_number == season_number {
        current.entries
    } else {
        Vec::new()
    };

    for entry in entries {
        merged_entries.retain(|existing| {
            !(existing.round_number == entry.round_number
                && existing.driver_id == entry.driver_id
                && existing.bucket_key == entry.bucket_key)
        });

        merged_entries.push(BriefingPhraseEntry {
            season_number,
            round_number: entry.round_number,
            driver_id: entry.driver_id,
            bucket_key: entry.bucket_key,
            phrase_id: entry.phrase_id,
        });
    }

    merged_entries.sort_by(|left, right| {
        right
            .round_number
            .cmp(&left.round_number)
            .then_with(|| left.driver_id.cmp(&right.driver_id))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });

    let mut per_bucket_counts: HashMap<(String, String), usize> = HashMap::new();
    merged_entries.retain(|entry| {
        let key = (entry.driver_id.clone(), entry.bucket_key.clone());
        let count = per_bucket_counts.entry(key).or_insert(0);
        if *count >= 5 {
            return false;
        }
        *count += 1;
        true
    });

    BriefingPhraseHistory {
        season_number,
        entries: merged_entries,
    }
}

fn persist_end_of_season_news(
    _conn: &rusqlite::Connection,
    _result: &EndOfSeasonResult,
    _season_number: i32,
) -> Result<(), String> {
    Ok(())
}

fn persist_market_week_news(
    _conn: &rusqlite::Connection,
    _state: &PreSeasonState,
    _week_result: &WeekResult,
) -> Result<(), String> {
    Ok(())
}

fn build_player_proposal_view(
    conn: &rusqlite::Connection,
    proposal: &MarketProposal,
) -> Result<PlayerProposalView, String> {
    let team = team_queries::get_team_by_id(conn, &proposal.equipe_id)
        .map_err(|e| format!("Falha ao carregar equipe da proposta: {e}"))?
        .ok_or_else(|| "Equipe da proposta nao encontrada.".to_string())?;
    let category = categories::get_category_config(&team.categoria)
        .ok_or_else(|| format!("Categoria '{}' nao encontrada", team.categoria))?;
    let companion_id = match proposal.papel {
        TeamRole::Numero1 => team
            .piloto_2_id
            .clone()
            .or_else(|| team.piloto_1_id.clone()),
        TeamRole::Numero2 => team
            .piloto_1_id
            .clone()
            .or_else(|| team.piloto_2_id.clone()),
    };
    let companion = companion_id
        .as_deref()
        .map(|id| driver_queries::get_driver(conn, id))
        .transpose()
        .map_err(|e| format!("Falha ao carregar companheiro de equipe: {e}"))?;
    Ok(PlayerProposalView {
        proposal_id: proposal.id.clone(),
        equipe_id: team.id.clone(),
        equipe_nome: team.nome.clone(),
        equipe_cor_primaria: team.cor_primaria.clone(),
        equipe_cor_secundaria: team.cor_secundaria.clone(),
        categoria: team.categoria.clone(),
        categoria_nome: category.nome_curto.to_string(),
        categoria_tier: category.tier,
        papel: if proposal.papel == TeamRole::Numero1 {
            "N1".to_string()
        } else {
            "N2".to_string()
        },
        salario_oferecido: proposal.salario_oferecido,
        duracao_anos: proposal.duracao_anos,
        car_performance: team.car_performance,
        car_performance_rating: normalize_car_performance(team.car_performance),
        reputacao: team.reputacao,
        companheiro_nome: companion.as_ref().map(|driver| driver.nome.clone()),
        companheiro_skill: companion
            .as_ref()
            .map(|driver| driver.atributos.skill.round().clamp(0.0, 100.0) as u8),
        status: proposal.status.as_str().to_string(),
    })
}

fn accept_player_proposal_tx(
    tx: &rusqlite::Transaction<'_>,
    player: &Driver,
    season: &Season,
    proposal: &MarketProposal,
) -> Result<(), String> {
    let previous_contract = contract_queries::get_active_regular_contract_for_pilot(tx, &player.id)
        .map_err(|e| format!("Falha ao buscar contrato regular atual do jogador: {e}"))?;
    let previous_team_id = previous_contract
        .as_ref()
        .map(|contract| contract.equipe_id.clone());

    if let Some(contract) = previous_contract {
        contract_queries::update_contract_status(tx, &contract.id, &ContractStatus::Rescindido)
            .map_err(|e| format!("Falha ao rescindir contrato atual: {e}"))?;
        team_queries::remove_pilot_from_team(tx, &player.id, &contract.equipe_id)
            .map_err(|e| format!("Falha ao remover jogador da equipe antiga: {e}"))?;
        refresh_team_hierarchy_now(tx, &contract.equipe_id)?;
    }

    let team = team_queries::get_team_by_id(tx, &proposal.equipe_id)
        .map_err(|e| format!("Falha ao carregar equipe da proposta: {e}"))?
        .ok_or_else(|| "Equipe da proposta nao encontrada.".to_string())?;
    ensure_driver_can_join_category(tx, &player.id, &player.nome, &team.categoria)?;
    let contract = crate::models::contract::Contract::new(
        next_id(tx, IdType::Contract).map_err(|e| format!("Falha ao gerar ID de contrato: {e}"))?,
        player.id.clone(),
        player.nome.clone(),
        team.id.clone(),
        team.nome.clone(),
        season.numero,
        proposal.duracao_anos,
        proposal.salario_oferecido,
        proposal.papel.clone(),
        team.categoria.clone(),
    );
    contract_queries::insert_contract(tx, &contract)
        .map_err(|e| format!("Falha ao criar novo contrato do jogador: {e}"))?;
    normalize_regular_contracts_for_team(tx, &team.id)?;
    refresh_team_hierarchy_now(tx, &team.id)?;

    let mut updated_player = player.clone();
    updated_player.categoria_atual = Some(team.categoria.clone());
    updated_player.status = crate::models::enums::DriverStatus::Ativo;
    driver_queries::update_driver(tx, &updated_player)
        .map_err(|e| format!("Falha ao atualizar categoria do jogador: {e}"))?;

    market_proposal_queries::update_proposal_status(tx, &proposal.id, "Aceita", None)
        .map_err(|e| format!("Falha ao marcar proposta como aceita: {e}"))?;
    market_proposal_queries::expire_remaining_proposals(tx, &season.id, &player.id, &proposal.id)
        .map_err(|e| format!("Falha ao expirar demais propostas: {e}"))?;

    if let Some(previous_team_id) = previous_team_id.filter(|old_team| old_team != &team.id) {
        backfill_team_vacancy(tx, &previous_team_id, season.numero)?;
        refresh_team_hierarchy_now(tx, &previous_team_id)?;
    }

    Ok(())
}

fn normalize_regular_contracts_for_team(
    conn: &rusqlite::Connection,
    team_id: &str,
) -> Result<bool, String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para normalizar contratos: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para normalizar contratos.".to_string())?;
    let mut active_regular_contracts = contract_queries::get_active_contracts_for_team(conn, team_id)
        .map_err(|e| format!("Falha ao carregar contratos ativos da equipe: {e}"))?
        .into_iter()
        .filter(|contract| contract.tipo == crate::models::enums::ContractType::Regular)
        .collect::<Vec<_>>();
    active_regular_contracts.sort_by(|a, b| {
        b.temporada_inicio
            .cmp(&a.temporada_inicio)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });

    let mut keep_n1 = None;
    let mut keep_n2 = None;
    let mut displaced_driver_ids = HashSet::new();

    for contract in active_regular_contracts {
        let slot = match contract.papel {
            TeamRole::Numero1 => &mut keep_n1,
            TeamRole::Numero2 => &mut keep_n2,
        };
        if slot.is_none() {
            *slot = Some(contract);
            continue;
        }

        contract_queries::update_contract_status(conn, &contract.id, &ContractStatus::Rescindido)
            .map_err(|e| {
                format!(
                    "Falha ao rescindir contrato regular excedente '{}': {e}",
                    contract.id
                )
            })?;
        displaced_driver_ids.insert(contract.piloto_id);
    }

    let piloto_1 = keep_n1.as_ref().map(|contract| contract.piloto_id.as_str());
    let piloto_2 = keep_n2.as_ref().map(|contract| contract.piloto_id.as_str());
    let changed = team.piloto_1_id.as_deref() != piloto_1
        || team.piloto_2_id.as_deref() != piloto_2
        || !displaced_driver_ids.is_empty();

    if team.piloto_1_id.as_deref() != piloto_1 || team.piloto_2_id.as_deref() != piloto_2 {
        team_queries::update_team_pilots(conn, team_id, piloto_1, piloto_2)
            .map_err(|e| format!("Falha ao atualizar lineup da equipe '{}': {e}", team.nome))?;
    }

    for driver_id in displaced_driver_ids {
        if contract_queries::get_active_contract_for_pilot(conn, &driver_id)
            .map_err(|e| format!("Falha ao verificar contrato remanescente de '{}': {e}", driver_id))?
            .is_some()
        {
            continue;
        }
        let mut driver = driver_queries::get_driver(conn, &driver_id)
            .map_err(|e| format!("Falha ao carregar piloto deslocado '{}': {e}", driver_id))?;
        if driver.categoria_atual.is_none() {
            continue;
        }
        driver.categoria_atual = None;
        driver_queries::update_driver(conn, &driver).map_err(|e| {
            format!(
                "Falha ao limpar categoria do piloto deslocado '{}': {e}",
                driver_id
            )
        })?;
    }

    Ok(changed)
}

fn place_driver_in_team(
    conn: &rusqlite::Connection,
    team_id: &str,
    driver_id: &str,
    role: TeamRole,
) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para encaixar jogador: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para encaixe do jogador.".to_string())?;
    let existing = [team.piloto_1_id.clone(), team.piloto_2_id.clone()]
        .into_iter()
        .flatten()
        .filter(|id| id != driver_id)
        .collect::<Vec<_>>();
    let (piloto_1, piloto_2) = match role {
        TeamRole::Numero1 => (Some(driver_id.to_string()), existing.first().cloned()),
        TeamRole::Numero2 => (existing.first().cloned(), Some(driver_id.to_string())),
    };
    team_queries::update_team_pilots(conn, team_id, piloto_1.as_deref(), piloto_2.as_deref())
        .map_err(|e| format!("Falha ao atualizar pilotos da nova equipe: {e}"))?;
    Ok(())
}

fn refresh_team_hierarchy_now(conn: &rusqlite::Connection, team_id: &str) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para hierarquia: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para hierarquia.".to_string())?;
    let mut candidates = [team.piloto_1_id.clone(), team.piloto_2_id.clone()]
        .into_iter()
        .flatten()
        .filter_map(|id| driver_queries::get_driver(conn, &id).ok())
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| b.atributos.skill.total_cmp(&a.atributos.skill));
    let n1_id = candidates.first().map(|driver| driver.id.as_str());
    let n2_id = candidates.get(1).map(|driver| driver.id.as_str());
    team_queries::update_team_hierarchy(
        conn,
        team_id,
        n1_id,
        n2_id,
        TeamHierarchyClimate::Estavel.as_str(),
        0.0,
    )
    .map_err(|e| format!("Falha ao atualizar hierarquia da equipe: {e}"))?;
    Ok(())
}

#[derive(Clone)]
struct TeamVacancy {
    team: Team,
    role: TeamRole,
}

fn list_team_vacancies(conn: &rusqlite::Connection) -> Result<Vec<TeamVacancy>, String> {
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao listar equipes: {e}"))?;
    let mut vacancies = Vec::new();
    for team in teams {
        if team.piloto_1_id.is_none() {
            vacancies.push(TeamVacancy {
                team: team.clone(),
                role: TeamRole::Numero1,
            });
        }
        if team.piloto_2_id.is_none() {
            vacancies.push(TeamVacancy {
                team,
                role: TeamRole::Numero2,
            });
        }
    }
    Ok(vacancies)
}

fn generate_emergency_player_proposals(
    conn: &rusqlite::Connection,
    player: &Driver,
    season: &Season,
) -> Result<Vec<MarketProposal>, String> {
    let player_tier = player
        .categoria_atual
        .as_deref()
        .and_then(categories::get_category_config)
        .map(|config| config.tier)
        .unwrap_or(0);
    let mut vacancies = Vec::new();
    for vacancy in list_team_vacancies(conn)? {
        let tier = categories::get_category_config(&vacancy.team.categoria)
            .map(|config| config.tier)
            .unwrap_or(0);
        let tier_ok = tier >= player_tier && tier <= player_tier + 1;
        if tier_ok
            && driver_has_required_license_for_category(conn, &player.id, &vacancy.team.categoria)?
        {
            vacancies.push(vacancy);
        }
    }
    if vacancies.is_empty() {
        for vacancy in list_team_vacancies(conn)? {
            if driver_has_required_license_for_category(conn, &player.id, &vacancy.team.categoria)?
            {
                vacancies.push(vacancy);
            }
        }
    }
    vacancies.sort_by(|a, b| b.team.car_performance.total_cmp(&a.team.car_performance));

    let mut created = Vec::new();
    for (index, vacancy) in vacancies.into_iter().take(2).enumerate() {
        let proposal = MarketProposal {
            id: format!(
                "MP-{}-{}-{}-EM-{}",
                season.numero, vacancy.team.id, player.id, index
            ),
            equipe_id: vacancy.team.id.clone(),
            equipe_nome: vacancy.team.nome.clone(),
            piloto_id: player.id.clone(),
            piloto_nome: player.nome.clone(),
            categoria: vacancy.team.categoria.clone(),
            papel: vacancy.role.clone(),
            salario_oferecido: calculate_offer_salary_for_team(&vacancy.team, player),
            duracao_anos: if categories::get_category_config(&vacancy.team.categoria)
                .map(|config| config.tier >= 3)
                .unwrap_or(false)
            {
                2
            } else {
                1
            },
            status: ProposalStatus::Pendente,
            motivo_recusa: None,
        };
        market_proposal_queries::insert_player_proposal(conn, &season.id, &proposal)
            .map_err(|e| format!("Falha ao persistir proposta emergencial: {e}"))?;
        created.push(proposal);
    }

    Ok(created)
}

fn force_place_player(
    conn: &rusqlite::Connection,
    player: &Driver,
    season: &Season,
    _news_items: &mut Vec<NewsItem>,
) -> Result<Option<String>, String> {
    let player_tier = player
        .categoria_atual
        .as_deref()
        .and_then(categories::get_category_config)
        .map(|config| config.tier)
        .unwrap_or(0);
    let mut vacancies = Vec::new();
    for vacancy in list_team_vacancies(conn)? {
        let tier_ok = categories::get_category_config(&vacancy.team.categoria)
            .map(|config| config.tier == player_tier)
            .unwrap_or(false);
        if tier_ok
            && driver_has_required_license_for_category(conn, &player.id, &vacancy.team.categoria)?
        {
            vacancies.push(vacancy);
        }
    }
    if vacancies.is_empty() {
        for vacancy in list_team_vacancies(conn)? {
            if driver_has_required_license_for_category(conn, &player.id, &vacancy.team.categoria)?
            {
                vacancies.push(vacancy);
            }
        }
    }
    vacancies.sort_by(|a, b| a.team.car_performance.total_cmp(&b.team.car_performance));
    let Some(vacancy) = vacancies.into_iter().next() else {
        return Ok(None);
    };
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Falha ao iniciar transacao de alocacao forcada: {e}"))?;
    ensure_driver_can_join_category(&tx, &player.id, &player.nome, &vacancy.team.categoria)?;

    let contract = crate::models::contract::Contract::new(
        next_id(&tx, IdType::Contract)
            .map_err(|e| format!("Falha ao gerar contrato forçado: {e}"))?,
        player.id.clone(),
        player.nome.clone(),
        vacancy.team.id.clone(),
        vacancy.team.nome.clone(),
        season.numero,
        1,
        calculate_offer_salary_for_team(&vacancy.team, player).max(5_000.0),
        vacancy.role.clone(),
        vacancy.team.categoria.clone(),
    );
    contract_queries::insert_contract(&tx, &contract)
        .map_err(|e| format!("Falha ao inserir contrato forçado: {e}"))?;
    place_driver_in_team(&tx, &vacancy.team.id, &player.id, vacancy.role.clone())?;
    refresh_team_hierarchy_now(&tx, &vacancy.team.id)?;
    let mut updated_player = player.clone();
    updated_player.categoria_atual = Some(vacancy.team.categoria.clone());
    updated_player.status = crate::models::enums::DriverStatus::Ativo;
    driver_queries::update_driver(&tx, &updated_player)
        .map_err(|e| format!("Falha ao atualizar jogador apos alocacao forcada: {e}"))?;
    tx.commit()
        .map_err(|e| format!("Falha ao confirmar alocacao forcada: {e}"))?;
    Ok(Some(vacancy.team.nome))
}

fn backfill_team_vacancy(
    conn: &rusqlite::Connection,
    team_id: &str,
    season_number: i32,
) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para reposicao: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para reposicao.".to_string())?;
    let role = if team.piloto_1_id.is_none() {
        TeamRole::Numero1
    } else if team.piloto_2_id.is_none() {
        TeamRole::Numero2
    } else {
        return Ok(());
    };

    let free_driver = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar pilotos para reposicao: {e}"))?
        .into_iter()
        .filter(|driver| driver.status == crate::models::enums::DriverStatus::Ativo)
        .filter(|driver| {
            contract_queries::get_active_regular_contract_for_pilot(conn, &driver.id)
                .ok()
                .flatten()
                .is_none()
        })
        .filter(|driver| {
            driver_has_required_license_for_category(conn, &driver.id, &team.categoria)
                .unwrap_or(false)
        })
        .max_by(|a, b| a.atributos.skill.total_cmp(&b.atributos.skill));

    let replacement = if let Some(driver) = free_driver {
        driver
    } else {
        let mut existing_names = driver_queries::get_all_drivers(conn)
            .map_err(|e| format!("Falha ao carregar nomes existentes: {e}"))?
            .into_iter()
            .map(|driver| driver.nome)
            .collect::<HashSet<_>>();
        let mut rng = rand::thread_rng();
        let mut rookie =
            crate::evolution::rookies::generate_rookies(1, &mut existing_names, &mut rng)
                .into_iter()
                .next()
                .ok_or_else(|| "Falha ao gerar rookie emergencial.".to_string())?;
        rookie.id = format!(
            "P-EM-{}",
            next_id(conn, IdType::Driver)
                .map_err(|e| format!("Falha ao gerar ID emergencial: {e}"))?
        );
        driver_queries::insert_driver(conn, &rookie)
            .map_err(|e| format!("Falha ao inserir rookie emergencial: {e}"))?;
        grant_driver_license_for_category_if_needed(conn, &rookie.id, &team.categoria)?;
        rookie
    };
    ensure_driver_can_join_category(conn, &replacement.id, &replacement.nome, &team.categoria)?;

    let contract = crate::models::contract::Contract::new(
        next_id(conn, IdType::Contract)
            .map_err(|e| format!("Falha ao gerar contrato de reposicao: {e}"))?,
        replacement.id.clone(),
        replacement.nome.clone(),
        team.id.clone(),
        team.nome.clone(),
        season_number,
        1,
        calculate_offer_salary_for_team(&team, &replacement).max(5_000.0),
        role.clone(),
        team.categoria.clone(),
    );
    contract_queries::insert_contract(conn, &contract)
        .map_err(|e| format!("Falha ao inserir contrato de reposicao: {e}"))?;
    place_driver_in_team(conn, &team.id, &replacement.id, role)?;
    let mut updated_driver = replacement.clone();
    updated_driver.categoria_atual = Some(team.categoria.clone());
    driver_queries::update_driver(conn, &updated_driver)
        .map_err(|e| format!("Falha ao atualizar piloto de reposicao: {e}"))?;
    Ok(())
}

fn calculate_offer_salary_for_team(team: &Team, player: &Driver) -> f64 {
    let tier_base = match categories::get_category_config(&team.categoria)
        .map(|config| config.tier)
        .unwrap_or(0)
    {
        0 => 12_000.0,
        1 => 28_000.0,
        2 => 55_000.0,
        3 => 105_000.0,
        4 => 210_000.0,
        _ => 160_000.0,
    };
    let skill_modifier = (player.atributos.skill / 70.0).max(0.7);
    let budget_modifier = (team.budget / 70.0).clamp(0.6, 1.5);
    (tier_base * skill_modifier * budget_modifier).max(5_000.0)
}

fn normalize_car_performance(car_performance: f64) -> u8 {
    (((car_performance + 5.0) / 21.0) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
}

fn pending_player_event_team_ids(event: &PendingAction, player_id: &str) -> Option<Vec<String>> {
    match event {
        PendingAction::ExpireContract {
            driver_id, team_id, ..
        } if driver_id == player_id => Some(vec![team_id.clone()]),
        PendingAction::RenewContract {
            driver_id, team_id, ..
        } if driver_id == player_id => Some(vec![team_id.clone()]),
        PendingAction::Transfer {
            driver_id,
            from_team_id,
            to_team_id,
            ..
        } if driver_id == player_id => {
            let mut team_ids = Vec::new();
            if let Some(from_team_id) = from_team_id {
                team_ids.push(from_team_id.clone());
            }
            team_ids.push(to_team_id.clone());
            Some(team_ids)
        }
        PendingAction::PlayerProposal { proposal } if proposal.piloto_id == player_id => {
            Some(vec![proposal.equipe_id.clone()])
        }
        PendingAction::PlaceRookie {
            driver, team_id, ..
        } if driver.id == player_id => Some(vec![team_id.clone()]),
        _ => None,
    }
}

fn is_team_role_vacant(
    conn: &rusqlite::Connection,
    team_id: &str,
    role: &str,
) -> Result<bool, String> {
    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para validar vaga: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para validar vaga.".to_string())?;
    let is_vacant = match TeamRole::from_str_strict(role)
        .map_err(|e| format!("Papel de equipe invalido ao validar vaga: {e}"))?
    {
        TeamRole::Numero1 => team.piloto_1_id.is_none(),
        TeamRole::Numero2 => team.piloto_2_id.is_none(),
    };
    Ok(is_vacant)
}

fn reconcile_plan_after_player_accept(
    career_dir: &Path,
    conn: &rusqlite::Connection,
    proposal: &MarketProposal,
) -> Result<(), String> {
    let Some(mut plan) = load_preseason_plan(career_dir)? else {
        return Ok(());
    };
    let mut affected_team_ids = HashSet::from([proposal.equipe_id.clone()]);
    plan.planned_events.retain(|event| {
        if event.executed {
            return true;
        }
        if let Some(team_ids) = pending_player_event_team_ids(&event.event, &proposal.piloto_id) {
            affected_team_ids.extend(team_ids);
            return false;
        }
        true
    });

    let stale_rookie_indices = plan
        .planned_events
        .iter()
        .enumerate()
        .filter(|(_, event)| !event.executed)
        .filter_map(|(index, event)| match &event.event {
            PendingAction::PlaceRookie { team_id, role, .. }
                if affected_team_ids.contains(team_id) =>
            {
                Some(
                    is_team_role_vacant(conn, team_id, role)
                        .map(|is_vacant| (!is_vacant).then_some(index)),
                )
            }
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    for index in stale_rookie_indices.into_iter().rev() {
        plan.planned_events.remove(index);
    }
    for team_id in affected_team_ids {
        refresh_planned_hierarchy_for_team(&mut plan, conn, &team_id)?;
    }
    plan.state.player_has_pending_proposals = false;
    save_preseason_plan(career_dir, &plan)
}

fn sync_preseason_pending_flag(career_dir: &Path, has_pending: bool) -> Result<(), String> {
    let Some(mut plan) = load_preseason_plan(career_dir)? else {
        return Ok(());
    };
    plan.state.player_has_pending_proposals = has_pending;
    save_preseason_plan(career_dir, &plan)
}

fn refresh_planned_hierarchy_for_team(
    plan: &mut PreSeasonPlan,
    conn: &rusqlite::Connection,
    team_id: &str,
) -> Result<(), String> {
    let hierarchy_week = plan
        .planned_events
        .iter()
        .filter_map(|event| match &event.event {
            PendingAction::UpdateHierarchy {
                team_id: current, ..
            } if current == team_id => Some(event.week),
            PendingAction::UpdateHierarchy { .. } => Some(event.week),
            _ => None,
        })
        .max()
        .unwrap_or(plan.state.total_weeks);
    plan.planned_events.retain(|event| {
        !(!event.executed
            && matches!(&event.event, PendingAction::UpdateHierarchy { team_id: current, .. } if current == team_id))
    });

    let team = team_queries::get_team_by_id(conn, team_id)
        .map_err(|e| format!("Falha ao carregar equipe para atualizar plano: {e}"))?
        .ok_or_else(|| "Equipe nao encontrada para atualizar plano.".to_string())?;
    let mut candidates = Vec::new();
    for driver_id in [team.piloto_1_id.clone(), team.piloto_2_id.clone()]
        .into_iter()
        .flatten()
    {
        let driver = driver_queries::get_driver(conn, &driver_id)
            .map_err(|e| format!("Falha ao carregar piloto da equipe para plano: {e}"))?;
        candidates.push((driver.id, driver.nome, driver.atributos.skill));
    }
    for event in plan.planned_events.iter() {
        if event.executed {
            continue;
        }
        if let PendingAction::PlaceRookie {
            driver,
            team_id: current,
            ..
        } = &event.event
        {
            if current == team_id {
                candidates.push((
                    driver.id.clone(),
                    driver.nome.clone(),
                    driver.atributos.skill,
                ));
            }
        }
    }
    candidates.sort_by(|a, b| b.2.total_cmp(&a.2));
    candidates.dedup_by(|a, b| a.0 == b.0);
    let n1 = candidates.first().cloned();
    let n2 = candidates.get(1).cloned();
    plan.planned_events.push(PlannedEvent {
        week: hierarchy_week,
        executed: false,
        event: PendingAction::UpdateHierarchy {
            team_id: team.id.clone(),
            team_name: team.nome.clone(),
            n1_id: n1.as_ref().map(|candidate| candidate.0.clone()),
            n1_name: n1
                .as_ref()
                .map(|candidate| candidate.1.clone())
                .unwrap_or_else(|| "Sem piloto".to_string()),
            n2_id: n2.as_ref().map(|candidate| candidate.0.clone()),
            n2_name: n2
                .as_ref()
                .map(|candidate| candidate.1.clone())
                .unwrap_or_else(|| "Sem piloto".to_string()),
            prev_n1_id: team.hierarquia_n1_id.clone(),
            prev_n2_id: team.hierarquia_n2_id.clone(),
            prev_tensao: team.hierarquia_tensao,
            prev_status: team.hierarquia_status.clone(),
            prev_categoria: team.categoria.clone(),
        },
    });
    Ok(())
}

fn open_career_resources(
    base_dir: &Path,
    career_id: &str,
) -> Result<(Database, std::path::PathBuf, SaveMeta), String> {
    let _career_number =
        career_number_from_id(career_id).ok_or_else(|| "ID de carreira invalido.".to_string())?;

    let config = AppConfig::load_or_default(base_dir);
    let career_dir = config.saves_dir().join(career_id);
    let db_path = career_dir.join("career.db");
    let meta_path = career_dir.join("meta.json");

    if !career_dir.exists() {
        return Err("Save nao encontrado.".to_string());
    }
    if !db_path.exists() {
        return Err("Banco da carreira nao encontrado.".to_string());
    }

    let db = Database::open_existing(&db_path)
        .map_err(|e| format!("Falha ao abrir banco da carreira: {e}"))?;
    repair_regular_contract_consistency(&db.conn)?;
    let meta = read_save_meta(&meta_path)?;

    Ok((db, career_dir, meta))
}

fn repair_regular_contract_consistency(conn: &rusqlite::Connection) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Falha ao iniciar reparo de contratos: {e}"))?;
    let mut affected_team_ids = HashSet::new();
    let active_regular_contracts = contract_queries::get_all_active_regular_contracts(&tx)
        .map_err(|e| format!("Falha ao carregar contratos regulares ativos: {e}"))?;
    let mut contracts_by_pilot = HashMap::<String, Vec<_>>::new();

    for contract in active_regular_contracts {
        contracts_by_pilot
            .entry(contract.piloto_id.clone())
            .or_default()
            .push(contract);
    }

    for contracts in contracts_by_pilot.values_mut() {
        if contracts.len() <= 1 {
            continue;
        }

        contracts.sort_by(|a, b| {
            b.temporada_inicio
                .cmp(&a.temporada_inicio)
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| b.id.cmp(&a.id))
        });

        for duplicate in contracts.iter().skip(1) {
            contract_queries::update_contract_status(
                &tx,
                &duplicate.id,
                &ContractStatus::Rescindido,
            )
            .map_err(|e| {
                format!(
                    "Falha ao rescindir contrato regular duplicado '{}': {e}",
                    duplicate.id
                )
            })?;
            affected_team_ids.insert(duplicate.equipe_id.clone());
        }

        if let Some(kept) = contracts.first() {
            affected_team_ids.insert(kept.equipe_id.clone());
        }
    }

    let teams =
        team_queries::get_all_teams(&tx).map_err(|e| format!("Falha ao carregar equipes: {e}"))?;
    let teams_by_id = teams
        .iter()
        .map(|team| (team.id.clone(), team.clone()))
        .collect::<HashMap<_, _>>();
    let drivers = driver_queries::get_all_drivers(&tx)
        .map_err(|e| format!("Falha ao carregar pilotos para reparo: {e}"))?;
    let drivers_by_id = drivers
        .into_iter()
        .map(|driver| (driver.id.clone(), driver))
        .collect::<HashMap<_, _>>();
    let active_regular_contracts = contract_queries::get_all_active_regular_contracts(&tx)
        .map_err(|e| format!("Falha ao recarregar contratos regulares ativos: {e}"))?;
    for contract in active_regular_contracts {
        let Some(team) = teams_by_id.get(&contract.equipe_id) else {
            continue;
        };
        if categories::is_especial(&team.categoria) {
            continue;
        }

        let Some(driver) = drivers_by_id.get(&contract.piloto_id) else {
            continue;
        };
        if driver.status == DriverStatus::Aposentado {
            contract_queries::update_contract_status(
                &tx,
                &contract.id,
                &ContractStatus::Rescindido,
            )
            .map_err(|e| {
                format!(
                    "Falha ao rescindir contrato regular invalido '{}': {e}",
                    contract.id
                )
            })?;
            affected_team_ids.insert(contract.equipe_id.clone());
            continue;
        }

        if driver.categoria_atual.as_deref() != Some(team.categoria.as_str()) {
            let mut updated_driver = driver.clone();
            updated_driver.categoria_atual = Some(team.categoria.clone());
            driver_queries::update_driver(&tx, &updated_driver).map_err(|e| {
                format!("Falha ao corrigir categoria do piloto '{}': {e}", driver.id)
            })?;
        }
    }

    for team in teams
        .iter()
        .filter(|team| !categories::is_especial(&team.categoria))
    {
        if normalize_regular_contracts_for_team(&tx, &team.id)? {
            affected_team_ids.insert(team.id.clone());
        }
    }

    for team_id in affected_team_ids {
        refresh_team_hierarchy_now(&tx, &team_id)?;
    }

    tx.commit()
        .map_err(|e| format!("Falha ao concluir reparo de contratos: {e}"))?;
    Ok(())
}

pub(crate) fn get_drivers_by_category_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    category: &str,
) -> Result<Vec<DriverSummary>, String> {
    let category = category.trim().to_lowercase();
    let (db, career_dir, _) = open_career_resources(base_dir, career_id)?;
    let drivers = driver_queries::get_drivers_by_category(&db.conn, &category)
        .map_err(|e| format!("Falha ao buscar pilotos da categoria: {e}"))?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let total_rounds = count_calendar_entries(&db.conn, &season.id, &category)
        .map_err(|e| format!("Falha ao contar corridas da categoria: {e}"))?
        as usize;
    let driver_ids: Vec<String> = drivers.iter().map(|driver| driver.id.clone()).collect();
    let history_map: HashMap<String, Vec<Option<RoundResult>>> =
        build_driver_histories(&career_dir, &category, total_rounds, &driver_ids)?
            .into_iter()
            .map(|history| (history.driver_id, history.results))
            .collect();

    let mut standings: Vec<DriverSummary> = drivers
        .into_iter()
        .map(|driver| {
            let driver_id = driver.id.clone();
            let team = find_player_team(&db.conn, &driver.id, season.fase)
                .ok()
                .flatten();
            DriverSummary {
                id: driver_id.clone(),
                nome: driver.nome,
                nacionalidade: driver.nacionalidade,
                idade: driver.idade as i32,
                skill: driver.atributos.skill.round().clamp(0.0, 100.0) as u8,
                categoria_especial_ativa: driver.categoria_especial_ativa.clone(),
                equipe_id: team.as_ref().map(|value| value.id.clone()),
                equipe_nome: team.as_ref().map(|value| value.nome.clone()),
                equipe_nome_curto: team.as_ref().map(|value| value.nome_curto.clone()),
                equipe_cor: team
                    .as_ref()
                    .map(|value| value.cor_primaria.clone())
                    .unwrap_or_else(|| "#7d8590".to_string()),
                is_jogador: driver.is_jogador,
                pontos: driver.stats_temporada.pontos.round() as i32,
                vitorias: driver.stats_temporada.vitorias as i32,
                podios: driver.stats_temporada.podios as i32,
                posicao_campeonato: 0,
                results: merge_recent_results_fallback(
                    history_map.get(&driver_id).cloned().unwrap_or_default(),
                    &driver.ultimos_resultados,
                    total_rounds,
                    driver.stats_temporada.corridas as usize,
                ),
            }
        })
        .collect();

    standings.sort_by(|a, b| {
        b.pontos
            .cmp(&a.pontos)
            .then_with(|| b.vitorias.cmp(&a.vitorias))
            .then_with(|| b.podios.cmp(&a.podios))
            .then_with(|| a.nome.cmp(&b.nome))
    });

    for (index, driver) in standings.iter_mut().enumerate() {
        driver.posicao_campeonato = index as i32 + 1;
    }

    Ok(standings)
}

fn merge_recent_results_fallback(
    history: Vec<Option<RoundResult>>,
    recent_results: &serde_json::Value,
    total_rounds: usize,
    raced_rounds: usize,
) -> Vec<Option<RoundResult>> {
    if history.iter().any(Option::is_some) {
        return history;
    }

    let fallback_results = parse_recent_results_json(recent_results);
    if fallback_results.is_empty() {
        return history;
    }

    let normalized_len = total_rounds.max(fallback_results.len());
    let mut merged = vec![None; normalized_len];
    let end_index = raced_rounds.min(normalized_len).max(fallback_results.len());
    let start_index = end_index.saturating_sub(fallback_results.len());

    for (offset, result) in fallback_results.into_iter().enumerate() {
        merged[start_index + offset] = Some(result);
    }

    merged
}

fn parse_recent_results_json(value: &serde_json::Value) -> Vec<RoundResult> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_recent_result_entry)
        .collect()
}

fn parse_recent_result_entry(value: &serde_json::Value) -> Option<RoundResult> {
    let object = value.as_object()?;
    let position = object
        .get("position")
        .and_then(|entry| entry.as_i64())
        .unwrap_or_default() as i32;
    let is_dnf = object
        .get("is_dnf")
        .and_then(|entry| entry.as_bool())
        .unwrap_or(false);

    if position <= 0 && !is_dnf {
        return None;
    }

    Some(RoundResult {
        position,
        is_dnf,
        has_fastest_lap: object
            .get("has_fastest_lap")
            .and_then(|entry| entry.as_bool())
            .unwrap_or(false),
        grid_position: object
            .get("grid_position")
            .and_then(|entry| entry.as_i64())
            .unwrap_or_default() as i32,
        positions_gained: object
            .get("positions_gained")
            .and_then(|entry| entry.as_i64())
            .unwrap_or_default() as i32,
    })
}

fn get_driver_slot_info(
    db: &Database,
    driver_id: Option<&String>,
    team_id: &str,
    active_season_number: i32,
) -> (Option<String>, Option<i32>) {
    let Some(driver_id) = driver_id else {
        return (None, None);
    };

    let driver_name = driver_queries::get_driver(&db.conn, driver_id)
        .ok()
        .map(|driver| driver.nome);
    let tenure_seasons =
        calculate_consecutive_team_tenure(&db.conn, driver_id, team_id, active_season_number);
    (driver_name, tenure_seasons)
}

fn calculate_consecutive_team_tenure(
    conn: &rusqlite::Connection,
    driver_id: &str,
    team_id: &str,
    active_season_number: i32,
) -> Option<i32> {
    let contracts = contract_queries::get_contracts_for_pilot(conn, driver_id).ok()?;
    consecutive_team_seasons_up_to(&contracts, team_id, active_season_number)
}

fn consecutive_team_seasons_up_to(
    contracts: &[crate::models::contract::Contract],
    team_id: &str,
    active_season_number: i32,
) -> Option<i32> {
    let mut intervals: Vec<(i32, i32)> = contracts
        .iter()
        .filter(|contract| {
            contract.tipo == crate::models::enums::ContractType::Regular
                && contract.equipe_id == team_id
                && contract.status != crate::models::enums::ContractStatus::Pendente
        })
        .map(|contract| {
            (
                contract.temporada_inicio,
                contract.temporada_fim.min(active_season_number),
            )
        })
        .filter(|(start, end)| *start <= *end)
        .collect();

    intervals.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

    let mut covered_until = active_season_number;
    let mut earliest_start = None;

    for (start, end) in intervals {
        if end < covered_until {
            if end + 1 != covered_until {
                continue;
            }
        } else if start > covered_until || end < covered_until {
            continue;
        }

        earliest_start = Some(start);
        covered_until = start - 1;
    }

    earliest_start.map(|start| active_season_number - start + 1)
}

pub(crate) fn get_teams_standings_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    category: &str,
) -> Result<Vec<TeamStanding>, String> {
    let category = category.trim().to_lowercase();
    let (db, _, _) = open_career_resources(base_dir, career_id)?;
    let teams = team_queries::get_teams_by_category(&db.conn, &category)
        .map_err(|e| format!("Falha ao buscar equipes da categoria: {e}"))?;
    let previous_champions = get_previous_champions_in_base_dir(base_dir, career_id, &category)?;
    let active_season_number = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .map(|season| season.numero)
        .unwrap_or(1);

    let mut standings: Vec<TeamStanding> = teams
        .into_iter()
        .map(|team| {
            let team_id = team.id.clone();
            let (piloto_1_nome, piloto_1_tenure_seasons) = get_driver_slot_info(
                &db,
                team.piloto_1_id.as_ref(),
                &team_id,
                active_season_number,
            );
            let (piloto_2_nome, piloto_2_tenure_seasons) = get_driver_slot_info(
                &db,
                team.piloto_2_id.as_ref(),
                &team_id,
                active_season_number,
            );

            TeamStanding {
                posicao: 0,
                id: team_id.clone(),
                nome: team.nome,
                nome_curto: team.nome_curto,
                cor_primaria: team.cor_primaria,
                pontos: team.stats_pontos,
                vitorias: team.stats_vitorias,
                piloto_1_nome,
                piloto_1_tenure_seasons,
                piloto_2_nome,
                piloto_2_tenure_seasons,
                trofeus: previous_champions
                    .constructor_champions
                    .iter()
                    .find(|champion| champion.team_id == team_id)
                    .map(|champion| {
                        vec![TrophyInfo {
                            tipo: "ouro".to_string(),
                            temporada: 0,
                            is_defending: champion.is_defending,
                        }]
                    })
                    .unwrap_or_default(),
                classe: team.classe.clone(),
                temp_posicao: team.temp_posicao,
                categoria_anterior: team.categoria_anterior.clone(),
            }
        })
        .collect();

    standings.sort_by(|a, b| {
        b.pontos
            .cmp(&a.pontos)
            .then_with(|| b.vitorias.cmp(&a.vitorias))
            .then_with(|| a.nome.cmp(&b.nome))
    });

    for (index, team) in standings.iter_mut().enumerate() {
        team.posicao = index as i32 + 1;
    }

    Ok(standings)
}

pub(crate) fn get_race_results_by_category_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    category: &str,
) -> Result<Vec<DriverRaceHistory>, String> {
    let category = category.trim().to_lowercase();
    let (db, career_dir, _) = open_career_resources(base_dir, career_id)?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let drivers = driver_queries::get_drivers_by_category(&db.conn, &category)
        .map_err(|e| format!("Falha ao buscar pilotos da categoria: {e}"))?;
    let total_rounds = count_calendar_entries(&db.conn, &season.id, &category)
        .map_err(|e| format!("Falha ao contar corridas da categoria: {e}"))?
        as usize;
    let driver_ids: Vec<String> = drivers.into_iter().map(|driver| driver.id).collect();

    build_driver_histories(&career_dir, &category, total_rounds, &driver_ids)
}

pub(crate) fn get_previous_champions_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    _category: &str,
) -> Result<PreviousChampions, String> {
    let (db, _, _) = open_career_resources(base_dir, career_id)?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

    if season.numero <= 1 {
        return Ok(empty_previous_champions());
    }

    Ok(PreviousChampions {
        driver_champion_id: None,
        constructor_champions: Vec::<ConstructorChampion>::new(),
    })
}

pub(crate) fn get_calendar_for_category_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    category: &str,
) -> Result<Vec<RaceSummary>, String> {
    let category = category.trim().to_lowercase();
    let (db, _, _) = open_career_resources(base_dir, career_id)?;
    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let entries = calendar_queries::get_calendar(&db.conn, &season.id, &category)
        .map_err(|e| format!("Falha ao buscar calendario da categoria: {e}"))?;

    Ok(entries
        .into_iter()
        .map(|race| RaceSummary {
            id: race.id,
            rodada: race.rodada,
            track_name: race.track_name,
            clima: race.clima.as_str().to_string(),
            duracao_corrida_min: race.duracao_corrida_min,
            status: race.status.as_str().to_string(),
            temperatura: race.temperatura,
            horario: race.horario.clone(),
            week_of_year: race.week_of_year,
            season_phase: race.season_phase.as_str().to_string(),
            display_date: race.display_date.clone(),
            event_interest: None,
        })
        .collect())
}

fn write_save_meta(path: &Path, meta: &SaveMeta) -> Result<(), String> {
    let json = serde_json::to_string_pretty(meta)
        .map_err(|e| format!("Falha ao serializar meta.json: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Falha ao gravar meta.json: {e}"))
}

fn save_meta_to_info(meta: SaveMeta) -> SaveInfo {
    SaveInfo {
        career_id: format!("career_{:03}", meta.career_number),
        player_name: meta.player_name,
        category_name: categories::get_category_config(&meta.category)
            .map(|category| category.nome.to_string())
            .unwrap_or_else(|| meta.category.clone()),
        category: meta.category,
        season: meta.current_season as i32,
        year: meta.current_year as i32,
        difficulty: meta.difficulty,
        created: meta.created_at,
        last_played: meta.last_played,
        total_races: meta.total_races,
    }
}

fn preferred_active_contract_for_phase(
    conn: &rusqlite::Connection,
    driver_id: &str,
    season_phase: SeasonPhase,
) -> Result<Option<crate::models::contract::Contract>, String> {
    if season_phase == SeasonPhase::BlocoEspecial {
        let special_contract =
            contract_queries::get_active_especial_contract_for_pilot(conn, driver_id)
                .map_err(|e| format!("Falha ao buscar contrato especial ativo: {e}"))?;
        if special_contract.is_some() {
            return Ok(special_contract);
        }
    }

    contract_queries::get_active_regular_contract_for_pilot(conn, driver_id)
        .map_err(|e| format!("Falha ao buscar contrato regular ativo: {e}"))
}

fn find_player_team(
    conn: &rusqlite::Connection,
    player_id: &str,
    season_phase: SeasonPhase,
) -> Result<Option<Team>, String> {
    let contract = preferred_active_contract_for_phase(conn, player_id, season_phase)?;
    resolve_driver_team(conn, player_id, contract.as_ref())
}

fn resolve_driver_team(
    conn: &rusqlite::Connection,
    driver_id: &str,
    contract: Option<&crate::models::contract::Contract>,
) -> Result<Option<Team>, String> {
    if let Some(contract) = contract {
        if let Some(team) = team_queries::get_team_by_id(conn, &contract.equipe_id)
            .map_err(|e| format!("Falha ao buscar equipe do contrato: {e}"))?
        {
            return Ok(Some(team));
        }
    }

    let mut stmt = conn
        .prepare("SELECT id FROM teams WHERE piloto_1_id = ?1 OR piloto_2_id = ?1 LIMIT 1")
        .map_err(|e| format!("Falha ao procurar equipe do piloto: {e}"))?;
    let team_id: Option<String> = stmt
        .query_row(rusqlite::params![driver_id], |row| row.get(0))
        .optional()
        .map_err(|e| format!("Falha ao procurar equipe do piloto: {e}"))?;

    match team_id {
        Some(id) => team_queries::get_team_by_id(conn, &id)
            .map_err(|e| format!("Falha ao carregar equipe do piloto: {e}")),
        None => Ok(None),
    }
}

fn resolve_driver_role(
    driver_id: &str,
    contract: Option<&crate::models::contract::Contract>,
    team: Option<&Team>,
) -> Option<String> {
    if let Some(contract) = contract {
        return Some(contract.papel.as_str().to_string());
    }

    team.and_then(|value| {
        if value.piloto_1_id.as_deref() == Some(driver_id) {
            Some("Numero1".to_string())
        } else if value.piloto_2_id.as_deref() == Some(driver_id) {
            Some("Numero2".to_string())
        } else {
            None
        }
    })
}

fn build_team_summary(conn: &rusqlite::Connection, team: &Team) -> Result<TeamSummary, String> {
    let piloto_1_nome = match &team.piloto_1_id {
        Some(id) => Some(
            driver_queries::get_driver(conn, id)
                .map_err(|e| format!("Falha ao carregar piloto 1 da equipe: {e}"))?
                .nome,
        ),
        None => None,
    };

    let piloto_2_nome = match &team.piloto_2_id {
        Some(id) => Some(
            driver_queries::get_driver(conn, id)
                .map_err(|e| format!("Falha ao carregar piloto 2 da equipe: {e}"))?
                .nome,
        ),
        None => None,
    };

    Ok(TeamSummary {
        id: team.id.clone(),
        nome: team.nome.clone(),
        nome_curto: team.nome_curto.clone(),
        cor_primaria: team.cor_primaria.clone(),
        cor_secundaria: team.cor_secundaria.clone(),
        categoria: team.categoria.clone(),
        classe: team.classe.clone(),
        car_performance: team.car_performance,
        car_build_profile: team.car_build_profile.as_str().to_string(),
        confiabilidade: team.confiabilidade,
        pit_strategy_risk: team.pit_strategy_risk,
        pit_crew_quality: team.pit_crew_quality,
        budget: team.budget,
        piloto_1_id: team.piloto_1_id.clone(),
        piloto_1_nome,
        piloto_2_id: team.piloto_2_id.clone(),
        piloto_2_nome,
    })
}

fn build_accepted_special_offer_summary(
    conn: &rusqlite::Connection,
    player: &crate::models::driver::Driver,
) -> Result<Option<AcceptedSpecialOfferSummary>, String> {
    if player.categoria_especial_ativa.is_none() {
        return Ok(None);
    }

    let Some(contract) = contract_queries::get_active_especial_contract_for_pilot(conn, &player.id)
        .map_err(|e| format!("Falha ao buscar contrato especial ativo: {e}"))?
    else {
        return Ok(None);
    };

    Ok(Some(AcceptedSpecialOfferSummary {
        id: contract.id,
        team_id: contract.equipe_id,
        team_name: contract.equipe_nome,
        special_category: contract.categoria,
        class_name: contract.classe.unwrap_or_default(),
        papel: contract.papel.as_str().to_string(),
    }))
}

fn empty_track_history_summary() -> TrackHistorySummary {
    TrackHistorySummary {
        has_data: false,
        starts: 0,
        best_finish: None,
        last_finish: None,
        dnfs: 0,
        last_visit_season: None,
        last_visit_round: None,
    }
}

fn empty_next_race_briefing_summary() -> NextRaceBriefingSummary {
    NextRaceBriefingSummary {
        track_history: Some(empty_track_history_summary()),
        primary_rival: None,
        weekend_stories: Vec::new(),
        contract_warning: None,
    }
}

fn build_next_race_briefing_summary(
    conn: &rusqlite::Connection,
    player_id: &str,
    season_number: i32,
    race: &CalendarEntry,
) -> Result<NextRaceBriefingSummary, String> {
    let contract_warning = contract_queries::get_active_regular_contract_for_pilot(conn, player_id)
        .map_err(|e| format!("Falha ao buscar contrato regular do jogador: {e}"))?
        .and_then(|c| {
            if c.is_ultimo_ano(season_number) {
                Some(ContractWarningInfo {
                    temporada_fim: c.temporada_fim,
                    equipe_nome: c.equipe_nome,
                })
            } else {
                None
            }
        });

    Ok(NextRaceBriefingSummary {
        track_history: Some(build_track_history_summary(
            conn,
            player_id,
            &race.track_name,
        )?),
        primary_rival: build_primary_rival_summary(conn, player_id, &race.categoria)?,
        weekend_stories: build_weekend_story_summaries(
            conn,
            season_number,
            &race.categoria,
            race.rodada,
        )?,
        contract_warning,
    })
}

fn build_track_history_summary(
    conn: &rusqlite::Connection,
    player_id: &str,
    track_name: &str,
) -> Result<TrackHistorySummary, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.numero, c.rodada, r.posicao_final, r.dnf
             FROM race_results r
             JOIN calendar c ON r.race_id = c.id
             JOIN seasons s ON COALESCE(c.season_id, c.temporada_id) = s.id
             WHERE r.piloto_id = ?1
               AND c.track_name = ?2
             ORDER BY s.numero DESC, c.rodada DESC",
        )
        .map_err(|e| format!("Falha ao preparar historico de pista: {e}"))?;

    let rows = stmt
        .query_map(rusqlite::params![player_id, track_name], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)? != 0,
            ))
        })
        .map_err(|e| format!("Falha ao buscar historico de pista: {e}"))?;

    let mut visits = Vec::new();
    for row in rows {
        visits.push(row.map_err(|e| format!("Falha ao ler historico de pista: {e}"))?);
    }

    if visits.is_empty() {
        return Ok(empty_track_history_summary());
    }

    let last_visit = visits[0];
    let best_finish = visits
        .iter()
        .filter(|(_, _, position, is_dnf)| !*is_dnf && *position > 0)
        .map(|(_, _, position, _)| *position)
        .min();
    let dnfs = visits.iter().filter(|(_, _, _, is_dnf)| *is_dnf).count() as i32;

    Ok(TrackHistorySummary {
        has_data: true,
        starts: visits.len() as i32,
        best_finish,
        last_finish: Some(last_visit.2),
        dnfs,
        last_visit_season: Some(last_visit.0),
        last_visit_round: Some(last_visit.1),
    })
}

fn build_primary_rival_summary(
    conn: &rusqlite::Connection,
    player_id: &str,
    categoria: &str,
) -> Result<Option<PrimaryRivalSummary>, String> {
    let mut drivers = driver_queries::get_drivers_by_category(conn, categoria)
        .map_err(|e| format!("Falha ao buscar pilotos da categoria para rival principal: {e}"))?;

    drivers.sort_by(|a, b| {
        b.stats_temporada
            .pontos
            .partial_cmp(&a.stats_temporada.pontos)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.stats_temporada.vitorias.cmp(&a.stats_temporada.vitorias))
            .then_with(|| b.stats_temporada.podios.cmp(&a.stats_temporada.podios))
            .then_with(|| a.nome.cmp(&b.nome))
    });

    let Some(player_index) = drivers.iter().position(|driver| driver.id == player_id) else {
        return Ok(None);
    };

    let player = &drivers[player_index];
    let rival_index = if player_index == 0 {
        if drivers.len() > 1 {
            1
        } else {
            return Ok(None);
        }
    } else {
        player_index - 1
    };
    let rival = &drivers[rival_index];
    let is_ahead = rival_index < player_index;
    let gap_points = if is_ahead {
        (rival.stats_temporada.pontos - player.stats_temporada.pontos)
            .max(0.0)
            .round() as i32
    } else {
        (player.stats_temporada.pontos - rival.stats_temporada.pontos)
            .max(0.0)
            .round() as i32
    };

    Ok(Some(PrimaryRivalSummary {
        driver_id: rival.id.clone(),
        driver_name: rival.nome.clone(),
        championship_position: rival_index as i32 + 1,
        gap_points,
        is_ahead,
        rivalry_label: None,
    }))
}

fn build_weekend_story_summaries(
    conn: &rusqlite::Connection,
    season_number: i32,
    categoria: &str,
    round_number: i32,
) -> Result<Vec<BriefingStorySummary>, String> {
    let mut stories = news_queries::get_news_by_season(conn, season_number, 200)
        .map_err(|e| format!("Falha ao buscar noticias da temporada para a previa: {e}"))?
        .into_iter()
        .filter(|item| {
            item.categoria_id.as_deref() == Some(categoria) && item.rodada == Some(round_number)
        })
        .collect::<Vec<_>>();

    stories.sort_by(|left, right| {
        briefing_importance_rank(&right.importancia)
            .cmp(&briefing_importance_rank(&left.importancia))
            .then_with(|| briefing_type_rank(&right.tipo).cmp(&briefing_type_rank(&left.tipo)))
            .then_with(|| right.timestamp.cmp(&left.timestamp))
    });

    Ok(stories
        .into_iter()
        .take(3)
        .map(|item| BriefingStorySummary {
            id: item.id,
            icon: item.icone,
            title: item.titulo,
            summary: build_briefing_story_summary_text(&item.texto),
            importance: item.importancia.as_str().to_string(),
        })
        .collect())
}

fn briefing_importance_rank(value: &NewsImportance) -> i32 {
    match value {
        NewsImportance::Destaque => 4,
        NewsImportance::Alta => 3,
        NewsImportance::Media => 2,
        NewsImportance::Baixa => 1,
    }
}

fn briefing_type_rank(value: &NewsType) -> i32 {
    match value {
        NewsType::Rivalidade => 5,
        NewsType::Hierarquia => 4,
        NewsType::Corrida => 3,
        NewsType::Incidente => 2,
        NewsType::FramingSazonal => 1,
        _ => 0,
    }
}

fn build_briefing_story_summary_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "O paddock segue produzindo contexto para a proxima largada.".to_string();
    }

    if let Some((first_sentence, _)) = trimmed.split_once('.') {
        let sentence = first_sentence.trim();
        if !sentence.is_empty() {
            return format!("{sentence}.");
        }
    }

    trimmed.chars().take(140).collect()
}

fn warn_if_noncritical<T>(result: Result<T, String>, context: &str) {
    if let Err(error) = result {
        eprintln!("Aviso: {context}: {error}");
    }
}

fn count_rows(conn: &rusqlite::Connection, table: &str) -> Result<usize, rusqlite::Error> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(count as usize)
}

pub(crate) fn count_calendar_entries(
    conn: &rusqlite::Connection,
    season_id: &str,
    categoria: &str,
) -> Result<i32, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1
           AND categoria = ?2",
        rusqlite::params![season_id, categoria],
        |row| row.get(0),
    )
}

fn count_season_calendar_entries(
    conn: &rusqlite::Connection,
    season_id: &str,
) -> Result<i32, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM calendar
         WHERE COALESCE(season_id, temporada_id) = ?1",
        rusqlite::params![season_id],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};
    use std::fs;

    use super::*;

    #[test]
    fn test_validate_input_valid() {
        let input = CreateCareerInput {
            player_name: "Joao Silva".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };
        assert!(validate_create_career_input(&input).is_ok());
    }

    #[test]
    fn test_validate_input_empty_name() {
        let input = CreateCareerInput {
            player_name: "   ".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };
        assert!(validate_create_career_input(&input).is_err());
    }

    #[test]
    fn test_validate_input_invalid_category() {
        let input = CreateCareerInput {
            player_name: "Joao".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "gt4".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };
        assert!(validate_create_career_input(&input).is_err());
    }

    #[test]
    fn test_validate_input_invalid_team_index() {
        let input = CreateCareerInput {
            player_name: "Joao".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "toyota_rookie".to_string(),
            team_index: 9,
            difficulty: "medio".to_string(),
        };
        assert!(validate_create_career_input(&input).is_err());
    }

    #[test]
    fn test_validate_input_invalid_difficulty() {
        let input = CreateCareerInput {
            player_name: "Joao".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "toyota_rookie".to_string(),
            team_index: 2,
            difficulty: "insano".to_string(),
        };
        assert!(validate_create_career_input(&input).is_err());
    }

    #[test]
    fn test_next_career_id_empty_dir() {
        let base = unique_test_dir("empty");
        let saves_dir = base.join("saves");
        let next = next_career_id(&saves_dir);
        assert_eq!(next, "career_001");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn test_next_career_id_with_existing() {
        let base = unique_test_dir("existing");
        let saves_dir = base.join("saves");
        fs::create_dir_all(saves_dir.join("career_001")).expect("career 001");
        fs::create_dir_all(saves_dir.join("career_003")).expect("career 003");
        let next = next_career_id(&saves_dir);
        assert_eq!(next, "career_004");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn test_create_career_full_flow() {
        let base_dir = unique_test_dir("full_flow");
        fs::create_dir_all(&base_dir).expect("base dir");

        let input = CreateCareerInput {
            player_name: "Joao Silva".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };

        let result = create_career_in_base_dir(&base_dir, input).expect("career should be created");
        assert!(result.success);
        assert_eq!(result.total_drivers, 196);
        assert_eq!(result.total_teams, 98);
        // Categorias especiais (production_challenger=10, endurance=6) não geram calendário
        // no BlocoRegular — calendário delas é criado na JanelaConvocação (Passos 6+).
        assert_eq!(result.total_races, 58);

        let db_path = std::path::PathBuf::from(&result.save_path).join("career.db");
        assert!(db_path.exists());
        let meta_path = std::path::PathBuf::from(&result.save_path).join("meta.json");
        assert!(meta_path.exists());

        let db = Database::open_existing(&db_path).expect("db should open");
        let drivers_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM drivers", [], |row| row.get(0))
            .expect("drivers count");
        let teams_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM teams", [], |row| row.get(0))
            .expect("teams count");
        let contracts_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM contracts", [], |row| row.get(0))
            .expect("contracts count");
        let seasons_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM seasons", [], |row| row.get(0))
            .expect("seasons count");
        let calendar_count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM calendar", [], |row| row.get(0))
            .expect("calendar count");

        assert_eq!(drivers_count, 196);
        assert_eq!(teams_count, 98);
        // 132 contratos: categorias especiais (production_challenger, endurance) não geram contratos
        assert_eq!(contracts_count, 132);
        assert_eq!(seasons_count, 1);
        // 58 corridas: sem as 16 das categorias especiais (10+6), geradas na JanelaConvocação
        assert_eq!(calendar_count, 58);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_create_career_seeds_initial_licenses_for_active_grid() {
        let base_dir = unique_test_dir("seed_initial_licenses");
        fs::create_dir_all(&base_dir).expect("base dir");

        let input = CreateCareerInput {
            player_name: "Joao Silva".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };

        let result = create_career_in_base_dir(&base_dir, input).expect("career should be created");
        let db_path = std::path::PathBuf::from(&result.save_path).join("career.db");
        let db = Database::open_existing(&db_path).expect("db should open");

        let seeded_licenses: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM licenses", [], |row| row.get(0))
            .expect("licenses count");
        let gt3_without_license: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*)
                 FROM contracts c
                 JOIN teams t ON t.id = c.equipe_id
                 LEFT JOIN licenses l
                   ON l.piloto_id = c.piloto_id
                  AND CAST(l.nivel AS INTEGER) >= 3
                 WHERE c.status = 'Ativo'
                   AND c.tipo = 'Regular'
                   AND t.categoria = 'gt3'
                   AND l.piloto_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("gt3 license coverage");
        let gt4_without_license: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*)
                 FROM contracts c
                 JOIN teams t ON t.id = c.equipe_id
                 LEFT JOIN licenses l
                   ON l.piloto_id = c.piloto_id
                  AND CAST(l.nivel AS INTEGER) >= 2
                 WHERE c.status = 'Ativo'
                   AND c.tipo = 'Regular'
                   AND t.categoria = 'gt4'
                   AND l.piloto_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("gt4 license coverage");

        assert_eq!(seeded_licenses, 108);
        assert_eq!(gt3_without_license, 0);
        assert_eq!(gt4_without_license, 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_returns_player() {
        let base_dir = create_test_career_dir("load_player");
        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");

        assert!(career.player.is_jogador);
        assert_eq!(career.player.nome, "Joao Silva");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_returns_team() {
        let base_dir = create_test_career_dir("load_team");
        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let player_team = career.player_team.as_ref().expect("player team");

        assert!(!player_team.id.is_empty());
        assert!(player_team.piloto_1_id.is_some());
        assert!(player_team.piloto_2_id.is_some());
        assert!((0.0..=100.0).contains(&player_team.pit_strategy_risk));
        assert!((0.0..=100.0).contains(&player_team.pit_crew_quality));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_returns_season() {
        let base_dir = create_test_career_dir("load_season");
        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");

        assert_eq!(career.season.numero, 1);
        assert_eq!(career.season.ano, 2024);
        assert!(career.season.total_rodadas > 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_includes_next_race_briefing() {
        let base_dir = create_test_career_dir("load_briefing_contract");
        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let career_json = serde_json::to_value(&career).expect("career json");

        assert!(
            career_json.get("next_race_briefing").is_some(),
            "expected load_career payload to expose next_race_briefing",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_restores_resume_context_snapshot() {
        let base_dir = create_test_career_dir("load_resume_context");
        mark_all_races_completed(&base_dir, "career_001");

        let result = advance_season_in_base_dir(&base_dir, "career_001")
            .expect("advance season should work");
        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let resume_context = career.resume_context.expect("resume context");

        assert_eq!(resume_context.active_view, CareerResumeView::EndOfSeason);
        assert_eq!(
            resume_context
                .end_of_season_result
                .as_ref()
                .map(|snapshot| snapshot.new_year),
            Some(result.new_year)
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_prefers_active_special_contract_team() {
        let base_dir = create_test_career_dir("load_active_special_team");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let mut player = driver_queries::get_player_driver(&db.conn).expect("player");
        player.categoria_atual = Some("gt4".to_string());
        player.atributos.skill = 98.0;
        driver_queries::update_driver(&db.conn, &player).expect("update player");

        crate::convocation::advance_to_convocation_window(&db.conn).expect("advance convocation");
        crate::convocation::run_convocation_window(&db.conn).expect("run convocation");
        let offers = crate::commands::convocation::get_player_special_offers_in_base_dir(
            &base_dir,
            "career_001",
        )
        .expect("special offers");
        crate::commands::convocation::respond_player_special_offer_in_base_dir(
            &base_dir,
            "career_001",
            &offers[0].id,
            true,
        )
        .expect("accept offer");
        crate::convocation::iniciar_bloco_especial(&db.conn).expect("start special block");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let player_team = career.player_team.as_ref().expect("player team");

        assert_eq!(player_team.categoria, "endurance");
        assert_eq!(
            career
                .next_race
                .as_ref()
                .map(|race| race.season_phase.as_str()),
            Some("BlocoEspecial")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_serializes_convocation_state_fields() {
        let base_dir = create_test_career_dir("load_convocation_contract_payload");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let mut player = driver_queries::get_player_driver(&db.conn).expect("player");
        player.categoria_atual = Some("gt4".to_string());
        player.atributos.skill = 98.0;
        driver_queries::update_driver(&db.conn, &player).expect("update player");

        crate::convocation::advance_to_convocation_window(&db.conn).expect("advance convocation");
        crate::convocation::run_convocation_window(&db.conn).expect("run convocation");
        let offers = crate::commands::convocation::get_player_special_offers_in_base_dir(
            &base_dir,
            "career_001",
        )
        .expect("special offers");
        crate::commands::convocation::respond_player_special_offer_in_base_dir(
            &base_dir,
            "career_001",
            &offers[0].id,
            true,
        )
        .expect("accept offer");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let payload = serde_json::to_value(&career).expect("serialize payload");

        assert_eq!(payload["season"]["fase"], "JanelaConvocacao");
        assert_eq!(payload["player"]["categoria_especial_ativa"], "endurance");
        assert!(
            payload["player_team"].get("classe").is_some(),
            "player_team.classe deveria ser serializado para a UI"
        );
        assert_eq!(
            payload["accepted_special_offer"]["special_category"],
            "endurance"
        );
        assert_eq!(
            payload["accepted_special_offer"]["team_name"],
            offers[0].team_name
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_prefers_regular_team_outside_special_phase() {
        let base_dir = create_test_career_dir("load_regular_team_outside_special_phase");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let regular_contract =
            contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
                .expect("regular contract")
                .expect("player regular contract");
        let special_team = team_queries::get_teams_by_category(&db.conn, "endurance")
            .expect("special teams")
            .into_iter()
            .next()
            .expect("endurance team");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");

        let special_contract = contract_queries::generate_especial_contract(
            next_id(&db.conn, IdType::Contract).expect("special contract id"),
            &player.id,
            &player.nome,
            &special_team.id,
            &special_team.nome,
            TeamRole::Numero1,
            "endurance",
            special_team.classe.as_deref().unwrap_or("gt4"),
            season.numero,
        );
        contract_queries::insert_contract(&db.conn, &special_contract).expect("insert special");
        driver_queries::update_driver_especial_category(&db.conn, &player.id, Some("endurance"))
            .expect("set special category");
        team_queries::update_team_pilots(&db.conn, &special_team.id, Some(&player.id), None)
            .expect("set special lineup");
        season_queries::update_season_fase(&db.conn, &season.id, &SeasonPhase::BlocoRegular)
            .expect("keep regular phase");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let player_team = career.player_team.as_ref().expect("player team");

        assert_eq!(player_team.id, regular_contract.equipe_id);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_repairs_duplicate_regular_contract_state() {
        let base_dir = create_test_career_dir("repair_duplicate_regular_contract_state");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let mut player = driver_queries::get_player_driver(&db.conn).expect("player");
        player.atributos.skill = 99.0;
        player.categoria_atual = Some("gt4".to_string());
        driver_queries::update_driver(&db.conn, &player).expect("update player");

        let original_contract =
            contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
                .expect("original contract")
                .expect("player regular contract");
        let replacement_team = team_queries::get_teams_by_category(&db.conn, "mazda_rookie")
            .expect("rookie teams")
            .into_iter()
            .find(|team| team.id != original_contract.equipe_id)
            .expect("replacement team");
        let displaced_contract =
            contract_queries::get_active_contracts_for_team(&db.conn, &replacement_team.id)
                .expect("replacement contracts")
                .into_iter()
                .find(|contract| contract.tipo == crate::models::enums::ContractType::Regular)
                .expect("regular driver to displace");
        contract_queries::update_contract_status(
            &db.conn,
            &displaced_contract.id,
            &ContractStatus::Rescindido,
        )
        .expect("rescind replacement seat");
        db.conn
            .execute_batch("DROP INDEX IF EXISTS idx_contracts_active_pilot_tipo;")
            .expect("drop active-contract uniqueness guard for corruption scenario");

        let mut replacement_contract = crate::models::contract::Contract::new(
            next_id(&db.conn, IdType::Contract).expect("replacement contract id"),
            player.id.clone(),
            player.nome.clone(),
            replacement_team.id.clone(),
            replacement_team.nome.clone(),
            original_contract.temporada_inicio,
            2,
            250_000.0,
            TeamRole::Numero1,
            replacement_team.categoria.clone(),
        );
        replacement_contract.created_at = "9999-12-31T23:59:59".to_string();
        contract_queries::insert_contract(&db.conn, &replacement_contract)
            .expect("insert replacement contract");

        let gt4_team = team_queries::get_teams_by_category(&db.conn, "gt4")
            .expect("gt4 teams")
            .into_iter()
            .next()
            .expect("gt4 team");
        team_queries::update_team_pilots(
            &db.conn,
            &gt4_team.id,
            Some(&player.id),
            gt4_team.piloto_2_id.as_deref(),
        )
        .expect("corrupt gt4 lineup");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let refreshed_player =
            driver_queries::get_player_driver(&refreshed_db.conn).expect("player");
        let active_regular_contracts =
            contract_queries::get_contracts_for_pilot(&refreshed_db.conn, &player.id)
                .expect("player contracts")
                .into_iter()
                .filter(|contract| {
                    contract.status == ContractStatus::Ativo
                        && contract.tipo == crate::models::enums::ContractType::Regular
                })
                .collect::<Vec<_>>();
        let original_contract_after =
            contract_queries::get_contract_by_id(&refreshed_db.conn, &original_contract.id)
                .expect("query original contract")
                .expect("original contract exists");
        let refreshed_replacement_team =
            team_queries::get_team_by_id(&refreshed_db.conn, &replacement_team.id)
                .expect("query replacement team")
                .expect("replacement team");
        let refreshed_gt4_team = team_queries::get_team_by_id(&refreshed_db.conn, &gt4_team.id)
            .expect("query gt4 team")
            .expect("gt4 team");
        let player_team = career.player_team.as_ref().expect("player team");

        assert_eq!(player_team.id, replacement_team.id);
        assert_eq!(active_regular_contracts.len(), 1);
        assert_eq!(active_regular_contracts[0].id, replacement_contract.id);
        assert_eq!(original_contract_after.status, ContractStatus::Rescindido);
        assert_eq!(
            refreshed_player.categoria_atual.as_deref(),
            Some(replacement_team.categoria.as_str())
        );
        assert!(
            refreshed_gt4_team.piloto_1_id.as_deref() != Some(player.id.as_str())
                && refreshed_gt4_team.piloto_2_id.as_deref() != Some(player.id.as_str())
        );
        assert!(
            refreshed_replacement_team.piloto_1_id.as_deref() == Some(player.id.as_str())
                || refreshed_replacement_team.piloto_2_id.as_deref() == Some(player.id.as_str())
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_next_race_briefing_summarizes_track_history() {
        let base_dir = create_test_career_dir("load_briefing_track_history");
        let career_id = "career_001";
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let calendar =
            calendar_queries::get_calendar(&db.conn, &season.id, "mazda_rookie").expect("calendar");
        let race_one = calendar.first().expect("race one");
        let race_two = calendar.get(1).expect("race two");

        db.conn
            .execute(
                "UPDATE calendar SET track_name = ?1 WHERE id IN (?2, ?3)",
                rusqlite::params!["Pista Espelho", race_one.id, race_two.id],
            )
            .expect("update track names");

        let race_result = crate::commands::race::simulate_race_weekend_in_base_dir(
            &base_dir,
            career_id,
            &race_one.id,
        )
        .expect("simulate race");
        let player_finish = race_result
            .player_race
            .race_results
            .iter()
            .find(|entry| entry.is_jogador)
            .map(|entry| entry.finish_position)
            .expect("player finish");
        let player_dnf = race_result
            .player_race
            .race_results
            .iter()
            .find(|entry| entry.is_jogador)
            .map(|entry| entry.is_dnf)
            .expect("player dnf flag");

        let career = load_career_in_base_dir(&base_dir, career_id).expect("load career");
        let track_history = career
            .next_race_briefing
            .as_ref()
            .and_then(|briefing| briefing.track_history.as_ref())
            .expect("track history");

        assert!(track_history.has_data);
        assert_eq!(track_history.starts, 1);
        assert_eq!(track_history.best_finish, Some(player_finish));
        assert_eq!(track_history.last_finish, Some(player_finish));
        assert_eq!(track_history.dnfs, if player_dnf { 1 } else { 0 });
        assert_eq!(track_history.last_visit_season, Some(1));
        assert_eq!(track_history.last_visit_round, Some(1));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_next_race_briefing_exposes_primary_rival() {
        let base_dir = create_test_career_dir("load_briefing_primary_rival");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let rival_driver = driver_queries::get_drivers_by_category(&db.conn, "mazda_rookie")
            .expect("category drivers")
            .into_iter()
            .find(|driver| !driver.is_jogador)
            .expect("ai rival");

        db.conn
            .execute(
                "UPDATE drivers SET temp_pontos = 90.0, temp_vitorias = 3, temp_podios = 4 WHERE id = ?1",
                rusqlite::params![player.id],
            )
            .expect("update player");
        db.conn
            .execute(
                "UPDATE drivers SET temp_pontos = 96.0, temp_vitorias = 4, temp_podios = 5 WHERE id = ?1",
                rusqlite::params![rival_driver.id],
            )
            .expect("update rival");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let rival = career
            .next_race_briefing
            .as_ref()
            .and_then(|briefing| briefing.primary_rival.as_ref())
            .expect("primary rival");

        assert_eq!(rival.driver_id, rival_driver.id);
        assert_eq!(rival.driver_name, rival_driver.nome);
        assert_eq!(rival.championship_position, 1);
        assert_eq!(rival.gap_points, 6);
        assert!(rival.is_ahead);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_next_race_briefing_filters_weekend_stories() {
        let base_dir = create_test_career_dir("load_briefing_weekend_stories");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");

        news_queries::insert_news_batch(
            &db.conn,
            &vec![
                NewsItem {
                    id: "BRF001".to_string(),
                    tipo: NewsType::Rivalidade,
                    icone: "R".to_string(),
                    titulo: "Duelo esquenta a abertura".to_string(),
                    texto: "A tensao entre os protagonistas cresce antes da etapa de abertura."
                        .to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Destaque,
                    timestamp: 300,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: Some("P002".to_string()),
                    team_id: None,
                },
                NewsItem {
                    id: "BRF002".to_string(),
                    tipo: NewsType::Hierarquia,
                    icone: "H".to_string(),
                    titulo: "Equipe reavalia ordem interna".to_string(),
                    texto: "O box chega atento ao equilibrio interno antes da largada.".to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 250,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                NewsItem {
                    id: "BRF003".to_string(),
                    tipo: NewsType::Corrida,
                    icone: "C".to_string(),
                    titulo: "Abertura promete grid apertado".to_string(),
                    texto:
                        "A etapa de abertura deve embaralhar o pelotao logo nas primeiras voltas."
                            .to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: NewsImportance::Alta,
                    timestamp: 200,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                NewsItem {
                    id: "BRF004".to_string(),
                    tipo: NewsType::Corrida,
                    icone: "X".to_string(),
                    titulo: "Outra categoria movimenta a semana".to_string(),
                    texto: "Essa noticia nao deve entrar na previa da etapa do jogador."
                        .to_string(),
                    rodada: Some(1),
                    semana_pretemporada: None,
                    temporada: season.numero,
                    categoria_id: Some("gt4".to_string()),
                    categoria_nome: Some("GT4".to_string()),
                    importancia: NewsImportance::Destaque,
                    timestamp: 400,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: None,
                },
            ],
        )
        .expect("seed news");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let stories = &career
            .next_race_briefing
            .as_ref()
            .expect("briefing")
            .weekend_stories;

        assert_eq!(stories.len(), 3);
        assert_eq!(stories[0].title, "Duelo esquenta a abertura");
        assert!(stories
            .iter()
            .all(|story| !story.title.contains("Outra categoria")));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_load_career_invalid_id() {
        let base_dir = unique_test_dir("load_invalid");
        fs::create_dir_all(&base_dir).expect("base dir");

        let error = load_career_in_base_dir(&base_dir, "career_999").expect_err("should fail");
        assert!(error.contains("Save nao encontrado"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_list_saves_format() {
        let base_dir = create_test_career_dir("list_saves");
        let saves = list_saves_in_base_dir(&base_dir).expect("list saves");

        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].career_id, "career_001");
        assert_eq!(saves[0].player_name, "Joao Silva");
        assert_eq!(saves[0].category, "mazda_rookie");
        assert_eq!(saves[0].season, 1);
        assert!(saves[0].total_races > 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_drivers_by_category_returns_ordered_standings() {
        let base_dir = create_test_career_dir("drivers_by_category");
        let standings =
            get_drivers_by_category_in_base_dir(&base_dir, "career_001", "mazda_rookie")
                .expect("driver standings");

        assert_eq!(standings.len(), 12);
        assert_eq!(standings[0].posicao_campeonato, 1);
        assert!(standings
            .windows(2)
            .all(|window| window[0].pontos >= window[1].pontos));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_drivers_by_category_uses_recent_results_fallback_from_driver_record() {
        let base_dir = create_test_career_dir("drivers_recent_fallback");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");

        let mut driver = driver_queries::get_player_driver(&db.conn).expect("player");
        driver.stats_temporada.corridas = 3;
        driver.ultimos_resultados = serde_json::json!([
            { "position": 8, "is_dnf": false },
            { "position": 6, "is_dnf": false },
            { "position": 4, "is_dnf": false }
        ]);
        driver_queries::update_driver(&db.conn, &driver).expect("update driver");

        let results_path = config
            .saves_dir()
            .join("career_001")
            .join("race_results.json");
        if results_path.exists() {
            fs::remove_file(&results_path).expect("remove history file");
        }

        let standings =
            get_drivers_by_category_in_base_dir(&base_dir, "career_001", "mazda_rookie")
                .expect("driver standings");
        let player = standings
            .into_iter()
            .find(|entry| entry.is_jogador)
            .expect("player standing");

        let fallback_tail: Vec<i32> = player
            .results
            .iter()
            .flatten()
            .map(|result| result.position)
            .collect();

        assert_eq!(fallback_tail, vec![8, 6, 4]);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_teams_standings_returns_category_grid() {
        let base_dir = create_test_career_dir("teams_standings");
        let standings = get_teams_standings_in_base_dir(&base_dir, "career_001", "mazda_rookie")
            .expect("team standings");

        assert_eq!(standings.len(), 6);
        assert_eq!(standings[0].posicao, 1);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_consecutive_team_seasons_up_to_counts_only_current_streak() {
        let mut season_one = crate::models::contract::Contract::new(
            "C001".to_string(),
            "P001".to_string(),
            "Piloto".to_string(),
            "T001".to_string(),
            "Equipe 1".to_string(),
            1,
            1,
            100_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        season_one.status = ContractStatus::Expirado;
        let mut season_two = crate::models::contract::Contract::new(
            "C002".to_string(),
            "P001".to_string(),
            "Piloto".to_string(),
            "T001".to_string(),
            "Equipe 1".to_string(),
            2,
            1,
            110_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        season_two.status = ContractStatus::Expirado;
        let season_three = crate::models::contract::Contract::new(
            "C003".to_string(),
            "P001".to_string(),
            "Piloto".to_string(),
            "T001".to_string(),
            "Equipe 1".to_string(),
            3,
            2,
            120_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        let mut different_team = crate::models::contract::Contract::new(
            "C004".to_string(),
            "P002".to_string(),
            "Piloto 2".to_string(),
            "T001".to_string(),
            "Equipe 1".to_string(),
            1,
            1,
            95_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        different_team.status = ContractStatus::Expirado;
        let current_other_team = crate::models::contract::Contract::new(
            "C005".to_string(),
            "P002".to_string(),
            "Piloto 2".to_string(),
            "T002".to_string(),
            "Equipe 2".to_string(),
            3,
            1,
            105_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );

        let veteran_streak =
            consecutive_team_seasons_up_to(&[season_one, season_two, season_three], "T001", 3);
        let newcomer_streak =
            consecutive_team_seasons_up_to(&[different_team, current_other_team], "T002", 3);

        assert_eq!(veteran_streak, Some(3));
        assert_eq!(newcomer_streak, Some(1));
    }

    #[test]
    fn test_get_calendar_for_category_returns_races() {
        let base_dir = create_test_career_dir("calendar_category");
        let races = get_calendar_for_category_in_base_dir(&base_dir, "career_001", "mazda_rookie")
            .expect("calendar");

        assert_eq!(races.len(), 5);
        assert_eq!(races[0].rodada, 1);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_race_results_by_category_returns_round_history_after_simulation() {
        let base_dir = create_test_career_dir("race_history");
        let career_id = "career_001";
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = calendar_queries::get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        crate::commands::race::simulate_race_weekend_in_base_dir(
            &base_dir,
            career_id,
            &next_race.id,
        )
        .expect("simulate race");

        let histories =
            get_race_results_by_category_in_base_dir(&base_dir, career_id, "mazda_rookie")
                .expect("race history");

        assert_eq!(histories.len(), 12);
        assert!(histories.iter().all(|history| history.results.len() == 5));
        assert!(histories.iter().any(|history| history.results[0].is_some()));
        assert!(
            histories.iter().any(|history| {
                history
                    .results
                    .iter()
                    .flatten()
                    .any(|result| result.has_fastest_lap)
            }),
            "expected persisted race history to retain the fastest-lap marker",
        );
        assert!(
            histories
                .iter()
                .flat_map(|history| history.results.iter().flatten())
                .all(|result| result.grid_position > 0),
            "expected persisted race history to retain grid positions",
        );
        assert!(
            histories
                .iter()
                .flat_map(|history| history.results.iter().flatten())
                .all(|result| result.positions_gained == result.grid_position - result.position),
            "expected persisted race history to retain positions gained",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_driver_detail_counts_fastest_laps_from_persisted_history() {
        let base_dir = create_test_career_dir("driver_detail_fastest_lap");
        let career_id = "career_001";
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = calendar_queries::get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        let race_result = crate::commands::race::simulate_race_weekend_in_base_dir(
            &base_dir,
            career_id,
            &next_race.id,
        )
        .expect("simulate race");
        let fastest_lap_driver_id = race_result
            .player_race
            .race_results
            .iter()
            .find(|entry| entry.has_fastest_lap)
            .map(|entry| entry.pilot_id.clone())
            .expect("fastest lap driver");

        let detail = get_driver_detail_in_base_dir(&base_dir, career_id, &fastest_lap_driver_id)
            .expect("driver detail");

        assert_eq!(detail.performance.temporada.voltas_rapidas, Some(1));
        assert_eq!(detail.performance.carreira.voltas_rapidas, Some(1));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_previous_champions_returns_empty_for_first_season() {
        let base_dir = create_test_career_dir("previous_champions");
        let champions = get_previous_champions_in_base_dir(&base_dir, "career_001", "mazda_rookie")
            .expect("previous champions");

        assert!(champions.driver_champion_id.is_none());
        assert!(champions.constructor_champions.is_empty());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_driver_detail_returns_contracted_ai_payload() {
        let base_dir = create_test_career_dir("driver_detail_contracted");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let mut driver = driver_queries::get_drivers_by_category(&db.conn, "mazda_rookie")
            .expect("drivers")
            .into_iter()
            .find(|candidate| !candidate.is_jogador)
            .expect("ai driver");

        driver.atributos.skill = 97.0;
        driver.atributos.gestao_pneus = 20.0;
        driver.motivacao = 82.0;
        driver.melhor_resultado_temp = Some(2);
        driver.stats_temporada.corridas = 3;
        driver.stats_temporada.pontos = 28.0;
        driver.stats_temporada.vitorias = 1;
        driver.stats_temporada.podios = 2;
        driver.stats_temporada.poles = 0;
        driver.stats_temporada.dnfs = 0;
        driver.stats_carreira.corridas = 9;
        driver.stats_carreira.pontos_total = 84.0;
        driver.stats_carreira.vitorias = 2;
        driver.stats_carreira.podios = 4;
        driver.stats_carreira.poles = 1;
        driver.stats_carreira.dnfs = 1;
        driver.stats_carreira.titulos = 2;
        driver_queries::update_driver(&db.conn, &driver).expect("update driver");

        let contract = contract_queries::get_active_contract_for_pilot(&db.conn, &driver.id)
            .expect("active contract")
            .expect("contract");
        let team = team_queries::get_team_by_id(&db.conn, &contract.equipe_id)
            .expect("team query")
            .expect("team");

        let detail = get_driver_detail_in_base_dir(&base_dir, "career_001", &driver.id)
            .expect("driver detail");
        let detail_json = serde_json::to_value(&detail).expect("serialize driver detail");

        assert_eq!(detail.id, driver.id);
        assert_eq!(detail.nome, driver.nome);
        assert_eq!(detail.status, "ativo");
        assert_eq!(
            detail.equipe_id.as_deref(),
            Some(contract.equipe_id.as_str())
        );
        assert_eq!(detail.equipe_nome.as_deref(), Some(team.nome.as_str()));
        assert_eq!(
            detail.equipe_cor_primaria.as_deref(),
            Some(team.cor_primaria.as_str())
        );
        assert_eq!(
            detail.equipe_cor_secundaria.as_deref(),
            Some(team.cor_secundaria.as_str())
        );
        assert_eq!(detail.papel.as_deref(), Some(contract.papel.as_str()));
        assert!(detail.personalidade_primaria.is_some());
        assert!(detail.personalidade_secundaria.is_some());
        assert_eq!(detail.motivacao, 82);
        assert_eq!(detail.stats_temporada.corridas, 3);
        assert_eq!(detail.stats_temporada.pontos, 28);
        assert_eq!(detail.stats_temporada.melhor_resultado, 2);
        assert_eq!(detail.stats_carreira.corridas, 9);
        assert_eq!(detail.stats_carreira.pontos, 84);
        assert_eq!(
            detail.contrato.as_ref().map(|value| value.anos_restantes),
            Some(contract.anos_restantes(season.numero))
        );
        assert!(detail.tags.iter().any(|tag| {
            tag.attribute_name == "skill"
                && tag.tag_text == "Alien"
                && tag.level == "elite"
                && tag.color == "#bc8cff"
        }));
        assert!(detail.tags.iter().any(|tag| {
            tag.attribute_name == "gestao_pneus" && tag.level == "defeito" && tag.color == "#db6d28"
        }));
        assert!(
            detail_json.get("perfil").is_some(),
            "expected modular profile block"
        );
        assert!(
            detail_json.get("competitivo").is_some(),
            "expected modular competitive block",
        );
        assert!(
            detail_json.get("performance").is_some(),
            "expected modular performance block",
        );
        assert!(
            detail_json.get("forma").is_some(),
            "expected current-form block"
        );
        assert!(
            detail_json.get("trajetoria").is_some(),
            "expected basic career-path block",
        );
        assert_eq!(detail.trajetoria.titulos, 2);
        assert!(detail.trajetoria.foi_campeao);
        assert!(
            detail_json.get("contrato_mercado").is_some(),
            "expected contract-and-market block",
        );
        assert_eq!(
            detail_json.pointer("/performance/temporada/pontos"),
            None,
            "expected points to stop being a primary dossier metric",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_driver_detail_marks_active_driver_without_contract_as_livre() {
        let base_dir = create_test_career_dir("driver_detail_free");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let free_driver = Driver::new(
            "P-LIVRE-001".to_string(),
            "Piloto Livre".to_string(),
            "🇧🇷 Brasileiro".to_string(),
            "M".to_string(),
            27,
            2020,
        );
        driver_queries::insert_driver(&db.conn, &free_driver).expect("insert free driver");

        let detail = get_driver_detail_in_base_dir(&base_dir, "career_001", &free_driver.id)
            .expect("driver detail");
        let detail_json = serde_json::to_value(&detail).expect("serialize driver detail");

        assert_eq!(detail.id, free_driver.id);
        assert_eq!(detail.status, "livre");
        assert!(detail.equipe_id.is_none());
        assert!(detail.equipe_nome.is_none());
        assert!(detail.papel.is_none());
        assert!(detail.contrato.is_none());
        assert_eq!(detail.stats_temporada.melhor_resultado, 0);
        assert_eq!(detail.stats_carreira.melhor_resultado, 0);
        assert!(
            detail_json.get("contrato_mercado").is_some(),
            "expected contract/market block to exist structurally",
        );
        assert!(
            detail_json.pointer("/contrato_mercado/mercado").is_none(),
            "expected market data to stay absent until real systems exist",
        );
        assert!(
            detail_json.get("relacionamentos").is_none()
                || detail_json
                    .get("relacionamentos")
                    .is_some_and(|value| value.is_null()),
            "expected relationships block to stay empty when there is no real data",
        );
        assert!(
            detail_json.get("reputacao").is_none()
                || detail_json
                    .get("reputacao")
                    .is_some_and(|value| value.is_null()),
            "expected reputation block to stay empty when there is no real data",
        );
        assert!(
            detail_json.get("saude").is_none()
                || detail_json
                    .get("saude")
                    .is_some_and(|value| value.is_null()),
            "expected health block to stay empty when there is no real data",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_season_rejects_pending_races() {
        let base_dir = create_test_career_dir("advance_pending");

        let error =
            advance_season_in_base_dir(&base_dir, "career_001").expect_err("should reject advance");

        assert!(error.contains("corridas pendentes"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_season_updates_meta_and_creates_next_season() {
        let base_dir = create_test_career_dir("advance_success");
        mark_all_races_completed(&base_dir, "career_001");

        let result = advance_season_in_base_dir(&base_dir, "career_001")
            .expect("advance season should work");

        assert_eq!(result.new_year, 2025);
        assert!(result.preseason_initialized);
        assert!(result.preseason_total_weeks >= 3);

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let active_season = season_queries::get_active_season(&db.conn)
            .expect("active season query")
            .expect("active season");
        let meta = read_save_meta(&config.saves_dir().join("career_001").join("meta.json"))
            .expect("read meta");
        let total_races =
            count_season_calendar_entries(&db.conn, &active_season.id).expect("season race count");
        let distinct_race_ids: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT id) FROM calendar
                 WHERE COALESCE(season_id, temporada_id) = ?1",
                rusqlite::params![&active_season.id],
                |row| row.get(0),
            )
            .expect("distinct race ids");

        assert_eq!(active_season.id, result.new_season_id);
        assert_eq!(active_season.numero, 2);
        assert_eq!(active_season.ano, 2025);
        assert_eq!(meta.current_season, 2);
        assert_eq!(meta.current_year, 2025);
        assert_eq!(meta.total_races, total_races);
        assert!(total_races > 0);
        assert_eq!(distinct_race_ids, total_races);
        assert!(config
            .saves_dir()
            .join("career_001")
            .join("preseason_plan.json")
            .exists());
        let resume_context = read_resume_context(&config.saves_dir().join("career_001"))
            .expect("read resume context")
            .expect("resume context");
        assert_eq!(resume_context.active_view, CareerResumeView::EndOfSeason);
        assert_eq!(
            resume_context
                .end_of_season_result
                .as_ref()
                .map(|snapshot| snapshot.new_year),
            Some(result.new_year)
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_season_succeeds_even_if_resume_context_write_fails() {
        let base_dir = create_test_career_dir("advance_resume_context_failure");
        mark_all_races_completed(&base_dir, "career_001");

        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        fs::create_dir_all(save_dir.join("resume_context.json"))
            .expect("block resume context path");

        let result = advance_season_in_base_dir(&base_dir, "career_001")
            .expect("advance season should still succeed");

        let db_path = save_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let active_season = season_queries::get_active_season(&db.conn)
            .expect("active season query")
            .expect("active season");

        assert_eq!(result.new_year, 2025);
        assert_eq!(active_season.numero, 2);
        assert_eq!(active_season.ano, 2025);
        assert!(save_dir.join("resume_context.json").is_dir());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_preseason_state_returns_initialized_state() {
        let base_dir = create_test_career_dir("preseason_state");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let state =
            get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");

        assert_eq!(state.current_week, 1);
        assert!(!state.is_complete);
        assert!(state.total_weeks >= 3);
        assert!(
            state.current_display_date.is_some(),
            "preseason state should expose a simulation date",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_market_week_updates_plan_state() {
        let base_dir = create_test_career_dir("advance_market_week");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let initial_state =
            get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");
        let initial_date = initial_state
            .current_display_date
            .as_deref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
            .expect("valid initial preseason date");

        let week =
            advance_market_week_in_base_dir(&base_dir, "career_001").expect("advance market week");
        let state =
            get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");
        let advanced_date = state
            .current_display_date
            .as_deref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
            .expect("valid advanced preseason date");

        assert_eq!(week.week_number, 1);
        assert!(state.current_week >= 2 || state.is_complete);
        assert_eq!(
            advanced_date.signed_duration_since(initial_date).num_days(),
            7,
            "advancing the preseason should move the simulated date by one week",
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_preseason_dates_stay_inside_december_to_february_window() {
        let base_dir = create_test_career_dir("preseason_market_window");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let mut state =
            get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");

        loop {
            let current_date = state
                .current_display_date
                .as_deref()
                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                .expect("valid preseason date");
            assert!(
                matches!(current_date.month(), 12 | 1 | 2),
                "preseason date {} should stay inside the december-february market window",
                current_date
            );

            if state.is_complete {
                break;
            }

            advance_market_week_in_base_dir(&base_dir, "career_001").expect("advance market week");
            state =
                get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");
        }

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_preseason_free_agents_payload_keeps_regular_history_when_special_exists() {
        let base_dir = create_test_career_dir("preseason_free_agents_regular_history");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");

        let regular_team = team_queries::get_teams_by_category(&db.conn, "mazda_amador")
            .expect("regular teams")
            .into_iter()
            .next()
            .expect("regular team");
        let special_team = team_queries::get_teams_by_category(&db.conn, "production_challenger")
            .expect("special teams")
            .into_iter()
            .next()
            .expect("special team");

        let mut driver = Driver::new(
            "P-PRESEASON-SPECIAL-001".to_string(),
            "Piloto Historico".to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            26,
            2021,
        );
        driver.status = DriverStatus::Ativo;
        driver.categoria_atual = Some("mazda_amador".to_string());
        driver_queries::insert_driver(&db.conn, &driver).expect("insert driver");

        let mut regular_contract = crate::models::contract::Contract::new(
            next_id(&db.conn, IdType::Contract).expect("regular contract id"),
            driver.id.clone(),
            driver.nome.clone(),
            regular_team.id.clone(),
            regular_team.nome.clone(),
            2,
            3,
            80_000.0,
            TeamRole::Numero1,
            "mazda_amador".to_string(),
        );
        regular_contract.status = ContractStatus::Expirado;
        regular_contract.created_at = "2026-01-01T08:00:00".to_string();
        contract_queries::insert_contract(&db.conn, &regular_contract).expect("insert regular");

        let mut special_contract = contract_queries::generate_especial_contract(
            next_id(&db.conn, IdType::Contract).expect("special contract id"),
            &driver.id,
            &driver.nome,
            &special_team.id,
            &special_team.nome,
            TeamRole::Numero2,
            "production_challenger",
            special_team.classe.as_deref().unwrap_or("mazda"),
            4,
        );
        special_contract.status = ContractStatus::Expirado;
        special_contract.created_at = "2026-06-01T08:00:00".to_string();
        contract_queries::insert_contract(&db.conn, &special_contract).expect("insert special");

        let free_agents =
            get_preseason_free_agents_in_base_dir(&base_dir, "career_001").expect("free agents");
        let preview = free_agents
            .into_iter()
            .find(|item| item.driver_id == driver.id)
            .expect("driver preview");

        assert_eq!(preview.categoria, "mazda_amador");
        assert_eq!(preview.previous_team_name.as_deref(), Some(regular_team.nome.as_str()));
        assert_eq!(
            preview.previous_team_color.as_deref(),
            Some(regular_team.cor_primaria.as_str())
        );
        assert_eq!(preview.seasons_at_last_team, 3);
        assert_eq!(preview.total_career_seasons, 3);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_season_clears_current_standings_results_and_archives_previous_season() {
        let base_dir = create_test_career_dir("advance_archives_recent_results");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");

        let mut player = driver_queries::get_player_driver(&db.conn).expect("player");
        player.stats_temporada.corridas = 3;
        player.stats_temporada.pontos = 41.0;
        player.stats_temporada.vitorias = 1;
        player.stats_temporada.podios = 2;
        player.ultimos_resultados = serde_json::json!([
            { "position": 9, "is_dnf": false },
            { "position": 5, "is_dnf": false },
            { "position": 1, "is_dnf": false }
        ]);
        driver_queries::update_driver(&db.conn, &player).expect("update player");

        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");

        let refreshed_player_record = driver_queries::get_player_driver(&db.conn).expect("player");
        let snapshot_json: String = db
            .conn
            .query_row(
                "SELECT snapshot_json
                 FROM driver_season_archive
                 WHERE piloto_id = ?1 AND season_number = 1",
                rusqlite::params![&player.id],
                |row| row.get(0),
            )
            .expect("archived season snapshot");
        let snapshot: serde_json::Value =
            serde_json::from_str(&snapshot_json).expect("valid snapshot json");

        assert!(
            refreshed_player_record.ultimos_resultados == serde_json::json!([]),
            "new season player record should not keep previous season recent results"
        );
        assert_eq!(
            refreshed_player_record.stats_temporada.corridas, 0,
            "new season player record should reset season race count"
        );
        assert_eq!(
            snapshot["ultimos_resultados"],
            serde_json::json!([
                { "position": 9, "is_dnf": false },
                { "position": 5, "is_dnf": false },
                { "position": 1, "is_dnf": false }
            ]),
            "snapshot should preserve ultimos_resultados from the archived season"
        );
        assert_eq!(snapshot["corridas"], 3, "snapshot should preserve corridas");
        assert!(
            snapshot["atributos"]["skill"].is_number(),
            "snapshot should include atributos"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_news_filters_by_season_and_type() {
        let base_dir = create_test_career_dir("get_news_filters");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        advance_market_week_in_base_dir(&base_dir, "career_001").expect("advance market week");

        // news generation is now stubbed; just check the query runs without error
        let _ =
            get_news_in_base_dir(&base_dir, "career_001", Some(1), None, Some(50)).expect("news");
        let _ = get_news_in_base_dir(&base_dir, "career_001", Some(2), Some("Mercado"), Some(50))
            .expect("market news");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_news_rejects_invalid_type_filter() {
        let base_dir = create_test_career_dir("get_news_invalid_type");
        let error = get_news_in_base_dir(
            &base_dir,
            "career_001",
            Some(1),
            Some("TipoInvalido"),
            Some(50),
        )
        .expect_err("invalid news type should fail");

        assert!(error.contains("NewsType"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_is_team_role_vacant_rejects_invalid_role() {
        let base_dir = create_test_career_dir("invalid_team_role_vacancy");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");

        let error = is_team_role_vacant(&db.conn, "T001", "PapelInvalido")
            .expect_err("invalid role should fail");

        assert!(error.contains("TeamRole"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_finalize_preseason_rejects_incomplete_plan() {
        let base_dir = create_test_career_dir("finalize_preseason_incomplete");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let error = finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect_err("should reject incomplete preseason");

        assert!(error.contains("nao foi concluida"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_player_proposals_returns_pending_only() {
        let base_dir = create_test_career_dir("player_proposals_pending_only");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T002", "Recusada");

        let proposals =
            get_player_proposals_in_base_dir(&base_dir, "career_001").expect("player proposals");

        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].status, "Pendente");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_player_proposals_enriched_with_team_data() {
        let base_dir = create_test_career_dir("player_proposals_enriched");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");

        let proposals =
            get_player_proposals_in_base_dir(&base_dir, "career_001").expect("player proposals");

        assert!(!proposals.is_empty());
        assert!(!proposals[0].equipe_nome.is_empty());
        assert!(!proposals[0].categoria_nome.is_empty());
        assert!(proposals[0].car_performance_rating <= 100);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_creates_contract_and_expires_other_proposals() {
        let base_dir = create_test_career_dir("accept_proposal");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T002", "Pendente");

        let response =
            respond_to_proposal_in_base_dir(&base_dir, "career_001", "MP-T001-P001", true)
                .expect("accept proposal");

        assert!(response.success);
        assert_eq!(response.action, "accepted");

        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let active_contract =
            contract_queries::get_active_contract_for_pilot(&refreshed_db.conn, &player.id)
                .expect("active contract")
                .expect("contract");
        assert_eq!(active_contract.equipe_id, "T001");
        let expired = crate::db::queries::market_proposals::get_market_proposal_by_id(
            &refreshed_db.conn,
            &season.id,
            "MP-T002-P001",
        )
        .expect("proposal query")
        .expect("proposal");
        assert_eq!(expired.status.as_str(), "Expirada");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_replaces_only_regular_contract_when_special_exists() {
        let base_dir = create_test_career_dir("accept_proposal_with_special_residue");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let special_team = team_queries::get_teams_by_category(&db.conn, "endurance")
            .expect("special teams")
            .into_iter()
            .next()
            .expect("endurance team");

        let special_contract = contract_queries::generate_especial_contract(
            next_id(&db.conn, IdType::Contract).expect("special contract id"),
            &player.id,
            &player.nome,
            &special_team.id,
            &special_team.nome,
            TeamRole::Numero1,
            "endurance",
            special_team.classe.as_deref().unwrap_or("gt4"),
            season.numero,
        );
        contract_queries::insert_contract(&db.conn, &special_contract).expect("insert special");
        driver_queries::update_driver_especial_category(&db.conn, &player.id, Some("endurance"))
            .expect("set special category");
        team_queries::update_team_pilots(&db.conn, &special_team.id, Some(&player.id), None)
            .expect("set special lineup");

        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");

        let response =
            respond_to_proposal_in_base_dir(&base_dir, "career_001", "MP-T001-P001", true)
                .expect("accept proposal");

        assert!(response.success);

        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let active_regular =
            contract_queries::get_active_regular_contract_for_pilot(&refreshed_db.conn, &player.id)
                .expect("regular contract query")
                .expect("active regular contract");
        let active_regular_count: i64 = refreshed_db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM contracts
                 WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Regular'",
                rusqlite::params![&player.id],
                |row| row.get(0),
            )
            .expect("count active regular contracts");

        assert_eq!(active_regular.equipe_id, "T001");
        assert_eq!(active_regular_count, 1);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_to_full_team_replaces_incumbent_instead_of_creating_third_driver() {
        let base_dir = create_test_career_dir("accept_proposal_replaces_full_team_incumbent");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let mut player = driver_queries::get_player_driver(&db.conn).expect("player");
        player.atributos.skill = 1.0;
        driver_queries::update_driver(&db.conn, &player).expect("downgrade player skill");

        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let current_contract = latest_regular_contract_for_driver(&db.conn, &player.id);
        let target_team = team_queries::get_teams_by_category(&db.conn, &current_contract.categoria)
            .expect("teams by category")
            .into_iter()
            .find(|team| team.id != current_contract.equipe_id)
            .expect("target team");
        let displaced_driver_id = target_team
            .piloto_1_id
            .clone()
            .expect("full target team should have n1 incumbent");

        seed_player_proposal(&db.conn, &season.id, &player.id, &target_team.id, "Pendente");

        respond_to_proposal_in_base_dir(
            &base_dir,
            "career_001",
            &format!("MP-{}-{}", target_team.id, player.id),
            true,
        )
        .expect("accept proposal");

        let career = load_career_in_base_dir(&base_dir, "career_001").expect("load career");
        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let refreshed_target_team = team_queries::get_team_by_id(&refreshed_db.conn, &target_team.id)
            .expect("query target team")
            .expect("target team");
        let target_contracts = contract_queries::get_active_contracts_for_team(
            &refreshed_db.conn,
            &target_team.id,
        )
        .expect("target team contracts")
        .into_iter()
        .filter(|contract| contract.tipo == crate::models::enums::ContractType::Regular)
        .collect::<Vec<_>>();
        let displaced_contract = contract_queries::get_active_regular_contract_for_pilot(
            &refreshed_db.conn,
            &displaced_driver_id,
        )
        .expect("displaced contract query");
        let player_team = career.player_team.as_ref().expect("player team");

        assert_eq!(player_team.id, target_team.id);
        assert_eq!(target_contracts.len(), 2);
        assert!(
            refreshed_target_team.piloto_1_id.as_deref() == Some(player.id.as_str())
                || refreshed_target_team.piloto_2_id.as_deref() == Some(player.id.as_str()),
            "accepted player should remain in the target lineup after consistency repair"
        );
        assert!(
            displaced_contract
                .as_ref()
                .is_none_or(|contract| contract.equipe_id != target_team.id),
            "incumbent displaced from the accepted role should no longer hold an active regular contract for the target team"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_rejects_team_without_required_license() {
        let base_dir = create_test_career_dir("accept_proposal_without_required_license");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let invalid_team = team_queries::get_teams_by_category(&db.conn, "gt4")
            .expect("gt4 teams")
            .into_iter()
            .next()
            .expect("gt4 team");

        seed_player_proposal(
            &db.conn,
            &season.id,
            &player.id,
            &invalid_team.id,
            "Pendente",
        );

        let error = respond_to_proposal_in_base_dir(
            &base_dir,
            "career_001",
            &format!("MP-{}-{}", invalid_team.id, player.id),
            true,
        )
        .expect_err("accept proposal should fail without required license");

        assert!(error.to_lowercase().contains("licenc"));

        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let active_regular =
            contract_queries::get_active_regular_contract_for_pilot(&refreshed_db.conn, &player.id);
        let active_regular = active_regular.expect("regular contract query");
        assert!(active_regular
            .as_ref()
            .is_none_or(|contract| contract.equipe_id != invalid_team.id));

        let invalid_team_contracts: i64 = refreshed_db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM contracts
                 WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Regular' AND equipe_id = ?2",
                rusqlite::params![&player.id, &invalid_team.id],
                |row| row.get(0),
            )
            .expect("count invalid team contracts");
        assert_eq!(invalid_team_contracts, 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_removes_pending_player_events_from_preseason_plan() {
        let base_dir = create_test_career_dir("accept_proposal_clears_pending_player_events");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let current_contract = latest_regular_contract_for_driver(&db.conn, &player.id);
        let gt4_team = team_queries::get_teams_by_category(&db.conn, "gt4")
            .expect("gt4 teams")
            .into_iter()
            .find(|team| team.id != current_contract.equipe_id)
            .expect("gt4 team");

        seed_player_proposal(
            &db.conn,
            &season.id,
            &player.id,
            &current_contract.equipe_id,
            "Pendente",
        );

        let mut plan = crate::market::preseason::load_preseason_plan(&save_dir)
            .expect("load plan")
            .expect("preseason plan");
        plan.planned_events.push(PlannedEvent {
            week: 2,
            executed: false,
            event: PendingAction::ExpireContract {
                contract_id: current_contract.id.clone(),
                driver_id: player.id.clone(),
                driver_name: player.nome.clone(),
                team_id: current_contract.equipe_id.clone(),
                team_name: current_contract.equipe_nome.clone(),
            },
        });
        plan.planned_events.push(PlannedEvent {
            week: 3,
            executed: false,
            event: PendingAction::Transfer {
                driver_id: player.id.clone(),
                driver_name: player.nome.clone(),
                from_team_id: Some(current_contract.equipe_id.clone()),
                from_team_name: Some(current_contract.equipe_nome.clone()),
                to_team_id: gt4_team.id.clone(),
                to_team_name: gt4_team.nome.clone(),
                salary: 120_000.0,
                duration: 1,
                role: TeamRole::Numero2.as_str().to_string(),
            },
        });
        save_preseason_plan(&save_dir, &plan).expect("save mutated plan");

        let response = respond_to_proposal_in_base_dir(
            &base_dir,
            "career_001",
            &format!("MP-{}-{}", current_contract.equipe_id, player.id),
            true,
        )
        .expect("accept proposal");

        assert!(response.success);

        let plan = crate::market::preseason::load_preseason_plan(&save_dir)
            .expect("reload plan")
            .expect("preseason plan");
        assert!(
            !plan.planned_events.iter().any(|event| {
                !event.executed
                    && matches!(
                        &event.event,
                        PendingAction::ExpireContract { driver_id, .. }
                            | PendingAction::RenewContract { driver_id, .. }
                            | PendingAction::Transfer { driver_id, .. }
                            if driver_id == &player.id
                    )
            }),
            "nenhum evento pendente do jogador deve sobreviver apos aceitar proposta"
        );
        assert!(
            !plan.planned_events.iter().any(|event| {
                !event.executed
                    && matches!(
                        &event.event,
                        PendingAction::PlayerProposal { proposal } if proposal.piloto_id == player.id
                    )
            }),
            "nenhuma proposta futura do jogador deve continuar pendente no plano"
        );

        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let active_regular =
            contract_queries::get_active_regular_contract_for_pilot(&refreshed_db.conn, &player.id)
                .expect("regular contract query")
                .expect("active regular contract");

        assert_eq!(active_regular.equipe_id, current_contract.equipe_id);
        assert_eq!(active_regular.categoria, current_contract.categoria);

        let gt4_contracts: i64 = refreshed_db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM contracts
                 WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Regular' AND equipe_id = ?2",
                rusqlite::params![&player.id, &gt4_team.id],
                |row| row.get(0),
            )
            .expect("count gt4 contracts");
        assert_eq!(gt4_contracts, 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_accept_proposal_removes_stale_place_rookie_for_accepted_team_role() {
        let base_dir = create_test_career_dir("accept_proposal_clears_backfilled_rookie");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");

        if contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
            .expect("active regular contract query")
            .is_none()
        {
            let mut news_items = Vec::new();
            force_place_player(&db.conn, &player, &season, &mut news_items)
                .expect("force place player");
        }

        let current_contract = latest_regular_contract_for_driver(&db.conn, &player.id);
        let target_team =
            team_queries::get_teams_by_category(&db.conn, &current_contract.categoria)
                .expect("teams by category")
                .into_iter()
                .find(|team| team.id != current_contract.equipe_id)
                .expect("target team");
        seed_player_proposal(
            &db.conn,
            &season.id,
            &player.id,
            &target_team.id,
            "Pendente",
        );

        let mut plan = crate::market::preseason::load_preseason_plan(&save_dir)
            .expect("load plan")
            .expect("preseason plan");
        plan.planned_events.push(PlannedEvent {
            week: 4,
            executed: false,
            event: PendingAction::PlaceRookie {
                driver: Driver::new(
                    "P-PLAN-ROOKIE".to_string(),
                    "Rookie de Plano".to_string(),
                    "🇧🇷 Brasileiro".to_string(),
                    "M".to_string(),
                    18,
                    2025,
                ),
                team_id: target_team.id.clone(),
                team_name: target_team.nome.clone(),
                salary: 22_000.0,
                duration: 1,
                role: TeamRole::Numero1.as_str().to_string(),
            },
        });
        save_preseason_plan(&save_dir, &plan).expect("save mutated plan");

        let response = respond_to_proposal_in_base_dir(
            &base_dir,
            "career_001",
            &format!("MP-{}-{}", target_team.id, player.id),
            true,
        )
        .expect("accept proposal");

        assert!(response.success);

        let plan = crate::market::preseason::load_preseason_plan(&save_dir)
            .expect("reload plan")
            .expect("preseason plan");
        assert!(
            !plan.planned_events.iter().any(|event| {
                !event.executed
                    && matches!(
                        &event.event,
                        PendingAction::PlaceRookie { team_id, role, .. }
                            if team_id == &target_team.id
                                && role == TeamRole::Numero1.as_str()
                    )
            }),
            "a vaga preenchida pelo aceite nao deve manter PlaceRookie pendente"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_reject_proposal_marks_recusada_and_generates_news() {
        let base_dir = create_test_career_dir("reject_proposal");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");

        let response =
            respond_to_proposal_in_base_dir(&base_dir, "career_001", "MP-T001-P001", false)
                .expect("reject proposal");

        assert!(response.success);
        assert_eq!(response.action, "rejected");

        let refreshed_db = Database::open_existing(&db_path).expect("db reopen");
        let proposal = crate::db::queries::market_proposals::get_market_proposal_by_id(
            &refreshed_db.conn,
            &season.id,
            "MP-T001-P001",
        )
        .expect("proposal query")
        .expect("proposal");
        assert_eq!(proposal.status.as_str(), "Recusada");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_finalize_blocks_with_pending_proposals() {
        let base_dir = create_test_career_dir("finalize_pending_proposals");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");
        force_complete_preseason_plan(&config.saves_dir().join("career_001"));

        let error = finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect_err("should block pending proposals");

        assert!(error.contains("pendente"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_finalize_allows_player_without_team_when_plan_is_resolved() {
        let base_dir = create_test_career_dir("finalize_without_team");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        if let Some(contract) =
            contract_queries::get_active_contract_for_pilot(&db.conn, &player.id)
                .expect("active contract")
        {
            contract_queries::update_contract_status(
                &db.conn,
                &contract.id,
                &crate::models::enums::ContractStatus::Rescindido,
            )
            .expect("rescind old contract");
            team_queries::remove_pilot_from_team(&db.conn, &player.id, &contract.equipe_id)
                .expect("remove from team");
        }
        force_complete_preseason_plan(&config.saves_dir().join("career_001"));

        finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect("should allow advancing even without an active player team");

        let save_dir = config.saves_dir().join("career_001");
        assert!(
            !save_dir.join("preseason_plan.json").exists(),
            "finalizacao deve limpar o plano da pre-temporada"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_finalize_succeeds_when_all_resolved() {
        let base_dir = create_test_career_dir("finalize_success");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        force_complete_preseason_plan(&save_dir);
        persist_resume_context_in_base_dir(
            &base_dir,
            "career_001",
            CareerResumeView::Preseason,
            None,
        )
        .expect("persist preseason resume context");

        finalize_preseason_in_base_dir(&base_dir, "career_001").expect("finalize preseason");

        assert!(!save_dir.join("preseason_plan.json").exists());
        assert!(read_resume_context(&save_dir)
            .expect("read resume context")
            .is_none());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_can_advance_from_second_season_after_finalizing_preseason() {
        let base_dir = create_test_career_dir("advance_second_season");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance to season 2");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("active season query")
            .expect("active season");
        if contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
            .expect("active regular contract query")
            .is_none()
        {
            let mut news_items = Vec::new();
            force_place_player(&db.conn, &player, &season, &mut news_items)
                .expect("force place player for season 2");
        }

        force_complete_preseason_plan(&save_dir);
        finalize_preseason_in_base_dir(&base_dir, "career_001").expect("finalize preseason");

        mark_all_races_completed(&base_dir, "career_001");
        let result = advance_season_in_base_dir(&base_dir, "career_001")
            .expect("advance to season 3 should work");

        let refreshed_db = Database::open_existing(&db_path).expect("db");
        let active_season = season_queries::get_active_season(&refreshed_db.conn)
            .expect("active season query")
            .expect("active season");

        assert_eq!(result.new_year, 2026);
        assert_eq!(active_season.numero, 3);
        assert_eq!(active_season.ano, 2026);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_skip_all_pending_races_allows_teamless_player_to_reach_next_preseason() {
        let base_dir = create_test_career_dir("skip_teamless_second_season");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance to season 2");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");

        if let Some(contract) =
            contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
                .expect("active regular contract")
        {
            contract_queries::update_contract_status(
                &db.conn,
                &contract.id,
                &crate::models::enums::ContractStatus::Rescindido,
            )
            .expect("rescind old contract");
            team_queries::remove_pilot_from_team(&db.conn, &player.id, &contract.equipe_id)
                .expect("remove from team");
        }

        force_complete_preseason_plan(&save_dir);
        finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect("finalize preseason without team");

        skip_all_pending_races_in_base_dir(&base_dir, "career_001")
            .expect("teamless player should be able to skip season");
        let result = advance_season_in_base_dir(&base_dir, "career_001")
            .expect("advance to season 3 should work after skipping teamless season");

        let refreshed_db = Database::open_existing(&db_path).expect("db");
        let active_season = season_queries::get_active_season(&refreshed_db.conn)
            .expect("active season query")
            .expect("active season");

        assert_eq!(result.new_year, 2026);
        assert_eq!(active_season.numero, 3);
        assert_eq!(active_season.ano, 2026);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_teamless_player_skip_path_keeps_special_grids_assignable() {
        let base_dir = create_test_career_dir("skip_teamless_special_grid");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance to season 2");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        let mut db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");

        if let Some(contract) =
            contract_queries::get_active_regular_contract_for_pilot(&db.conn, &player.id)
                .expect("active regular contract")
        {
            contract_queries::update_contract_status(
                &db.conn,
                &contract.id,
                &crate::models::enums::ContractStatus::Rescindido,
            )
            .expect("rescind old contract");
            team_queries::remove_pilot_from_team(&db.conn, &player.id, &contract.equipe_id)
                .expect("remove from team");
        }

        force_complete_preseason_plan(&save_dir);
        finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect("finalize preseason without team");

        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        let pending_regular =
            calendar_queries::get_pending_races(&db.conn, &season.id).expect("pending races");
        for race in &pending_regular {
            crate::commands::race::simulate_category_race(&mut db, race, false)
                .expect("simulate regular race while skipping");
        }

        crate::convocation::advance_to_convocation_window(&db.conn)
            .expect("advance to convocation");
        let convocation = crate::convocation::run_convocation_window(&db.conn)
            .expect("run convocation");
        assert!(
            convocation.errors.is_empty(),
            "convocation should not report structural errors: {:?}",
            convocation.errors
        );
        crate::convocation::iniciar_bloco_especial(&db.conn).expect("start special block");

        for category_id in ["production_challenger", "endurance"] {
            let active_drivers = driver_queries::get_drivers_by_active_category(&db.conn, category_id)
                .expect("active special drivers");
            let teams =
                team_queries::get_teams_by_category(&db.conn, category_id).expect("special teams");
            let assigned_ids: std::collections::HashSet<String> = teams
                .iter()
                .flat_map(|team| [team.piloto_1_id.clone(), team.piloto_2_id.clone()])
                .flatten()
                .collect();
            let orphaned: Vec<String> = active_drivers
                .iter()
                .filter(|driver| !assigned_ids.contains(&driver.id))
                .map(|driver| format!("{} ({})", driver.nome, driver.id))
                .collect();

            assert!(
                orphaned.is_empty(),
                "special category '{}' should not contain drivers without lineup: {}",
                category_id,
                orphaned.join(", ")
            );
        }

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_cannot_accept_already_resolved_proposal() {
        let base_dir = create_test_career_dir("accept_resolved_proposal");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Recusada");

        let error = respond_to_proposal_in_base_dir(&base_dir, "career_001", "MP-T001-P001", true)
            .expect_err("should reject resolved proposal");

        assert!(error.contains("nao esta mais pendente"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_player_rejects_all_gets_emergency_proposals() {
        let base_dir = create_test_career_dir("reject_all_emergency");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let player = driver_queries::get_player_driver(&db.conn).expect("player");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season query")
            .expect("active season");
        if let Some(contract) =
            contract_queries::get_active_contract_for_pilot(&db.conn, &player.id)
                .expect("active contract")
        {
            contract_queries::update_contract_status(
                &db.conn,
                &contract.id,
                &crate::models::enums::ContractStatus::Rescindido,
            )
            .expect("rescind old contract");
            team_queries::remove_pilot_from_team(&db.conn, &player.id, &contract.equipe_id)
                .expect("remove from team");
        }
        seed_player_proposal(&db.conn, &season.id, &player.id, "T001", "Pendente");

        let response =
            respond_to_proposal_in_base_dir(&base_dir, "career_001", "MP-T001-P001", false)
                .expect("reject proposal");

        assert_eq!(response.action, "rejected");
        assert!(response.remaining_proposals > 0);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_briefing_phrase_history_persists_and_keeps_only_last_five_rounds_per_driver_bucket() {
        let base_dir = create_test_career_dir("briefing_phrase_history");
        let career_id = "career_001";

        for round_number in 1..=7 {
            save_briefing_phrase_history_in_base_dir(
                &base_dir,
                career_id,
                1,
                vec![BriefingPhraseEntryInput {
                    round_number,
                    driver_id: "drv-player".to_string(),
                    bucket_key: "p1".to_string(),
                    phrase_id: format!("p1-baseline-{round_number}"),
                }],
            )
            .expect("save phrase history");
        }

        let history =
            get_briefing_phrase_history_in_base_dir(&base_dir, career_id).expect("phrase history");

        assert_eq!(history.season_number, 1);
        assert_eq!(history.entries.len(), 5);
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| entry.round_number)
                .collect::<Vec<_>>(),
            vec![7, 6, 5, 4, 3]
        );
        assert!(history
            .entries
            .iter()
            .all(|entry| entry.driver_id == "drv-player" && entry.bucket_key == "p1"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_briefing_phrase_history_resets_when_season_changes() {
        let base_dir = create_test_career_dir("briefing_phrase_history_reset");
        let career_id = "career_001";

        save_briefing_phrase_history_in_base_dir(
            &base_dir,
            career_id,
            1,
            vec![BriefingPhraseEntryInput {
                round_number: 5,
                driver_id: "drv-player".to_string(),
                bucket_key: "p2".to_string(),
                phrase_id: "p2-stable-1".to_string(),
            }],
        )
        .expect("save season one");

        let history = save_briefing_phrase_history_in_base_dir(
            &base_dir,
            career_id,
            2,
            vec![BriefingPhraseEntryInput {
                round_number: 1,
                driver_id: "drv-player".to_string(),
                bucket_key: "p2".to_string(),
                phrase_id: "p2-stable-2".to_string(),
            }],
        )
        .expect("save season two");

        assert_eq!(history.season_number, 2);
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].round_number, 1);
        assert_eq!(history.entries[0].phrase_id, "p2-stable-2");

        let _ = fs::remove_dir_all(base_dir);
    }

    fn create_test_career_dir(label: &str) -> std::path::PathBuf {
        let base_dir = unique_test_dir(label);
        fs::create_dir_all(&base_dir).expect("base dir");

        let input = CreateCareerInput {
            player_name: "Joao Silva".to_string(),
            player_nationality: "br".to_string(),
            player_age: Some(22),
            category: "mazda_rookie".to_string(),
            team_index: 2,
            difficulty: "medio".to_string(),
        };

        let _ = create_career_in_base_dir(&base_dir, input).expect("career should be created");
        base_dir
    }

    fn mark_all_races_completed(base_dir: &Path, career_id: &str) {
        let config = AppConfig::load_or_default(base_dir);
        let db_path = config.saves_dir().join(career_id).join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        db.conn
            .execute("UPDATE calendar SET status = 'Concluida'", [])
            .expect("mark all races completed");
    }

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("iracerapp_{label}_{nanos}"))
    }

    fn seed_player_proposal(
        conn: &rusqlite::Connection,
        season_id: &str,
        player_id: &str,
        team_id: &str,
        status: &str,
    ) {
        let team = team_queries::get_team_by_id(conn, team_id)
            .expect("team query")
            .expect("team");
        let player = driver_queries::get_driver(conn, player_id).expect("player");
        crate::db::queries::market_proposals::insert_player_proposal(
            conn,
            season_id,
            &crate::market::proposals::MarketProposal {
                id: format!("MP-{team_id}-{player_id}"),
                equipe_id: team.id.clone(),
                equipe_nome: team.nome.clone(),
                piloto_id: player.id.clone(),
                piloto_nome: player.nome.clone(),
                categoria: team.categoria.clone(),
                papel: crate::models::enums::TeamRole::Numero1,
                salario_oferecido: 95_000.0,
                duracao_anos: 2,
                status: match status {
                    "Aceita" => crate::market::proposals::ProposalStatus::Aceita,
                    "Recusada" => crate::market::proposals::ProposalStatus::Recusada,
                    "Expirada" => crate::market::proposals::ProposalStatus::Expirada,
                    _ => crate::market::proposals::ProposalStatus::Pendente,
                },
                motivo_recusa: None,
            },
        )
        .expect("insert player proposal");
    }

    fn force_complete_preseason_plan(save_dir: &Path) {
        let mut plan = crate::market::preseason::load_preseason_plan(save_dir)
            .expect("load plan")
            .expect("plan");
        plan.state.is_complete = true;
        plan.state.current_week = plan.state.total_weeks + 1;
        plan.state.phase = crate::market::preseason::PreSeasonPhase::Complete;
        plan.state.player_has_pending_proposals = false;
        crate::market::preseason::save_preseason_plan(save_dir, &plan).expect("save plan");
    }

    fn latest_regular_contract_for_driver(
        conn: &rusqlite::Connection,
        driver_id: &str,
    ) -> crate::models::contract::Contract {
        contract_queries::get_contracts_for_pilot(conn, driver_id)
            .expect("driver contracts query")
            .into_iter()
            .filter(|contract| contract.tipo == crate::models::enums::ContractType::Regular)
            .max_by(|a, b| {
                a.temporada_inicio
                    .cmp(&b.temporada_inicio)
                    .then_with(|| a.created_at.cmp(&b.created_at))
            })
            .expect("latest regular contract")
    }
}
