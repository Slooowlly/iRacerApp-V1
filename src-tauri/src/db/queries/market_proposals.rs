use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};

use crate::constants::categories::get_category_config;
use crate::db::connection::DbError;
use crate::market::proposals::{MarketProposal, ProposalStatus};
use crate::models::enums::TeamRole;

pub fn insert_player_proposal(
    conn: &Connection,
    season_id: &str,
    proposal: &MarketProposal,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO market_proposals (
            id, temporada_id, equipe_id, piloto_id, papel, salario, status, motivo_recusa, criado_em
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &proposal.id,
            season_id,
            &proposal.equipe_id,
            &proposal.piloto_id,
            proposal.papel.as_str(),
            proposal.salario_oferecido,
            proposal.status.as_str(),
            proposal.motivo_recusa.clone(),
            timestamp_now(),
        ],
    )?;
    Ok(())
}

pub fn get_pending_player_proposals(
    conn: &Connection,
    player_id: &str,
) -> Result<Vec<MarketProposal>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT
            mp.id,
            mp.equipe_id,
            t.nome,
            mp.piloto_id,
            d.nome,
            t.categoria,
            mp.papel,
            mp.salario,
            mp.status,
            mp.motivo_recusa
         FROM market_proposals mp
         INNER JOIN teams t ON t.id = mp.equipe_id
         INNER JOIN drivers d ON d.id = mp.piloto_id
         WHERE mp.piloto_id = ?1 AND mp.status = 'Pendente'
         ORDER BY mp.salario DESC, mp.criado_em DESC",
    )?;
    let rows = stmt.query_map(params![player_id], proposal_from_row)?;
    collect_proposals(rows)
}

pub fn count_pending_player_proposals(conn: &Connection, player_id: &str) -> Result<i32, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM market_proposals WHERE piloto_id = ?1 AND status = 'Pendente'",
        params![player_id],
        |row| row.get(0),
    )?;
    Ok(count as i32)
}

pub fn get_market_proposal_by_id(
    conn: &Connection,
    proposal_id: &str,
) -> Result<Option<MarketProposal>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT
            mp.id,
            mp.equipe_id,
            t.nome,
            mp.piloto_id,
            d.nome,
            t.categoria,
            mp.papel,
            mp.salario,
            mp.status,
            mp.motivo_recusa
         FROM market_proposals mp
         INNER JOIN teams t ON t.id = mp.equipe_id
         INNER JOIN drivers d ON d.id = mp.piloto_id
         WHERE mp.id = ?1
         LIMIT 1",
    )?;
    stmt.query_row(params![proposal_id], proposal_from_row)
        .optional()
        .map_err(DbError::from)
}

pub fn update_proposal_status(
    conn: &Connection,
    proposal_id: &str,
    new_status: &str,
    reason: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE market_proposals
         SET status = ?1, motivo_recusa = COALESCE(?2, motivo_recusa), respondido_em = ?3
         WHERE id = ?4",
        params![new_status, reason, timestamp_now(), proposal_id],
    )?;
    Ok(())
}

pub fn expire_remaining_proposals(
    conn: &Connection,
    player_id: &str,
    except_proposal_id: &str,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE market_proposals
         SET status = 'Expirada', respondido_em = ?1
         WHERE piloto_id = ?2 AND status = 'Pendente' AND id <> ?3",
        params![timestamp_now(), player_id, except_proposal_id],
    )?;
    Ok(())
}

fn timestamp_now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn collect_proposals(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<MarketProposal>,
    >,
) -> Result<Vec<MarketProposal>, DbError> {
    let mut proposals = Vec::new();
    for row in rows {
        proposals.push(row?);
    }
    Ok(proposals)
}

