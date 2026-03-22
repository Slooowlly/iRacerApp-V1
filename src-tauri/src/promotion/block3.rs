use rand::Rng;
use rusqlite::Connection;

use crate::promotion::standings::{
    calculate_constructor_standings, calculate_constructor_standings_by_class,
};
use crate::promotion::{MovementType, TeamMovement};

pub fn execute_block3(conn: &Connection, _rng: &mut impl Rng) -> Result<Vec<TeamMovement>, String> {
    let mut movements = Vec::new();
    append_endurance_pair(
        &mut movements,
        conn,
        "gt4",
        "endurance",
        "gt4",
        "Top 3 de construtores da GT4",
    )?;
    append_endurance_pair(
        &mut movements,
        conn,
        "gt3",
        "endurance",
        "gt3",
        "Top 3 de construtores da GT3",
    )?;
    Ok(movements)
}

fn append_endurance_pair(
    movements: &mut Vec<TeamMovement>,
    conn: &Connection,
    source_category: &str,
    endurance_category: &str,
    endurance_class: &str,
    promotion_reason: &str,
) -> Result<(), String> {
    let promoted: Vec<_> = calculate_constructor_standings(conn, source_category)?
        .into_iter()
        .take(3)
        .collect();
    if promoted.len() != 3 {
        return Err(format!(
            "Equipes insuficientes para promover de '{source_category}'"
        ));
    }

    let relegated: Vec<_> =
        calculate_constructor_standings_by_class(conn, endurance_category, endurance_class)?
            .into_iter()
            .rev()
            .take(3)
            .collect();
    if relegated.len() != 3 {
        return Err(format!(
            "Equipes insuficientes para rebaixar da classe '{endurance_class}'"
        ));
    }

    for team in promoted {
        movements.push(TeamMovement {
            team_id: team.team_id,
            team_name: team.team_name,
            from_category: source_category.to_string(),
            to_category: endurance_category.to_string(),
            movement_type: MovementType::Promocao,
            reason: promotion_reason.to_string(),
        });
    }

    for team in relegated {
        movements.push(TeamMovement {
            team_id: team.team_id,
            team_name: team.team_name,
            from_category: endurance_category.to_string(),
            to_category: source_category.to_string(),
            movement_type: MovementType::Rebaixamento,
            reason: format!("Bottom 3 da classe {endurance_class} em Endurance"),
        });
    }

    Ok(())
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
    fn test_block3_gt4_promotion() {
        let conn = setup_block3_db();
        let mut rng = StdRng::seed_from_u64(30);

        let movements = execute_block3(&conn, &mut rng).expect("block3 should run");

        for team_id in ["GT41", "GT42", "GT43"] {
            assert!(movements.iter().any(|movement| {
                movement.team_id == team_id
                    && movement.from_category == "gt4"
                    && movement.to_category == "endurance"
                    && movement.movement_type == MovementType::Promocao
            }));
        }
    }

    #[test]
    fn test_block3_gt3_promotion() {
        let conn = setup_block3_db();
        let mut rng = StdRng::seed_from_u64(31);

        let movements = execute_block3(&conn, &mut rng).expect("block3 should run");

        for team_id in ["GT31", "GT32", "GT33"] {
            assert!(movements.iter().any(|movement| {
                movement.team_id == team_id
                    && movement.from_category == "gt3"
                    && movement.to_category == "endurance"
                    && movement.movement_type == MovementType::Promocao
            }));
        }
    }

    #[test]
    fn test_block3_lmp2_untouched() {
        let conn = setup_block3_db();
        let mut rng = StdRng::seed_from_u64(32);

        let movements = execute_block3(&conn, &mut rng).expect("block3 should run");

        assert!(!movements
            .iter()
            .any(|movement| movement.team_id.starts_with("LMP")));
    }

    #[test]
    fn test_block3_sizes_preserved() {
        let conn = setup_block3_db();
        let mut rng = StdRng::seed_from_u64(33);

        let movements = execute_block3(&conn, &mut rng).expect("block3 should run");

        assert_eq!(
            movements
                .iter()
                .filter(|movement| movement.from_category == "endurance")
                .count(),
            6
        );
        assert_eq!(
            movements
                .iter()
                .filter(|movement| movement.to_category == "endurance")
                .count(),
            6
        );
    }

    fn setup_block3_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

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
