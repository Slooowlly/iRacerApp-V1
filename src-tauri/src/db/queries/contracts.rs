use rusqlite::{params, types::ValueRef, Connection, OptionalExtension};

use crate::constants::categories::get_category_config;
use crate::db::connection::DbError;
use crate::models::contract::Contract;
use crate::models::enums::{ContractStatus, ContractType, TeamRole};

pub fn insert_contract(conn: &Connection, contract: &Contract) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO contracts (
            id, piloto_id, piloto_nome, equipe_id, equipe_nome,
            temporada_inicio, duracao_anos, temporada_fim,
            salario, salario_anual, papel, status, tipo, categoria, classe, created_at
        ) VALUES (
            :id, :piloto_id, :piloto_nome, :equipe_id, :equipe_nome,
            :temporada_inicio, :duracao_anos, :temporada_fim,
            :salario, :salario_anual, :papel, :status, :tipo, :categoria, :classe, :created_at
        )",
        rusqlite::named_params! {
            ":id": &contract.id,
            ":piloto_id": &contract.piloto_id,
            ":piloto_nome": &contract.piloto_nome,
            ":equipe_id": &contract.equipe_id,
            ":equipe_nome": &contract.equipe_nome,
            ":temporada_inicio": contract.temporada_inicio,
            ":duracao_anos": contract.duracao_anos,
            ":temporada_fim": contract.temporada_fim,
            ":salario": contract.salario_anual,
            ":salario_anual": contract.salario_anual,
            ":papel": contract.papel.as_str(),
            ":status": contract.status.as_str(),
            ":tipo": contract.tipo.as_str(),
            ":categoria": &contract.categoria,
            ":classe": &contract.classe,
            ":created_at": &contract.created_at,
        },
    )?;
    Ok(())
}

pub fn insert_contracts(conn: &Connection, contracts: &[Contract]) -> Result<(), DbError> {
    for contract in contracts {
        insert_contract(conn, contract)?;
    }
    Ok(())
}

pub fn get_contract_by_id(conn: &Connection, id: &str) -> Result<Option<Contract>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM contracts WHERE id = ?1")?;
    let contract = stmt.query_row(params![id], contract_from_row).optional()?;
    Ok(contract)
}

/// Retorna o contrato ativo mais recente para o piloto (qualquer tipo).
/// ATENÇÃO: com dual contrato (Regular + Especial), esta função pode retornar
/// qualquer um dos dois. Para semântica precisa, use
/// `get_active_regular_contract_for_pilot` ou `get_active_especial_contract_for_pilot`.
pub fn get_active_contract_for_pilot(
    conn: &Connection,
    piloto_id: &str,
) -> Result<Option<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE piloto_id = ?1 AND status = 'Ativo'
         ORDER BY temporada_inicio DESC, created_at DESC
         LIMIT 1",
    )?;
    let contract = stmt
        .query_row(params![piloto_id], contract_from_row)
        .optional()?;
    Ok(contract)
}

pub fn get_contracts_for_pilot(
    conn: &Connection,
    piloto_id: &str,
) -> Result<Vec<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE piloto_id = ?1
         ORDER BY temporada_inicio DESC, created_at DESC",
    )?;
    let mapped = stmt.query_map(params![piloto_id], contract_from_row)?;
    collect_contracts(mapped)
}

pub fn get_active_contracts_for_team(
    conn: &Connection,
    equipe_id: &str,
) -> Result<Vec<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE equipe_id = ?1 AND status = 'Ativo'
         ORDER BY papel ASC, piloto_nome ASC",
    )?;
    let mapped = stmt.query_map(params![equipe_id], contract_from_row)?;
    collect_contracts(mapped)
}

pub fn get_all_active_contracts(conn: &Connection) -> Result<Vec<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE status = 'Ativo'
         ORDER BY categoria, equipe_nome, piloto_nome",
    )?;
    let mapped = stmt.query_map([], contract_from_row)?;
    collect_contracts(mapped)
}

