use rusqlite::{params, types::FromSql, Connection, OptionalExtension};

use crate::db::connection::DbError;
use crate::models::team::{placeholder_team_from_db, HierarchyStatus, Team};

pub fn insert_team(conn: &Connection, team: &Team) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO teams (
            id, nome, nome_curto, cor_primaria, cor_secundaria, pais_sede,
            ano_fundacao, categoria, ativa, marca, classe, piloto_1_id, piloto_2_id,
            is_player_team, car_performance, reliability, budget, facilities,
            engineering, prestige, morale, aerodinamica, motor, chassi,
            hierarquia_n1_id, hierarquia_n2_id, hierarquia_status, hierarquia_tensao,
            hierarquia_duelos_total, hierarquia_duelos_n2_vencidos, hierarquia_sequencia_n2,
            hierarquia_sequencia_n1, hierarquia_inversoes_temporada,
            parent_team_id, aceita_rookies, meta_posicao, stats_vitorias, stats_podios,
            stats_poles, stats_pontos, stats_melhor_resultado, temp_pontos,
            temp_posicao, temp_vitorias, historico_vitorias, historico_podios,
            historico_poles, historico_pontos, historico_titulos_pilotos,
            carreira_titulos, carreira_vitorias, temporada_atual, created_at, updated_at
        ) VALUES (
            :id, :nome, :nome_curto, :cor_primaria, :cor_secundaria, :pais_sede,
            :ano_fundacao, :categoria, :ativa, :marca, :classe, :piloto_1_id, :piloto_2_id,
            :is_player_team, :car_performance, :reliability, :budget, :facilities,
            :engineering, :prestige, :morale, :aerodinamica, :motor, :chassi,
            :hierarquia_n1_id, :hierarquia_n2_id, :hierarquia_status, :hierarquia_tensao,
            :hierarquia_duelos_total, :hierarquia_duelos_n2_vencidos, :hierarquia_sequencia_n2,
            :hierarquia_sequencia_n1, :hierarquia_inversoes_temporada,
            :parent_team_id, :aceita_rookies, :meta_posicao, :stats_vitorias, :stats_podios,
            :stats_poles, :stats_pontos, :stats_melhor_resultado, :temp_pontos,
            :temp_posicao, :temp_vitorias, :historico_vitorias, :historico_podios,
            :historico_poles, :historico_pontos, :historico_titulos_pilotos,
            :carreira_titulos, :carreira_vitorias, :temporada_atual, :created_at, :updated_at
        )",
        rusqlite::named_params! {
            ":id": &team.id,
            ":nome": &team.nome,
            ":nome_curto": &team.nome_curto,
            ":cor_primaria": &team.cor_primaria,
            ":cor_secundaria": &team.cor_secundaria,
            ":pais_sede": &team.pais_sede,
            ":ano_fundacao": team.ano_fundacao,
            ":categoria": &team.categoria,
            ":ativa": team.ativa as i64,
            ":marca": &team.marca,
            ":classe": &team.classe,
            ":piloto_1_id": &team.piloto_1_id,
            ":piloto_2_id": &team.piloto_2_id,
            ":is_player_team": team.is_player_team as i64,
            ":car_performance": team.car_performance,
            ":reliability": team.confiabilidade,
            ":budget": team.budget,
            ":facilities": team.facilities,
            ":engineering": team.engineering,
            ":prestige": team.reputacao,
            ":morale": team.morale,
            ":aerodinamica": team.aerodinamica,
            ":motor": team.motor,
            ":chassi": team.chassi,
            ":hierarquia_n1_id": &team.hierarquia_n1_id,
            ":hierarquia_n2_id": &team.hierarquia_n2_id,
            ":hierarquia_status": &team.hierarquia_status,
            ":hierarquia_tensao": team.hierarquia_tensao,
            ":hierarquia_duelos_total": team.hierarquia_duelos_total,
            ":hierarquia_duelos_n2_vencidos": team.hierarquia_duelos_n2_vencidos,
            ":hierarquia_sequencia_n2": team.hierarquia_sequencia_n2,
            ":hierarquia_sequencia_n1": team.hierarquia_sequencia_n1,
            ":hierarquia_inversoes_temporada": team.hierarquia_inversoes_temporada,
            ":parent_team_id": &team.parent_team_id,
            ":aceita_rookies": team.aceita_rookies as i64,
            ":meta_posicao": team.meta_posicao,
            ":stats_vitorias": team.stats_vitorias,
            ":stats_podios": team.stats_podios,
            ":stats_poles": team.stats_poles,
            ":stats_pontos": team.stats_pontos,
            ":stats_melhor_resultado": team.stats_melhor_resultado,
            ":temp_pontos": team.stats_pontos as f64,
            ":temp_posicao": team.temp_posicao,
            ":temp_vitorias": team.stats_vitorias,
            ":historico_vitorias": team.historico_vitorias,
            ":historico_podios": team.historico_podios,
            ":historico_poles": team.historico_poles,
            ":historico_pontos": team.historico_pontos,
            ":historico_titulos_pilotos": team.historico_titulos_pilotos,
            ":carreira_titulos": team.historico_titulos_construtores,
            ":carreira_vitorias": team.historico_vitorias,
            ":temporada_atual": team.temporada_atual,
            ":created_at": &team.created_at,
            ":updated_at": &team.updated_at,
        },
    )?;
    Ok(())
}