fn proposal_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MarketProposal> {
    let categoria: String = row.get(5)?;
    let duracao_anos = get_category_config(&categoria)
        .map(|config| if config.tier >= 3 { 2 } else { 1 })
        .unwrap_or(1);
    Ok(MarketProposal {
        id: row.get(0)?,
        equipe_id: row.get(1)?,
        equipe_nome: row.get(2)?,
        piloto_id: row.get(3)?,
        piloto_nome: row.get(4)?,
        categoria,
        papel: TeamRole::from_str(&row.get::<_, String>(6)?),
        salario_oferecido: row.get(7)?,
        duracao_anos,
        status: match row.get::<_, String>(8)?.as_str() {
            "Aceita" => ProposalStatus::Aceita,
            "Recusada" => ProposalStatus::Recusada,
            "Expirada" => ProposalStatus::Expirada,
            _ => ProposalStatus::Pendente,
        },
        motivo_recusa: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pending_player_proposals_returns_pending_only() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, "T001", "Team One", "gt4").expect("insert team");
        insert_driver(&conn, "P001", "Jogador").expect("insert player");
        insert_driver(&conn, "P002", "Outro").expect("insert other");
        insert_season(&conn, "S002", 2).expect("insert season");

        insert_player_proposal(
            &conn,
            "S002",
            &MarketProposal {
                id: "MP-001".to_string(),
                equipe_id: "T001".to_string(),
                equipe_nome: "Team One".to_string(),
                piloto_id: "P001".to_string(),
                piloto_nome: "Jogador".to_string(),
                categoria: "gt4".to_string(),
                papel: TeamRole::Numero1,
                salario_oferecido: 100_000.0,
                duracao_anos: 2,
                status: ProposalStatus::Pendente,
                motivo_recusa: None,
            },
        )
        .expect("insert pending proposal");
        insert_player_proposal(
            &conn,
            "S002",
            &MarketProposal {
                id: "MP-002".to_string(),
                equipe_id: "T001".to_string(),
                equipe_nome: "Team One".to_string(),
                piloto_id: "P001".to_string(),
                piloto_nome: "Jogador".to_string(),
                categoria: "gt4".to_string(),
                papel: TeamRole::Numero2,
                salario_oferecido: 80_000.0,
                duracao_anos: 2,
                status: ProposalStatus::Recusada,
                motivo_recusa: Some("Sem interesse".to_string()),
            },
        )
        .expect("insert rejected proposal");

        let proposals = get_pending_player_proposals(&conn, "P001").expect("pending proposals");

        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, "MP-001");
        assert_eq!(proposals[0].equipe_nome, "Team One");
        assert_eq!(proposals[0].categoria, "gt4");
        assert_eq!(proposals[0].duracao_anos, 2);
    }

    #[test]
    fn test_update_proposal_status_and_count_pending() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, "T001", "Team One", "mazda_amador").expect("insert team");
        insert_driver(&conn, "P001", "Jogador").expect("insert player");
        insert_season(&conn, "S002", 2).expect("insert season");

        insert_player_proposal(&conn, "S002", &sample_proposal("MP-010", "P001", "T001"))
            .expect("insert proposal");

        assert_eq!(
            count_pending_player_proposals(&conn, "P001").expect("count pending"),
            1
        );

        update_proposal_status(&conn, "MP-010", "Recusada", Some("Preferiu esperar"))
            .expect("update proposal");

        assert_eq!(
            count_pending_player_proposals(&conn, "P001").expect("count pending"),
            0
        );
        let proposal = get_market_proposal_by_id(&conn, "MP-010")
            .expect("proposal lookup")
            .expect("proposal");
        assert_eq!(proposal.status, ProposalStatus::Recusada);
        assert_eq!(proposal.motivo_recusa.as_deref(), Some("Preferiu esperar"));
    }

    #[test]
    fn test_expire_remaining_proposals_keeps_selected_one() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, "T001", "Team One", "gt4").expect("insert team");
        insert_team(&conn, "T002", "Team Two", "gt4").expect("insert team");
        insert_driver(&conn, "P001", "Jogador").expect("insert player");
        insert_season(&conn, "S002", 2).expect("insert season");

        insert_player_proposal(&conn, "S002", &sample_proposal("MP-101", "P001", "T001"))
            .expect("insert proposal one");
        insert_player_proposal(&conn, "S002", &sample_proposal("MP-102", "P001", "T002"))
            .expect("insert proposal two");

        expire_remaining_proposals(&conn, "P001", "MP-101").expect("expire remaining");

        let kept = get_market_proposal_by_id(&conn, "MP-101")
            .expect("lookup")
            .expect("kept");
        let expired = get_market_proposal_by_id(&conn, "MP-102")
            .expect("lookup")
            .expect("expired");
        assert_eq!(kept.status, ProposalStatus::Pendente);
        assert_eq!(expired.status, ProposalStatus::Expirada);
    }

    fn sample_proposal(id: &str, player_id: &str, team_id: &str) -> MarketProposal {
        MarketProposal {
            id: id.to_string(),
            equipe_id: team_id.to_string(),
            equipe_nome: format!("Team {team_id}"),
            piloto_id: player_id.to_string(),
            piloto_nome: "Jogador".to_string(),
            categoria: "gt4".to_string(),
            papel: TeamRole::Numero1,
            salario_oferecido: 90_000.0,
            duracao_anos: 2,
            status: ProposalStatus::Pendente,
            motivo_recusa: None,
        }
    }

    fn setup_test_db() -> Result<Connection, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE seasons (
                id TEXT PRIMARY KEY,
                numero INTEGER NOT NULL
            );
            CREATE TABLE drivers (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL
            );
            CREATE TABLE teams (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL,
                categoria TEXT NOT NULL
            );
            CREATE TABLE market_proposals (
                id TEXT PRIMARY KEY,
                temporada_id TEXT NOT NULL,
                equipe_id TEXT NOT NULL,
                piloto_id TEXT NOT NULL,
                papel TEXT NOT NULL DEFAULT 'Numero2',
                salario REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'Pendente',
                motivo_recusa TEXT,
                criado_em TEXT NOT NULL DEFAULT '',
                respondido_em TEXT
            );",
        )?;
        Ok(conn)
    }

    fn insert_season(conn: &Connection, id: &str, numero: i32) -> Result<(), DbError> {
        conn.execute(
            "INSERT INTO seasons (id, numero) VALUES (?1, ?2)",
            params![id, numero],
        )?;
        Ok(())
    }

    fn insert_driver(conn: &Connection, id: &str, nome: &str) -> Result<(), DbError> {
        conn.execute(
            "INSERT INTO drivers (id, nome) VALUES (?1, ?2)",
            params![id, nome],
        )?;
        Ok(())
    }

    fn insert_team(
        conn: &Connection,
        id: &str,
        nome: &str,
        categoria: &str,
    ) -> Result<(), DbError> {
        conn.execute(
            "INSERT INTO teams (id, nome, categoria) VALUES (?1, ?2, ?3)",
            params![id, nome, categoria],
        )?;
        Ok(())
    }
}
