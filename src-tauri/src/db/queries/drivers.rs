use rusqlite::Connection;

use crate::db::connection::DbError;
use crate::models::driver::{Driver, DriverAttributes, DriverCareerStats, DriverSeasonStats};
use crate::models::enums::{DriverStatus, PrimaryPersonality, SecondaryPersonality};

// ── INSERT ────────────────────────────────────────────────────────────────────

pub fn insert_driver(conn: &Connection, driver: &Driver) -> Result<(), DbError> {
    let historico =
        serde_json::to_string(&driver.historico_circuitos).unwrap_or_else(|_| "{}".to_string());
    let ultimos =
        serde_json::to_string(&driver.ultimos_resultados).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO drivers (
            id, nome, is_jogador, idade, nacionalidade, genero, categoria_atual,
            categoria_especial_ativa,
            status, personalidade_primaria, personalidade_secundaria, ano_inicio_carreira,
            skill, consistencia, racecraft, defesa, ritmo_classificacao, gestao_pneus,
            habilidade_largada, adaptabilidade, fator_chuva, fitness, experiencia,
            desenvolvimento, aggression, smoothness, midia, mentalidade, confianca,
            temp_pontos, temp_vitorias, temp_podios, temp_poles, temp_corridas, temp_dnfs,
            temp_posicao_media, carreira_pontos_total, carreira_vitorias, carreira_podios,
            carreira_poles, carreira_corridas, carreira_temporadas, carreira_titulos,
            carreira_dnfs, motivacao, historico_circuitos, ultimos_resultados,
            melhor_resultado_temp, temporadas_na_categoria, corridas_na_categoria,
            temporadas_motivacao_baixa
        ) VALUES (
            :id, :nome, :is_jogador, :idade, :nacionalidade, :genero, :categoria_atual,
            :categoria_especial_ativa,
            :status, :personalidade_primaria, :personalidade_secundaria, :ano_inicio_carreira,
            :skill, :consistencia, :racecraft, :defesa, :ritmo_classificacao, :gestao_pneus,
            :habilidade_largada, :adaptabilidade, :fator_chuva, :fitness, :experiencia,
            :desenvolvimento, :aggression, :smoothness, :midia, :mentalidade, :confianca,
            :temp_pontos, :temp_vitorias, :temp_podios, :temp_poles, :temp_corridas, :temp_dnfs,
            :temp_posicao_media, :carreira_pontos_total, :carreira_vitorias, :carreira_podios,
            :carreira_poles, :carreira_corridas, :carreira_temporadas, :carreira_titulos,
            :carreira_dnfs, :motivacao, :historico_circuitos, :ultimos_resultados,
            :melhor_resultado_temp, :temporadas_na_categoria, :corridas_na_categoria,
            :temporadas_motivacao_baixa
        )",
        rusqlite::named_params! {
            ":id":                        &driver.id,
            ":nome":                      &driver.nome,
            ":is_jogador":                driver.is_jogador as i64,
            ":idade":                     driver.idade as i64,
            ":nacionalidade":             &driver.nacionalidade,
            ":genero":                    &driver.genero,
            ":categoria_atual":           &driver.categoria_atual,
            ":categoria_especial_ativa":  &driver.categoria_especial_ativa,
            ":status":                    driver.status.as_str(),
            ":personalidade_primaria":    driver.personalidade_primaria.as_ref().map(|p| p.as_str()),
            ":personalidade_secundaria":  driver.personalidade_secundaria.as_ref().map(|p| p.as_str()),
            ":ano_inicio_carreira":       driver.ano_inicio_carreira as i64,
            ":skill":                     driver.atributos.skill,
            ":consistencia":              driver.atributos.consistencia,
            ":racecraft":                 driver.atributos.racecraft,
            ":defesa":                    driver.atributos.defesa,
            ":ritmo_classificacao":       driver.atributos.ritmo_classificacao,
            ":gestao_pneus":              driver.atributos.gestao_pneus,
            ":habilidade_largada":        driver.atributos.habilidade_largada,
            ":adaptabilidade":            driver.atributos.adaptabilidade,
            ":fator_chuva":               driver.atributos.fator_chuva,
            ":fitness":                   driver.atributos.fitness,
            ":experiencia":               driver.atributos.experiencia,
            ":desenvolvimento":           driver.atributos.desenvolvimento,
            ":aggression":                driver.atributos.aggression,
            ":smoothness":                driver.atributos.smoothness,
            ":midia":                     driver.atributos.midia,
            ":mentalidade":               driver.atributos.mentalidade,
            ":confianca":                 driver.atributos.confianca,
            ":temp_pontos":               driver.stats_temporada.pontos,
            ":temp_vitorias":             driver.stats_temporada.vitorias as i64,
            ":temp_podios":               driver.stats_temporada.podios as i64,
            ":temp_poles":                driver.stats_temporada.poles as i64,
            ":temp_corridas":             driver.stats_temporada.corridas as i64,
            ":temp_dnfs":                 driver.stats_temporada.dnfs as i64,
            ":temp_posicao_media":        driver.stats_temporada.posicao_media,
            ":carreira_pontos_total":     driver.stats_carreira.pontos_total,
            ":carreira_vitorias":         driver.stats_carreira.vitorias as i64,
            ":carreira_podios":           driver.stats_carreira.podios as i64,
            ":carreira_poles":            driver.stats_carreira.poles as i64,
            ":carreira_corridas":         driver.stats_carreira.corridas as i64,
            ":carreira_temporadas":       driver.stats_carreira.temporadas as i64,
            ":carreira_titulos":          driver.stats_carreira.titulos as i64,
            ":carreira_dnfs":             driver.stats_carreira.dnfs as i64,
            ":motivacao":                 driver.motivacao,
            ":historico_circuitos":       &historico,
            ":ultimos_resultados":        &ultimos,
            ":melhor_resultado_temp":     driver.melhor_resultado_temp.map(|v| v as i64),
            ":temporadas_na_categoria":   driver.temporadas_na_categoria as i64,
            ":corridas_na_categoria":     driver.corridas_na_categoria as i64,
            ":temporadas_motivacao_baixa": driver.temporadas_motivacao_baixa as i64,
        },
    )?;
    Ok(())
}