pub fn insert_teams(conn: &Connection, teams: &[Team]) -> Result<(), DbError> {
    for team in teams {
        insert_team(conn, team)?;
    }
    Ok(())
}

pub fn get_team_by_id(conn: &Connection, id: &str) -> Result<Option<Team>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM teams WHERE id = ?1")?;
    let team = stmt.query_row(params![id], team_from_row).optional()?;
    Ok(team)
}

pub fn get_all_teams(conn: &Connection) -> Result<Vec<Team>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM teams ORDER BY nome")?;
    let mapped = stmt.query_map([], team_from_row)?;
    let teams = collect_teams(mapped)?;
    Ok(teams)
}

pub fn get_teams_by_category(conn: &Connection, category_id: &str) -> Result<Vec<Team>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM teams WHERE categoria = ?1 ORDER BY nome")?;
    let mapped = stmt.query_map(params![category_id], team_from_row)?;
    let teams = collect_teams(mapped)?;
    Ok(teams)
}

/// Equipes de uma categoria filtradas por classe, ordenadas por desempenho desc.
/// Usado na convocação especial para montar o grid classe a classe.
pub fn get_teams_by_category_and_class(
    conn: &Connection,
    categoria: &str,
    classe: &str,
) -> Result<Vec<crate::models::team::Team>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM teams WHERE categoria = ?1 AND classe = ?2 ORDER BY car_performance DESC",
    )?;
    let mapped = stmt.query_map(params![categoria, classe], team_from_row)?;
    collect_teams(mapped)
}

/// Limpa `piloto_1_id` e `piloto_2_id` de todas as equipes especiais.
/// Afeta production_challenger (mazda/toyota/bmw) e endurance (gt4/gt3/lmp2).
/// Equipes LMP2 nunca recebem lineup neste redesign inicial, portanto a operação
/// sobre elas é inócua — o WHERE não as exclui explicitamente para manter a
/// semântica de "limpar tudo das categorias especiais".
pub fn clear_special_team_lineups(conn: &Connection) -> Result<usize, DbError> {
    let n = conn.execute(
        "UPDATE teams SET piloto_1_id = NULL, piloto_2_id = NULL
         WHERE categoria IN ('production_challenger', 'endurance')",
        [],
    )?;
    Ok(n)
}

/// Reseta todos os campos de hierarquia das equipes especiais.
/// Mesma nota de LMP2: afeta toda a categoria endurance, mas LMP2 está sempre
/// sem lineup, então o reset é inócuo para essas equipes.
pub fn reset_special_team_hierarchies(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams SET
            hierarquia_n1_id = NULL, hierarquia_n2_id = NULL,
            hierarquia_status = 'estavel', hierarquia_tensao = 0.0,
            hierarquia_duelos_total = 0, hierarquia_duelos_n2_vencidos = 0,
            hierarquia_sequencia_n2 = 0, hierarquia_sequencia_n1 = 0,
            hierarquia_inversoes_temporada = 0
         WHERE categoria IN ('production_challenger', 'endurance')",
        [],
    )?;
    Ok(())
}

