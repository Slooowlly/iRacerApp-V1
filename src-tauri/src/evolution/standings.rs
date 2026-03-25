use std::collections::HashMap;

use rusqlite::Connection;

use crate::constants::categories::get_all_categories;
use crate::db::queries::drivers as driver_queries;
use crate::evolution::context::StandingEntry;
use crate::evolution::growth::SeasonStats;
use crate::models::contract::Contract;
use crate::models::season::Season;

pub(crate) fn build_and_persist_standings(
    conn: &Connection,
    season: &Season,
    contracts_by_driver: &HashMap<String, Contract>,
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