// ── SELECT ────────────────────────────────────────────────────────────────────

pub fn get_driver(conn: &Connection, id: &str) -> Result<Driver, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM drivers WHERE id = ?1")?;
    stmt.query_row(rusqlite::params![id], driver_from_row)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DbError::NotFound(format!("Piloto '{}' não encontrado", id))
            }
            other => DbError::Sqlite(other),
        })
}

pub fn get_driver_by_name(conn: &Connection, nome: &str) -> Result<Driver, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM drivers WHERE nome = ?1 LIMIT 1")?;
    stmt.query_row(rusqlite::params![nome], driver_from_row)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DbError::NotFound(format!("Piloto '{}' não encontrado", nome))
            }
            other => DbError::Sqlite(other),
        })
}

pub fn get_all_drivers(conn: &Connection) -> Result<Vec<Driver>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM drivers ORDER BY nome")?;
    let result = collect_drivers(stmt.query_map([], driver_from_row)?);
    result
}

pub fn get_drivers_by_category(conn: &Connection, categoria: &str) -> Result<Vec<Driver>, DbError> {
    let mut stmt =
        conn.prepare("SELECT * FROM drivers WHERE categoria_atual = ?1 ORDER BY nome")?;
    let result = collect_drivers(stmt.query_map(rusqlite::params![categoria], driver_from_row)?);
    result
}

pub fn get_drivers_by_status(conn: &Connection, status: &str) -> Result<Vec<Driver>, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM drivers WHERE status = ?1 ORDER BY nome")?;
    let result = collect_drivers(stmt.query_map(rusqlite::params![status], driver_from_row)?);
    result
}

pub fn get_player_driver(conn: &Connection) -> Result<Driver, DbError> {
    let mut stmt = conn.prepare("SELECT * FROM drivers WHERE is_jogador = 1 LIMIT 1")?;
    stmt.query_row([], driver_from_row).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            DbError::NotFound("Piloto do jogador não encontrado".to_string())
        }
        other => DbError::Sqlite(other),
    })
}

pub fn get_free_drivers(conn: &Connection) -> Result<Vec<Driver>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT * FROM drivers WHERE categoria_atual IS NULL AND status = 'Ativo' ORDER BY nome",
    )?;
    let result = collect_drivers(stmt.query_map([], driver_from_row)?);
    result
}

