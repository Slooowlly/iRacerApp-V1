use std::collections::HashMap;
use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::commands::race_history::append_race_result;
use crate::config::app_config::{AppConfig, SaveMeta};
use crate::constants::categories::{get_all_categories, get_category_config};
use crate::db::connection::Database;
use crate::db::connection::DbError;
use crate::db::queries::calendar as calendar_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::news as news_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::standings as standings_queries;
use crate::db::queries::standings::ChampionshipContext;
use crate::event_interest::{
    calculate_expected_event_interest, calculate_realized_event_interest, EventInterestContext,
    RealizedEventInterest,
};
use crate::db::queries::teams as team_queries;
use crate::generators::ids::{next_ids, IdType};
use crate::news::generator::generate_news_from_race;
use crate::news::{NewsImportance, NewsItem, NewsType};
use crate::simulation::batch::{BriefRaceResult, CategorySimResult, SimHighlight, SimultaneousResults};
use crate::simulation::context::{SimDriver, SimulationContext};
use crate::simulation::engine::run_full_race;
use crate::simulation::race::RaceResult;
use crate::{calendar::CalendarEntry, models::team::Team};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceWeekendResult {
    pub player_race: RaceResult,
    pub other_categories: SimultaneousResults,
}

fn compute_post_race_impact(
    conn: &rusqlite::Connection,
    race_entry: &CalendarEntry,
    player_race: &RaceResult,
) -> Option<RealizedEventInterest> {
    let category = get_category_config(&race_entry.categoria)?;
    let total_rodadas = category.corridas_por_temporada as i32;
    let player = driver_queries::get_player_driver(conn).ok()?;
    let champ = standings_queries::get_championship_context(conn, &race_entry.categoria)
        .unwrap_or(ChampionshipContext { player_position: 0, gap_to_leader: 0 });
    let player_result = player_race.race_results.iter().find(|r| r.is_jogador)?;

    let remaining = total_rodadas - race_entry.rodada;
    let is_title_decider = remaining <= 2 && champ.gap_to_leader <= 50 && champ.player_position > 0;
    let is_final_round_decider = race_entry.rodada == total_rodadas && is_title_decider;

    let ctx = EventInterestContext {
        categoria: race_entry.categoria.clone(),
        season_phase: race_entry.season_phase,
        rodada: race_entry.rodada,
        total_rodadas,
        week_of_year: race_entry.week_of_year,
        track_id: race_entry.track_id as i32,
        track_name: race_entry.track_name.clone(),
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
        thematic_slot: race_entry.thematic_slot,
    };
    let expected = calculate_expected_event_interest(&ctx);
    Some(calculate_realized_event_interest(
        &expected,
        &ctx,
        Some(player_result.finish_position),
        Some(player_result.grid_position),
        player_result.finish_position == 1,
        player_result.finish_position <= 3 && !player_result.is_dnf,
        player_result.is_dnf,
        is_final_round_decider,
    ))
}

#[tauri::command]
pub async fn simulate_race_weekend(
    app: AppHandle,
    career_id: String,
    race_id: String,
) -> Result<RaceWeekendResult, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter app_data_dir: {e}"))?;

    simulate_race_weekend_in_base_dir(&base_dir, &career_id, &race_id)
}