pub fn update_team(conn: &Connection, team: &Team) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams SET
            nome = :nome,
            nome_curto = :nome_curto,
            cor_primaria = :cor_primaria,
            cor_secundaria = :cor_secundaria,
            pais_sede = :pais_sede,
            ano_fundacao = :ano_fundacao,
            categoria = :categoria,
            ativa = :ativa,
            marca = :marca,
            classe = :classe,
            piloto_1_id = :piloto_1_id,
            piloto_2_id = :piloto_2_id,
            is_player_team = :is_player_team,
            car_performance = :car_performance,
            reliability = :reliability,
            budget = :budget,
            facilities = :facilities,
            engineering = :engineering,
            prestige = :prestige,
            morale = :morale,
            aerodinamica = :aerodinamica,
            motor = :motor,
            chassi = :chassi,
            hierarquia_n1_id = :hierarquia_n1_id,
            hierarquia_n2_id = :hierarquia_n2_id,
            hierarquia_status = :hierarquia_status,
            hierarquia_tensao = :hierarquia_tensao,
            hierarquia_duelos_total = :hierarquia_duelos_total,
            hierarquia_duelos_n2_vencidos = :hierarquia_duelos_n2_vencidos,
            hierarquia_sequencia_n2 = :hierarquia_sequencia_n2,
            hierarquia_sequencia_n1 = :hierarquia_sequencia_n1,
            hierarquia_inversoes_temporada = :hierarquia_inversoes_temporada,
            parent_team_id = :parent_team_id,
            aceita_rookies = :aceita_rookies,
            meta_posicao = :meta_posicao,
            stats_vitorias = :stats_vitorias,
            stats_podios = :stats_podios,
            stats_poles = :stats_poles,
            stats_pontos = :stats_pontos,
            stats_melhor_resultado = :stats_melhor_resultado,
            temp_pontos = :temp_pontos,
            temp_posicao = :temp_posicao,
            temp_vitorias = :temp_vitorias,
            historico_vitorias = :historico_vitorias,
            historico_podios = :historico_podios,
            historico_poles = :historico_poles,
            historico_pontos = :historico_pontos,
            historico_titulos_pilotos = :historico_titulos_pilotos,
            carreira_titulos = :carreira_titulos,
            carreira_vitorias = :carreira_vitorias,
            temporada_atual = :temporada_atual,
            updated_at = :updated_at
        WHERE id = :id",
        rusqlite::named_params! {
            ":id": &team.id,
            ":nome": &team.nome,
            ":nome_curto": &team.nome_curto,
            ":cor_primaria": &team.cor_primaria,
            ":cor_secundaria": &team.cor_secundaria,
            ":pais_sede": &team.pais_sede,
            ":ano_fundacao": team.ano_fundacao,
            ":categoria": &team.categoria,
            ":ativa": team.ativa as i64,
            ":marca": &team.marca,
            ":classe": &team.classe,
            ":piloto_1_id": &team.piloto_1_id,
            ":piloto_2_id": &team.piloto_2_id,
            ":is_player_team": team.is_player_team as i64,
            ":car_performance": team.car_performance,
            ":reliability": team.confiabilidade,
            ":budget": team.budget,
            ":facilities": team.facilities,
            ":engineering": team.engineering,
            ":prestige": team.reputacao,
            ":morale": team.morale,
            ":aerodinamica": team.aerodinamica,
            ":motor": team.motor,
            ":chassi": team.chassi,
            ":hierarquia_n1_id": &team.hierarquia_n1_id,
            ":hierarquia_n2_id": &team.hierarquia_n2_id,
            ":hierarquia_status": &team.hierarquia_status,
            ":hierarquia_tensao": team.hierarquia_tensao,
            ":hierarquia_duelos_total": team.hierarquia_duelos_total,
            ":hierarquia_duelos_n2_vencidos": team.hierarquia_duelos_n2_vencidos,
            ":hierarquia_sequencia_n2": team.hierarquia_sequencia_n2,
            ":hierarquia_sequencia_n1": team.hierarquia_sequencia_n1,
            ":hierarquia_inversoes_temporada": team.hierarquia_inversoes_temporada,
            ":parent_team_id": &team.parent_team_id,
            ":aceita_rookies": team.aceita_rookies as i64,
            ":meta_posicao": team.meta_posicao,
            ":stats_vitorias": team.stats_vitorias,
            ":stats_podios": team.stats_podios,
            ":stats_poles": team.stats_poles,
            ":stats_pontos": team.stats_pontos,
            ":stats_melhor_resultado": team.stats_melhor_resultado,
            ":temp_pontos": team.stats_pontos as f64,
            ":temp_posicao": team.temp_posicao,
            ":temp_vitorias": team.stats_vitorias,
            ":historico_vitorias": team.historico_vitorias,
            ":historico_podios": team.historico_podios,
            ":historico_poles": team.historico_poles,
            ":historico_pontos": team.historico_pontos,
            ":historico_titulos_pilotos": team.historico_titulos_pilotos,
            ":carreira_titulos": team.historico_titulos_construtores,
            ":carreira_vitorias": team.historico_vitorias,
            ":temporada_atual": team.temporada_atual,
            ":updated_at": &team.updated_at,
        },
    )?;
    Ok(())
}