/// Pool global de convocação especial.
/// "Livre" = sem contrato Regular ativo E sem contrato Especial ativo.
/// Diferente de `get_free_drivers` que usa `categoria_atual IS NULL`
/// (que pode ser referência histórica preservada, não indica disponibilidade real).
pub fn get_drivers_without_active_contract(conn: &Connection) -> Result<Vec<Driver>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT d.* FROM drivers d
         WHERE d.status = 'Ativo'
           AND NOT EXISTS (
               SELECT 1 FROM contracts c
               WHERE c.piloto_id = d.id AND c.status = 'Ativo' AND c.tipo = 'Regular'
           )
           AND NOT EXISTS (
               SELECT 1 FROM contracts c
               WHERE c.piloto_id = d.id AND c.status = 'Ativo' AND c.tipo = 'Especial'
           )
         ORDER BY d.nome",
    )?;
    let rows = stmt.query_map([], driver_from_row)?;
    collect_drivers(rows)
}

// ── UPDATE ────────────────────────────────────────────────────────────────────

pub fn update_driver(conn: &Connection, driver: &Driver) -> Result<(), DbError> {
    let historico =
        serde_json::to_string(&driver.historico_circuitos).unwrap_or_else(|_| "{}".to_string());
    let ultimos =
        serde_json::to_string(&driver.ultimos_resultados).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "UPDATE drivers SET
            nome = :nome, is_jogador = :is_jogador, idade = :idade,
            nacionalidade = :nacionalidade, genero = :genero,
            categoria_atual = :categoria_atual,
            categoria_especial_ativa = :categoria_especial_ativa,
            status = :status,
            personalidade_primaria = :personalidade_primaria,
            personalidade_secundaria = :personalidade_secundaria,
            ano_inicio_carreira = :ano_inicio_carreira,
            skill = :skill, consistencia = :consistencia, racecraft = :racecraft,
            defesa = :defesa, ritmo_classificacao = :ritmo_classificacao,
            gestao_pneus = :gestao_pneus, habilidade_largada = :habilidade_largada,
            adaptabilidade = :adaptabilidade, fator_chuva = :fator_chuva,
            fitness = :fitness, experiencia = :experiencia,
            desenvolvimento = :desenvolvimento, aggression = :aggression,
            smoothness = :smoothness, midia = :midia,
            mentalidade = :mentalidade, confianca = :confianca,
            temp_pontos = :temp_pontos, temp_vitorias = :temp_vitorias,
            temp_podios = :temp_podios, temp_poles = :temp_poles,
            temp_corridas = :temp_corridas, temp_dnfs = :temp_dnfs,
            temp_posicao_media = :temp_posicao_media,
            carreira_pontos_total = :carreira_pontos_total,
            carreira_vitorias = :carreira_vitorias, carreira_podios = :carreira_podios,
            carreira_poles = :carreira_poles, carreira_corridas = :carreira_corridas,
            carreira_temporadas = :carreira_temporadas, carreira_titulos = :carreira_titulos,
            carreira_dnfs = :carreira_dnfs, motivacao = :motivacao,
            historico_circuitos = :historico_circuitos,
            ultimos_resultados = :ultimos_resultados,
            melhor_resultado_temp = :melhor_resultado_temp,
            temporadas_na_categoria = :temporadas_na_categoria,
            corridas_na_categoria = :corridas_na_categoria,
            temporadas_motivacao_baixa = :temporadas_motivacao_baixa
        WHERE id = :id",
        rusqlite::named_params! {
            ":id":                        &driver.id,
            ":nome":                      &driver.nome,
            ":is_jogador":                driver.is_jogador as i64,
            ":idade":                     driver.idade as i64,
            ":nacionalidade":             &driver.nacionalidade,
            ":genero":                    &driver.genero,
            ":categoria_atual":           &driver.categoria_atual,
            ":categoria_especial_ativa":  &driver.categoria_especial_ativa,
            ":status":                    driver.status.as_str(),
            ":personalidade_primaria":    driver.personalidade_primaria.as_ref().map(|p| p.as_str()),
            ":personalidade_secundaria":  driver.personalidade_secundaria.as_ref().map(|p| p.as_str()),
            ":ano_inicio_carreira":       driver.ano_inicio_carreira as i64,
            ":skill":                     driver.atributos.skill,
            ":consistencia":             driver.atributos.consistencia,
            ":racecraft":                driver.atributos.racecraft,
            ":defesa":                   driver.atributos.defesa,
            ":ritmo_classificacao":      driver.atributos.ritmo_classificacao,
            ":gestao_pneus":             driver.atributos.gestao_pneus,
            ":habilidade_largada":       driver.atributos.habilidade_largada,
            ":adaptabilidade":           driver.atributos.adaptabilidade,
            ":fator_chuva":              driver.atributos.fator_chuva,
            ":fitness":                  driver.atributos.fitness,
            ":experiencia":              driver.atributos.experiencia,
            ":desenvolvimento":          driver.atributos.desenvolvimento,
            ":aggression":               driver.atributos.aggression,
            ":smoothness":              driver.atributos.smoothness,
            ":midia":                    driver.atributos.midia,
            ":mentalidade":              driver.atributos.mentalidade,
            ":confianca":                driver.atributos.confianca,
            ":temp_pontos":              driver.stats_temporada.pontos,
            ":temp_vitorias":            driver.stats_temporada.vitorias as i64,
            ":temp_podios":              driver.stats_temporada.podios as i64,
            ":temp_poles":               driver.stats_temporada.poles as i64,
            ":temp_corridas":            driver.stats_temporada.corridas as i64,
            ":temp_dnfs":                driver.stats_temporada.dnfs as i64,
            ":temp_posicao_media":       driver.stats_temporada.posicao_media,
            ":carreira_pontos_total":    driver.stats_carreira.pontos_total,
            ":carreira_vitorias":        driver.stats_carreira.vitorias as i64,
            ":carreira_podios":          driver.stats_carreira.podios as i64,
            ":carreira_poles":           driver.stats_carreira.poles as i64,
            ":carreira_corridas":        driver.stats_carreira.corridas as i64,
            ":carreira_temporadas":      driver.stats_carreira.temporadas as i64,
            ":carreira_titulos":         driver.stats_carreira.titulos as i64,
            ":carreira_dnfs":            driver.stats_carreira.dnfs as i64,
            ":motivacao":                driver.motivacao,
            ":historico_circuitos":      &historico,
            ":ultimos_resultados":       &ultimos,
            ":melhor_resultado_temp":    driver.melhor_resultado_temp.map(|v| v as i64),
            ":temporadas_na_categoria":  driver.temporadas_na_categoria as i64,
            ":corridas_na_categoria":    driver.corridas_na_categoria as i64,
            ":temporadas_motivacao_baixa": driver.temporadas_motivacao_baixa as i64,
        },
    )?;
    Ok(())
}