pub fn get_expiring_contracts(conn: &Connection, temporada: i32) -> Result<Vec<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE status = 'Ativo' AND CAST(temporada_fim AS INTEGER) = ?1
         ORDER BY categoria, equipe_nome, piloto_nome",
    )?;
    let mapped = stmt.query_map(params![temporada], contract_from_row)?;
    collect_contracts(mapped)
}

pub fn get_contracts_by_category(
    conn: &Connection,
    categoria: &str,
) -> Result<Vec<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE categoria = ?1
         ORDER BY equipe_nome, piloto_nome",
    )?;
    let mapped = stmt.query_map(params![categoria], contract_from_row)?;
    collect_contracts(mapped)
}

pub fn update_contract_status(
    conn: &Connection,
    id: &str,
    status: &ContractStatus,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE contracts SET status = ?1 WHERE id = ?2",
        params![status.as_str(), id],
    )?;
    Ok(())
}

/// Expira todos os contratos Especial ativos da temporada indicada.
/// Chamado durante PosEspecial — nenhum contrato Especial deve sobreviver ao bloco.
///
/// Filtra por `temporada_inicio = season_number` para precisão semântica e proteção
/// contra bugs futuros. No modelo atual só existe um ciclo especial ativo por vez,
/// portanto o resultado seria idêntico sem o filtro.
pub fn expire_especial_contracts(conn: &Connection, season_number: i32) -> Result<usize, DbError> {
    let n = conn.execute(
        "UPDATE contracts SET status = 'Expirado'
         WHERE tipo = 'Especial' AND status = 'Ativo' AND temporada_inicio = ?1",
        params![season_number],
    )?;
    Ok(n)
}

pub fn expire_ending_contracts(conn: &Connection, temporada_atual: i32) -> Result<i32, DbError> {
    let updated = conn.execute(
        "UPDATE contracts
         SET status = 'Expirado'
         WHERE status = 'Ativo' AND CAST(temporada_fim AS INTEGER) <= ?1",
        params![temporada_atual],
    )?;
    Ok(updated as i32)
}

pub fn count_active_contracts_for_team(conn: &Connection, equipe_id: &str) -> Result<i32, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM contracts WHERE equipe_id = ?1 AND status = 'Ativo'",
        params![equipe_id],
        |row| row.get(0),
    )?;
    Ok(count as i32)
}

pub fn get_free_pilots(conn: &Connection) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT d.id
         FROM drivers d
         LEFT JOIN contracts c
           ON c.piloto_id = d.id AND c.status = 'Ativo'
         WHERE c.id IS NULL
         ORDER BY d.nome",
    )?;

    let mapped = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut pilots = Vec::new();
    for row in mapped {
        pilots.push(row?);
    }
    Ok(pilots)
}

/// Retorna true se o piloto já possui um contrato Especial ativo.
pub fn has_active_especial_contract(
    conn: &Connection,
    piloto_id: &str,
) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM contracts
         WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Especial'",
        params![piloto_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Retorna true se o piloto já possui um contrato Regular ativo.
pub fn has_active_regular_contract(
    conn: &Connection,
    piloto_id: &str,
) -> Result<bool, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM contracts
         WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Regular'",
        params![piloto_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Retorna o contrato Regular ativo do piloto, se houver.
pub fn get_active_regular_contract_for_pilot(
    conn: &Connection,
    piloto_id: &str,
) -> Result<Option<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Regular'
         ORDER BY temporada_inicio DESC, created_at DESC
         LIMIT 1",
    )?;
    let contract = stmt
        .query_row(params![piloto_id], contract_from_row)
        .optional()?;
    Ok(contract)
}