pub(crate) fn simulate_race_weekend_in_base_dir(
    base_dir: &Path,
    career_id: &str,
    race_id: &str,
) -> Result<RaceWeekendResult, String> {
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

    let mut db = Database::open_existing(&db_path)
        .map_err(|e| format!("Falha ao abrir banco da carreira: {e}"))?;
    let race_entry = calendar_queries::get_calendar_entry_by_id(&db.conn, race_id)
        .map_err(|e| format!("Falha ao buscar corrida: {e}"))?
        .ok_or_else(|| "Corrida nao encontrada.".to_string())?;

    if race_entry.status.as_str() != "Pendente" {
        return Err("A corrida selecionada ja foi simulada.".to_string());
    }

    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;
    let player_race = simulate_category_race(&mut db, &race_entry, true)?;

    // Calcular repercussão pós-corrida e aplicar efeitos (fallback silencioso)
    let post_race_bias = if let Some(realized) =
        compute_post_race_impact(&db.conn, &race_entry, &player_race)
    {
        if let Ok(player) = driver_queries::get_player_driver(&db.conn) {
            let player_result = player_race.race_results.iter().find(|r| r.is_jogador);
            let base_midia_delta = if player_result.map_or(false, |r| r.finish_position == 1) {
                3.0
            } else if player_result.map_or(false, |r| r.finish_position <= 3 && !r.is_dnf) {
                2.0
            } else if player_result.map_or(false, |r| r.finish_position <= 5) {
                1.0
            } else if player_result.map_or(false, |r| r.is_dnf) {
                -2.0
            } else {
                -1.0
            };
            let new_midia = (player.atributos.midia
                + base_midia_delta * realized.media_delta_modifier as f64)
                .clamp(0.0, 100.0);
            let _ = driver_queries::update_driver_midia(&db.conn, &player.id, new_midia);

            let base_mot_delta = if player_result.map_or(false, |r| r.finish_position == 1) {
                4.0
            } else if player_result.map_or(false, |r| r.finish_position <= 3 && !r.is_dnf) {
                2.5
            } else if player_result.map_or(false, |r| r.finish_position <= 5) {
                1.0
            } else if player_result.map_or(false, |r| r.is_dnf) {
                -3.5
            } else {
                -0.5
            };
            let new_motivacao = (player.motivacao
                + base_mot_delta * realized.motivation_delta_modifier as f64)
                .clamp(0.0, 100.0);
            let _ = driver_queries::update_driver_motivation(&db.conn, &player.id, new_motivacao);
        }
        realized.news_importance_bias
    } else {
        0
    };

    append_race_result(
        &career_dir,
        &race_entry.categoria,
        race_entry.rodada,
        &player_race.race_results,
    )?;
    persist_race_news(
        &db.conn,
        &player_race,
        active_season.numero,
        race_entry.rodada,
        &race_entry.categoria,
        post_race_bias,
        race_entry.thematic_slot,
    )?;
    let other_categories = simulate_other_categories(
        &mut db,
        &career_dir,
        &race_entry.categoria,
        race_entry.week_of_year,
        &active_season.id,
        active_season.numero,
    )?;
    update_last_played(&meta_path)?;

    Ok(RaceWeekendResult {
        player_race,
        other_categories,
    })
}

fn simulate_category_race(
    db: &mut Database,
    race_entry: &CalendarEntry,
    advance_player_round: bool,
) -> Result<RaceResult, String> {
    let category = get_category_config(&race_entry.categoria)
        .ok_or_else(|| "Categoria da corrida nao encontrada.".to_string())?;
    let teams = team_queries::get_teams_by_category(&db.conn, &race_entry.categoria)
        .map_err(|e| format!("Falha ao buscar equipes da categoria: {e}"))?;
    let drivers = driver_queries::get_drivers_by_category(&db.conn, &race_entry.categoria)
        .map_err(|e| format!("Falha ao buscar pilotos da categoria: {e}"))?;
    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

    let team_by_driver = build_team_lookup(&teams);
    let sim_drivers: Vec<SimDriver> = drivers
        .iter()
        .filter(|d| d.status != crate::models::enums::DriverStatus::Lesionado)
        .map(|driver| {
            let team = team_by_driver
                .get(&driver.id)
                .ok_or_else(|| format!("Equipe nao encontrada para o piloto {}", driver.nome))?;
            Ok(SimDriver::from_driver_and_team(driver, team))
        })
        .collect::<Result<_, String>>()?;

    if sim_drivers.is_empty() {
        return Err(format!(
            "Nenhum piloto ativo encontrado para a categoria '{}' na rodada {}. \
             A corrida nao foi disputada.",
            race_entry.categoria, race_entry.rodada
        ));
    }

    let ctx = SimulationContext::from_calendar_entry(
        race_entry,
        category.tier,
        race_entry.rodada >= category.corridas_por_temporada as i32,
    );
    let mut rng = rand::thread_rng();
    let result = run_full_race(&sim_drivers, &ctx, category.id == "endurance", &mut rng);
    let next_round = if advance_player_round {
        Some((active_season.rodada_atual + 1).min(category.corridas_por_temporada as i32))
    } else {
        None
    };

    db.transaction(|tx| {
        // 1. Processo de recuperação das lesões já ativas
        crate::evolution::injury::process_injury_recovery(tx, &race_entry.categoria)?;

        // 2. Aplica pontuações normais
        apply_race_result_to_database(tx, &result, &teams)?;

        // 3. Verifica os incidentes recém-gerados e processa possíveis lesões
        let flat_incidents: Vec<_> = result.race_results.iter().flat_map(|r| r.incidents.clone()).collect();
        crate::evolution::injury::process_new_injuries(
            tx, 
            active_season.numero as i32, 
            &race_entry.id, 
            &flat_incidents,
            &mut rng
        )?;

        // 4. Salva o resumo da corrida e avança
        crate::db::queries::races::insert_race_results_batch(tx, &race_entry.id, &result.race_results)?;
        calendar_queries::mark_race_completed(tx, &race_entry.id)?;
        if let Some(round) = next_round {
            season_queries::update_season_rodada(tx, &active_season.id, round)?;
        }

        // 5. Processa hierarquia interna das equipes da categoria
        crate::hierarchy::orders::process_hierarchy_for_category(
            tx,
            &result.race_results,
            &race_entry.categoria,
            race_entry.rodada,
            category.corridas_por_temporada as i32,
            active_season.numero,
        )?;

        // 6. Processa rivalidades por disputa de campeonato (últimas rodadas)
        crate::rivalry::process_championship_rivalry(
            tx,
            &race_entry.categoria,
            race_entry.rodada,
            category.corridas_por_temporada as i32,
            active_season.numero,
        )?;

        Ok(())
    })
    .map_err(|e| format!("Falha ao persistir resultado da corrida: {e}"))?;

    Ok(result)
}

