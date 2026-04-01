use std::collections::{HashMap, HashSet};
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
use crate::db::queries::seasons as season_queries;
use crate::db::queries::standings as standings_queries;
use crate::db::queries::standings::ChampionshipContext;
use crate::db::queries::teams as team_queries;
use crate::event_interest::{
    calculate_expected_event_interest, calculate_realized_event_interest, EventInterestContext,
    InterestTier, RealizedEventInterest,
};
use crate::models::injury::Injury;
use crate::models::season::Season;
use crate::simulation::batch::{
    BriefRaceResult, CategorySimResult, SimHighlight, SimultaneousResults,
};
use crate::simulation::catalog::IncidentCatalog;
use crate::simulation::context::{SimDriver, SimulationContext};
use crate::simulation::engine::run_full_race;
use crate::simulation::incidents::IncidentResult;
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
    let champ = standings_queries::get_championship_context(conn, &race_entry.categoria).unwrap_or(
        ChampionshipContext {
            player_position: 0,
            gap_to_leader: 0,
        },
    );
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

    let active_season = season_queries::get_active_season(&db.conn)
        .map_err(|e| format!("Falha ao buscar temporada ativa: {e}"))?
        .ok_or_else(|| "Temporada ativa nao encontrada.".to_string())?;

    // Validar que a corrida pertence à temporada atual e está pendente
    if race_entry.season_id != active_season.id {
        return Err("A corrida selecionada nao pertence a temporada atual.".to_string());
    }
    if race_entry.status.as_str() != "Pendente" {
        return Err("A corrida selecionada ja foi concluida ou simulada.".to_string());
    }

    let (player_race, player_new_injuries) = simulate_category_race(&mut db, &race_entry, true)?;

    // Calcular repercussão pós-corrida e aplicar efeitos (fallback silencioso)
    // ID do jogador extraído para exclusão no bloco de world-facing media impact
    let excluded_driver_id = player_race
        .race_results
        .iter()
        .find(|r| r.is_jogador)
        .map(|r| r.pilot_id.clone())
        .unwrap_or_default();

    let (post_race_bias, ai_media_impacts, interest_tier) = if let Some(realized) =
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

        // World-facing media impact — pilotos AI relevantes.
        // Dependência semântica intencional: sem `realized`, este bloco não roda.
        // `excluded_driver_id` (jogador) omitido para evitar dupla aplicação com o pipeline player-facing.
        let main_incident_pilot: Option<String> = player_race
            .notable_incident_pilot_ids
            .iter()
            .find(|id| id.as_str() != excluded_driver_id.as_str())
            .cloned();

        // P2 e P3 elegíveis — winner explicitamente excluído (mutuidade Win/Podium)
        let podium_pilot_ids: Vec<&str> = player_race
            .race_results
            .iter()
            .filter(|r| {
                r.finish_position >= 2
                    && r.finish_position <= 3
                    && !r.is_dnf
                    && r.pilot_id != player_race.winner_id
            })
            .map(|r| r.pilot_id.as_str())
            .collect();

        let race_ctx = crate::event_interest::RaceEventContext {
            winner_id: &player_race.winner_id,
            pole_sitter_id: &player_race.pole_sitter_id,
            podium_ids: &podium_pilot_ids,
            main_incident_pilot_id: main_incident_pilot.as_deref(),
            excluded_driver_id: &excluded_driver_id,
        };

        // `player_new_injuries` contém lesões novas geradas na corrida (pode incluir pilotos AI).
        let impacts = crate::event_interest::compute_public_media_impacts(
            &race_ctx,
            &player_new_injuries,
            &realized,
        );

        (
            realized.news_importance_bias,
            impacts,
            realized.final_tier.clone(),
        )
    } else {
        (0, vec![], InterestTier::Baixo)
    };

    for impact in &ai_media_impacts {
        driver_queries::update_driver_midia_delta(&db.conn, &impact.driver_id, impact.delta)
            .map_err(|e| {
                format!(
                    "Falha ao aplicar impacto de mídia para '{}': {e}",
                    impact.driver_id
                )
            })?;
    }

    append_race_result(
        &career_dir,
        &race_entry.categoria,
        race_entry.rodada,
        &player_race.race_results,
    )?;
    persist_race_news(
        &db.conn,
        &player_race,
        &active_season,
        race_entry.rodada,
        &race_entry.categoria,
        post_race_bias,
        race_entry.thematic_slot,
        &interest_tier,
        &player_race
            .race_results
            .iter()
            .flat_map(|r| r.incidents.clone())
            .collect::<Vec<_>>(),
        &player_new_injuries,
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
) -> Result<(RaceResult, Vec<Injury>), String> {
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
    let catalog = IncidentCatalog::load(&db.conn).unwrap_or_else(|_| IncidentCatalog::empty());
    let result = run_full_race(
        &sim_drivers,
        &ctx,
        category.id == "endurance",
        &catalog,
        &mut rng,
    );
    let next_round = if advance_player_round {
        Some((active_season.rodada_atual + 1).min(category.corridas_por_temporada as i32))
    } else {
        None
    };

    let mut new_injuries_out: Vec<Injury> = Vec::new();
    db.transaction(|tx| {
        // 1. Processo de recuperação das lesões já ativas
        crate::evolution::injury::process_injury_recovery(tx, &race_entry.categoria)?;

        // 2. Aplica pontuações normais
        apply_race_result_to_database(tx, &result, &teams)?;

        // 3. Verifica os incidentes recém-gerados e processa possíveis lesões
        let flat_incidents: Vec<_> = result
            .race_results
            .iter()
            .flat_map(|r| r.incidents.clone())
            .collect();
        let new_injuries = crate::evolution::injury::process_new_injuries(
            tx,
            active_season.numero as i32,
            &race_entry.id,
            &flat_incidents,
            &mut rng,
        )?;
        new_injuries_out = new_injuries;

        // 4. Salva o resumo da corrida e avança
        crate::db::queries::races::insert_race_results_batch(
            tx,
            &race_entry.id,
            &result.race_results,
        )?;
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

        // 7. Processa rivalidades geradas por colisões bilaterais (fatos da corrida)
        crate::rivalry::process_collisions_rivalry(
            tx,
            &flat_incidents,
            &race_entry.categoria,
            race_entry.rodada,
            active_season.numero,
        )?;

        Ok(())
    })
    .map_err(|e| format!("Falha ao persistir resultado da corrida: {e}"))?;

    Ok((result, new_injuries_out))
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
            let (result, _) = simulate_category_race(db, &entry, false)?;
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
        let mut driver = driver_queries::get_driver(tx, &race_driver.pilot_id)?;
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

        driver.stats_temporada = season_stats;
        driver.stats_carreira = career_stats;
        driver.melhor_resultado_temp = better_result;
        driver.corridas_na_categoria += 1;
        driver.ultimos_resultados =
            append_recent_result(&driver.ultimos_resultados, race_driver.finish_position, race_driver.is_dnf);

        driver_queries::update_driver(tx, &driver)?;
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

fn append_recent_result(
    existing: &serde_json::Value,
    finish_position: i32,
    is_dnf: bool,
) -> serde_json::Value {
    let mut results: Vec<serde_json::Value> = existing
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.as_object().cloned())
        .map(serde_json::Value::Object)
        .collect();

    results.push(serde_json::json!({
        "position": finish_position,
        "is_dnf": is_dnf,
        "has_fastest_lap": false,
        "grid_position": 0,
        "positions_gained": 0
    }));

    if results.len() > 5 {
        let keep_from = results.len() - 5;
        results.drain(0..keep_from);
    }

    serde_json::Value::Array(results)
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

fn race_news_importance(bias: i32, tier: &InterestTier, finish_position: i32) -> crate::news::NewsImportance {
    use crate::news::NewsImportance;
    use crate::event_interest::InterestTier;
    let tier_score = match tier {
        InterestTier::Baixo => 0,
        InterestTier::Moderado => 1,
        InterestTier::Alto => 2,
        InterestTier::MuitoAlto => 3,
        InterestTier::EventoPrincipal => 4,
    };
    let position_bonus = if finish_position == 1 { 2 } else if finish_position <= 3 { 1 } else { 0 };
    let total = bias + tier_score + position_bonus;
    let importance = if total >= 5 { NewsImportance::Destaque }
        else if total >= 3 { NewsImportance::Alta }
        else if total >= 1 { NewsImportance::Media }
        else { NewsImportance::Baixa };
    // Vitória sempre dispara pelo menos Alta para que detect_race_trigger acione LeaderWon/ShockWin/etc.
    if finish_position == 1 && matches!(importance, NewsImportance::Baixa | NewsImportance::Media) {
        NewsImportance::Alta
    } else {
        importance
    }
}

fn persist_race_news(
    conn: &rusqlite::Connection,
    race_result: &RaceResult,
    active_season: &Season,
    round: i32,
    category_id: &str,
    news_importance_bias: i32,
    _thematic_slot: crate::models::enums::ThematicSlot,
    interest_tier: &InterestTier,
    flat_incidents: &[IncidentResult],
    new_injuries: &[Injury],
) -> Result<(), String> {
    use crate::db::queries::news as news_queries;
    use crate::generators::ids::{next_id, IdType};
    use crate::news::{NewsImportance, NewsItem, NewsType};

    use crate::db::queries::drivers as driver_queries;

    let now = chrono::Local::now().timestamp();
    let mut items: Vec<NewsItem> = Vec::new();

    // 1. Corrida — notícia sobre o VENCEDOR da corrida (não o jogador)
    // O sistema editorial foi projetado para compor histórias sobre quem ganhou.
    // A importância Alta garante que detect_race_trigger gera algo além do FallbackRaceResult.
    {
        let winner_id = &race_result.winner_id;
        let winner_name = driver_queries::get_driver(conn, winner_id)
            .map(|d| d.nome)
            .unwrap_or_else(|_| winner_id.clone());
        let importance = race_news_importance(news_importance_bias, interest_tier, 1);
        
        let total_rodadas = crate::constants::categories::get_category_config(category_id)
            .map(|c| c.corridas_por_temporada as i32)
            .unwrap_or(round);
        let fallback_races = total_rodadas - round;

        let (titulo, texto) = if fallback_races == 0 {
            (
                format!("{} vence a corrida final da temporada em {}", winner_name, race_result.track_name),
                format!("{} cruzou a linha de chegada em primeiro lugar na última rodada da temporada {}.", winner_name, active_season.numero),
            )
        } else if fallback_races <= 2 {
            (
                format!("{} conquista vitória crucial na reta final em {}", winner_name, race_result.track_name),
                format!("Com a temporada se aproximando do fim, {} garantiu o primeiro lugar na rodada {}.", winner_name, round),
            )
        } else {
            (
                format!("{} vence em {}", winner_name, race_result.track_name),
                format!("{} cruzou a linha de chegada em primeiro lugar na rodada {} da temporada {}.", winner_name, round, active_season.numero),
            )
        };

        let winner_team = race_result.race_results.iter()
            .find(|r| &r.pilot_id == winner_id)
            .map(|r| r.team_id.clone());
        let id = next_id(conn, IdType::News)
            .map_err(|e| format!("next_id news: {e:?}"))?;
        items.push(NewsItem {
            id,
            tipo: NewsType::Corrida,
            icone: NewsType::Corrida.icone().to_string(),
            titulo,
            texto,
            rodada: Some(round),
            semana_pretemporada: None,
            temporada: active_season.numero,
            categoria_id: Some(category_id.to_string()),
            categoria_nome: None,
            importancia: importance,
            timestamp: now,
            driver_id: Some(winner_id.clone()),
            driver_id_secondary: None,
            team_id: winner_team.map(Some).unwrap_or(None),
        });

        if fallback_races == 0 {
            if let Ok(mut standings) = crate::db::queries::race_history::get_category_standings(conn, &active_season.id, category_id) {
                if let Some(champion) = standings.into_iter().next() {
                    let champ_id = next_id(conn, IdType::News).unwrap_or_else(|_| "news_champ".to_string());
                    items.push(NewsItem {
                        id: champ_id,
                        tipo: NewsType::FramingSazonal,
                        icone: NewsType::FramingSazonal.icone().to_string(),
                        titulo: format!("{} é o grande campeão da temporada {}!", champion.pilot_name, active_season.numero),
                        texto: format!("Após {} rodadas intensas, {} conquista o título da categoria. Uma temporada inesquecível chegou ao fim.", total_rodadas, champion.pilot_name),
                        rodada: Some(round),
                        semana_pretemporada: None,
                        temporada: active_season.numero,
                        categoria_id: Some(category_id.to_string()),
                        categoria_nome: None,
                        importancia: NewsImportance::Destaque,
                        timestamp: now,
                        driver_id: Some(champion.pilot_id),
                        driver_id_secondary: None,
                        team_id: None,
                    });
                }
            }
        }
    }

    // 2. Incidentes — um item por DNF + incidentes de hint >= 2 não-DNF
    // Evita duplicatas: se um piloto já tem DNF, não gera segundo item por hint >= 2 dele.
    let mut seen_incident_pilots: HashSet<String> = HashSet::new();
    let mut noticiable: Vec<&IncidentResult> = flat_incidents
        .iter()
        .filter(|i| i.is_dnf || i.narrative_importance_hint >= 2)
        .collect();
    // DNFs primeiro, depois por hint decrescente
    noticiable.sort_by_key(|i| (std::cmp::Reverse(i.is_dnf as u8), std::cmp::Reverse(i.narrative_importance_hint)));

    for inc in noticiable {
        if !seen_incident_pilots.insert(inc.pilot_id.clone()) {
            continue; // piloto já tem notícia nesta rodada
        }
        let driver_name = driver_queries::get_driver(conn, &inc.pilot_id)
            .map(|d| d.nome)
            .unwrap_or_else(|_| inc.pilot_id.clone());
        let id = next_id(conn, IdType::News)
            .map_err(|e| format!("next_id incident: {e:?}"))?;
        let titulo = if inc.is_dnf {
            format!("{} abandona a corrida após incidente", driver_name)
        } else {
            format!("{} envolvido em incidente durante a prova", driver_name)
        };
        let texto = inc.description.clone();
        let inc_importance = if inc.narrative_importance_hint >= 3 {
            NewsImportance::Destaque
        } else {
            NewsImportance::Alta
        };
        items.push(NewsItem {
            id,
            tipo: NewsType::Incidente,
            icone: NewsType::Incidente.icone().to_string(),
            titulo,
            texto,
            rodada: Some(round),
            semana_pretemporada: None,
            temporada: active_season.numero,
            categoria_id: Some(category_id.to_string()),
            categoria_nome: None,
            importancia: inc_importance,
            timestamp: now,
            driver_id: Some(inc.pilot_id.clone()),
            driver_id_secondary: inc.linked_pilot_id.clone(),
            team_id: None,
        });
    }

    // 3. Lesão — uma notícia por piloto lesionado
    for injury in new_injuries {
        let driver_name = driver_queries::get_driver(conn, &injury.pilot_id)
            .map(|d| d.nome)
            .unwrap_or_else(|_| injury.pilot_id.clone());
        let id = next_id(conn, IdType::News)
            .map_err(|e| format!("next_id injury: {e:?}"))?;
        let titulo = "desfalque confirmado".to_string();
        let texto = format!(
            "{} está fora da próxima etapa após lesão confirmada. Situação será reavaliada nos próximos dias.",
            driver_name
        );
        items.push(NewsItem {
            id,
            tipo: NewsType::Lesao,
            icone: NewsType::Lesao.icone().to_string(),
            titulo,
            texto,
            rodada: Some(round),
            semana_pretemporada: None,
            temporada: active_season.numero,
            categoria_id: Some(category_id.to_string()),
            categoria_nome: None,
            importancia: NewsImportance::Alta,
            timestamp: now,
            driver_id: Some(injury.pilot_id.clone()),
            driver_id_secondary: None,
            team_id: None,
        });
    }

    if !items.is_empty() {
        news_queries::insert_news_batch(conn, &items)
            .map_err(|e| format!("insert_news_batch: {e:?}"))?;
    }

    Ok(())
}

fn persist_other_category_news(
    _conn: &rusqlite::Connection,
    _highlights: &[SimHighlight],
    _season_number: i32,
) -> Result<(), String> {
    Ok(())
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

        assert!(
            error.contains("ja foi concluida ou simulada"),
            "Erro inesperado: {}",
            error
        );

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
