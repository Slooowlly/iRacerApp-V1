use rusqlite::{params, Transaction};

use crate::db::connection::DbError;
use crate::models::enums::InjuryType;
use crate::models::injury::Injury;

pub fn insert_injury(tx: &Transaction, injury: &Injury) -> Result<(), DbError> {
    tx.execute(
        "INSERT INTO injuries (
            id, pilot_id, type, modifier, races_total, races_remaining, skill_penalty, season, race_occurred, active
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            injury.id,
            injury.pilot_id,
            injury.injury_type.as_str(),
            injury.modifier,
            injury.races_total,
            injury.races_remaining,
            injury.skill_penalty,
            injury.season,
            injury.race_occurred,
            if injury.active { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub fn get_active_injuries_for_category(
    tx: &Transaction,
    category_id: &str,
) -> Result<Vec<Injury>, DbError> {
    let mut stmt = tx.prepare(
        "SELECT i.id, i.pilot_id, i.type, i.modifier, i.races_total, i.races_remaining, i.skill_penalty, i.season, i.race_occurred, i.active
         FROM injuries i
         JOIN drivers d ON i.pilot_id = d.id
         WHERE i.active = 1 AND d.categoria_atual = ?1",
    )?;

    let iter = stmt.query_map(params![category_id], |row| {
        Ok(Injury {
            id: row.get(0)?,
            pilot_id: row.get(1)?,
            injury_type: InjuryType::from_str(&row.get::<_, String>(2)?),
            modifier: row.get(3)?,
            races_total: row.get(4)?,
            races_remaining: row.get(5)?,
            skill_penalty: row.get(6)?,
            season: row.get(7)?,
            race_occurred: row.get(8)?,
            active: row.get::<_, i32>(9)? == 1,
        })
    })?;

    let mut injuries = Vec::new();
    for i in iter {
        injuries.push(i?);
    }
    Ok(injuries)
}

pub fn update_injury_status(
    tx: &Transaction,
    injury_id: &str,
    races_remaining: i32,
    active: bool,
) -> Result<(), DbError> {
    tx.execute(
        "UPDATE injuries SET races_remaining = ?1, active = ?2 WHERE id = ?3",
        params![races_remaining, if active { 1 } else { 0 }, injury_id],
    )?;
    Ok(())
}