fn simulate_other_categories(
    db: &mut Database,
    career_dir: &Path,
    player_category: &str,
    target_week: i32,
    season_id: &str,
    season_number: i32,
) -> Result<SimultaneousResults, String> {
    let mut categories_simulated = Vec::new();
    let mut total_races_simulated = 0;
    let mut highlights = Vec::new();

    for category in get_all_categories() {
        if category.id == player_category {
            continue;
        }

        // Busca corridas pendentes com week_of_year <= target_week, em ordem cronológica.
        // Categorias especiais (weeks 41–50) são excluídas naturalmente durante o BlocoRegular
        // (target_week ≤ 40) e incluídas automaticamente no BlocoEspecial.
        let pending = calendar_queries::get_pending_races_up_to_week(
            &db.conn,
            season_id,
            category.id,
            target_week,
        )
        .map_err(|e| format!("Falha ao buscar corridas pendentes de {}: {e}", category.id))?;

        if pending.is_empty() {
            continue;
        }

        let mut summaries = Vec::new();
        for entry in pending {
            let result = simulate_category_race(db, &entry, false)?;
            append_race_result(
                career_dir,
                &entry.categoria,
                entry.rodada,
                &result.race_results,
            )?;

            let winner = result
                .race_results
                .iter()
                .find(|driver| driver.finish_position == 1);
            summaries.push(BriefRaceResult {
                race_id: entry.id.clone(),
                track_name: entry.track_name.clone(),
                winner_name: winner
                    .map(|driver| driver.pilot_name.clone())
                    .unwrap_or_default(),
                winner_team: winner
                    .map(|driver| driver.team_name.clone())
                    .unwrap_or_default(),
            });
            total_races_simulated += 1;
        }

        if let Some(last) = summaries.last() {
            highlights.push(SimHighlight {
                headline: format!(
                    "{} vence em {} ({})",
                    last.winner_name, last.track_name, category.nome_curto
                ),
                category: category.id.to_string(),
            });
        }

        let races_simulated = summaries.len() as i32;
        categories_simulated.push(CategorySimResult {
            category_id: category.id.to_string(),
            category_name: category.nome.to_string(),
            races_simulated,
            results: summaries,
        });
    }

    persist_other_category_news(&db.conn, &highlights, season_number)?;

    Ok(SimultaneousResults {
        categories_simulated,
        total_races_simulated,
        highlights,
    })
}

