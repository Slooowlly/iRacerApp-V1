use rand::Rng;
use rusqlite::Connection;

use std::collections::HashSet;

use crate::db::queries::drivers as driver_queries;
use crate::db::queries::teams as team_queries;
use crate::promotion::block1::execute_block1;
use crate::promotion::block2::execute_block2_with_exclusions;
use crate::promotion::block3::execute_block3;
use crate::promotion::effects::{
    apply_attribute_deltas, calculate_promotion_effects, calculate_relegation_effects,
};
use crate::promotion::pilots::{apply_pilot_effect, resolve_pilot_situations};
use crate::promotion::{MovementType, PromotionResult, TeamMovement};

pub fn run_promotion_relegation(
    conn: &Connection,
    season_number: i32,
    rng: &mut impl Rng,
) -> Result<PromotionResult, String> {
    if season_number <= 1 {
        return Ok(PromotionResult::empty());
    }

    let mut all_movements = Vec::new();
    let block1_movements = execute_block1(conn, rng)?;
    let excluded_from_block2: HashSet<String> = block1_movements
        .iter()
        .filter(|movement| {
            movement.movement_type == MovementType::Rebaixamento
                && (movement.from_category == "mazda_amador"
                    || movement.from_category == "toyota_amador")
        })
        .map(|movement| movement.team_id.clone())
        .collect();
    let block2_movements = execute_block2_with_exclusions(conn, &excluded_from_block2, rng)?;
    let block3_movements = execute_block3(conn, rng)?;

    all_movements.extend(block1_movements);
    all_movements.extend(block2_movements);
    all_movements.extend(block3_movements);

    for movement in &all_movements {
        apply_team_category_change(conn, movement)?;
    }

    let pilot_effects = resolve_pilot_situations(conn, &all_movements)?;
    for effect in &pilot_effects {
        apply_pilot_effect(conn, effect, &all_movements)?;
    }

    let mut attribute_deltas = Vec::new();
    for movement in &all_movements {
        let team = team_queries::get_team_by_id(conn, &movement.team_id)
            .map_err(|e| format!("Falha ao buscar equipe '{}': {e}", movement.team_id))?
            .ok_or_else(|| format!("Equipe '{}' nao encontrada", movement.team_id))?;
        let delta = match movement.movement_type {
            MovementType::Promocao => calculate_promotion_effects(&team, rng),
            MovementType::Rebaixamento => calculate_relegation_effects(&team, rng),
        };
        apply_attribute_deltas(conn, &movement.team_id, &delta)?;
        attribute_deltas.push(delta);
    }

    let mut errors = verify_team_driver_consistency(conn, &all_movements);
    errors.extend(verify_category_sizes(conn));
    Ok(PromotionResult {
        movements: all_movements,
        pilot_effects,
        attribute_deltas,
        errors,
    })
}

fn apply_team_category_change(conn: &Connection, movement: &TeamMovement) -> Result<(), String> {
    let mut team = team_queries::get_team_by_id(conn, &movement.team_id)
        .map_err(|e| format!("Falha ao buscar equipe '{}': {e}", movement.team_id))?
        .ok_or_else(|| format!("Equipe '{}' nao encontrada", movement.team_id))?;
    team.categoria = movement.to_category.clone();
    team.classe = infer_team_class(movement);
    team_queries::update_team(conn, &team)
        .map_err(|e| format!("Falha ao atualizar equipe '{}': {e}", team.nome))?;
    Ok(())
}