pub fn update_driver_stats(
    conn: &Connection,
    id: &str,
    stats: &DriverSeasonStats,
    stats_carreira: &DriverCareerStats,
    motivacao: f64,
    melhor_resultado_temp: Option<u32>,
    temporadas_na_categoria: u32,
    corridas_na_categoria: u32,
    temporadas_motivacao_baixa: u32,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET
            temp_pontos = :temp_pontos, temp_vitorias = :temp_vitorias,
            temp_podios = :temp_podios, temp_poles = :temp_poles,
            temp_corridas = :temp_corridas, temp_dnfs = :temp_dnfs,
            temp_posicao_media = :temp_posicao_media,
            carreira_pontos_total = :carreira_pontos_total,
            carreira_vitorias = :carreira_vitorias, carreira_podios = :carreira_podios,
            carreira_poles = :carreira_poles, carreira_corridas = :carreira_corridas,
            carreira_temporadas = :carreira_temporadas, carreira_titulos = :carreira_titulos,
            carreira_dnfs = :carreira_dnfs,
            motivacao = :motivacao,
            melhor_resultado_temp = :melhor_resultado_temp,
            temporadas_na_categoria = :temporadas_na_categoria,
            corridas_na_categoria = :corridas_na_categoria,
            temporadas_motivacao_baixa = :temporadas_motivacao_baixa
        WHERE id = :id",
        rusqlite::named_params! {
            ":id":                        id,
            ":temp_pontos":               stats.pontos,
            ":temp_vitorias":             stats.vitorias as i64,
            ":temp_podios":               stats.podios as i64,
            ":temp_poles":                stats.poles as i64,
            ":temp_corridas":             stats.corridas as i64,
            ":temp_dnfs":                 stats.dnfs as i64,
            ":temp_posicao_media":        stats.posicao_media,
            ":carreira_pontos_total":     stats_carreira.pontos_total,
            ":carreira_vitorias":         stats_carreira.vitorias as i64,
            ":carreira_podios":           stats_carreira.podios as i64,
            ":carreira_poles":            stats_carreira.poles as i64,
            ":carreira_corridas":         stats_carreira.corridas as i64,
            ":carreira_temporadas":       stats_carreira.temporadas as i64,
            ":carreira_titulos":          stats_carreira.titulos as i64,
            ":carreira_dnfs":             stats_carreira.dnfs as i64,
            ":motivacao":                 motivacao,
            ":melhor_resultado_temp":     melhor_resultado_temp.map(|v| v as i64),
            ":temporadas_na_categoria":   temporadas_na_categoria as i64,
            ":corridas_na_categoria":     corridas_na_categoria as i64,
            ":temporadas_motivacao_baixa": temporadas_motivacao_baixa as i64,
        },
    )?;
    Ok(())
}