pub fn update_team_pilots(
    conn: &Connection,
    team_id: &str,
    piloto_1_id: Option<&str>,
    piloto_2_id: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams SET piloto_1_id = ?1, piloto_2_id = ?2 WHERE id = ?3",
        params![piloto_1_id, piloto_2_id, team_id],
    )?;
    Ok(())
}

pub fn update_team_hierarchy(
    conn: &Connection,
    team_id: &str,
    n1_id: Option<&str>,
    n2_id: Option<&str>,
    status: &str,
    tensao: f64,
) -> Result<(), DbError> {
    let normalized = HierarchyStatus::from_str(status).as_str().to_string();
    conn.execute(
        "UPDATE teams
         SET hierarquia_n1_id = ?1,
             hierarquia_n2_id = ?2,
             hierarquia_status = ?3,
             hierarquia_tensao = ?4
         WHERE id = ?5",
        params![n1_id, n2_id, normalized, tensao, team_id],
    )?;
    Ok(())
}

/// Persiste todos os 9 campos da hierarquia interna de uma equipe de uma vez.
/// Use este após processar o sistema de hierarquia pós-corrida.
pub fn update_team_hierarchy_full(conn: &Connection, team: &Team) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams
         SET hierarquia_n1_id = ?1,
             hierarquia_n2_id = ?2,
             hierarquia_status = ?3,
             hierarquia_tensao = ?4,
             hierarquia_duelos_total = ?5,
             hierarquia_duelos_n2_vencidos = ?6,
             hierarquia_sequencia_n2 = ?7,
             hierarquia_sequencia_n1 = ?8,
             hierarquia_inversoes_temporada = ?9
         WHERE id = ?10",
        rusqlite::params![
            &team.hierarquia_n1_id,
            &team.hierarquia_n2_id,
            &team.hierarquia_status,
            team.hierarquia_tensao,
            team.hierarquia_duelos_total,
            team.hierarquia_duelos_n2_vencidos,
            team.hierarquia_sequencia_n2,
            team.hierarquia_sequencia_n1,
            team.hierarquia_inversoes_temporada,
            &team.id,
        ],
    )?;
    Ok(())
}

pub fn update_team_duel_counters(
    conn: &Connection,
    team_id: &str,
    duelos_total: i32,
    duelos_n2_vencidos: i32,
    sequencia_n2: i32,
    sequencia_n1: i32,
    inversoes_temporada: i32,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams
         SET hierarquia_duelos_total = ?1,
             hierarquia_duelos_n2_vencidos = ?2,
             hierarquia_sequencia_n2 = ?3,
             hierarquia_sequencia_n1 = ?4,
             hierarquia_inversoes_temporada = ?5
         WHERE id = ?6",
        params![duelos_total, duelos_n2_vencidos, sequencia_n2, sequencia_n1, inversoes_temporada, team_id],
    )?;
    Ok(())
}

pub fn remove_pilot_from_team(
    conn: &Connection,
    driver_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    let team = get_team_by_id(conn, team_id)?
        .ok_or_else(|| DbError::NotFound(format!("Equipe '{team_id}' nao encontrada")))?;
    let piloto_1 = if team.piloto_1_id.as_deref() == Some(driver_id) {
        None
    } else {
        team.piloto_1_id.as_deref()
    };
    let piloto_2 = if team.piloto_2_id.as_deref() == Some(driver_id) {
        None
    } else {
        team.piloto_2_id.as_deref()
    };
    update_team_pilots(conn, team_id, piloto_1, piloto_2)?;
    Ok(())
}