/// Retorna o contrato Especial ativo do piloto, se houver.
pub fn get_active_especial_contract_for_pilot(
    conn: &Connection,
    piloto_id: &str,
) -> Result<Option<Contract>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM contracts
         WHERE piloto_id = ?1 AND status = 'Ativo' AND tipo = 'Especial'
         ORDER BY temporada_inicio DESC, created_at DESC
         LIMIT 1",
    )?;
    let contract = stmt
        .query_row(params![piloto_id], contract_from_row)
        .optional()?;
    Ok(contract)
}

/// Pilotos com contrato Regular ativo e sem contrato Especial ativo.
/// Representa elegibilidade mínima para convocação especial.
/// A seleção final (score, classe, wildcards) é responsabilidade dos Passos 6+.
pub fn get_pilots_available_for_especial(conn: &Connection) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT d.id
         FROM drivers d
         INNER JOIN contracts c_reg
           ON c_reg.piloto_id = d.id AND c_reg.status = 'Ativo' AND c_reg.tipo = 'Regular'
         LEFT JOIN contracts c_esp
           ON c_esp.piloto_id = d.id AND c_esp.status = 'Ativo' AND c_esp.tipo = 'Especial'
         WHERE c_esp.id IS NULL
         ORDER BY d.nome",
    )?;
    let mapped = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut pilots = Vec::new();
    for row in mapped {
        pilots.push(row?);
    }
    Ok(pilots)
}

/// Retorna IDs de pilotos que já tiveram contrato Especial numa categoria+classe específica.
/// Usado para montar a Fonte B (ContinuidadeHistorica) da convocação especial.
pub fn get_pilots_with_especial_history(
    conn: &Connection,
    special_category: &str,
    class_name: &str,
) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT piloto_id FROM contracts
         WHERE tipo = 'Especial' AND categoria = ?1 AND classe = ?2
         ORDER BY piloto_id",
    )?;
    let mapped = stmt.query_map(params![special_category, class_name], |row| {
        row.get::<_, String>(0)
    })?;
    let mut pilots = Vec::new();
    for row in mapped {
        pilots.push(row?);
    }
    Ok(pilots)
}

/// Contagem de contratos especiais anteriores de um piloto em categoria+classe.
/// Usado no cálculo do score da Fonte B.
pub fn get_especial_contract_count(
    conn: &Connection,
    piloto_id: &str,
    special_category: &str,
    class_name: &str,
) -> Result<u32, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM contracts
         WHERE piloto_id = ?1 AND tipo = 'Especial' AND categoria = ?2 AND classe = ?3",
        params![piloto_id, special_category, class_name],
        |row| row.get(0),
    )?;
    Ok(count as u32)
}

/// Gera um contrato especial sazonal.
/// tipo = Especial, duracao_anos = 1 (placeholder: válido até fim do BlocoEspecial).
/// Salário ≈ 50% do range regular do tier correspondente.
/// O pipeline de encerramento do bloco especial expirará esses contratos explicitamente.
pub fn generate_especial_contract(
    id: String,
    piloto_id: &str,
    piloto_nome: &str,
    equipe_id: &str,
    equipe_nome: &str,
    papel: TeamRole,
    categoria: &str,
    classe: &str,
    temporada: i32,
) -> Contract {
    let tier = get_category_config(categoria)
        .map(|c| c.tier)
        .unwrap_or(2);
    let salario_anual = salary_midpoint_for_tier(tier) * 0.5;
    let mut contract = Contract::new(
        id,
        piloto_id.to_string(),
        piloto_nome.to_string(),
        equipe_id.to_string(),
        equipe_nome.to_string(),
        temporada,
        1,
        salario_anual,
        papel,
        categoria.to_string(),
    );
    contract.tipo = ContractType::Especial;
    contract.classe = Some(classe.to_string());
    contract
}

fn salary_midpoint_for_tier(tier: u8) -> f64 {
    match tier {
        0 => 10_000.0,
        1 => 27_500.0,
        2 => 55_000.0,
        3 => 105_000.0,
        4 => 200_000.0,
        5 => 165_000.0,
        _ => 10_000.0,
    }
}