pub fn update_driver_attributes(
    conn: &Connection,
    id: &str,
    attrs: &DriverAttributes,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET
            skill = :skill, consistencia = :consistencia, racecraft = :racecraft,
            defesa = :defesa, ritmo_classificacao = :ritmo_classificacao,
            gestao_pneus = :gestao_pneus, habilidade_largada = :habilidade_largada,
            adaptabilidade = :adaptabilidade, fator_chuva = :fator_chuva,
            fitness = :fitness, experiencia = :experiencia,
            desenvolvimento = :desenvolvimento, aggression = :aggression,
            smoothness = :smoothness, midia = :midia,
            mentalidade = :mentalidade, confianca = :confianca
        WHERE id = :id",
        rusqlite::named_params! {
            ":id":                  id,
            ":skill":               attrs.skill,
            ":consistencia":        attrs.consistencia,
            ":racecraft":           attrs.racecraft,
            ":defesa":              attrs.defesa,
            ":ritmo_classificacao": attrs.ritmo_classificacao,
            ":gestao_pneus":        attrs.gestao_pneus,
            ":habilidade_largada":  attrs.habilidade_largada,
            ":adaptabilidade":      attrs.adaptabilidade,
            ":fator_chuva":         attrs.fator_chuva,
            ":fitness":             attrs.fitness,
            ":experiencia":         attrs.experiencia,
            ":desenvolvimento":     attrs.desenvolvimento,
            ":aggression":          attrs.aggression,
            ":smoothness":          attrs.smoothness,
            ":midia":               attrs.midia,
            ":mentalidade":         attrs.mentalidade,
            ":confianca":           attrs.confianca,
        },
    )?;
    Ok(())
}

/// Define ou limpa a categoria especial ativa do piloto.
/// Deve ser chamada ao assinar/encerrar contrato Especial.
pub fn update_driver_especial_category(
    conn: &Connection,
    driver_id: &str,
    categoria_especial: Option<&str>,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET categoria_especial_ativa = ?1 WHERE id = ?2",
        rusqlite::params![categoria_especial, driver_id],
    )?;
    Ok(())
}

/// Remove `categoria_especial_ativa` de todos os pilotos que a possuem.
/// Chamado durante PosEspecial para liberar os pilotos do bloco especial encerrado.
pub fn clear_all_categoria_especial_ativa(conn: &Connection) -> Result<usize, DbError> {
    let n = conn.execute(
        "UPDATE drivers SET categoria_especial_ativa = NULL WHERE categoria_especial_ativa IS NOT NULL",
        [],
    )?;
    Ok(n)
}

pub fn update_driver_status(
    conn: &Connection,
    id: &str,
    status: &DriverStatus,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET status = ?1 WHERE id = ?2",
        rusqlite::params![status.as_str(), id],
    )?;
    Ok(())
}

// ── DELETE ────────────────────────────────────────────────────────────────────

pub fn update_driver_motivation(conn: &Connection, id: &str, motivacao: f64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET motivacao = ?1 WHERE id = ?2",
        rusqlite::params![motivacao.clamp(0.0, 100.0), id],
    )?;
    Ok(())
}

pub fn update_driver_midia(conn: &Connection, id: &str, midia: f64) -> Result<(), DbError> {
    conn.execute(
        "UPDATE drivers SET midia = ?1 WHERE id = ?2",
        rusqlite::params![midia.clamp(0.0, 100.0), id],
    )?;
    Ok(())
}

pub fn delete_driver(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM drivers WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

// ── COUNT ─────────────────────────────────────────────────────────────────────

pub fn count_drivers(conn: &Connection) -> Result<u32, DbError> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM drivers", [], |row| row.get(0))?;
    Ok(n as u32)
}