pub fn update_team_season_stats(
    conn: &Connection,
    team_id: &str,
    vitorias: i32,
    podios: i32,
    poles: i32,
    pontos: i32,
    melhor_resultado: i32,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams
         SET stats_vitorias = ?1,
             stats_podios = ?2,
             stats_poles = ?3,
             stats_pontos = ?4,
             stats_melhor_resultado = ?5,
             temp_vitorias = ?1,
             temp_pontos = ?6
         WHERE id = ?7",
        params![
            vitorias,
            podios,
            poles,
            pontos,
            melhor_resultado,
            pontos as f64,
            team_id
        ],
    )?;
    Ok(())
}

pub fn reset_team_season_stats(conn: &Connection, team_id: &str) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams
         SET stats_vitorias = 0,
             stats_podios = 0,
             stats_poles = 0,
             stats_pontos = 0,
             stats_melhor_resultado = 99,
             temp_vitorias = 0,
             temp_pontos = 0.0,
             temp_posicao = 0
         WHERE id = ?1",
        params![team_id],
    )?;
    Ok(())
}

pub fn update_team_morale(conn: &Connection, team_id: &str, morale: f64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE teams SET morale = ?1 WHERE id = ?2",
        params![morale, team_id],
    )?;
    Ok(())
}

pub fn delete_team(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM teams WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn count_teams_by_category(conn: &Connection, category_id: &str) -> Result<i32, DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM teams WHERE categoria = ?1",
        params![category_id],
        |row| row.get(0),
    )?;
    Ok(count as i32)
}

fn collect_teams(
    mapped: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Team>>,
) -> Result<Vec<Team>, DbError> {
    let mut result = Vec::new();
    for row in mapped {
        result.push(row?);
    }
    Ok(result)
}

