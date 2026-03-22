use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Local;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::calendar::generate_all_calendars_with_id_factory;
use crate::constants::categories::{get_all_categories, get_category_config};
use crate::db::queries::calendar as calendar_queries;
use crate::db::queries::contracts as contract_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::seasons as season_queries;
use crate::db::queries::teams as team_queries;
use crate::evolution::decline::apply_age_decline;
use crate::evolution::growth::{calculate_growth, GrowthReport, SeasonStats};
use crate::evolution::motivation::{adjust_end_of_season_motivation, MotivationReport};
use crate::evolution::retirement::{check_retirement, process_retirement};
use crate::evolution::rookies::{classify_rookie, generate_rookies};
use crate::generators::ids::{next_id, next_ids, IdType};
use crate::market::preseason::{initialize_preseason, save_preseason_plan};
use crate::models::driver::Driver;
use crate::models::enums::DriverStatus;
use crate::models::season::Season;
use crate::promotion::pipeline::run_promotion_relegation;
use crate::promotion::PromotionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndOfSeasonResult {
    pub growth_reports: Vec<GrowthReport>,
    pub motivation_reports: Vec<MotivationReport>,
    pub retirements: Vec<RetirementInfo>,
    pub rookies_generated: Vec<RookieInfo>,
    pub new_season_id: String,
    pub new_year: i32,
    pub licenses_earned: Vec<LicenseEarned>,
    pub promotion_result: PromotionResult,
    pub preseason_initialized: bool,
    pub preseason_total_weeks: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementInfo {
    pub driver_id: String,
    pub driver_name: String,
    pub age: i32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RookieInfo {
    pub driver_id: String,
    pub driver_name: String,
    pub nationality: String,
    pub age: i32,
    pub skill: u8,
    pub tipo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseEarned {
    pub driver_id: String,
    pub driver_name: String,
    pub license_level: u8,
    pub category: String,
}

#[derive(Debug, Clone)]
struct StandingEntry {
    driver_id: String,
    driver_name: String,
    category: String,
    team_id: Option<String>,
    position: i32,
    total_drivers: i32,
    stats: SeasonStats,
}

pub fn run_end_of_season(
    conn: &Connection,
    season: &Season,
    save_path: &Path,
) -> Result<EndOfSeasonResult, String> {
    let mut rng = StdRng::seed_from_u64(((season.numero as u64) << 32) | season.ano as u64);
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao buscar equipes: {e}"))?;
    let teams_by_id: HashMap<String, crate::models::team::Team> = teams
        .into_iter()
        .map(|team| (team.id.clone(), team))
        .collect();
    let active_contracts = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao buscar contratos ativos: {e}"))?;
    let contracts_by_driver: HashMap<String, crate::models::contract::Contract> = active_contracts
        .into_iter()
        .map(|contract| (contract.piloto_id.clone(), contract))
        .collect();

    let standings = build_and_persist_standings(conn, season, &contracts_by_driver)?;
    let standings_by_driver: HashMap<String, StandingEntry> = standings
        .iter()
        .cloned()
        .map(|entry| (entry.driver_id.clone(), entry))
        .collect();

    let licenses_earned = persist_licenses(conn, &standings, &standings_by_driver)
        .map_err(|e| format!("Falha ao persistir licencas: {e}"))?;

    season_queries::finalize_season(conn, &season.id)
        .map_err(|e| format!("Falha ao finalizar temporada: {e}"))?;

    let mut all_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao buscar pilotos: {e}"))?;
    let mut existing_names: HashSet<String> = all_drivers
        .iter()
        .map(|driver| driver.nome.clone())
        .collect();
    let mut growth_reports = Vec::new();
    let mut motivation_reports = Vec::new();
    let mut retirements = Vec::new();

    for driver in &mut all_drivers {
        if driver.status != DriverStatus::Ativo {
            continue;
        }

        let standing = standings_by_driver.get(&driver.id).cloned();
        if let Some(standing) = standing {
            let team_car_performance = contracts_by_driver
                .get(&driver.id)
                .and_then(|contract| teams_by_id.get(&contract.equipe_id))
                .map(|team| team.car_performance)
                .unwrap_or(0.0);

            let category_tier = get_category_config(&standing.category)
                .map(|config| config.tier)
                .unwrap_or(0);
            let growth_report = calculate_growth(
                driver,
                &standing.stats,
                team_car_performance,
                category_tier,
                &mut rng,
            );
            if !growth_report.changes.is_empty() {
                growth_reports.push(growth_report);
            }

            let _decline_changes = apply_age_decline(driver, &mut rng);
            let seasons_in_category = driver.temporadas_na_categoria as i32 + 1;
            let motivation_report = adjust_end_of_season_motivation(
                driver,
                &standing.stats,
                standing.position == 1,
                false,
                false,
                false,
                false,
                seasons_in_category,
                &mut rng,
            );
            motivation_reports.push(motivation_report);

            driver.temporadas_na_categoria += 1;
            driver.corridas_na_categoria += standing.stats.corridas.max(0) as u32;
        }

        driver.idade += 1;
        if driver.motivacao < 20.0 {
            driver.temporadas_motivacao_baixa += 1;
        } else {
            driver.temporadas_motivacao_baixa = 0;
        }

        driver.accumulate_career_stats();
        if standings_by_driver
            .get(&driver.id)
            .is_some_and(|standing| standing.position == 1)
        {
            driver.stats_carreira.titulos += 1;
        }

        let retirement = check_retirement(
            driver,
            driver.temporadas_motivacao_baixa as i32,
            false,
            &mut rng,
        );
        if retirement.should_retire {
            let reason = retirement
                .reason
                .clone()
                .unwrap_or_else(|| "Aposentadoria".to_string());
            persist_retired_driver(conn, driver, season, &reason)
                .map_err(|e| format!("Falha ao registrar aposentadoria: {e}"))?;
            process_retirement(driver);
            driver.categoria_atual = None;
            retirements.push(RetirementInfo {
                driver_id: driver.id.clone(),
                driver_name: driver.nome.clone(),
                age: driver.idade as i32,
                reason,
            });
        }
        driver_queries::update_driver(conn, driver)
            .map_err(|e| format!("Falha ao salvar piloto '{}': {e}", driver.nome))?;
    }

    let rookie_count = rng.gen_range(2..=4);
    let mut rookies = generate_rookies(rookie_count, &mut existing_names, &mut rng);
    let rookie_ids = next_ids(conn, IdType::Driver, rookie_count as u32)
        .map_err(|e| format!("Falha ao gerar IDs de rookies: {e}"))?;
    let mut rookies_generated = Vec::new();
    for (driver, rookie_id) in rookies.iter_mut().zip(rookie_ids.into_iter()) {
        driver.id = rookie_id.clone();
        driver_queries::insert_driver(conn, driver)
            .map_err(|e| format!("Falha ao inserir rookie '{}': {e}", driver.nome))?;
        rookies_generated.push(RookieInfo {
            driver_id: rookie_id,
            driver_name: driver.nome.clone(),
            nationality: driver.nacionalidade.clone(),
            age: driver.idade as i32,
            skill: driver.atributos.skill.round().clamp(0.0, 100.0) as u8,
            tipo: classify_rookie(driver.atributos.skill.round() as u8).to_string(),
        });
    }

    let promotion_result = run_promotion_relegation(conn, season.numero, &mut rng)
        .map_err(|e| format!("Erro na promocao/rebaixamento: {e}"))?;

    let new_season_id = next_id(conn, IdType::Season)
        .map_err(|e| format!("Falha ao gerar ID da nova temporada: {e}"))?;
    let new_year = season.ano + 1;
    let new_season = Season::new(new_season_id.clone(), season.numero + 1, new_year);
    season_queries::insert_season(conn, &new_season)
        .map_err(|e| format!("Falha ao inserir nova temporada: {e}"))?;

    let mut drivers_after_rotation = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao recarregar pilotos apos a nova temporada: {e}"))?;
    for driver in &mut drivers_after_rotation {
        driver.reset_season_stats();
        driver_queries::update_driver(conn, driver)
            .map_err(|e| format!("Falha ao resetar stats do piloto '{}': {e}", driver.nome))?;
    }

    let teams_after_rotation = team_queries::get_all_teams(conn)
        .map_err(|e| format!("Falha ao recarregar equipes: {e}"))?;
    for team in &teams_after_rotation {
        team_queries::reset_team_season_stats(conn, &team.id)
            .map_err(|e| format!("Falha ao resetar stats da equipe '{}': {e}", team.id))?;
        conn.execute(
            "UPDATE teams SET temporada_atual = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            rusqlite::params![season.numero + 1, &team.id],
        )
        .map_err(|e| format!("Falha ao atualizar temporada da equipe '{}': {e}", team.id))?;
    }

    let total_new_races: u32 = get_all_categories()
        .iter()
        .map(|category| category.corridas_por_temporada as u32)
        .sum();
    let race_ids = next_ids(conn, IdType::Race, total_new_races)
        .map_err(|e| format!("Falha ao gerar IDs do calendario: {e}"))?;
    let mut race_ids_iter = race_ids.into_iter();
    let calendars = generate_all_calendars_with_id_factory(
        &new_season_id,
        &mut || race_ids_iter.next().expect("calendar race id"),
        &mut rng,
    )?;
    let all_entries: Vec<_> = calendars
        .values()
        .flat_map(|entries| entries.iter().cloned())
        .collect();
    calendar_queries::insert_calendar_entries(conn, &all_entries)
        .map_err(|e| format!("Falha ao inserir calendario da nova temporada: {e}"))?;

    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'current_season'",
        rusqlite::params![(season.numero + 1).to_string()],
    )
    .map_err(|e| format!("Falha ao atualizar meta current_season: {e}"))?;
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'current_year'",
        rusqlite::params![new_year.to_string()],
    )
    .map_err(|e| format!("Falha ao atualizar meta current_year: {e}"))?;

    let preseason_plan = initialize_preseason(conn, new_season.numero, &mut rng)
        .map_err(|e| format!("Erro ao inicializar pre-temporada: {e}"))?;
    save_preseason_plan(save_path, &preseason_plan)
        .map_err(|e| format!("Erro ao salvar plano da pre-temporada: {e}"))?;

    Ok(EndOfSeasonResult {
        growth_reports,
        motivation_reports,
        retirements,
        rookies_generated,
        new_season_id,
        new_year,
        licenses_earned,
        promotion_result,
        preseason_initialized: true,
        preseason_total_weeks: preseason_plan.state.total_weeks,
    })
}

fn build_and_persist_standings(
    conn: &Connection,
    season: &Season,
    contracts_by_driver: &HashMap<String, crate::models::contract::Contract>,
) -> Result<Vec<StandingEntry>, String> {
    conn.execute(
        "DELETE FROM standings WHERE temporada_id = ?1",
        rusqlite::params![&season.id],
    )
    .map_err(|e| format!("Falha ao limpar standings existentes: {e}"))?;

    let mut all_standings = Vec::new();
    for category in get_all_categories() {
        let mut drivers = driver_queries::get_drivers_by_category(conn, category.id)
            .map_err(|e| format!("Falha ao buscar pilotos de '{}': {e}", category.id))?;
        if drivers.is_empty() {
            continue;
        }

        drivers.sort_by(|a, b| {
            b.stats_temporada
                .pontos
                .total_cmp(&a.stats_temporada.pontos)
                .then_with(|| b.stats_temporada.vitorias.cmp(&a.stats_temporada.vitorias))
                .then_with(|| b.stats_temporada.podios.cmp(&a.stats_temporada.podios))
                .then_with(|| a.nome.cmp(&b.nome))
        });

        let total_drivers = drivers.len() as i32;
        for (index, driver) in drivers.into_iter().enumerate() {
            let team_id = contracts_by_driver
                .get(&driver.id)
                .map(|contract| contract.equipe_id.clone());
            let standing = StandingEntry {
                driver_id: driver.id.clone(),
                driver_name: driver.nome.clone(),
                category: category.id.to_string(),
                team_id: team_id.clone(),
                position: index as i32 + 1,
                total_drivers,
                stats: SeasonStats {
                    posicao_campeonato: index as i32 + 1,
                    total_pilotos: total_drivers,
                    pontos: driver.stats_temporada.pontos.round() as i32,
                    vitorias: driver.stats_temporada.vitorias as i32,
                    podios: driver.stats_temporada.podios as i32,
                    corridas: driver.stats_temporada.corridas as i32,
                    dnfs: driver.stats_temporada.dnfs as i32,
                },
            };

            if let Some(team_id) = &team_id {
                conn.execute(
                    "INSERT INTO standings (
                        temporada_id, piloto_id, equipe_id, categoria, posicao, pontos, vitorias, podios, poles, corridas
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        &season.id,
                        &standing.driver_id,
                        team_id,
                        &standing.category,
                        standing.position,
                        standing.stats.pontos as f64,
                        standing.stats.vitorias,
                        standing.stats.podios,
                        0,
                        standing.stats.corridas,
                    ],
                )
                .map_err(|e| format!("Falha ao persistir standings: {e}"))?;
            }

            all_standings.push(standing);
        }
    }

    Ok(all_standings)
}

