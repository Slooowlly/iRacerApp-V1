use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Local;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::calendar::{generate_all_calendars_with_year, CalendarEntry};
use crate::commands::career_detail::build_driver_detail_payload;
use crate::commands::career_types::{
    CareerData, CreateCareerResult, DriverDetail, DriverSummary, RaceSummary, SaveInfo,
    SeasonSummary, TeamStanding, TeamSummary, VerifyDatabaseResponse,
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
use crate::db::queries::news as news_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::standings as standings_queries;
use crate::db::queries::teams as team_queries;
use crate::event_interest::{
    calculate_expected_event_interest, to_summary, EventInterestContext, EventInterestSummary,
};
use crate::db::queries::standings::ChampionshipContext;
use crate::evolution::pipeline::{run_end_of_season, EndOfSeasonResult};
use crate::generators::ids::{next_id, next_ids, IdType};
use crate::generators::nationality::{format_nationality, get_nationality};
use crate::generators::world::generate_world;
use crate::market::pipeline::{fill_all_remaining_vacancies, run_market};
use crate::market::preseason::{
    advance_week, delete_preseason_plan, load_preseason_plan, save_preseason_plan, PendingAction,
    PlannedEvent, PreSeasonPlan, PreSeasonState, WeekResult,
};
use crate::market::proposals::{MarketProposal, ProposalStatus};
use crate::db::queries::meta as meta_queries;
use crate::models::driver::Driver;
use crate::models::enums::{ContractStatus, SeasonPhase, TeamRole};
use crate::models::season::Season;
use crate::models::team::{TeamHierarchyClimate, Team};
use crate::news::generator::{
    generate_news_from_end_of_season, generate_news_from_market_events,
    generate_player_rejection_news, generate_player_signing_news,
};
use crate::public_presence::team::{derive_team_public_presence, TeamPublicPresenceTier};
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
        let calendars = generate_all_calendars_with_year(&season_id, season.ano, &mut rand::thread_rng())?;
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
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar piloto do jogador: {e}"))?;
    let player_team = find_player_team(&db.conn, &player.id)?
        .ok_or_else(|| "Equipe do jogador nao encontrada.".to_string())?;
    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let next_race =
        calendar_queries::get_next_race(&db.conn, &active_season.id, &player_team.categoria)
            .map_err(|e| format!("Falha ao carregar proxima corrida: {e}"))?;

    let total_drivers = driver_queries::count_drivers(&db.conn)
        .map_err(|e| format!("Falha ao contar pilotos: {e}"))? as usize;
    let total_teams =
        count_rows(&db.conn, "teams").map_err(|e| format!("Falha ao contar equipes: {e}"))?;
    let total_rodadas = count_calendar_entries(&db.conn, &active_season.id, &player_team.categoria)
        .map_err(|e| format!("Falha ao contar corridas da temporada: {e}"))?;

    // Calcular interesse esperado da próxima corrida (fallback silencioso se falhar).
    // Usa race.categoria como fonte semântica do campeonato do evento.
    let event_interest_summary: Option<EventInterestSummary> = next_race.as_ref().map(|race| {
        let champ = standings_queries::get_championship_context(&db.conn, &race.categoria)
            .unwrap_or(ChampionshipContext { player_position: 0, gap_to_leader: 0 });
        let remaining = total_rodadas - race.rodada;
        let is_title_decider = remaining <= 2
            && champ.gap_to_leader <= 50
            && champ.player_position > 0;
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

    let team_summary = build_team_summary(&db.conn, &player_team)
        .map_err(|e| format!("Falha ao montar resumo da equipe: {e}"))?;

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
            equipe_id: Some(player_team.id.clone()),
            equipe_nome: Some(player_team.nome.clone()),
            equipe_nome_curto: Some(player_team.nome_curto.clone()),
            equipe_cor: player_team.cor_primaria.clone(),
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
        },
        next_race: next_race.map(|race| RaceSummary {
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
            event_interest: event_interest_summary,
        }),
        total_drivers,
        total_teams,
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
    meta_queries::set_meta_value(conn, "next_driver_id",   &(total_drivers   as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "next_team_id",     &(total_teams     as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "next_contract_id", &(total_contracts as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "next_season_id",   &(total_seasons   as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "next_race_id",     &(total_races     as u32 + 1).to_string())?;
    meta_queries::set_meta_value(conn, "current_season",   &total_seasons.to_string())?;
    Ok(())
}

// Internal diagnostic helper kept out of the production Tauri command surface.
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
    let (db, career_dir, mut meta) = open_career_resources(base_dir, career_id)?;
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

    let result = run_end_of_season(&db.conn, &season, &career_dir)?;
    persist_end_of_season_news(&db.conn, &result, season.numero)?;
    let total_races = count_season_calendar_entries(&db.conn, &result.new_season_id)
        .map_err(|e| format!("Falha ao contar corridas da nova temporada: {e}"))?;
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    meta.current_season = (season.numero + 1).max(1) as u32;
    meta.current_year = result.new_year.max(0) as u32;
    meta.last_played = now;
    meta.total_races = total_races;
    write_save_meta(&meta_path, &meta)?;

    config.last_career = Some(career_number);
    config
        .save()
        .map_err(|e| format!("Falha ao atualizar config do app: {e}"))?;

    Ok(result)
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
    let result = advance_week(&db.conn, &mut plan)?;
    persist_market_week_news(&db.conn, &plan.state, &result)?;
    crate::market::preseason::save_preseason_plan(&career_dir, &plan)?;

    meta.last_played = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    write_save_meta(&meta_path, &meta)?;
    Ok(result)
}

pub(crate) fn get_preseason_state_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<PreSeasonState, String> {
    let (_db, career_dir, _) = open_career_resources(base_dir, career_id)?;
    let plan = load_preseason_plan(&career_dir)?
        .ok_or_else(|| "Plano da pre-temporada nao encontrado.".to_string())?;
    Ok(plan.state)
}

pub(crate) fn get_player_proposals_in_base_dir(
    base_dir: &Path,
    career_id: &str,
) -> Result<Vec<PlayerProposalView>, String> {
    let (db, _career_dir, _meta) = open_career_resources(base_dir, career_id)?;
    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    let mut proposals = market_proposal_queries::get_pending_player_proposals(&db.conn, &player.id)
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
    let proposal = market_proposal_queries::get_market_proposal_by_id(&db.conn, proposal_id)
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

        reconcile_plan_after_player_accept(&career_dir, &db.conn, &proposal)?;
        news_items.push(generate_player_signing_news(
            &player.nome,
            &proposal.equipe_nome,
            &proposal.categoria,
            proposal.papel.as_str(),
            season.numero,
        ));
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
        news_items.push(generate_player_rejection_news(
            &player.nome,
            &proposal.equipe_nome,
            season.numero,
        ));
    }

    let mut remaining =
        market_proposal_queries::count_pending_player_proposals(&db.conn, &player.id)
            .map_err(|e| format!("Falha ao contar propostas pendentes: {e}"))?;

    if !accept && remaining == 0 {
        if contract_queries::get_active_contract_for_pilot(&db.conn, &player.id)
            .map_err(|e| format!("Falha ao verificar equipe atual do jogador: {e}"))?
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

    sync_preseason_pending_flag(&career_dir, remaining > 0)?;
    persist_generated_news(&db.conn, &mut news_items)?;
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
        let tipo_normalizado = NewsType::from_str(tipo);
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

    let player = driver_queries::get_player_driver(&db.conn)
        .map_err(|e| format!("Falha ao carregar jogador: {e}"))?;
    let pending = market_proposal_queries::count_pending_player_proposals(&db.conn, &player.id)
        .map_err(|e| format!("Falha ao contar propostas pendentes: {e}"))?;
    if pending > 0 {
        return Err(format!(
            "Voce tem {} proposta(s) pendente(s). Resolva antes de iniciar a temporada.",
            pending
        ));
    }
    if contract_queries::get_active_contract_for_pilot(&db.conn, &player.id)
        .map_err(|e| format!("Falha ao verificar equipe do jogador: {e}"))?
        .is_none()
    {
        return Err(
            "Voce nao tem equipe! Aceite uma proposta antes de iniciar a temporada.".to_string(),
        );
    }

    let season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao carregar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

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
    meta.last_played = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    write_save_meta(&meta_path, &meta)?;
    
    let mut news = vec![NewsItem {
        id: String::new(),
        tipo: NewsType::PreTemporada,
        icone: NewsType::PreTemporada.icone().to_string(),
        titulo: format!("Temporada {} esta aberta!", season.numero),
        texto: "A pre-temporada chegou ao fim. As corridas estao prestes a comecar!".to_string(),
        rodada: Some(0),
        semana_pretemporada: None,
        temporada: season.numero,
        categoria_id: None,
        categoria_nome: None,
        importancia: NewsImportance::Alta,
        timestamp: Local::now().timestamp(),
        driver_id: None,
        team_id: None,
    }];
    persist_generated_news(&db.conn, &mut news)?;
    Ok(())
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
    let contract = contract_queries::get_active_contract_for_pilot(&db.conn, driver_id)
        .map_err(|e| format!("Falha ao buscar contrato ativo: {e}"))?;
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

fn persist_end_of_season_news(
    conn: &rusqlite::Connection,
    result: &EndOfSeasonResult,
    season_number: i32,
) -> Result<(), String> {
    let mut temp_id = temp_news_id_generator();
    let mut timestamp = news_queries::get_latest_news_timestamp(conn)
        .map_err(|e| format!("Falha ao buscar timestamp de noticias: {e}"))?
        + 1;
    // Carrega visibilidade apenas dos pilotos boostáveis (rookies + decliner candidatos).
    // Degrada silenciosamente para HashMap vazio se falhar — camada narrativa, não factual.
    let driver_midia: std::collections::HashMap<String, f64> = {
        let ids: std::collections::HashSet<&str> = result
            .rookies_generated
            .iter()
            .map(|r| r.driver_id.as_str())
            .chain(result.growth_reports.iter().map(|g| g.driver_id.as_str()))
            .collect();
        ids.into_iter()
            .filter_map(|id| {
                driver_queries::get_driver(conn, id)
                    .ok()
                    .map(|d| (d.id, d.atributos.midia))
            })
            .collect()
    };
    let mut items = generate_news_from_end_of_season(
        result,
        season_number,
        &mut temp_id,
        &mut timestamp,
        &driver_midia,
    );
    persist_generated_news(conn, &mut items)
}

fn persist_market_week_news(
    conn: &rusqlite::Connection,
    state: &PreSeasonState,
    week_result: &WeekResult,
) -> Result<(), String> {
    let mut temp_id = temp_news_id_generator();
    let mut timestamp = news_queries::get_latest_news_timestamp(conn)
        .map_err(|e| format!("Falha ao buscar timestamp de noticias: {e}"))?
        + 1;
    // Carrega visibilidade apenas dos pilotos presentes nos eventos da semana.
    // Degrada silenciosamente para HashMap vazio se falhar — camada narrativa, não factual.
    let driver_midia: std::collections::HashMap<String, f64> = {
        let ids: std::collections::HashSet<&str> = week_result
            .events
            .iter()
            .filter_map(|e| e.driver_id.as_deref())
            .collect();
        ids.into_iter()
            .filter_map(|id| {
                driver_queries::get_driver(conn, id)
                    .ok()
                    .map(|d| (d.id, d.atributos.midia))
            })
            .collect()
    };
    // Carrega presença pública das equipes envolvidas nos eventos da semana.
    // Degrada silenciosamente para HashMap vazio se falhar — camada narrativa, não factual.
    // Custo v1: get_active_contracts_for_team + get_driver por equipe única nos eventos da semana
    // (tipicamente 2–5 equipes distintas por semana de preseason — aceito).
    let team_presence: std::collections::HashMap<String, TeamPublicPresenceTier> = {
        let team_ids: std::collections::HashSet<&str> = week_result
            .events
            .iter()
            .filter_map(|e| e.team_id.as_deref())
            .collect();
        team_ids
            .into_iter()
            .filter_map(|tid| {
                let medias: Vec<f64> = contract_queries::get_active_contracts_for_team(conn, tid)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|c| {
                        driver_queries::get_driver(conn, &c.piloto_id)
                            .ok()
                            .map(|d| d.atributos.midia)
                    })
                    .collect();
                if medias.is_empty() {
                    return None;
                }
                Some((tid.to_string(), derive_team_public_presence(&medias).tier))
            })
            .collect()
    };
    let mut items = generate_news_from_market_events(
        &week_result.events,
        state.season_number,
        week_result.week_number,
        &mut temp_id,
        &mut timestamp,
        &driver_midia,
        &team_presence,
    );
    persist_generated_news(conn, &mut items)
}

fn persist_generated_news(
    conn: &rusqlite::Connection,
    items: &mut Vec<NewsItem>,
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let ids = next_ids(conn, IdType::News, items.len() as u32)
        .map_err(|e| format!("Falha ao gerar IDs de noticias: {e}"))?;
    for (item, id) in items.iter_mut().zip(ids.into_iter()) {
        item.id = id;
    }

    news_queries::insert_news_batch(conn, items)
        .map_err(|e| format!("Falha ao persistir noticias: {e}"))?;
    news_queries::trim_news(conn, 400).map_err(|e| format!("Falha ao aparar feed: {e}"))?;
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
    let previous_contract = contract_queries::get_active_contract_for_pilot(tx, &player.id)
        .map_err(|e| format!("Falha ao buscar contrato atual do jogador: {e}"))?;
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
    place_driver_in_team(tx, &team.id, &player.id, proposal.papel.clone())?;
    refresh_team_hierarchy_now(tx, &team.id)?;

    let mut updated_player = player.clone();
    updated_player.categoria_atual = Some(team.categoria.clone());
    updated_player.status = crate::models::enums::DriverStatus::Ativo;
    driver_queries::update_driver(tx, &updated_player)
        .map_err(|e| format!("Falha ao atualizar categoria do jogador: {e}"))?;

    market_proposal_queries::update_proposal_status(tx, &proposal.id, "Aceita", None)
        .map_err(|e| format!("Falha ao marcar proposta como aceita: {e}"))?;
    market_proposal_queries::expire_remaining_proposals(tx, &player.id, &proposal.id)
        .map_err(|e| format!("Falha ao expirar demais propostas: {e}"))?;

    if let Some(previous_team_id) = previous_team_id.filter(|old_team| old_team != &team.id) {
        backfill_team_vacancy(tx, &previous_team_id, season.numero)?;
        refresh_team_hierarchy_now(tx, &previous_team_id)?;
    }

    Ok(())
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
    let mut vacancies = list_team_vacancies(conn)?
        .into_iter()
        .filter(|vacancy| {
            let tier = categories::get_category_config(&vacancy.team.categoria)
                .map(|config| config.tier)
                .unwrap_or(0);
            tier >= player_tier && tier <= player_tier + 1
        })
        .collect::<Vec<_>>();
    if vacancies.is_empty() {
        vacancies = list_team_vacancies(conn)?;
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
    news_items: &mut Vec<NewsItem>,
) -> Result<Option<String>, String> {
    let player_tier = player
        .categoria_atual
        .as_deref()
        .and_then(categories::get_category_config)
        .map(|config| config.tier)
        .unwrap_or(0);
    let mut vacancies = list_team_vacancies(conn)?
        .into_iter()
        .filter(|vacancy| {
            categories::get_category_config(&vacancy.team.categoria)
                .map(|config| config.tier == player_tier)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if vacancies.is_empty() {
        vacancies = list_team_vacancies(conn)?;
    }
    vacancies.sort_by(|a, b| a.team.car_performance.total_cmp(&b.team.car_performance));
    let Some(vacancy) = vacancies.into_iter().next() else {
        return Ok(None);
    };

    let contract = crate::models::contract::Contract::new(
        next_id(conn, IdType::Contract)
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
    contract_queries::insert_contract(conn, &contract)
        .map_err(|e| format!("Falha ao inserir contrato forçado: {e}"))?;
    place_driver_in_team(conn, &vacancy.team.id, &player.id, vacancy.role.clone())?;
    refresh_team_hierarchy_now(conn, &vacancy.team.id)?;
    let mut updated_player = player.clone();
    updated_player.categoria_atual = Some(vacancy.team.categoria.clone());
    updated_player.status = crate::models::enums::DriverStatus::Ativo;
    driver_queries::update_driver(conn, &updated_player)
        .map_err(|e| format!("Falha ao atualizar jogador apos alocacao forcada: {e}"))?;
    news_items.push(generate_player_signing_news(
        &player.nome,
        &vacancy.team.nome,
        &vacancy.team.categoria,
        vacancy.role.as_str(),
        season.numero,
    ));
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
            contract_queries::get_active_contract_for_pilot(conn, &driver.id)
                .ok()
                .flatten()
                .is_none()
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
        rookie
    };

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

fn reconcile_plan_after_player_accept(
    career_dir: &Path,
    conn: &rusqlite::Connection,
    proposal: &MarketProposal,
) -> Result<(), String> {
    let Some(mut plan) = load_preseason_plan(career_dir)? else {
        return Ok(());
    };
    if let Some(index) = plan
        .planned_events
        .iter()
        .position(|event| {
            !event.executed
                && matches!(&event.event,
                    PendingAction::PlaceRookie { team_id, role, .. }
                    if team_id == &proposal.equipe_id && role == proposal.papel.as_str()
                )
        })
        .or_else(|| {
            plan.planned_events.iter().position(|event| {
                !event.executed
                    && matches!(&event.event,
                        PendingAction::PlaceRookie { team_id, .. } if team_id == &proposal.equipe_id
                    )
            })
        })
    {
        plan.planned_events.remove(index);
    }
    refresh_planned_hierarchy_for_team(&mut plan, conn, &proposal.equipe_id)?;
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

fn temp_news_id_generator() -> impl FnMut() -> String {
    let mut counter = 0;
    move || {
        counter += 1;
        format!("TMP{counter:03}")
    }
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
    let meta = read_save_meta(&meta_path)?;

    Ok((db, career_dir, meta))
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
            let team = find_player_team(&db.conn, &driver.id).ok().flatten();
            DriverSummary {
                id: driver_id.clone(),
                nome: driver.nome,
                nacionalidade: driver.nacionalidade,
                idade: driver.idade as i32,
                skill: driver.atributos.skill.round().clamp(0.0, 100.0) as u8,
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
                results: history_map.get(&driver_id).cloned().unwrap_or_default(),
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

    let mut standings: Vec<TeamStanding> = teams
        .into_iter()
        .map(|team| {
            let team_id = team.id.clone();
            let piloto_1_nome = team
                .piloto_1_id
                .as_ref()
                .and_then(|id| driver_queries::get_driver(&db.conn, id).ok())
                .map(|driver| driver.nome);
            let piloto_2_nome = team
                .piloto_2_id
                .as_ref()
                .and_then(|id| driver_queries::get_driver(&db.conn, id).ok())
                .map(|driver| driver.nome);

            TeamStanding {
                posicao: 0,
                id: team_id.clone(),
                nome: team.nome,
                nome_curto: team.nome_curto,
                cor_primaria: team.cor_primaria,
                pontos: team.stats_pontos,
                vitorias: team.stats_vitorias,
                piloto_1_nome,
                piloto_2_nome,
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

fn find_player_team(conn: &rusqlite::Connection, player_id: &str) -> Result<Option<Team>, String> {
    let contract = contract_queries::get_active_contract_for_pilot(conn, player_id)
        .map_err(|e| format!("Falha ao buscar contrato ativo: {e}"))?;
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
        car_performance: team.car_performance,
        confiabilidade: team.confiabilidade,
        budget: team.budget,
        piloto_1_id: team.piloto_1_id.clone(),
        piloto_1_nome,
        piloto_2_id: team.piloto_2_id.clone(),
        piloto_2_nome,
    })
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

        assert!(!career.player_team.id.is_empty());
        assert!(career.player_team.piloto_1_id.is_some());
        assert!(career.player_team.piloto_2_id.is_some());

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
    fn test_get_teams_standings_returns_category_grid() {
        let base_dir = create_test_career_dir("teams_standings");
        let standings = get_teams_standings_in_base_dir(&base_dir, "career_001", "mazda_rookie")
            .expect("team standings");

        assert_eq!(standings.len(), 6);
        assert_eq!(standings[0].posicao, 1);

        let _ = fs::remove_dir_all(base_dir);
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

        let news = news_queries::get_news_by_season(&db.conn, 1, 50).expect("season news");
        assert!(!news.is_empty());

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

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_advance_market_week_updates_plan_state() {
        let base_dir = create_test_career_dir("advance_market_week");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let week =
            advance_market_week_in_base_dir(&base_dir, "career_001").expect("advance market week");
        let state =
            get_preseason_state_in_base_dir(&base_dir, "career_001").expect("preseason state");

        assert_eq!(week.week_number, 1);
        assert!(state.current_week >= 2 || state.is_complete);

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let market_news =
            news_queries::get_news_by_preseason_week(&db.conn, 2, 1).expect("market week news");
        assert!(!market_news.is_empty());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_get_news_filters_by_season_and_type() {
        let base_dir = create_test_career_dir("get_news_filters");
        mark_all_races_completed(&base_dir, "career_001");

        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        advance_market_week_in_base_dir(&base_dir, "career_001").expect("advance market week");

        let season_news =
            get_news_in_base_dir(&base_dir, "career_001", Some(1), None, Some(50)).expect("news");
        assert!(!season_news.is_empty());
        assert!(season_news.iter().all(|item| item.temporada == 1));

        let market_news =
            get_news_in_base_dir(&base_dir, "career_001", Some(2), Some("Mercado"), Some(50))
                .expect("market news");
        assert!(!market_news.is_empty());
        assert!(market_news
            .iter()
            .all(|item| item.temporada == 2 && item.tipo == NewsType::Mercado));

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
            "MP-T002-P001",
        )
        .expect("proposal query")
        .expect("proposal");
        assert_eq!(expired.status.as_str(), "Expirada");

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
            "MP-T001-P001",
        )
        .expect("proposal query")
        .expect("proposal");
        assert_eq!(proposal.status.as_str(), "Recusada");
        let recent_news = news_queries::get_recent_news(&refreshed_db.conn, 20).expect("news");
        assert!(recent_news
            .iter()
            .any(|item| item.titulo.contains("recusou proposta")));

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
    fn test_finalize_blocks_without_team() {
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

        let error = finalize_preseason_in_base_dir(&base_dir, "career_001")
            .expect_err("should block without team");

        assert!(error.contains("nao tem equipe"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_finalize_succeeds_when_all_resolved() {
        let base_dir = create_test_career_dir("finalize_success");
        mark_all_races_completed(&base_dir, "career_001");
        advance_season_in_base_dir(&base_dir, "career_001").expect("advance season");
        let config = AppConfig::load_or_default(&base_dir);
        let save_dir = config.saves_dir().join("career_001");
        let db_path = save_dir.join("career.db");
        force_complete_preseason_plan(&save_dir);

        finalize_preseason_in_base_dir(&base_dir, "career_001").expect("finalize preseason");

        assert!(!save_dir.join("preseason_plan.json").exists());
        let db = Database::open_existing(&db_path).expect("db");
        let recent_news = news_queries::get_recent_news(&db.conn, 20).expect("news");
        assert!(recent_news
            .iter()
            .any(|item| item.titulo.contains("esta aberta")));

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
}
