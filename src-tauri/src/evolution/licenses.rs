use std::collections::HashMap;

use rusqlite::Connection;

use crate::common::time::current_timestamp;
use crate::constants::categories::get_category_config;
use crate::evolution::context::{LicenseEarned, StandingEntry};

pub(crate) fn persist_licenses(
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

    let timestamp = current_timestamp();
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
