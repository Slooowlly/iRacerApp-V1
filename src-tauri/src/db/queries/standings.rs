use rusqlite::{Connection, Result as DbResult};

pub struct ChampionshipContext {
    pub player_position: i32,
    pub gap_to_leader: i32,
}

/// Retorna posição e gap do jogador na categoria do evento.
/// v1: usa drivers.categoria_atual = race_categoria como aproximação leve.
/// Não representa standings canônicos — migrar para fonte oficial em iteração futura.
pub fn get_championship_context(
    conn: &Connection,
    race_categoria: &str,
) -> DbResult<ChampionshipContext> {
    let mut stmt = conn.prepare(
        "SELECT id, temp_pontos, temp_vitorias, temp_podios, is_jogador
         FROM drivers
         WHERE categoria_atual = ?1 AND status != 'Aposentado'
         ORDER BY temp_pontos DESC, temp_vitorias DESC, temp_podios DESC",
    )?;

    struct Row {
        pontos: f64,
        is_jogador: bool,
    }
    let rows: Vec<Row> = stmt
        .query_map(rusqlite::params![race_categoria], |r| {
            Ok(Row {
                pontos: r.get(1)?,
                is_jogador: r.get::<_, i32>(4)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let leader_points = rows.first().map(|r| r.pontos).unwrap_or(0.0);
    let player_idx = rows.iter().position(|r| r.is_jogador);
    let player_position = player_idx.map(|i| i as i32 + 1).unwrap_or(0);
    let player_points = player_idx
        .and_then(|i| rows.get(i))
        .map(|r| r.pontos)
        .unwrap_or(0.0);
    let gap = if leader_points > player_points {
        (leader_points - player_points).round() as i32
    } else {
        0
    };

    Ok(ChampionshipContext {
        player_position,
        gap_to_leader: gap,
    })
}