fn apply_race_result_to_database(
    tx: &rusqlite::Transaction<'_>,
    result: &RaceResult,
    teams: &[Team],
) -> Result<(), DbError> {
    for race_driver in &result.race_results {
        let driver = driver_queries::get_driver(tx, &race_driver.pilot_id)?;
        let mut season_stats = driver.stats_temporada.clone();
        let mut career_stats = driver.stats_carreira.clone();

        let previous_races = season_stats.corridas;
        season_stats.pontos += race_driver.points_earned as f64;
        season_stats.vitorias += u32::from(race_driver.finish_position == 1);
        season_stats.podios += u32::from(race_driver.finish_position <= 3);
        season_stats.poles += u32::from(race_driver.pilot_id == result.pole_sitter_id);
        season_stats.corridas += 1;
        season_stats.dnfs += u32::from(race_driver.is_dnf);
        season_stats.posicao_media = recalculate_average_position(
            season_stats.posicao_media,
            previous_races,
            race_driver.finish_position,
        );

        career_stats.pontos_total += race_driver.points_earned as f64;
        career_stats.vitorias += u32::from(race_driver.finish_position == 1);
        career_stats.podios += u32::from(race_driver.finish_position <= 3);
        career_stats.poles += u32::from(race_driver.pilot_id == result.pole_sitter_id);
        career_stats.corridas += 1;
        career_stats.dnfs += u32::from(race_driver.is_dnf);

        let better_result = driver
            .melhor_resultado_temp
            .map(|current| current.min(race_driver.finish_position as u32))
            .or(Some(race_driver.finish_position as u32));

        driver_queries::update_driver_stats(
            tx,
            &driver.id,
            &season_stats,
            &career_stats,
            driver.motivacao,
            better_result,
            driver.temporadas_na_categoria,
            driver.corridas_na_categoria + 1,
            driver.temporadas_motivacao_baixa,
        )?;
    }

    let race_results_by_team = group_results_by_team(result);
    for team in teams {
        let Some(team_results) = race_results_by_team.get(&team.id) else {
            continue;
        };

        let added_points: i32 = team_results.iter().map(|entry| entry.points_earned).sum();
        let added_victories: i32 = team_results
            .iter()
            .filter(|entry| entry.finish_position == 1)
            .count() as i32;
        let added_podiums: i32 = team_results
            .iter()
            .filter(|entry| entry.finish_position <= 3)
            .count() as i32;
        let added_poles: i32 = i32::from(
            team_results
                .iter()
                .any(|entry| entry.pilot_id == result.pole_sitter_id),
        );
        let best_result = team_results
            .iter()
            .map(|entry| entry.finish_position)
            .min()
            .unwrap_or(99);
        let current_best = if team.stats_melhor_resultado <= 0 {
            99
        } else {
            team.stats_melhor_resultado
        };

        team_queries::update_team_season_stats(
            tx,
            &team.id,
            team.stats_vitorias + added_victories,
            team.stats_podios + added_podiums,
            team.stats_poles + added_poles,
            team.stats_pontos + added_points,
            current_best.min(best_result),
        )?;
    }

    Ok(())
}

fn build_team_lookup<'a>(
    teams: &'a [crate::models::team::Team],
) -> HashMap<String, &'a crate::models::team::Team> {
    let mut lookup = HashMap::new();
    for team in teams {
        if let Some(driver_id) = &team.piloto_1_id {
            lookup.insert(driver_id.clone(), team);
        }
        if let Some(driver_id) = &team.piloto_2_id {
            lookup.insert(driver_id.clone(), team);
        }
    }
    lookup
}

fn group_results_by_team<'a>(
    result: &'a RaceResult,
) -> HashMap<String, Vec<&'a crate::simulation::race::RaceDriverResult>> {
    let mut grouped: HashMap<String, Vec<&crate::simulation::race::RaceDriverResult>> =
        HashMap::new();
    for driver_result in &result.race_results {
        grouped
            .entry(driver_result.team_id.clone())
            .or_default()
            .push(driver_result);
    }
    grouped
}