pub fn count_drivers_by_category(conn: &Connection, categoria: &str) -> Result<u32, DbError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM drivers WHERE categoria_atual = ?1",
        rusqlite::params![categoria],
        |row| row.get(0),
    )?;
    Ok(n as u32)
}

// ── Helpers internos ──────────────────────────────────────────────────────────

fn collect_drivers(
    mapped: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Driver>>,
) -> Result<Vec<Driver>, DbError> {
    let mut result = Vec::new();
    for row in mapped {
        result.push(row?);
    }
    Ok(result)
}

fn driver_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Driver> {
    let historico_str: String = row.get("historico_circuitos")?;
    let ultimos_str: String = row.get("ultimos_resultados")?;

    Ok(Driver {
        id: row.get("id")?,
        nome: row.get("nome")?,
        is_jogador: row.get::<_, i64>("is_jogador")? != 0,
        idade: row.get::<_, i64>("idade")? as u32,
        nacionalidade: row.get("nacionalidade")?,
        genero: row.get("genero")?,
        categoria_atual: row.get("categoria_atual")?,
        categoria_especial_ativa: row.get("categoria_especial_ativa")?,
        status: DriverStatus::from_str(
            &row.get::<_, String>("status")
                .unwrap_or_else(|_| "Ativo".to_string()),
        ),
        personalidade_primaria: row
            .get::<_, Option<String>>("personalidade_primaria")?
            .map(|s| PrimaryPersonality::from_str(&s)),
        personalidade_secundaria: row
            .get::<_, Option<String>>("personalidade_secundaria")?
            .map(|s| SecondaryPersonality::from_str(&s)),
        ano_inicio_carreira: row.get::<_, i64>("ano_inicio_carreira")? as u32,
        atributos: DriverAttributes {
            skill: row.get("skill")?,
            consistencia: row.get("consistencia")?,
            racecraft: row.get("racecraft")?,
            defesa: row.get("defesa")?,
            ritmo_classificacao: row.get("ritmo_classificacao")?,
            gestao_pneus: row.get("gestao_pneus")?,
            habilidade_largada: row.get("habilidade_largada")?,
            adaptabilidade: row.get("adaptabilidade")?,
            fator_chuva: row.get("fator_chuva")?,
            fitness: row.get("fitness")?,
            experiencia: row.get("experiencia")?,
            desenvolvimento: row.get("desenvolvimento")?,
            aggression: row.get("aggression")?,
            smoothness: row.get("smoothness")?,
            midia: row.get("midia")?,
            mentalidade: row.get("mentalidade")?,
            confianca: row.get("confianca")?,
        },
        stats_temporada: DriverSeasonStats {
            pontos: row.get("temp_pontos")?,
            vitorias: row.get::<_, i64>("temp_vitorias")? as u32,
            podios: row.get::<_, i64>("temp_podios")? as u32,
            poles: row.get::<_, i64>("temp_poles")? as u32,
            corridas: row.get::<_, i64>("temp_corridas")? as u32,
            dnfs: row.get::<_, i64>("temp_dnfs")? as u32,
            posicao_media: row.get("temp_posicao_media")?,
        },
        stats_carreira: DriverCareerStats {
            pontos_total: row.get("carreira_pontos_total")?,
            vitorias: row.get::<_, i64>("carreira_vitorias")? as u32,
            podios: row.get::<_, i64>("carreira_podios")? as u32,
            poles: row.get::<_, i64>("carreira_poles")? as u32,
            corridas: row.get::<_, i64>("carreira_corridas")? as u32,
            temporadas: row.get::<_, i64>("carreira_temporadas")? as u32,
            titulos: row.get::<_, i64>("carreira_titulos")? as u32,
            dnfs: row.get::<_, i64>("carreira_dnfs")? as u32,
        },
        motivacao: row.get("motivacao")?,
        historico_circuitos: serde_json::from_str(&historico_str).unwrap_or(serde_json::json!({})),
        ultimos_resultados: serde_json::from_str(&ultimos_str).unwrap_or(serde_json::json!([])),
        melhor_resultado_temp: row
            .get::<_, Option<i64>>("melhor_resultado_temp")?
            .map(|v| v as u32),
        temporadas_na_categoria: row.get::<_, i64>("temporadas_na_categoria")? as u32,
        corridas_na_categoria: row.get::<_, i64>("corridas_na_categoria")? as u32,
        temporadas_motivacao_baixa: row.get::<_, i64>("temporadas_motivacao_baixa")? as u32,
    })
}
