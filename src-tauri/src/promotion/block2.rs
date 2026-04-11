#![allow(dead_code)]

use std::collections::HashSet;

use rand::Rng;
use rusqlite::Connection;

use crate::promotion::standings::{
    calculate_constructor_standings, calculate_constructor_standings_by_class,
};
use crate::promotion::{MovementType, TeamMovement};

pub fn execute_block2(conn: &Connection, rng: &mut impl Rng) -> Result<Vec<TeamMovement>, String> {
    execute_block2_with_exclusions(conn, &HashSet::new(), rng)
}

pub(crate) fn execute_block2_with_exclusions(
    conn: &Connection,
    excluded_team_ids: &HashSet<String>,
    _rng: &mut impl Rng,
) -> Result<Vec<TeamMovement>, String> {
    let mut movements = Vec::new();
    append_block2_movements(
        &mut movements,
        conn,
        "mazda_amador",
        "production_challenger",
        Some("mazda"),
        "Top 3 de construtores da categoria",
        excluded_team_ids,
    )?;
    append_block2_movements(
        &mut movements,
        conn,
        "toyota_amador",
        "production_challenger",
        Some("toyota"),
        "Top 3 de construtores da categoria",
        excluded_team_ids,
    )?;
    append_block2_movements(
        &mut movements,
        conn,
        "bmw_m2",
        "production_challenger",
        Some("bmw"),
        "Top 3 de construtores da categoria",
        excluded_team_ids,
    )?;
    Ok(movements)
}

fn append_block2_movements(
    movements: &mut Vec<TeamMovement>,
    conn: &Connection,
    source_category: &str,
    target_category: &str,
    production_class: Option<&str>,
    promotion_reason: &str,
    excluded_team_ids: &HashSet<String>,
) -> Result<(), String> {
    let source_candidates: Vec<_> = calculate_constructor_standings(conn, source_category)?
        .into_iter()
        .filter(|standing| !excluded_team_ids.contains(&standing.team_id))
        .take(3)
        .collect();
    if source_candidates.len() != 3 {
        return Err(format!(
            "Equipes insuficientes para promover de '{source_category}'"
        ));
    }

    let relegation_class =
        production_class.ok_or_else(|| "Classe de production obrigatoria".to_string())?;
    let relegated: Vec<_> =
        calculate_constructor_standings_by_class(conn, target_category, relegation_class)?
            .into_iter()
            .rev()
            .take(3)
            .collect();
    if relegated.len() != 3 {
        return Err(format!(
            "Equipes insuficientes para rebaixar da classe '{relegation_class}'"
        ));
    }

    for promoted in source_candidates {
        movements.push(TeamMovement {
            team_id: promoted.team_id,
            team_name: promoted.team_name,
            from_category: source_category.to_string(),
            to_category: target_category.to_string(),
            movement_type: MovementType::Promocao,
            reason: promotion_reason.to_string(),
        });
    }

    for relegated_team in relegated {
        movements.push(TeamMovement {
            team_id: relegated_team.team_id,
            team_name: relegated_team.team_name,
            from_category: target_category.to_string(),
            to_category: source_category.to_string(),
            movement_type: MovementType::Rebaixamento,
            reason: "Bottom 3 da classe na Production Challenger".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::constants::teams::get_team_templates;
    use crate::db::migrations;
    use crate::db::queries::teams as team_queries;
    use crate::models::team::Team;

    #[test]
    fn test_block2_top3_promoted_to_production() {
        let conn = setup_block2_db();
        let mut rng = StdRng::seed_from_u64(20);
        let mut excluded = HashSet::new();
        excluded.insert("MA10".to_string());

        let movements =
            execute_block2_with_exclusions(&conn, &excluded, &mut rng).expect("block2 should run");

        for team_id in ["MA1", "MA2", "MA3"] {
            assert!(movements.iter().any(|movement| {
                movement.team_id == team_id
                    && movement.to_category == "production_challenger"
                    && movement.movement_type == MovementType::Promocao
            }));
        }
    }

    #[test]
    fn test_block2_bottom3_relegated_from_production() {
        let conn = setup_block2_db();
        let mut rng = StdRng::seed_from_u64(21);

        let movements = execute_block2(&conn, &mut rng).expect("block2 should run");

        for team_id in ["PM5", "PM4", "PM3"] {
            assert!(movements.iter().any(|movement| {
                movement.team_id == team_id
                    && movement.from_category == "production_challenger"
                    && movement.to_category == "mazda_amador"
                    && movement.movement_type == MovementType::Rebaixamento
            }));
        }
    }

    #[test]
    fn test_block2_sizes_preserved() {
        let conn = setup_block2_db();
        let mut rng = StdRng::seed_from_u64(22);

        let movements = execute_block2(&conn, &mut rng).expect("block2 should run");

        assert_eq!(
            movements
                .iter()
                .filter(|movement| movement.to_category == "production_challenger")
                .count(),
            9
        );
        assert_eq!(
            movements
                .iter()
                .filter(|movement| movement.from_category == "production_challenger")
                .count(),
            9
        );
    }

    fn setup_block2_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        insert_ranked_teams(&conn, "mazda_amador", "MA", 10, None);
        insert_ranked_teams(&conn, "toyota_amador", "TA", 10, None);
        insert_ranked_teams(&conn, "bmw_m2", "BM", 10, None);
        insert_ranked_teams(&conn, "production_challenger", "PM", 5, Some("mazda"));
        insert_ranked_teams(&conn, "production_challenger", "PT", 5, Some("toyota"));
        insert_ranked_teams(&conn, "production_challenger", "PB", 5, Some("bmw"));

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