fn recalculate_average_position(
    current_average: f64,
    previous_races: u32,
    finish_position: i32,
) -> f64 {
    let total = current_average * previous_races as f64 + finish_position as f64;
    total / (previous_races as f64 + 1.0)
}

fn update_last_played(meta_path: &Path) -> Result<(), String> {
    let content =
        std::fs::read_to_string(meta_path).map_err(|e| format!("Falha ao ler meta.json: {e}"))?;
    let mut meta: SaveMeta =
        serde_json::from_str(&content).map_err(|e| format!("Falha ao parsear meta.json: {e}"))?;
    meta.last_played = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Falha ao serializar meta.json: {e}"))?;
    std::fs::write(meta_path, json).map_err(|e| format!("Falha ao gravar meta.json: {e}"))
}

fn persist_race_news(
    conn: &rusqlite::Connection,
    race_result: &RaceResult,
    season_number: i32,
    round: i32,
    category_id: &str,
    news_importance_bias: i32,
    thematic_slot: crate::models::enums::ThematicSlot,
) -> Result<(), String> {
    let mut temp_id = temp_news_id_generator();
    let mut timestamp = news_queries::get_latest_news_timestamp(conn)
        .map_err(|e| format!("Falha ao buscar timestamp de noticias: {e}"))?
        + 1;
    let mut items = generate_news_from_race(
        race_result,
        season_number,
        round,
        category_id,
        thematic_slot,
        &mut temp_id,
        &mut timestamp,
    );
    // Em eventos principais (bias >= 2), elevar a primeira notícia de Corrida de Alta → Destaque
    if news_importance_bias >= 2 {
        for item in items.iter_mut() {
            if item.tipo == NewsType::Corrida && item.importancia == NewsImportance::Alta {
                item.importancia = NewsImportance::Destaque;
                break;
            }
        }
    }
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
        .map_err(|e| format!("Falha ao persistir noticias de corrida: {e}"))?;
    news_queries::trim_news(conn, 400).map_err(|e| format!("Falha ao aparar feed: {e}"))?;
    Ok(())
}

fn persist_other_category_news(
    conn: &rusqlite::Connection,
    highlights: &[SimHighlight],
    season_number: i32,
) -> Result<(), String> {
    if highlights.is_empty() {
        return Ok(());
    }

    let mut timestamp = news_queries::get_latest_news_timestamp(conn)
        .map_err(|e| format!("Falha ao buscar timestamp de noticias: {e}"))?
        + 1;
    let mut items = highlights
        .iter()
        .map(|highlight| {
            let category_name = get_category_config(&highlight.category)
                .map(|category| category.nome.to_string())
                .unwrap_or_else(|| highlight.category.clone());
            let item = NewsItem {
                id: String::new(),
                tipo: NewsType::Corrida,
                icone: "🌍".to_string(),
                titulo: highlight.headline.clone(),
                texto: format!("Resultado paralelo em {}.", category_name),
                rodada: None,
                semana_pretemporada: None,
                temporada: season_number,
                categoria_id: Some(highlight.category.clone()),
                categoria_nome: Some(category_name),
                importancia: NewsImportance::Media,
                timestamp,
                driver_id: None,
                team_id: None,
            };
            timestamp += 1;
            item
        })
        .collect::<Vec<_>>();

    persist_generated_news(conn, &mut items)
}