fn team_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Team> {
    let id: String = row.get("id")?;
    let nome: String = row.get("nome")?;
    let categoria: String = row.get("categoria")?;
    let created_at: String = row.get("created_at")?;
    let mut team = placeholder_team_from_db(id, nome, categoria, created_at);

    team.is_player_team = row.get::<_, i64>("is_player_team").unwrap_or(0) != 0;
    team.car_performance = row.get("car_performance").unwrap_or(50.0);
    team.confiabilidade = row.get("reliability").unwrap_or(50.0);
    team.budget = row.get("budget").unwrap_or(50.0);
    team.facilities = row.get("facilities").unwrap_or(50.0);
    team.engineering = row.get("engineering").unwrap_or(50.0);
    team.reputacao = row.get("prestige").unwrap_or(50.0);
    team.morale = row.get("morale").unwrap_or(1.0);
    team.aerodinamica = row.get("aerodinamica").unwrap_or(50.0);
    team.motor = row.get("motor").unwrap_or(50.0);
    team.chassi = row.get("chassi").unwrap_or(50.0);
    team.hierarquia_status = row
        .get::<_, String>("hierarquia_status")
        .map(|value| HierarchyStatus::from_str(&value).as_str().to_string())
        .unwrap_or_else(|_| HierarchyStatus::Estavel.as_str().to_string());
    team.parent_team_id = optional_column(row, "parent_team_id")?;
    team.aceita_rookies = row.get::<_, i64>("aceita_rookies").unwrap_or(1) != 0;
    team.meta_posicao = row.get::<_, i64>("meta_posicao").unwrap_or(10) as i32;
    team.stats_pontos = optional_column::<i64>(row, "stats_pontos")?
        .map(|value| value as i32)
        .or_else(|| {
            row.get::<_, f64>("temp_pontos")
                .ok()
                .map(|value| value.round() as i32)
        })
        .unwrap_or(0);
    team.temp_posicao = row.get::<_, i64>("temp_posicao").unwrap_or(0) as i32;
    team.stats_vitorias = optional_column::<i64>(row, "stats_vitorias")?
        .map(|value| value as i32)
        .unwrap_or_else(|| row.get::<_, i64>("temp_vitorias").unwrap_or(0) as i32);
    team.historico_titulos_construtores = row.get::<_, i64>("carreira_titulos").unwrap_or(0) as i32;
    team.historico_vitorias = optional_column::<i64>(row, "historico_vitorias")?
        .map(|value| value as i32)
        .unwrap_or_else(|| row.get::<_, i64>("carreira_vitorias").unwrap_or(0) as i32);

    team.nome_curto =
        optional_column::<String>(row, "nome_curto")?.unwrap_or_else(|| team.nome.clone());
    team.cor_primaria =
        optional_column::<String>(row, "cor_primaria")?.unwrap_or_else(|| "#FFFFFF".to_string());
    team.cor_secundaria =
        optional_column::<String>(row, "cor_secundaria")?.unwrap_or_else(|| "#000000".to_string());
    team.pais_sede =
        optional_column::<String>(row, "pais_sede")?.unwrap_or_else(|| "Unknown".to_string());
    team.ano_fundacao = optional_column::<i64>(row, "ano_fundacao")?.unwrap_or(2024) as i32;
    team.ativa = optional_column::<i64>(row, "ativa")?.unwrap_or(1) != 0;
    team.marca = optional_column(row, "marca")?;
    team.classe = optional_column(row, "classe")?;
    team.piloto_1_id = optional_column(row, "piloto_1_id")?;
    team.piloto_2_id = optional_column(row, "piloto_2_id")?;
    team.hierarquia_n1_id = optional_column(row, "hierarquia_n1_id")?;
    team.hierarquia_n2_id = optional_column(row, "hierarquia_n2_id")?;
    team.hierarquia_tensao = optional_column::<f64>(row, "hierarquia_tensao")?.unwrap_or(0.0);
    team.hierarquia_duelos_total = optional_column::<i64>(row, "hierarquia_duelos_total")?.unwrap_or(0) as i32;
    team.hierarquia_duelos_n2_vencidos = optional_column::<i64>(row, "hierarquia_duelos_n2_vencidos")?.unwrap_or(0) as i32;
    team.hierarquia_sequencia_n2 = optional_column::<i64>(row, "hierarquia_sequencia_n2")?.unwrap_or(0) as i32;
    team.hierarquia_sequencia_n1 = optional_column::<i64>(row, "hierarquia_sequencia_n1")?.unwrap_or(0) as i32;
    team.hierarquia_inversoes_temporada = optional_column::<i64>(row, "hierarquia_inversoes_temporada")?.unwrap_or(0) as i32;
    team.stats_podios = optional_column::<i64>(row, "stats_podios")?.unwrap_or(0) as i32;
    team.stats_poles = optional_column::<i64>(row, "stats_poles")?.unwrap_or(0) as i32;
    team.stats_melhor_resultado =
        optional_column::<i64>(row, "stats_melhor_resultado")?.unwrap_or(99) as i32;
    team.historico_podios = optional_column::<i64>(row, "historico_podios")?.unwrap_or(0) as i32;
    team.historico_poles = optional_column::<i64>(row, "historico_poles")?.unwrap_or(0) as i32;
    team.historico_pontos = optional_column::<i64>(row, "historico_pontos")?.unwrap_or(0) as i32;
    team.historico_titulos_pilotos =
        optional_column::<i64>(row, "historico_titulos_pilotos")?.unwrap_or(0) as i32;
    team.temporada_atual = optional_column::<i64>(row, "temporada_atual")?.unwrap_or(1) as i32;
    team.updated_at =
        optional_column::<String>(row, "updated_at")?.unwrap_or_else(|| team.created_at.clone());

    Ok(team)
}

