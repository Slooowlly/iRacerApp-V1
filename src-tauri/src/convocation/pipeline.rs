use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::calendar::generate_and_insert_special_calendars;
use crate::db::connection::DbError;
use crate::db::queries::{contracts as contract_queries, drivers as driver_queries,
                          news as news_queries, seasons as season_queries, teams as team_queries};
use crate::generators::ids::{next_id, next_ids, IdType};
use crate::models::enums::{SeasonPhase, TeamRole};
use crate::news::generator::generate_news_from_pos_especial;

use super::eligibility::{coletar_candidatos, FonteConvocacao};
use super::quotas::calcular_cotas;
use super::scoring::calcular_score;

// ── Estruturas públicas ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverAssignment {
    pub driver_id: String,
    pub team_id: String,
    pub papel: TeamRole,
    pub fonte: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridClasse {
    pub class_name: String,
    pub assignments: Vec<DriverAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvocationResult {
    pub grids: Vec<GridClasse>,
    pub total_contratos: usize,
    pub errors: Vec<String>,
}

// ── Classes convocadas ────────────────────────────────────────────────────────

/// Classes que participam da convocação (LMP2 excluído).
struct ClasseConfig {
    special_category: &'static str,
    class_name: &'static str,
    feeder_category: &'static str,
}

const CLASSES_CONVOCADAS: &[ClasseConfig] = &[
    ClasseConfig { special_category: "production_challenger", class_name: "mazda", feeder_category: "mazda_amador" },
    ClasseConfig { special_category: "production_challenger", class_name: "toyota", feeder_category: "toyota_amador" },
    ClasseConfig { special_category: "production_challenger", class_name: "bmw", feeder_category: "bmw_m2" },
    ClasseConfig { special_category: "endurance", class_name: "gt4", feeder_category: "gt4" },
    ClasseConfig { special_category: "endurance", class_name: "gt3", feeder_category: "gt3" },
];

// ── Transições de fase ────────────────────────────────────────────────────────

/// BlocoRegular → JanelaConvocacao.
/// Requer que a temporada ativa esteja em BlocoRegular.
pub fn advance_to_convocation_window(conn: &Connection) -> Result<(), DbError> {
    let season = season_queries::get_active_season(conn)?
        .ok_or_else(|| DbError::NotFound("Nenhuma temporada ativa".into()))?;

    if season.fase != SeasonPhase::BlocoRegular {
        return Err(DbError::Migration(format!(
            "Fase atual é '{}'; esperado BlocoRegular",
            season.fase
        )));
    }

    season_queries::update_season_fase(conn, &season.id, &SeasonPhase::JanelaConvocacao)?;
    Ok(())
}

/// JanelaConvocacao → BlocoEspecial.
/// Deve ser chamada APÓS run_convocation_window.
/// Gera o calendário das categorias especiais (semanas 41–50).
pub fn iniciar_bloco_especial(conn: &Connection) -> Result<(), DbError> {
    let season = season_queries::get_active_season(conn)?
        .ok_or_else(|| DbError::NotFound("Nenhuma temporada ativa".into()))?;

    if season.fase != SeasonPhase::JanelaConvocacao {
        return Err(DbError::Migration(format!(
            "Fase atual é '{}'; esperado JanelaConvocacao",
            season.fase
        )));
    }

    season_queries::update_season_fase(conn, &season.id, &SeasonPhase::BlocoEspecial)?;

    // Gerar calendário das categorias especiais (production_challenger e endurance)
    let mut rng = rand::thread_rng();
    generate_and_insert_special_calendars(conn, &season.id, season.ano, &mut rng)
        .map_err(|e| DbError::Migration(format!("Falha ao gerar calendário especial: {e}")))?;

    Ok(())
}

// ── Pipeline principal ────────────────────────────────────────────────────────

/// Monta os grids das categorias especiais em memória e persiste em uma única
/// transação. Não muda a fase da temporada (permanece JanelaConvocacao).
pub fn run_convocation_window(conn: &Connection) -> Result<ConvocationResult, DbError> {
    let season = season_queries::get_active_season(conn)?
        .ok_or_else(|| DbError::NotFound("Nenhuma temporada ativa".into()))?;

    if season.fase != SeasonPhase::JanelaConvocacao {
        return Err(DbError::Migration(format!(
            "Fase atual é '{}'; convocação só ocorre na JanelaConvocacao",
            season.fase
        )));
    }

    let season_number = season.numero;

    // ── Passo 1: construir todos os grids em memória ──────────────────────────
    // Manter conjunto global de drivers já alocados para evitar duplicatas entre classes
    let mut all_grids: Vec<GridClasse> = Vec::new();
    let mut all_errors: Vec<String> = Vec::new();
    let mut globally_assigned: std::collections::HashSet<String> = std::collections::HashSet::new();

    for cfg in CLASSES_CONVOCADAS {
        match montar_grid_classe(conn, cfg, season_number, &globally_assigned) {
            Ok(grid) => {
                for a in &grid.assignments {
                    globally_assigned.insert(a.driver_id.clone());
                }
                all_grids.push(grid);
            }
            Err(e) => all_errors.push(format!("[{}/{}] {}", cfg.special_category, cfg.class_name, e)),
        }
    }

    // ── Passo 2: validar (sem efeitos colaterais) ─────────────────────────────
    let validation_errors = validar_grids(&all_grids);
    if !validation_errors.is_empty() {
        return Ok(ConvocationResult {
            grids: Vec::new(),
            total_contratos: 0,
            errors: validation_errors,
        });
    }

    // ── Passo 3: persistir em transação atômica ───────────────────────────────
    let total_contratos = all_grids.iter().map(|g| g.assignments.len()).sum();
    persistir_grids(conn, &all_grids, season_number)?;

    Ok(ConvocationResult {
        grids: all_grids,
        total_contratos,
        errors: all_errors,
    })
}

// ── Montagem de grid por classe ───────────────────────────────────────────────

fn montar_grid_classe(
    conn: &Connection,
    cfg: &ClasseConfig,
    season_number: i32,
    globally_excluded: &std::collections::HashSet<String>,
) -> Result<GridClasse, DbError> {
    // 1. Equipes da classe ordenadas por car_performance desc
    let teams = team_queries::get_teams_by_category_and_class(conn, cfg.special_category, cfg.class_name)?;
    if teams.is_empty() {
        return Err(DbError::NotFound(format!(
            "Nenhuma equipe para {}/{}",
            cfg.special_category, cfg.class_name
        )));
    }

    let total_assentos = teams.len() * 2;
    let cotas = calcular_cotas(total_assentos);

    // 2. Candidatos de todas as fontes
    let candidatos = coletar_candidatos(conn, cfg.special_category, cfg.class_name, cfg.feeder_category)?;

    // 3. Calcular scores e separar por fonte (excluir já alocados globalmente)
    let mut fonte_a: Vec<(String, f64)> = Vec::new();
    let mut fonte_b: Vec<(String, f64)> = Vec::new();
    let mut fonte_c: Vec<(String, f64)> = Vec::new();
    let mut fonte_d: Vec<(String, f64)> = Vec::new();

    for c in candidatos.iter().filter(|c| !globally_excluded.contains(&c.driver_id)) {
        let historico = contract_queries::get_especial_contract_count(
            conn, &c.driver_id, cfg.special_category, cfg.class_name,
        ).unwrap_or(0);
        let score = calcular_score(&c.driver, &c.fonte, historico);
        match c.fonte {
            FonteConvocacao::MeritoRegular => fonte_a.push((c.driver_id.clone(), score)),
            FonteConvocacao::ContinuidadeHistorica => fonte_b.push((c.driver_id.clone(), score)),
            FonteConvocacao::PoolGlobal => fonte_c.push((c.driver_id.clone(), score)),
            FonteConvocacao::Wildcard => fonte_d.push((c.driver_id.clone(), score)),
        }
    }

    // 4. Ordenar cada fonte por score desc
    for v in [&mut fonte_a, &mut fonte_b, &mut fonte_c, &mut fonte_d] {
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 5. Selecionar por cota com overflow B/C → A
    let mut selecionados: Vec<(String, FonteConvocacao, f64)> = Vec::new();

    // D (wildcard): máximo 1
    let d_count = cotas.wildcard.min(fonte_d.len());
    for (id, score) in fonte_d.iter().take(d_count) {
        selecionados.push((id.clone(), FonteConvocacao::Wildcard, *score));
    }

    // B (continuidade)
    let b_count = cotas.continuidade.min(fonte_b.len());
    let b_overflow = cotas.continuidade.saturating_sub(b_count);
    for (id, score) in fonte_b.iter().take(b_count) {
        selecionados.push((id.clone(), FonteConvocacao::ContinuidadeHistorica, *score));
    }

    // C (pool)
    let c_count = cotas.pool_global.min(fonte_c.len());
    let c_overflow = cotas.pool_global.saturating_sub(c_count);
    for (id, score) in fonte_c.iter().take(c_count) {
        selecionados.push((id.clone(), FonteConvocacao::PoolGlobal, *score));
    }

    // A (mérito) + overflow de B e C
    let a_total = cotas.merito_regular + b_overflow + c_overflow;

    // Remover da pool A quem já foi selecionado via outra fonte
    let ja_selecionados: std::collections::HashSet<String> =
        selecionados.iter().map(|(id, _, _)| id.clone()).collect();

    let mut idx = 0;
    for (id, score) in &fonte_a {
        if ja_selecionados.contains(id) {
            continue;
        }
        if idx >= a_total {
            break;
        }
        selecionados.push((id.clone(), FonteConvocacao::MeritoRegular, *score));
        idx += 1;
    }

    // 6. Ordenar selecionados por score desc para distribuição equitativa
    selecionados.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // 7. Distribuir: posição 2i → team[i] N1, posição 2i+1 → team[i] N2
    let mut assignments: Vec<DriverAssignment> = Vec::new();
    for (i, (driver_id, fonte, score)) in selecionados.iter().enumerate() {
        let team_idx = i / 2;
        if team_idx >= teams.len() {
            break; // mais pilotos que assentos (não deve ocorrer, mas defensivo)
        }
        let papel = if i % 2 == 0 { TeamRole::Numero1 } else { TeamRole::Numero2 };
        assignments.push(DriverAssignment {
            driver_id: driver_id.clone(),
            team_id: teams[team_idx].id.clone(),
            papel,
            fonte: fonte_label(fonte),
            score: *score,
        });
    }

    Ok(GridClasse {
        class_name: cfg.class_name.to_string(),
        assignments,
    })
}

fn fonte_label(fonte: &FonteConvocacao) -> String {
    match fonte {
        FonteConvocacao::MeritoRegular => "MeritoRegular".into(),
        FonteConvocacao::ContinuidadeHistorica => "ContinuidadeHistorica".into(),
        FonteConvocacao::PoolGlobal => "PoolGlobal".into(),
        FonteConvocacao::Wildcard => "Wildcard".into(),
    }
}

// ── Validação em memória ──────────────────────────────────────────────────────

fn validar_grids(grids: &[GridClasse]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut global_driver_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for grid in grids {
        // Sem duplicatas intra-grid
        let mut ids_neste: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for a in &grid.assignments {
            if !ids_neste.insert(a.driver_id.as_str()) {
                errors.push(format!(
                    "[{}] driver_id duplicado no grid: {}",
                    grid.class_name, a.driver_id
                ));
            }
            if !global_driver_ids.insert(a.driver_id.as_str()) {
                errors.push(format!(
                    "[{}] driver {} já foi alocado em outra classe",
                    grid.class_name, a.driver_id
                ));
            }
        }
    }

    errors
}

// ── Persistência transacional ─────────────────────────────────────────────────

fn persistir_grids(
    conn: &Connection,
    grids: &[GridClasse],
    season_number: i32,
) -> Result<(), DbError> {
    // Coletar todos os dados necessários antes da transação (next_id precisa de conn)
    // Gerar IDs de contrato antecipadamente
    let total = grids.iter().map(|g| g.assignments.len()).sum::<usize>();
    let contract_ids = crate::generators::ids::next_ids(conn, IdType::Contract, total as u32)?;

    let mut contract_idx = 0;

    // Agrupar assignments por team para update_team_pilots
    // Estrutura: team_id → (n1_id, n2_id)
    let mut team_lineup: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    // Coletar dados de teams para obter nome
    let mut team_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Coletar dados de drivers para obter nome
    let mut driver_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Pre-carregar teams e drivers necessários
    for grid in grids {
        for a in &grid.assignments {
            if !team_map.contains_key(&a.team_id) {
                if let Ok(Some(team)) = team_queries::get_team_by_id(conn, &a.team_id) {
                    team_map.insert(team.id.clone(), team.nome.clone());
                }
            }
            if !driver_map.contains_key(&a.driver_id) {
                if let Ok(driver) = driver_queries::get_driver(conn, &a.driver_id) {
                    driver_map.insert(driver.id.clone(), driver.nome.clone());
                }
            }
        }
    }

    // Construir contratos e lineup updates em memória
    struct PendingOp {
        contract: crate::models::contract::Contract,
        driver_id: String,
        special_category: String,
        team_id: String,
        papel: TeamRole,
    }

    let mut ops: Vec<PendingOp> = Vec::new();

    // Mapa de class_name → special_category
    let class_to_cat: std::collections::HashMap<&str, &str> = CLASSES_CONVOCADAS
        .iter()
        .map(|c| (c.class_name, c.special_category))
        .collect();

    for grid in grids {
        let special_cat = class_to_cat
            .get(grid.class_name.as_str())
            .copied()
            .unwrap_or("unknown");

        for a in &grid.assignments {
            let contract_id = contract_ids[contract_idx].clone();
            contract_idx += 1;

            let team_nome = team_map
                .get(&a.team_id)
                .cloned()
                .unwrap_or_else(|| a.team_id.clone());
            let driver_nome = driver_map
                .get(&a.driver_id)
                .cloned()
                .unwrap_or_else(|| a.driver_id.clone());

            let papel = if a.papel == TeamRole::Numero1 {
                TeamRole::Numero1
            } else {
                TeamRole::Numero2
            };

            let contract = contract_queries::generate_especial_contract(
                contract_id,
                &a.driver_id,
                &driver_nome,
                &a.team_id,
                &team_nome,
                papel.clone(),
                special_cat,
                &grid.class_name,
                season_number,
            );

            // Atualizar lineup
            let entry = team_lineup.entry(a.team_id.clone()).or_insert((None, None));
            match papel {
                TeamRole::Numero1 => entry.0 = Some(a.driver_id.clone()),
                TeamRole::Numero2 => entry.1 = Some(a.driver_id.clone()),
            }

            ops.push(PendingOp {
                contract,
                driver_id: a.driver_id.clone(),
                special_category: special_cat.to_string(),
                team_id: a.team_id.clone(),
                papel,
            });
        }
    }

    // Persistir tudo
    for op in &ops {
        contract_queries::insert_contract(conn, &op.contract)?;
        driver_queries::update_driver_especial_category(
            conn,
            &op.driver_id,
            Some(&op.special_category),
        )?;
    }

    for (team_id, (n1, n2)) in &team_lineup {
        team_queries::update_team_pilots(conn, team_id, n1.as_deref(), n2.as_deref())?;

        // Hierarquia: N1 = hierarquia_n1_id, N2 = hierarquia_n2_id
        if let (Some(n1_id), Some(n2_id)) = (n1, n2) {
            team_queries::update_team_hierarchy(
                conn,
                team_id,
                Some(n1_id.as_str()),
                Some(n2_id.as_str()),
                "Claro",
                0.0,
            )?;
        }
    }

    Ok(())
}

// ── PosEspecial ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosEspecialResult {
    pub contratos_encerrados: usize,
    pub pilotos_liberados: usize,
    pub equipes_limpas: usize,
    pub errors: Vec<String>,
}

/// BlocoEspecial → PosEspecial (transição esportiva: as corridas especiais terminaram).
/// Deve ser chamada antes de run_pos_especial.
pub fn encerrar_bloco_especial(conn: &Connection) -> Result<(), DbError> {
    let season = season_queries::get_active_season(conn)?
        .ok_or_else(|| DbError::NotFound("Nenhuma temporada ativa".into()))?;

    if season.fase != SeasonPhase::BlocoEspecial {
        return Err(DbError::Migration(format!(
            "Fase atual é '{}'; esperado BlocoEspecial",
            season.fase
        )));
    }

    season_queries::update_season_fase(conn, &season.id, &SeasonPhase::PosEspecial)?;
    Ok(())
}

/// Desmontagem administrativa do bloco especial: expira contratos, limpa pilotos e
/// hierarquias das equipes especiais. Gera e persiste notícias de campeões.
///
/// Escopo neste bloco: "core cleanup + news de encerramento".
/// Fora de escopo (implementar em blocos posteriores):
///   ajustes de motivação pós-special, reputação, espectadores, prêmios.
pub fn run_pos_especial(conn: &Connection) -> Result<PosEspecialResult, DbError> {
    let season = season_queries::get_active_season(conn)?
        .ok_or_else(|| DbError::NotFound("Nenhuma temporada ativa".into()))?;

    if season.fase != SeasonPhase::PosEspecial {
        return Err(DbError::Migration(format!(
            "Fase atual é '{}'; esperado PosEspecial",
            season.fase
        )));
    }

    // Coletar campeões ANTES do cleanup (contratos ainda ativos)
    let campeoes = query_campeoes_especiais(conn, season.numero)?;

    // Cleanup em uma transação
    let tx = conn.unchecked_transaction()?;

    let contratos_encerrados = contract_queries::expire_especial_contracts(&tx, season.numero)?;
    let pilotos_liberados = driver_queries::clear_all_categoria_especial_ativa(&tx)?;
    let equipes_limpas = team_queries::clear_special_team_lineups(&tx)?;
    team_queries::reset_special_team_hierarchies(&tx)?;

    tx.commit()?;

    // Gerar e persistir notícias (fora da transação de cleanup)
    let mut temp_counter = 0u32;
    let mut temp_id = || {
        temp_counter += 1;
        format!("TMP{temp_counter:03}")
    };
    let mut timestamp = news_queries::get_latest_news_timestamp(conn)
        .unwrap_or(0)
        + 1;
    let mut items = generate_news_from_pos_especial(&campeoes, season.numero, &mut temp_id, &mut timestamp);

    if !items.is_empty() {
        if let Ok(ids) = next_ids(conn, IdType::News, items.len() as u32) {
            for (item, id) in items.iter_mut().zip(ids) {
                item.id = id;
            }
            let _ = news_queries::insert_news_batch(conn, &items);
            let _ = news_queries::trim_news(conn, 400);
        }
    }

    Ok(PosEspecialResult {
        contratos_encerrados,
        pilotos_liberados,
        equipes_limpas,
        errors: vec![],
    })
}

/// Retorna o campeão de cada classe especial (maior temp_pontos com contrato Especial ativo).
/// Chamada antes do cleanup para ter acesso aos contratos ainda ativos.
fn query_campeoes_especiais(
    conn: &Connection,
    season_number: i32,
) -> Result<Vec<(String, String, Option<String>)>, DbError> {
    let mut resultado = Vec::new();

    for cfg in CLASSES_CONVOCADAS {
        let nome: Option<String> = conn.query_row(
            "SELECT d.nome FROM drivers d
             INNER JOIN contracts c ON c.piloto_id = d.id
             WHERE c.tipo = 'Especial' AND c.status = 'Ativo'
               AND c.temporada_inicio = ?1
               AND c.categoria = ?2
               AND c.classe = ?3
             ORDER BY d.temp_pontos DESC
             LIMIT 1",
            rusqlite::params![season_number, cfg.special_category, cfg.class_name],
            |row| row.get(0),
        ).optional().map_err(DbError::Sqlite)?;

        resultado.push((
            cfg.special_category.to_string(),
            cfg.class_name.to_string(),
            nome,
        ));
    }

    Ok(resultado)
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::db::migrations;
    use crate::db::queries::{contracts as cq, drivers as dq, seasons as sq};
    use crate::generators::world::generate_world_with_rng;
    use crate::models::enums::SeasonPhase;

    fn setup_world_db() -> (Connection, String) {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("migrations");

        let mut rng = StdRng::seed_from_u64(99);
        let world = generate_world_with_rng(
            "Test Player",
            "🇧🇷 Brasileiro",
            20,
            "mazda_rookie",
            0,
            "medio",
            &mut rng,
        )
        .expect("world generation");

        let season_id = "S001".to_string();
        let season = crate::models::season::Season::new(season_id.clone(), 1, 2024);
        sq::insert_season(&conn, &season).expect("insert season");
        for driver in &world.drivers {
            dq::insert_driver(&conn, driver).expect("insert driver");
        }
        crate::db::queries::teams::insert_teams(&conn, &world.teams).expect("insert teams");
        cq::insert_contracts(&conn, &world.contracts).expect("insert contracts");

        // Sincronizar o contador de IDs com a quantidade de contratos inseridos
        let next_contract = world.contracts.len() + 1;
        conn.execute(
            "UPDATE meta SET value = ?1 WHERE key = 'next_contract_id'",
            rusqlite::params![next_contract.to_string()],
        ).expect("update meta contract counter");

        (conn, season_id)
    }

    #[test]
    fn test_season_phase_transitions() {
        let (conn, season_id) = setup_world_db();

        // Começa em BlocoRegular
        let s = sq::get_season_by_id(&conn, &season_id).unwrap().unwrap();
        assert_eq!(s.fase, SeasonPhase::BlocoRegular);

        // advance → JanelaConvocacao
        advance_to_convocation_window(&conn).expect("advance");
        let s = sq::get_season_by_id(&conn, &season_id).unwrap().unwrap();
        assert_eq!(s.fase, SeasonPhase::JanelaConvocacao);

        // iniciar_bloco_especial → BlocoEspecial
        iniciar_bloco_especial(&conn).expect("iniciar");
        let s = sq::get_season_by_id(&conn, &season_id).unwrap().unwrap();
        assert_eq!(s.fase, SeasonPhase::BlocoEspecial);
    }

    #[test]
    fn test_advance_requires_bloco_regular() {
        let (conn, _) = setup_world_db();
        // Avançar duas vezes deve falhar na segunda
        advance_to_convocation_window(&conn).expect("primeira avançada");
        let result = advance_to_convocation_window(&conn);
        assert!(result.is_err(), "deveria falhar se não estiver em BlocoRegular");
    }

    #[test]
    fn test_run_convocation_requires_janela() {
        let (conn, _) = setup_world_db();
        // Tentar convocação em BlocoRegular deve falhar
        let result = run_convocation_window(&conn);
        assert!(result.is_err(), "deveria falhar fora de JanelaConvocacao");
    }

    #[test]
    fn test_run_convocation_no_duplicate_drivers() {
        let (conn, _) = setup_world_db();
        advance_to_convocation_window(&conn).expect("advance");

        let result = run_convocation_window(&conn).expect("convocação");

        // Nenhum driver_id duplicado em todos os grids
        let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for grid in &result.grids {
            for a in &grid.assignments {
                assert!(
                    all_ids.insert(a.driver_id.clone()),
                    "driver {} duplicado entre grids",
                    a.driver_id
                );
            }
        }
    }

    #[test]
    fn test_run_convocation_contracts_are_especial() {
        let (conn, _) = setup_world_db();
        advance_to_convocation_window(&conn).expect("advance");
        let result = run_convocation_window(&conn).expect("convocação");
        assert!(result.errors.is_empty(), "erros na convocação: {:?}", result.errors);

        // Todos os contratos especiais gerados devem ter tipo=Especial
        let especiais: Vec<_> = conn
            .prepare("SELECT tipo FROM contracts WHERE tipo = 'Especial'")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(!especiais.is_empty(), "nenhum contrato especial gerado");
    }

    #[test]
    fn test_run_convocation_contracts_have_classe() {
        let (conn, _) = setup_world_db();
        advance_to_convocation_window(&conn).expect("advance");
        run_convocation_window(&conn).expect("convocação");

        // Contratos especiais devem ter classe não nula
        let null_classe: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contracts WHERE tipo='Especial' AND classe IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        assert_eq!(null_classe, 0, "contratos especiais com classe=NULL: {}", null_classe);
    }

    #[test]
    fn test_run_convocation_drivers_keep_regular_category() {
        let (conn, _) = setup_world_db();
        advance_to_convocation_window(&conn).expect("advance");
        let result = run_convocation_window(&conn).expect("convocação");

        // categoria_atual dos pilotos convocados deve estar intacta
        for grid in &result.grids {
            for a in &grid.assignments {
                let driver = dq::get_driver(&conn, &a.driver_id).expect("get driver");
                // categoria_especial_ativa deve estar preenchida
                assert!(
                    driver.categoria_especial_ativa.is_some(),
                    "piloto {} não tem categoria_especial_ativa após convocação",
                    driver.nome
                );
            }
        }
    }

    #[test]
    fn test_lmp2_teams_remain_empty() {
        let (conn, _) = setup_world_db();
        advance_to_convocation_window(&conn).expect("advance");
        run_convocation_window(&conn).expect("convocação");

        // Equipes LMP2 não devem ter pilotos
        let lmp2_with_pilots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE categoria='endurance' AND classe='lmp2' AND piloto_1_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        assert_eq!(lmp2_with_pilots, 0, "equipes lmp2 com pilotos: {}", lmp2_with_pilots);
    }

    // ── Testes PosEspecial ────────────────────────────────────────────────────

    /// Helper: avança até BlocoEspecial com convocação completa.
    fn setup_bloco_especial(conn: &Connection) {
        advance_to_convocation_window(conn).expect("advance to janela");
        run_convocation_window(conn).expect("run convocação");
        iniciar_bloco_especial(conn).expect("iniciar bloco especial");
    }

    #[test]
    fn test_encerrar_bloco_especial_transitions_phase() {
        let (conn, season_id) = setup_world_db();
        setup_bloco_especial(&conn);

        encerrar_bloco_especial(&conn).expect("encerrar bloco especial");
        let s = sq::get_season_by_id(&conn, &season_id).unwrap().unwrap();
        assert_eq!(s.fase, SeasonPhase::PosEspecial);
    }

    #[test]
    fn test_encerrar_bloco_especial_rejects_wrong_phase() {
        let (conn, _) = setup_world_db();
        // Estamos em BlocoRegular, não BlocoEspecial
        let result = encerrar_bloco_especial(&conn);
        assert!(result.is_err(), "deveria rejeitar fora de BlocoEspecial");
    }

    #[test]
    fn test_run_pos_especial_rejects_wrong_phase() {
        let (conn, _) = setup_world_db();
        // Estamos em BlocoRegular, não PosEspecial
        let result = run_pos_especial(&conn);
        assert!(result.is_err(), "deveria rejeitar fora de PosEspecial");
    }

    #[test]
    fn test_run_pos_especial_expires_especial_contracts() {
        let (conn, _) = setup_world_db();
        setup_bloco_especial(&conn);
        encerrar_bloco_especial(&conn).expect("encerrar");

        run_pos_especial(&conn).expect("run pos especial");

        let ativos: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contracts WHERE tipo='Especial' AND status='Ativo'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(ativos, 0, "contratos Especial ainda ativos após PosEspecial: {}", ativos);
    }

    #[test]
    fn test_run_pos_especial_clears_categoria_especial_ativa() {
        let (conn, _) = setup_world_db();
        setup_bloco_especial(&conn);
        encerrar_bloco_especial(&conn).expect("encerrar");

        run_pos_especial(&conn).expect("run pos especial");

        let com_especial: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM drivers WHERE categoria_especial_ativa IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(com_especial, 0, "pilotos com categoria_especial_ativa após PosEspecial: {}", com_especial);
    }

    #[test]
    fn test_run_pos_especial_clears_team_lineups() {
        let (conn, _) = setup_world_db();
        setup_bloco_especial(&conn);
        encerrar_bloco_especial(&conn).expect("encerrar");

        run_pos_especial(&conn).expect("run pos especial");

        let com_pilotos: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE categoria IN ('production_challenger','endurance') AND piloto_1_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(com_pilotos, 0, "equipes especiais com piloto após PosEspecial: {}", com_pilotos);
    }

    #[test]
    fn test_run_pos_especial_resets_hierarchy() {
        let (conn, _) = setup_world_db();
        setup_bloco_especial(&conn);
        encerrar_bloco_especial(&conn).expect("encerrar");

        run_pos_especial(&conn).expect("run pos especial");

        let com_hierarquia: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE categoria IN ('production_challenger','endurance') AND hierarquia_n1_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(com_hierarquia, 0, "equipes especiais com hierarquia após PosEspecial: {}", com_hierarquia);
    }
}