fn temp_news_id_generator() -> impl FnMut() -> String {
    let mut counter = 0;
    move || {
        counter += 1;
        format!("TMP{counter:03}")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::commands::career::{create_career_in_base_dir, CreateCareerInput};
    use crate::db::queries::calendar::get_next_race;

    #[test]
    fn test_simulate_race_weekend_updates_state() {
        let base_dir = unique_test_dir("simulate_weekend");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        let result = simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("simulate");

        assert_eq!(result.player_race.race_results.len(), 12);
        assert!(result.other_categories.total_races_simulated > 0);

        let updated_db = Database::open_existing(&db_path).expect("reopen db");
        let season_after = season_queries::get_active_season(&updated_db.conn)
            .expect("season after")
            .expect("active season after");
        assert_eq!(season_after.rodada_atual, 2);

        let completed = calendar_queries::get_calendar_entry_by_id(&updated_db.conn, &next_race.id)
            .expect("race by id")
            .expect("calendar entry");
        assert_eq!(completed.status.as_str(), "Concluida");

        let driver = driver_queries::get_player_driver(&updated_db.conn).expect("player driver");
        assert!(driver.stats_temporada.corridas >= 1);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_simulate_race_weekend_rejects_completed_race() {
        let base_dir = unique_test_dir("simulate_completed");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("first simulation");
        let error = simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect_err("second simulation should fail");

        assert!(error.contains("ja foi simulada"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_simulate_race_weekend_persists_news() {
        let base_dir = unique_test_dir("simulate_news");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("simulate");

        let updated_db = Database::open_existing(&db_path).expect("reopen db");
        let items = news_queries::get_news_by_season(&updated_db.conn, 1, 20).expect("news");
        assert!(!items.is_empty());
        assert!(items
            .iter()
            .any(|item| item.tipo == crate::news::NewsType::Corrida));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_simulate_race_weekend_returns_other_results() {
        let base_dir = unique_test_dir("simulate_other_results");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");

        let result = simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("simulate");

        assert_eq!(result.player_race.track_name, next_race.track_name);
        assert!(result.other_categories.total_races_simulated > 0);
        assert!(result
            .other_categories
            .categories_simulated
            .iter()
            .all(|category| category.category_id != "mazda_rookie"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_all_categories_complete_after_last_race() {
        let base_dir = unique_test_dir("simulate_all_categories");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");

        loop {
            let db = Database::open_existing(&db_path).expect("db");
            let season = season_queries::get_active_season(&db.conn)
                .expect("season")
                .expect("active season");
            let Some(next_race) =
                get_next_race(&db.conn, &season.id, "mazda_rookie").expect("next race")
            else {
                break;
            };
            drop(db);

            simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
                .expect("simulate round");
        }

        let updated_db = Database::open_existing(&db_path).expect("updated db");
        let season = season_queries::get_active_season(&updated_db.conn)
            .expect("season")
            .expect("active season");
        let pending =
            calendar_queries::get_pending_races(&updated_db.conn, &season.id).expect("pending");

        assert!(pending.is_empty(), "all categories should be complete");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_stats_updated_for_other_categories() {
        let base_dir = unique_test_dir("simulate_other_stats");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let db_path = config.saves_dir().join("career_001").join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");
        drop(db);

        simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("simulate");

        let updated_db = Database::open_existing(&db_path).expect("updated db");
        let gt3_driver = driver_queries::get_drivers_by_category(&updated_db.conn, "gt3")
            .expect("gt3 drivers")
            .into_iter()
            .next()
            .expect("at least one gt3 driver");

        assert!(
            gt3_driver.stats_temporada.corridas > 0,
            "other categories should update driver stats"
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn test_race_history_saved_for_other_categories() {
        let base_dir = unique_test_dir("simulate_other_history");
        fs::create_dir_all(&base_dir).expect("base dir");

        create_career_in_base_dir(
            &base_dir,
            CreateCareerInput {
                player_name: "Joao Silva".to_string(),
                player_nationality: "br".to_string(),
                player_age: Some(20),
                category: "mazda_rookie".to_string(),
                team_index: 0,
                difficulty: "medio".to_string(),
            },
        )
        .expect("career");

        let config = AppConfig::load_or_default(&base_dir);
        let career_dir = config.saves_dir().join("career_001");
        let db_path = career_dir.join("career.db");
        let db = Database::open_existing(&db_path).expect("db");
        let season = season_queries::get_active_season(&db.conn)
            .expect("season")
            .expect("active season");
        let next_race = get_next_race(&db.conn, &season.id, "mazda_rookie")
            .expect("next race")
            .expect("pending race");
        drop(db);

        simulate_race_weekend_in_base_dir(&base_dir, "career_001", &next_race.id)
            .expect("simulate");

        let history_path = career_dir.join("race_results.json");
        let history =
            fs::read_to_string(history_path).expect("race history should be written to disk");

        assert!(history.contains("\"mazda_rookie\""));
        assert!(history.contains("\"gt3\""));

        let _ = fs::remove_dir_all(base_dir);
    }

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("iracerapp_{label}_{nanos}"))
    }
}