pub fn delete_contract(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM contracts WHERE id = ?1", params![id])?;
    Ok(())
}

fn collect_contracts(
    mapped: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Contract>>,
) -> Result<Vec<Contract>, DbError> {
    let mut result = Vec::new();
    for row in mapped {
        result.push(row?);
    }
    Ok(result)
}

fn contract_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Contract> {
    let salario_anual = optional_f64(row, "salario_anual")?
        .or_else(|| optional_f64(row, "salario").ok().flatten())
        .unwrap_or(0.0);

    // status, papel e tipo são campos obrigatórios com semântica crítica.
    // Erros de leitura (NULL, coluna ausente, valor desconhecido) devem ser
    // propagados, não silenciados em defaults que distorcem o estado do mundo.
    let status_str: String = row.get("status")?;
    let papel_str: String = row.get("papel")?;
    let tipo_str: String = row.get("tipo")?;

    Ok(Contract {
        id: row.get("id")?,
        piloto_id: row.get("piloto_id")?,
        piloto_nome: optional_string(row, "piloto_nome")?.unwrap_or_default(),
        equipe_id: row.get("equipe_id")?,
        equipe_nome: optional_string(row, "equipe_nome")?.unwrap_or_default(),
        temporada_inicio: parse_i32_column(row, "temporada_inicio")?.unwrap_or(0),
        duracao_anos: parse_i32_column(row, "duracao_anos")?.unwrap_or(1),
        temporada_fim: parse_i32_column(row, "temporada_fim")?.unwrap_or(0),
        salario_anual,
        papel: parse_contract_role(&papel_str)?,
        status: parse_contract_status(&status_str)?,
        tipo: parse_contract_tipo(&tipo_str)?,
        categoria: optional_string(row, "categoria")?.unwrap_or_default(),
        classe: optional_string(row, "classe")?,
        created_at: optional_string(row, "created_at")?.unwrap_or_default(),
    })
}

fn parse_contract_status(s: &str) -> rusqlite::Result<ContractStatus> {
    match s {
        "Ativo" => Ok(ContractStatus::Ativo),
        "Expirado" => Ok(ContractStatus::Expirado),
        "Rescindido" => Ok(ContractStatus::Rescindido),
        "Pendente" => Ok(ContractStatus::Pendente),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "status de contrato desconhecido: '{other}'"
        ))),
    }
}

fn parse_contract_tipo(s: &str) -> rusqlite::Result<ContractType> {
    ContractType::from_str_strict(s).map_err(rusqlite::Error::InvalidParameterName)
}

fn parse_contract_role(s: &str) -> rusqlite::Result<TeamRole> {
    match s {
        "Numero1" | "N1" | "Titular" => Ok(TeamRole::Numero1),
        "Numero2" | "N2" | "Reserva" | "Junior" => Ok(TeamRole::Numero2),
        other => Err(rusqlite::Error::InvalidParameterName(format!(
            "papel de contrato desconhecido: '{other}'"
        ))),
    }
}