fn infer_team_class(movement: &TeamMovement) -> Option<String> {
    match movement.to_category.as_str() {
        "production_challenger" => match movement.from_category.as_str() {
            "mazda_amador" => Some("mazda".to_string()),
            "toyota_amador" => Some("toyota".to_string()),
            "bmw_m2" => Some("bmw".to_string()),
            _ => None,
        },
        "endurance" => match movement.from_category.as_str() {
            "gt4" => Some("gt4".to_string()),
            "gt3" => Some("gt3".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn verify_team_driver_consistency(conn: &Connection, movements: &[TeamMovement]) -> Vec<String> {
    let mut errors = Vec::new();
    for movement in movements {
        let Ok(Some(team)) = team_queries::get_team_by_id(conn, &movement.team_id) else {
            continue;
        };
        for pilot_id in [team.piloto_1_id.as_deref(), team.piloto_2_id.as_deref()]
            .into_iter()
            .flatten()
        {
            match driver_queries::get_driver(conn, pilot_id) {
                Err(_) => errors.push(format!(
                    "CONSISTENCIA: equipe '{}' referencia piloto '{pilot_id}' inexistente",
                    team.nome
                )),
                Ok(driver) => {
                    if driver.categoria_atual.as_deref() != Some(team.categoria.as_str()) {
                        errors.push(format!(
                            "CONSISTENCIA: piloto '{}' tem categoria '{:?}' mas equipe '{}' esta em '{}'",
                            driver.nome, driver.categoria_atual, team.nome, team.categoria
                        ));
                    }
                }
            }
        }
    }
    errors
}

fn verify_category_sizes(conn: &Connection) -> Vec<String> {
    let expected_sizes = [
        ("mazda_rookie", 6),
        ("toyota_rookie", 6),
        ("mazda_amador", 10),
        ("toyota_amador", 10),
        ("bmw_m2", 10),
        ("production_challenger", 15),
        ("gt4", 10),
        ("gt3", 14),
        ("endurance", 17),
    ];
    let mut errors = Vec::new();
    for (category, expected) in expected_sizes {
        let actual = team_queries::count_teams_by_category(conn, category).unwrap_or(0);
        if actual != expected {
            errors.push(format!(
                "INVARIANTE VIOLADO: {category} tem {actual} equipes (esperado {expected})"
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::constants::teams::get_team_templates;
    use crate::db::migrations;
    use crate::db::queries::teams as team_queries;
    use crate::models::team::Team;

    #[test]
    fn test_no_promotion_season_1() {
        let conn = setup_promotion_db();
        let mut rng = StdRng::seed_from_u64(50);

        let result = run_promotion_relegation(&conn, 1, &mut rng).expect("promotion should run");

        assert!(result.movements.is_empty());
        assert!(result.pilot_effects.is_empty());
        assert!(result.attribute_deltas.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_full_promotion_all_blocks() {
        let conn = setup_promotion_db();
        let mut rng = StdRng::seed_from_u64(51);

        let result = run_promotion_relegation(&conn, 2, &mut rng).expect("promotion should run");

        assert_eq!(result.movements.len(), 34);
        assert!(!result.attribute_deltas.is_empty());
    }

    #[test]
    fn test_invariant_maintained() {
        let conn = setup_promotion_db();
        let mut rng = StdRng::seed_from_u64(52);

        let result = run_promotion_relegation(&conn, 2, &mut rng).expect("promotion should run");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    fn setup_promotion_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        insert_ranked_teams(&conn, "mazda_rookie", "MR", 6, None);
        insert_ranked_teams(&conn, "toyota_rookie", "TR", 6, None);
        insert_ranked_teams(&conn, "mazda_amador", "MA", 10, None);
        insert_ranked_teams(&conn, "toyota_amador", "TA", 10, None);
        insert_ranked_teams(&conn, "bmw_m2", "BM", 10, None);
        insert_ranked_teams(&conn, "production_challenger", "PM", 5, Some("mazda"));
        insert_ranked_teams(&conn, "production_challenger", "PT", 5, Some("toyota"));
        insert_ranked_teams(&conn, "production_challenger", "PB", 5, Some("bmw"));
        insert_ranked_teams(&conn, "gt4", "GT4", 10, None);
        insert_ranked_teams(&conn, "gt3", "GT3", 14, None);
        insert_ranked_teams(&conn, "endurance", "EG4", 6, Some("gt4"));
        insert_ranked_teams(&conn, "endurance", "EG3", 6, Some("gt3"));
        insert_ranked_teams(&conn, "endurance", "LMP", 5, Some("lmp2"));

        conn
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
            let mut team = sample_team(
                category,
                &format!("{prefix}{rank}"),
                &format!("{prefix} Team {rank}"),
                class,
            );
            team.stats_pontos = ((count - index) * 10) as i32;
            team.stats_vitorias = (count - index) as i32;
            team.stats_melhor_resultado = rank as i32;
            team_queries::insert_team(conn, &team).expect("insert ranked team");
        }
    }

    fn sample_team(category: &str, id: &str, name: &str, class: Option<&str>) -> Team {
        let template = get_team_templates(category)[0];
        let mut rng = StdRng::seed_from_u64(id.bytes().map(u64::from).sum());
        let mut team =
            Team::from_template_with_rng(template, category, id.to_string(), 2025, &mut rng);
        team.nome = name.to_string();
        team.nome_curto = name.to_string();
        team.classe = class.map(str::to_string);
        team
    }
}