fn persist_licenses(
    conn: &Connection,
    standings: &[StandingEntry],
    standings_by_driver: &HashMap<String, StandingEntry>,
) -> Result<Vec<LicenseEarned>, rusqlite::Error> {
    let mut grouped: HashMap<&str, Vec<&StandingEntry>> = HashMap::new();
    for standing in standings {
        grouped
            .entry(&standing.category)
            .or_default()
            .push(standing);
    }

    let timestamp = timestamp_now();
    let mut licenses_earned = Vec::new();
    for (category, entries) in grouped {
        let license_level = get_category_config(category)
            .map(|config| config.tier)
            .unwrap_or(0);
        let cutoff = (entries.len() + 1) / 2;
        for standing in entries.into_iter().take(cutoff) {
            let seasons_in_category = standings_by_driver
                .get(&standing.driver_id)
                .map(|_| standing.stats.corridas)
                .unwrap_or(0);
            conn.execute(
                "INSERT INTO licenses (piloto_id, nivel, categoria_origem, data_obtencao, temporadas_na_categoria)
                 SELECT ?1, ?2, ?3, ?4, ?5
                 WHERE NOT EXISTS (
                     SELECT 1 FROM licenses WHERE piloto_id = ?1 AND nivel = ?2 AND categoria_origem = ?3
                 )",
                rusqlite::params![
                    &standing.driver_id,
                    license_level.to_string(),
                    category,
                    &timestamp,
                    seasons_in_category,
                ],
            )?;

            licenses_earned.push(LicenseEarned {
                driver_id: standing.driver_id.clone(),
                driver_name: standing.driver_name.clone(),
                license_level,
                category: category.to_string(),
            });
        }
    }

    Ok(licenses_earned)
}