fn optional_column<T>(row: &rusqlite::Row<'_>, column_name: &str) -> rusqlite::Result<Option<T>>
where
    T: FromSql,
{
    match row.get(column_name) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::InvalidColumnName(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnIndex(_)) => Ok(None),
        Err(rusqlite::Error::InvalidColumnType(_, _, _)) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::constants::teams::get_team_templates;
    use crate::models::team::Team;

    #[test]
    fn test_insert_and_get_team() {
        let conn = setup_test_db().expect("test db");
        let team = sample_team("gt3", "T001");

        insert_team(&conn, &team).expect("insert team");
        let loaded = get_team_by_id(&conn, "T001")
            .expect("get team")
            .expect("team should exist");

        assert_eq!(loaded.id, "T001");
        assert_eq!(loaded.nome, team.nome);
        assert_eq!(loaded.categoria, "gt3");
        assert_eq!(loaded.stats_vitorias, 0);
    }

    #[test]
    fn test_insert_and_get_team_persists_extended_fields() {
        let conn = setup_test_db().expect("test db");
        let mut team = sample_team("gt3", "T010");
        team.piloto_1_id = Some("P001".to_string());
        team.piloto_2_id = Some("P002".to_string());
        team.hierarquia_n1_id = Some("P001".to_string());
        team.hierarquia_n2_id = Some("P002".to_string());
        team.hierarquia_tensao = 33.0;
        team.stats_podios = 4;
        team.stats_poles = 2;
        team.stats_pontos = 87;
        team.stats_melhor_resultado = 1;
        team.historico_podios = 12;
        team.historico_poles = 5;
        team.historico_pontos = 230;
        team.historico_titulos_pilotos = 1;

        insert_team(&conn, &team).expect("insert team");
        update_team_pilots(&conn, "T010", Some("P001"), Some("P002")).expect("update pilots");
        update_team_hierarchy(
            &conn,
            "T010",
            Some("P001"),
            Some("P002"),
            "competitivo",
            33.0,
        )
        .expect("update hierarchy");
        update_team_season_stats(&conn, "T010", 3, 4, 2, 87, 1).expect("update season stats");

        let loaded = get_team_by_id(&conn, "T010")
            .expect("get team")
            .expect("team should exist");

        assert_eq!(loaded.nome_curto, team.nome_curto);
        assert_eq!(loaded.cor_primaria, team.cor_primaria);
        assert_eq!(loaded.cor_secundaria, team.cor_secundaria);
        assert_eq!(loaded.pais_sede, team.pais_sede);
        assert_eq!(loaded.piloto_1_id.as_deref(), Some("P001"));
        assert_eq!(loaded.piloto_2_id.as_deref(), Some("P002"));
        assert_eq!(loaded.hierarquia_n1_id.as_deref(), Some("P001"));
        assert_eq!(loaded.hierarquia_n2_id.as_deref(), Some("P002"));
        assert_eq!(loaded.hierarquia_status, "competitivo");
        assert_eq!(loaded.hierarquia_tensao, 33.0);
        assert_eq!(loaded.stats_podios, 4);
        assert_eq!(loaded.stats_poles, 2);
        assert_eq!(loaded.stats_pontos, 87);
        assert_eq!(loaded.stats_melhor_resultado, 1);
    }

    #[test]
    fn test_get_teams_by_category() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, &sample_team("gt3", "T001")).expect("insert team 1");
        insert_team(&conn, &sample_team("gt3", "T002")).expect("insert team 2");
        insert_team(&conn, &sample_team("gt4", "T003")).expect("insert team 3");

        let gt3_teams = get_teams_by_category(&conn, "gt3").expect("query teams");

        assert_eq!(gt3_teams.len(), 2);
        assert!(gt3_teams.iter().all(|team| team.categoria == "gt3"));
    }

    #[test]
    fn test_update_team_pilots() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, &sample_team("gt3", "T001")).expect("insert team");

        update_team_pilots(&conn, "T001", Some("P001"), Some("P002")).expect("update pilots");
        let loaded = get_team_by_id(&conn, "T001")
            .expect("get team")
            .expect("team should exist");

        assert_eq!(loaded.piloto_1_id.as_deref(), Some("P001"));
        assert_eq!(loaded.piloto_2_id.as_deref(), Some("P002"));
    }

    #[test]
    fn test_count_teams_by_category() {
        let conn = setup_test_db().expect("test db");
        insert_team(&conn, &sample_team("gt3", "T001")).expect("insert team 1");
        insert_team(&conn, &sample_team("gt3", "T002")).expect("insert team 2");
        insert_team(&conn, &sample_team("endurance", "T003")).expect("insert team 3");

        let count = count_teams_by_category(&conn, "gt3").expect("count teams");

        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_pilot_from_team_clears_matching_slot() {
        let conn = setup_test_db().expect("test db");
        let mut team = sample_team("gt3", "T001");
        team.piloto_1_id = Some("P001".to_string());
        team.piloto_2_id = Some("P002".to_string());
        insert_team(&conn, &team).expect("insert team");

        remove_pilot_from_team(&conn, "P002", "T001").expect("remove pilot");

        let refreshed = get_team_by_id(&conn, "T001")
            .expect("team query")
            .expect("team");
        assert_eq!(refreshed.piloto_1_id.as_deref(), Some("P001"));
        assert!(refreshed.piloto_2_id.is_none());
    }

    fn sample_team(category_id: &str, team_id: &str) -> Team {
        let template = get_team_templates(category_id)[0];
        let mut rng = StdRng::seed_from_u64(55);
        Team::from_template_with_rng(template, category_id, team_id.to_string(), 2026, &mut rng)
    }

    fn setup_test_db() -> Result<Connection, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE teams (
                id TEXT PRIMARY KEY,
                nome TEXT NOT NULL,
                nome_curto TEXT NOT NULL,
                cor_primaria TEXT NOT NULL DEFAULT '#FFFFFF',
                cor_secundaria TEXT NOT NULL DEFAULT '#000000',
                pais_sede TEXT NOT NULL DEFAULT 'Unknown',
                ano_fundacao INTEGER NOT NULL DEFAULT 2024,
                categoria TEXT NOT NULL,
                ativa INTEGER NOT NULL DEFAULT 1,
                marca TEXT,
                classe TEXT,
                piloto_1_id TEXT,
                piloto_2_id TEXT,
                is_player_team INTEGER NOT NULL DEFAULT 0,
                car_performance REAL NOT NULL DEFAULT 0.0,
                reliability REAL NOT NULL DEFAULT 60.0,
                budget REAL NOT NULL DEFAULT 50.0,
                facilities REAL NOT NULL DEFAULT 50.0,
                engineering REAL NOT NULL DEFAULT 50.0,
                prestige REAL NOT NULL DEFAULT 50.0,
                morale REAL NOT NULL DEFAULT 1.0,
                aerodinamica REAL NOT NULL DEFAULT 50.0,
                motor REAL NOT NULL DEFAULT 50.0,
                chassi REAL NOT NULL DEFAULT 50.0,
                hierarquia_n1_id TEXT,
                hierarquia_n2_id TEXT,
                hierarquia_status TEXT NOT NULL DEFAULT 'estavel',
                hierarquia_tensao REAL NOT NULL DEFAULT 0.0,
                hierarquia_duelos_total INTEGER NOT NULL DEFAULT 0,
                hierarquia_duelos_n2_vencidos INTEGER NOT NULL DEFAULT 0,
                hierarquia_sequencia_n2 INTEGER NOT NULL DEFAULT 0,
                hierarquia_sequencia_n1 INTEGER NOT NULL DEFAULT 0,
                hierarquia_inversoes_temporada INTEGER NOT NULL DEFAULT 0,
                parent_team_id TEXT,
                aceita_rookies INTEGER NOT NULL DEFAULT 1,
                meta_posicao INTEGER NOT NULL DEFAULT 10,
                stats_vitorias INTEGER NOT NULL DEFAULT 0,
                stats_podios INTEGER NOT NULL DEFAULT 0,
                stats_poles INTEGER NOT NULL DEFAULT 0,
                stats_pontos INTEGER NOT NULL DEFAULT 0,
                stats_melhor_resultado INTEGER NOT NULL DEFAULT 99,
                temp_pontos REAL NOT NULL DEFAULT 0.0,
                temp_posicao INTEGER NOT NULL DEFAULT 0,
                temp_vitorias INTEGER NOT NULL DEFAULT 0,
                historico_vitorias INTEGER NOT NULL DEFAULT 0,
                historico_podios INTEGER NOT NULL DEFAULT 0,
                historico_poles INTEGER NOT NULL DEFAULT 0,
                historico_pontos INTEGER NOT NULL DEFAULT 0,
                historico_titulos_pilotos INTEGER NOT NULL DEFAULT 0,
                carreira_titulos INTEGER NOT NULL DEFAULT 0,
                carreira_vitorias INTEGER NOT NULL DEFAULT 0,
                temporada_atual INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            );",
        )?;
        Ok(conn)
    }
}