fn optional_string(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<String>> {
    match row.get(column_name) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::InvalidColumnName(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnIndex(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnType(_, _, _)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn optional_f64(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<f64>> {
    match row.get(column_name) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::InvalidColumnName(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnIndex(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnType(_, _, _)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn parse_i32_column(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<i32>> {
    match row.get_ref(column_name) {
        Ok(ValueRef::Null) => Ok(None),
        Ok(ValueRef::Integer(value)) => Ok(Some(value as i32)),
        Ok(ValueRef::Real(value)) => Ok(Some(value.round() as i32)),
        Ok(ValueRef::Text(bytes)) => {
            let text = std::str::from_utf8(bytes).ok().map(str::trim).unwrap_or("");
            Ok(text.parse::<i32>().ok())
        }
        Ok(ValueRef::Blob(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnName(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnIndex(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::contract::Contract;

    #[test]
    fn test_insert_and_get_contract() {
        let conn = setup_test_db().expect("test db");
        let contract = sample_contract("C001", "P001", "T001", ContractStatus::Ativo);

        insert_contract(&conn, &contract).expect("insert contract");
        let loaded = get_contract_by_id(&conn, "C001")
            .expect("query contract")
            .expect("contract should exist");

        assert_eq!(loaded.id, "C001");
        assert_eq!(loaded.piloto_nome, contract.piloto_nome);
        assert_eq!(loaded.equipe_nome, contract.equipe_nome);
        assert_eq!(loaded.papel, TeamRole::Numero1);
    }

    #[test]
    fn test_get_active_contract_for_pilot() {
        let conn = setup_test_db().expect("test db");
        let expired = sample_contract("C001", "P001", "T001", ContractStatus::Expirado);
        let active = sample_contract("C002", "P001", "T002", ContractStatus::Ativo);
        insert_contracts(&conn, &[expired, active]).expect("insert contracts");

        let loaded = get_active_contract_for_pilot(&conn, "P001")
            .expect("query active contract")
            .expect("active contract should exist");

        assert_eq!(loaded.id, "C002");
        assert_eq!(loaded.equipe_id, "T002");
    }

    #[test]
    fn test_get_active_contracts_for_team() {
        let conn = setup_test_db().expect("test db");
        insert_contract(
            &conn,
            &sample_contract("C001", "P001", "T001", ContractStatus::Ativo),
        )
        .expect("insert 1");
        insert_contract(
            &conn,
            &sample_contract("C002", "P002", "T001", ContractStatus::Ativo),
        )
        .expect("insert 2");
        insert_contract(
            &conn,
            &sample_contract("C003", "P003", "T001", ContractStatus::Expirado),
        )
        .expect("insert 3");

        let contracts = get_active_contracts_for_team(&conn, "T001").expect("query team contracts");

        assert_eq!(contracts.len(), 2);
        assert!(contracts
            .iter()
            .all(|contract| contract.status == ContractStatus::Ativo));
    }

    #[test]
    fn test_expire_ending_contracts() {
        let conn = setup_test_db().expect("test db");
        let mut contract = sample_contract("C001", "P001", "T001", ContractStatus::Ativo);
        contract.temporada_fim = 3;
        insert_contract(&conn, &contract).expect("insert contract");

        let updated = expire_ending_contracts(&conn, 3).expect("expire contracts");
        assert_eq!(updated, 1);

        let loaded = get_contract_by_id(&conn, "C001")
            .expect("query contract")
            .expect("contract should exist");
        assert_eq!(loaded.status, ContractStatus::Expirado);
    }

    #[test]
    fn test_get_expiring_contracts() {
        let conn = setup_test_db().expect("test db");
        let mut expiring = sample_contract("C001", "P001", "T001", ContractStatus::Ativo);
        expiring.temporada_fim = 4;
        let mut long = sample_contract("C002", "P002", "T002", ContractStatus::Ativo);
        long.temporada_fim = 5;
        insert_contracts(&conn, &[expiring, long]).expect("insert contracts");

        let expiring_contracts =
            get_expiring_contracts(&conn, 4).expect("query expiring contracts");

        assert_eq!(expiring_contracts.len(), 1);
        assert_eq!(expiring_contracts[0].id, "C001");
    }

    #[test]
    fn test_count_active_contracts_for_team() {
        let conn = setup_test_db().expect("test db");
        insert_contract(
            &conn,
            &sample_contract("C001", "P001", "T001", ContractStatus::Ativo),
        )
        .expect("insert 1");
        insert_contract(
            &conn,
            &sample_contract("C002", "P002", "T001", ContractStatus::Ativo),
        )
        .expect("insert 2");
        insert_contract(
            &conn,
            &sample_contract("C003", "P003", "T001", ContractStatus::Rescindido),
        )
        .expect("insert 3");

        let count = count_active_contracts_for_team(&conn, "T001").expect("count active");
        assert_eq!(count, 2);
    }

    fn sample_contract(
        id: &str,
        piloto_id: &str,
        equipe_id: &str,
        status: ContractStatus,
    ) -> Contract {
        Contract {
            id: id.to_string(),
            piloto_id: piloto_id.to_string(),
            piloto_nome: format!("Piloto {}", &piloto_id[1..]),
            equipe_id: equipe_id.to_string(),
            equipe_nome: format!("Equipe {}", &equipe_id[1..]),
            temporada_inicio: 1,
            duracao_anos: 2,
            temporada_fim: 2,
            salario_anual: 100_000.0,
            papel: TeamRole::Numero1,
            status,
            tipo: ContractType::Regular,
            categoria: "gt3".to_string(),
            classe: None,
            created_at: "2026-01-01T12:00:00".to_string(),
        }
    }

    #[test]
    fn test_unknown_contract_status_returns_error() {
        let conn = setup_test_db().expect("test db");
        conn.execute(
            "INSERT INTO contracts (
                id, piloto_id, piloto_nome, equipe_id, equipe_nome,
                temporada_inicio, duracao_anos, temporada_fim,
                salario, salario_anual, papel, status, tipo, categoria, created_at
             ) VALUES ('C_BAD', 'P001', 'Piloto 1', 'T001', 'Equipe', 1, 1, 2,
                       100000, 100000, 'Numero1', 'Suspenso', 'Regular', 'gt3', '2026-01-01')",
            [],
        )
        .expect("insert contract with unknown status");

        let result = get_contract_by_id(&conn, "C_BAD");
        assert!(
            result.is_err(),
            "status desconhecido deve retornar erro, nao default silencioso"
        );
    }

    #[test]
    fn test_unknown_contract_role_returns_error() {
        let conn = setup_test_db().expect("test db");
        conn.execute(
            "INSERT INTO contracts (
                id, piloto_id, piloto_nome, equipe_id, equipe_nome,
                temporada_inicio, duracao_anos, temporada_fim,
                salario, salario_anual, papel, status, tipo, categoria, created_at
             ) VALUES ('C_BAD2', 'P001', 'Piloto 1', 'T001', 'Equipe', 1, 1, 2,
                       100000, 100000, 'Wildcard', 'Ativo', 'Regular', 'gt3', '2026-01-01')",
            [],
        )
        .expect("insert contract with unknown role");

        let result = get_contract_by_id(&conn, "C_BAD2");
        assert!(
            result.is_err(),
            "papel desconhecido deve retornar erro, nao default silencioso"
        );
    }

    fn setup_test_db() -> Result<Connection, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE drivers (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL
            );
            INSERT INTO drivers (id, nome) VALUES
                ('P001', 'Piloto 1'),
                ('P002', 'Piloto 2'),
                ('P003', 'Piloto 3');

            CREATE TABLE contracts (
                id TEXT PRIMARY KEY NOT NULL,
                piloto_id TEXT NOT NULL,
                piloto_nome TEXT NOT NULL,
                equipe_id TEXT NOT NULL,
                equipe_nome TEXT NOT NULL,
                temporada_inicio INTEGER NOT NULL,
                duracao_anos INTEGER NOT NULL,
                temporada_fim INTEGER NOT NULL,
                salario REAL NOT NULL DEFAULT 0.0,
                salario_anual REAL NOT NULL DEFAULT 0.0,
                papel TEXT NOT NULL DEFAULT 'Numero2',
                status TEXT NOT NULL DEFAULT 'Ativo',
                tipo TEXT NOT NULL DEFAULT 'Regular',
                categoria TEXT NOT NULL,
                classe TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(conn)
    }
}