fn persist_retired_driver(
    conn: &Connection,
    driver: &Driver,
    season: &Season,
    reason: &str,
) -> Result<(), rusqlite::Error> {
    let stats_json =
        serde_json::to_string(&driver.stats_carreira).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO retired (
            piloto_id, nome, temporada_aposentadoria, categoria_final, estatisticas, motivo
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &driver.id,
            &driver.nome,
            season.numero.to_string(),
            driver.categoria_atual.clone().unwrap_or_default(),
            stats_json,
            reason,
        ],
    )?;
    Ok(())
}

fn timestamp_now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::calendar::generate_calendar_for_category;
    use crate::constants::teams::get_team_templates;
    use crate::db::migrations;
    use crate::models::contract::Contract;
    use crate::models::driver::Driver;
    use crate::models::enums::TeamRole;
    use crate::models::team::Team;

    #[test]
    fn test_end_of_season_increments_year() {
        let (conn, season) = setup_pipeline_fixture();
        let save_path = unique_test_dir("eos_year");

        let result = run_end_of_season(&conn, &season, &save_path).expect("pipeline should run");

        assert_eq!(result.new_year, season.ano + 1);
        assert!(result.promotion_result.movements.is_empty());
        assert!(result.preseason_initialized);
        assert!(result.preseason_total_weeks >= 3);
        let meta_year: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'current_year'",
                [],
                |row| row.get(0),
            )
            .expect("meta current year");
        assert_eq!(meta_year, (season.ano + 1).to_string());
        assert!(save_path.join("preseason_plan.json").exists());
        let _ = std::fs::remove_dir_all(save_path);
    }

    #[test]
    fn test_end_of_season_creates_new_season() {
        let (conn, season) = setup_pipeline_fixture();
        let save_path = unique_test_dir("eos_new_season");

        let result = run_end_of_season(&conn, &season, &save_path).expect("pipeline should run");

        let active = season_queries::get_active_season(&conn)
            .expect("active season query")
            .expect("new active season");
        assert_eq!(active.id, result.new_season_id);
        assert_eq!(active.numero, season.numero + 1);
        assert_eq!(active.ano, season.ano + 1);
        let _ = std::fs::remove_dir_all(save_path);
    }

    #[test]
    fn test_end_of_season_resets_stats() {
        let (conn, season) = setup_pipeline_fixture();
        let save_path = unique_test_dir("eos_reset_stats");

        run_end_of_season(&conn, &season, &save_path).expect("pipeline should run");

        let drivers = driver_queries::get_drivers_by_category(&conn, "mazda_rookie")
            .expect("drivers should load");
        assert!(drivers
            .iter()
            .all(|driver| driver.stats_temporada.corridas == 0));
        assert!(drivers
            .iter()
            .all(|driver| driver.stats_temporada.pontos == 0.0));

        let teams =
            team_queries::get_teams_by_category(&conn, "mazda_rookie").expect("teams should load");
        assert!(teams.iter().all(|team| team.stats_pontos == 0));
        assert!(teams.iter().all(|team| team.stats_vitorias == 0));
        let _ = std::fs::remove_dir_all(save_path);
    }

    #[test]
    fn test_promotion_initializes_preseason_after_movements() {
        let (conn, season, promoted_team_id, freed_driver_id) = setup_promotion_order_fixture();
        let save_path = unique_test_dir("eos_preseason_order");

        let result = run_end_of_season(&conn, &season, &save_path).expect("pipeline should run");

        assert!(result
            .promotion_result
            .movements
            .iter()
            .any(|movement| movement.team_id == promoted_team_id
                && movement.from_category == "gt4"
                && movement.to_category == "endurance"));
        assert!(result.preseason_initialized);
        assert!(result.preseason_total_weeks >= 3);

        let promoted_team = team_queries::get_team_by_id(&conn, &promoted_team_id)
            .expect("team query")
            .expect("promoted team");
        assert_eq!(promoted_team.categoria, "endurance");
        assert!(promoted_team.piloto_1_id.is_some() || promoted_team.piloto_2_id.is_some());

        assert!(result
            .promotion_result
            .pilot_effects
            .iter()
            .any(|effect| effect.driver_id == freed_driver_id
                && matches!(
                    effect.effect,
                    crate::promotion::PilotEffectType::FreedNoLicense
                )));
        assert_ne!(
            promoted_team.piloto_1_id.as_deref(),
            Some(freed_driver_id.as_str())
        );
        assert_ne!(
            promoted_team.piloto_2_id.as_deref(),
            Some(freed_driver_id.as_str())
        );
        assert!(save_path.join("preseason_plan.json").exists());
        let _ = std::fs::remove_dir_all(save_path);
    }

    fn setup_pipeline_fixture() -> (Connection, Season) {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        let season = Season::new("S001".to_string(), 1, 2024);
        season_queries::insert_season(&conn, &season).expect("season insert");

        let mut rng = StdRng::seed_from_u64(10);
        let team_a = sample_team("mazda_rookie", "T001", &mut rng);
        let team_b = sample_team("mazda_rookie", "T002", &mut rng);
        team_queries::insert_team(&conn, &team_a).expect("team a");
        team_queries::insert_team(&conn, &team_b).expect("team b");

        let driver_a = sample_driver("P001", "Piloto A", "mazda_rookie", 120.0, 3, 5, 0);
        let driver_b = sample_driver("P002", "Piloto B", "mazda_rookie", 90.0, 1, 4, 1);
        driver_queries::insert_driver(&conn, &driver_a).expect("driver a");
        driver_queries::insert_driver(&conn, &driver_b).expect("driver b");

        let contract_a = Contract::new(
            "C001".to_string(),
            driver_a.id.clone(),
            driver_a.nome.clone(),
            team_a.id.clone(),
            team_a.nome.clone(),
            1,
            2,
            100_000.0,
            TeamRole::Numero1,
            "mazda_rookie".to_string(),
        );
        let contract_b = Contract::new(
            "C002".to_string(),
            driver_b.id.clone(),
            driver_b.nome.clone(),
            team_b.id.clone(),
            team_b.nome.clone(),
            1,
            2,
            90_000.0,
            TeamRole::Numero1,
            "mazda_rookie".to_string(),
        );
        contract_queries::insert_contract(&conn, &contract_a).expect("contract a");
        contract_queries::insert_contract(&conn, &contract_b).expect("contract b");

        let mut calendar_rng = StdRng::seed_from_u64(20);
        let entry = generate_calendar_for_category(&season.id, "mazda_rookie", &mut calendar_rng)
            .expect("calendar")
            .into_iter()
            .next()
            .expect("calendar entry");
        calendar_queries::insert_calendar_entry(&conn, &entry).expect("calendar insert");
        calendar_queries::mark_race_completed(&conn, &entry.id).expect("mark complete");
        conn.execute(
            "UPDATE meta SET value = '3' WHERE key = 'next_driver_id'",
            [],
        )
        .expect("meta driver counter");
        conn.execute(
            "UPDATE meta SET value = '3' WHERE key = 'next_contract_id'",
            [],
        )
        .expect("meta contract counter");
        conn.execute(
            "UPDATE meta SET value = '2' WHERE key = 'next_season_id'",
            [],
        )
        .expect("meta season counter");
        conn.execute("UPDATE meta SET value = '2' WHERE key = 'next_race_id'", [])
            .expect("meta race counter");

        (conn, season)
    }

    fn setup_promotion_order_fixture() -> (Connection, Season, String, String) {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        let previous = Season::new("OLD1".to_string(), 1, 2024);
        season_queries::insert_season(&conn, &previous).expect("previous season");
        season_queries::finalize_season(&conn, &previous.id).expect("finalize previous season");

        let season = Season::new("CUR2".to_string(), 2, 2025);
        season_queries::insert_season(&conn, &season).expect("current season");

        seed_promotion_teams(&conn);
        seed_gt4_promotion_drivers(&conn);

        conn.execute(
            "UPDATE meta SET value = '2' WHERE key = 'current_season'",
            [],
        )
        .expect("meta current season");
        conn.execute(
            "UPDATE meta SET value = '2025' WHERE key = 'current_year'",
            [],
        )
        .expect("meta current year");

        (conn, season, "GT4PROMO".to_string(), "GT4LOW".to_string())
    }

    fn seed_promotion_teams(conn: &Connection) {
        insert_ranked_teams(conn, "mazda_rookie", "MR", 6, None);
        insert_ranked_teams(conn, "toyota_rookie", "TR", 6, None);
        insert_ranked_teams(conn, "mazda_amador", "MA", 10, None);
        insert_ranked_teams(conn, "toyota_amador", "TA", 10, None);
        insert_ranked_teams(conn, "bmw_m2", "BM", 10, None);
        insert_ranked_teams(conn, "production_challenger", "PM", 5, Some("mazda"));
        insert_ranked_teams(conn, "production_challenger", "PT", 5, Some("toyota"));
        insert_ranked_teams(conn, "production_challenger", "PB", 5, Some("bmw"));
        insert_ranked_teams(conn, "gt4", "GT4", 9, None);
        insert_ranked_teams(conn, "gt3", "GT3", 14, None);
        insert_ranked_teams(conn, "endurance", "EG4", 6, Some("gt4"));
        insert_ranked_teams(conn, "endurance", "EG3", 6, Some("gt3"));
        insert_ranked_teams(conn, "endurance", "LMP", 5, Some("lmp2"));

        let mut promoted_team = sample_named_team("gt4", "GT4PROMO", "GT4 Promo Team", None, 9001);
        promoted_team.stats_pontos = 999;
        promoted_team.stats_vitorias = 8;
        promoted_team.stats_melhor_resultado = 1;
        team_queries::insert_team(conn, &promoted_team).expect("insert promoted gt4 team");
    }

    fn seed_gt4_promotion_drivers(conn: &Connection) {
        let licensed_driver = sample_driver("GT4TOP", "Piloto Licenciado", "gt4", 200.0, 4, 10, 0);
        let unlicensed_driver = sample_driver("GT4LOW", "Piloto Sem Licenca", "gt4", 5.0, 0, 10, 2);
        let support_drivers = [
            sample_driver("GT4D1", "GT4 Driver 1", "gt4", 150.0, 3, 10, 0),
            sample_driver("GT4D2", "GT4 Driver 2", "gt4", 130.0, 2, 10, 0),
            sample_driver("GT4D3", "GT4 Driver 3", "gt4", 110.0, 2, 10, 0),
            sample_driver("GT4D4", "GT4 Driver 4", "gt4", 90.0, 1, 10, 1),
            sample_driver("GT4D5", "GT4 Driver 5", "gt4", 70.0, 1, 10, 1),
            sample_driver("GT4D6", "GT4 Driver 6", "gt4", 50.0, 0, 10, 1),
        ];

        for driver in [&licensed_driver, &unlicensed_driver] {
            driver_queries::insert_driver(conn, driver).expect("insert promoted team driver");
        }
        for driver in &support_drivers {
            driver_queries::insert_driver(conn, driver).expect("insert support driver");
        }

        let contract_1 = Contract::new(
            "KGT401".to_string(),
            licensed_driver.id.clone(),
            licensed_driver.nome.clone(),
            "GT4PROMO".to_string(),
            "GT4 Promo Team".to_string(),
            2,
            2,
            150_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        let contract_2 = Contract::new(
            "KGT402".to_string(),
            unlicensed_driver.id.clone(),
            unlicensed_driver.nome.clone(),
            "GT4PROMO".to_string(),
            "GT4 Promo Team".to_string(),
            2,
            2,
            120_000.0,
            TeamRole::Numero2,
            "gt4".to_string(),
        );
        contract_queries::insert_contract(conn, &contract_1).expect("insert contract 1");
        contract_queries::insert_contract(conn, &contract_2).expect("insert contract 2");
        team_queries::update_team_pilots(
            conn,
            "GT4PROMO",
            Some(&licensed_driver.id),
            Some(&unlicensed_driver.id),
        )
        .expect("assign promoted team pilots");
    }

    fn insert_ranked_teams(
        conn: &Connection,
        category: &str,
        prefix: &str,
        count: usize,
        class: Option<&str>,
    ) {
        for index in 0..count {
            let rank = index + 1;
            let mut team = sample_named_team(
                category,
                &format!("{prefix}{rank}"),
                &format!("{prefix} Team {rank}"),
                class,
                rank as u64 + prefix.bytes().map(u64::from).sum::<u64>(),
            );
            team.stats_pontos = ((count - index) * 10) as i32;
            team.stats_vitorias = (count - index) as i32;
            team.stats_melhor_resultado = rank as i32;
            team_queries::insert_team(conn, &team).expect("insert ranked team");
        }
    }

    fn sample_driver(
        id: &str,
        name: &str,
        category: &str,
        points: f64,
        wins: u32,
        races: u32,
        dnfs: u32,
    ) -> Driver {
        let mut driver = Driver::new(
            id.to_string(),
            name.to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            24,
            2020,
        );
        driver.categoria_atual = Some(category.to_string());
        driver.stats_temporada.pontos = points;
        driver.stats_temporada.vitorias = wins;
        driver.stats_temporada.podios = wins + 1;
        driver.stats_temporada.corridas = races;
        driver.stats_temporada.dnfs = dnfs;
        driver.stats_temporada.poles = wins;
        driver.stats_temporada.posicao_media = 4.0;
        driver
    }

    fn sample_team(category: &str, id: &str, rng: &mut StdRng) -> Team {
        let template = get_team_templates(category)[0];
        Team::from_template_with_rng(template, category, id.to_string(), 2024, rng)
    }

    fn sample_named_team(
        category: &str,
        id: &str,
        name: &str,
        class: Option<&str>,
        seed: u64,
    ) -> Team {
        let template = get_team_templates(category)[0];
        let mut rng = StdRng::seed_from_u64(seed);
        let mut team =
            Team::from_template_with_rng(template, category, id.to_string(), 2025, &mut rng);
        team.nome = name.to_string();
        team.nome_curto = name.to_string();
        team.classe = class.map(str::to_string);
        team
    }

    fn unique_test_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("iracerapp_eos_{label}_{nanos}"));
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }
}
