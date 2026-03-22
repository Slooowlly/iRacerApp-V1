use std::collections::{HashMap, HashSet};

use chrono::Local;
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};

use crate::constants::categories::get_category_config;
use crate::db::queries::contracts as contract_queries;
use crate::db::queries::drivers as driver_queries;
use crate::db::queries::teams as team_queries;
use crate::generators::ids::{next_id, IdType};
use crate::market::driver_ai::evaluate_proposal;
use crate::market::evaluation::{estimate_expected_position, evaluate_driver_performance};
use crate::market::proposals::{
    MarketProposal, MarketReport, SigningInfo, Vacancy,
};
use crate::market::renewal::should_renew_contract;
use crate::market::team_ai::{generate_team_proposals, AvailableDriver};
use crate::market::visibility::calculate_visibility;
use crate::models::contract::Contract;
use crate::models::driver::Driver;
use crate::models::enums::{ContractStatus, DriverStatus, TeamRole};
use crate::models::team::HierarchyStatus;

#[derive(Debug, Clone)]
struct DriverMarketContext {
    posicao_campeonato: i32,
    total_pilotos: i32,
    categoria: String,
    category_tier: u8,
    vitorias: i32,
    poles: i32,
    titulos: i32,
    papel: TeamRole,
}

pub fn run_market(
    conn: &Connection,
    new_season_number: i32,
    rng: &mut impl Rng,
) -> Result<MarketReport, String> {
    let new_season = get_season_by_number(conn, new_season_number)?
        .ok_or_else(|| format!("Temporada {new_season_number} nao encontrada"))?;
    let previous_season = get_season_by_number(conn, new_season_number - 1)?;

    let mut report = MarketReport::default();
    reset_market_state(conn, &new_season.id)?;

    let all_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar pilotos: {e}"))?;
    let drivers_by_id: HashMap<String, Driver> = all_drivers
        .iter()
        .cloned()
        .map(|driver| (driver.id.clone(), driver))
        .collect();
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao carregar equipes: {e}"))?;
    let teams_by_id: HashMap<String, crate::models::team::Team> = teams
        .iter()
        .cloned()
        .map(|team| (team.id.clone(), team))
        .collect();
    let team_count_by_category = build_team_count_map(&teams);

    let active_contracts_before = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao carregar contratos ativos: {e}"))?;
    let expiring_contracts: Vec<Contract> = active_contracts_before
        .iter()
        .filter(|contract| contract.temporada_fim < new_season_number)
        .cloned()
        .collect();
    let expiring_by_driver: HashMap<String, Contract> = expiring_contracts
        .iter()
        .cloned()
        .map(|contract| (contract.piloto_id.clone(), contract))
        .collect();

    report.contracts_expired =
        contract_queries::expire_ending_contracts(conn, new_season_number - 1)
            .map_err(|e| format!("Falha ao expirar contratos: {e}"))?;

    let retired_contract_ids: Vec<String> = active_contracts_before
        .iter()
        .filter(|contract| {
            drivers_by_id
                .get(&contract.piloto_id)
                .is_some_and(|driver| driver.status == DriverStatus::Aposentado)
        })
        .map(|contract| contract.id.clone())
        .collect();
    for contract_id in &retired_contract_ids {
        contract_queries::update_contract_status(conn, contract_id, &ContractStatus::Rescindido)
            .map_err(|e| format!("Falha ao rescindir contrato de aposentado: {e}"))?;
    }
    report.retirements_replaced = retired_contract_ids.len() as i32;

    let standings_by_driver = load_market_contexts(
        conn,
        previous_season.as_ref().map(|season| season.id.as_str()),
        &drivers_by_id,
        &expiring_by_driver,
    )?;
    let mut player_was_expiring = false;

    for contract in &expiring_contracts {
        let Some(driver) = drivers_by_id.get(&contract.piloto_id) else {
            continue;
        };
        if driver.status != DriverStatus::Ativo {
            continue;
        }
        if driver.is_jogador {
            player_was_expiring = true;
            continue;
        }

        let Some(team) = teams_by_id.get(&contract.equipe_id) else {
            continue;
        };
        let context = standings_by_driver
            .get(&driver.id)
            .cloned()
            .unwrap_or_else(|| default_market_context(driver));
        let total_teams = team_count_by_category
            .get(&team.categoria)
            .copied()
            .unwrap_or(1);
        let expected_position = estimate_expected_position(team.car_performance, total_teams);
        let performance_score = evaluate_driver_performance(
            context.posicao_campeonato,
            context.total_pilotos,
            context.vitorias,
            driver.atributos.consistencia,
            expected_position,
        );
        let decision = should_renew_contract(driver, performance_score, contract, team.budget, rng);
        if !decision.should_renew {
            continue;
        }

        let new_contract = Contract::new(
            next_id(conn, IdType::Contract)
                .map_err(|e| format!("Falha ao gerar ID de contrato: {e}"))?,
            driver.id.clone(),
            driver.nome.clone(),
            team.id.clone(),
            team.nome.clone(),
            new_season_number,
            decision.new_duration.unwrap_or(1),
            decision.new_salary.unwrap_or(contract.salario_anual),
            decision
                .new_role
                .clone()
                .unwrap_or_else(|| contract.papel.clone()),
            team.categoria.clone(),
        );
        contract_queries::insert_contract(conn, &new_contract)
            .map_err(|e| format!("Falha ao inserir renovacao: {e}"))?;
        report.contracts_renewed += 1;
        report.new_signings.push(SigningInfo {
            driver_id: driver.id.clone(),
            driver_name: driver.nome.clone(),
            team_id: team.id.clone(),
            team_name: team.nome.clone(),
            categoria: team.categoria.clone(),
            papel: new_contract.papel.as_str().to_string(),
            tipo: "renovacao".to_string(),
        });
    }

    let mut refreshed_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao recarregar pilotos: {e}"))?;
    let mut refreshed_by_id: HashMap<String, Driver> = refreshed_drivers
        .iter()
        .cloned()
        .map(|driver| (driver.id.clone(), driver))
        .collect();
    sync_team_slots(conn, &teams, &refreshed_by_id)?;
    let initial_vacancies = find_vacancies(conn)?;
    let mut available = find_available_drivers(conn, &standings_by_driver)?;

    for vacancy in &initial_vacancies {
        let proposals = generate_team_proposals(vacancy, &available, new_season_number, rng);
        report.proposals_made += proposals.len() as i32;

        for proposal in proposals {
            let Some(index) = available
                .iter()
                .position(|candidate| candidate.driver.id == proposal.piloto_id)
            else {
                continue;
            };
            let candidate = available[index].clone();
            let previous_contract = expiring_by_driver.get(&candidate.driver.id);
            let decision = evaluate_proposal(
                &candidate.driver,
                &proposal,
                previous_contract,
                candidate.category_tier,
                vacancy.category_tier,
                vacancy.car_performance,
                vacancy.reputacao,
                rng,
            );

            if !decision.accepted {
                report.proposals_rejected += 1;
                continue;
            }

            sign_driver_to_team(
                conn,
                &candidate.driver,
                vacancy,
                new_season_number,
                proposal.salario_oferecido,
                proposal.duracao_anos,
                proposal.papel.clone(),
            )?;
            let tipo = if candidate.driver.categoria_atual.is_none() {
                report.rookies_placed += 1;
                "rookie"
            } else {
                "transferencia"
            };
            report.proposals_accepted += 1;
            report.new_signings.push(SigningInfo {
                driver_id: candidate.driver.id.clone(),
                driver_name: candidate.driver.nome.clone(),
                team_id: vacancy.team_id.clone(),
                team_name: vacancy.team_name.clone(),
                categoria: vacancy.categoria.clone(),
                papel: proposal.papel.as_str().to_string(),
                tipo: tipo.to_string(),
            });
            available.remove(index);

            refreshed_drivers = driver_queries::get_all_drivers(conn)
                .map_err(|e| format!("Falha ao recarregar pilotos apos assinatura: {e}"))?;
            refreshed_by_id = refreshed_drivers
                .iter()
                .cloned()
                .map(|driver| (driver.id.clone(), driver))
                .collect();
            sync_team_slots(conn, &teams, &refreshed_by_id)?;
            break;
        }
    }

    let player_proposals = generate_player_proposals(
        conn,
        &new_season.id,
        new_season_number,
        &find_vacancies(conn)?,
        player_was_expiring,
        &standings_by_driver,
        rng,
    )?;
    report.proposals_made += player_proposals.len() as i32;
    report.player_proposals = player_proposals;

    fill_remaining_vacancies_with_rookies(conn, &teams, new_season_number, &mut report, rng)?;

    refreshed_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao recarregar pilotos finais: {e}"))?;
    refreshed_by_id = refreshed_drivers
        .into_iter()
        .map(|driver| (driver.id.clone(), driver))
        .collect();
    refresh_team_hierarchy(conn, &teams, &refreshed_by_id)?;
    report.unresolved_vacancies = find_vacancies(conn)?.len() as i32;

    persist_market_state(conn, &new_season.id)?;
    Ok(report)
}

fn get_season_by_number(
    conn: &Connection,
    season_number: i32,
) -> Result<Option<crate::models::season::Season>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, numero, ano, status, rodada_atual, created_at, updated_at
             FROM seasons
             WHERE numero = ?1
             LIMIT 1",
        )
        .map_err(|e| format!("Falha ao preparar busca de temporada: {e}"))?;
    stmt.query_row(params![season_number], |row| {
        Ok(crate::models::season::Season {
            id: row.get(0)?,
            numero: row.get(1)?,
            ano: row.get(2)?,
            status: crate::models::enums::SeasonStatus::from_str(&row.get::<_, String>(3)?),
            rodada_atual: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })
    .optional()
    .map_err(|e| format!("Falha ao buscar temporada {season_number}: {e}"))
}

fn reset_market_state(conn: &Connection, season_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM market_proposals WHERE temporada_id = ?1",
        params![season_id],
    )
    .map_err(|e| format!("Falha ao limpar propostas de mercado: {e}"))?;
    conn.execute(
        "DELETE FROM market WHERE temporada_id = ?1",
        params![season_id],
    )
    .map_err(|e| format!("Falha ao limpar estado do mercado: {e}"))?;
    Ok(())
}

fn persist_market_state(conn: &Connection, season_id: &str) -> Result<(), String> {
    let now = timestamp_now();
    conn.execute(
        "INSERT INTO market (temporada_id, status, fase, inicio, fim)
         VALUES (?1, 'Fechado', 'PreTemporada', ?2, ?3)",
        params![season_id, now, now],
    )
    .map_err(|e| format!("Falha ao persistir estado do mercado: {e}"))?;
    Ok(())
}

fn build_team_count_map(teams: &[crate::models::team::Team]) -> HashMap<String, i32> {
    let mut counts = HashMap::new();
    for team in teams {
        *counts.entry(team.categoria.clone()).or_insert(0) += 1;
    }
    counts
}

fn load_market_contexts(
    conn: &Connection,
    previous_season_id: Option<&str>,
    drivers_by_id: &HashMap<String, Driver>,
    expiring_by_driver: &HashMap<String, Contract>,
) -> Result<HashMap<String, DriverMarketContext>, String> {
    let mut contexts = HashMap::new();
    if let Some(season_id) = previous_season_id {
        let mut stmt = conn
            .prepare(
                "SELECT piloto_id, categoria, posicao, vitorias, poles
                 FROM standings
                 WHERE temporada_id = ?1",
            )
            .map_err(|e| format!("Falha ao preparar standings do mercado: {e}"))?;
        let mut rows = stmt
            .query(params![season_id])
            .map_err(|e| format!("Falha ao ler standings do mercado: {e}"))?;
        let mut totals_by_category: HashMap<String, i32> = HashMap::new();
        let mut raw_rows = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Falha ao iterar standings do mercado: {e}"))?
        {
            let piloto_id: String = row
                .get("piloto_id")
                .map_err(|e| format!("Falha ao ler piloto_id do standings: {e}"))?;
            let categoria: String = row
                .get("categoria")
                .map_err(|e| format!("Falha ao ler categoria do standings: {e}"))?;
            let posicao: i32 = row
                .get("posicao")
                .map_err(|e| format!("Falha ao ler posicao do standings: {e}"))?;
            let vitorias: i32 = row
                .get("vitorias")
                .map_err(|e| format!("Falha ao ler vitorias do standings: {e}"))?;
            let poles: i32 = row
                .get("poles")
                .map_err(|e| format!("Falha ao ler poles do standings: {e}"))?;
            *totals_by_category.entry(categoria.clone()).or_insert(0) += 1;
            raw_rows.push((piloto_id, categoria, posicao, vitorias, poles));
        }

        for (piloto_id, categoria, posicao, vitorias, poles) in raw_rows {
            let driver = drivers_by_id.get(&piloto_id);
            contexts.insert(
                piloto_id.clone(),
                DriverMarketContext {
                    posicao_campeonato: posicao,
                    total_pilotos: totals_by_category.get(&categoria).copied().unwrap_or(1),
                    category_tier: get_category_config(&categoria)
                        .map(|config| config.tier)
                        .unwrap_or(0),
                    categoria: categoria.clone(),
                    vitorias,
                    poles,
                    titulos: driver.map(|d| d.stats_carreira.titulos as i32).unwrap_or(0),
                    papel: expiring_by_driver
                        .get(&piloto_id)
                        .map(|contract| contract.papel.clone())
                        .unwrap_or(TeamRole::Numero2),
                },
            );
        }
    }

    for driver in drivers_by_id.values() {
        contexts
            .entry(driver.id.clone())
            .or_insert_with(|| default_market_context(driver));
    }
    Ok(contexts)
}

fn default_market_context(driver: &Driver) -> DriverMarketContext {
    let categoria = driver.categoria_atual.clone().unwrap_or_default();
    DriverMarketContext {
        posicao_campeonato: 99,
        total_pilotos: 99,
        category_tier: get_category_config(&categoria)
            .map(|config| config.tier)
            .unwrap_or(0),
        categoria,
        vitorias: driver.stats_temporada.vitorias as i32,
        poles: driver.stats_temporada.poles as i32,
        titulos: driver.stats_carreira.titulos as i32,
        papel: TeamRole::Numero2,
    }
}

fn sync_team_slots(
    conn: &Connection,
    teams: &[crate::models::team::Team],
    drivers_by_id: &HashMap<String, Driver>,
) -> Result<(), String> {
    let active_contracts = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao sincronizar contratos ativos: {e}"))?;
    let mut valid_contracts: HashMap<String, Vec<Contract>> = HashMap::new();

    for contract in active_contracts {
        let Some(driver) = drivers_by_id.get(&contract.piloto_id) else {
            continue;
        };
        if driver.status == DriverStatus::Aposentado {
            contract_queries::update_contract_status(
                conn,
                &contract.id,
                &ContractStatus::Rescindido,
            )
            .map_err(|e| format!("Falha ao rescindir contrato invalido: {e}"))?;
            continue;
        }
        valid_contracts
            .entry(contract.equipe_id.clone())
            .or_default()
            .push(contract);
    }

    for team in teams {
        let mut contracts = valid_contracts.remove(&team.id).unwrap_or_default();
        contracts.sort_by(|a, b| {
            let skill_a = drivers_by_id
                .get(&a.piloto_id)
                .map(|driver| driver.atributos.skill)
                .unwrap_or(0.0);
            let skill_b = drivers_by_id
                .get(&b.piloto_id)
                .map(|driver| driver.atributos.skill)
                .unwrap_or(0.0);
            skill_b.total_cmp(&skill_a)
        });
        let piloto_1 = contracts
            .first()
            .map(|contract| contract.piloto_id.as_str());
        let piloto_2 = contracts.get(1).map(|contract| contract.piloto_id.as_str());
        team_queries::update_team_pilots(conn, &team.id, piloto_1, piloto_2).map_err(|e| {
            format!(
                "Falha ao sincronizar pilotos da equipe '{}': {e}",
                team.nome
            )
        })?;
    }

    Ok(())
}

fn find_vacancies(conn: &Connection) -> Result<Vec<Vacancy>, String> {
    let teams =
        team_queries::get_all_teams(conn).map_err(|e| format!("Falha ao buscar equipes: {e}"))?;
    let mut vacancies = Vec::new();

    for team in teams {
        let category_tier = get_category_config(&team.categoria)
            .map(|config| config.tier)
            .unwrap_or(0);
        match (&team.piloto_1_id, &team.piloto_2_id) {
            (None, None) => {
                vacancies.push(Vacancy {
                    team_id: team.id.clone(),
                    team_name: team.nome.clone(),
                    categoria: team.categoria.clone(),
                    category_tier,
                    car_performance: team.car_performance,
                    budget: team.budget,
                    reputacao: team.reputacao,
                    papel_necessario: TeamRole::Numero1,
                    piloto_existente_id: None,
                });
                vacancies.push(Vacancy {
                    team_id: team.id.clone(),
                    team_name: team.nome.clone(),
                    categoria: team.categoria.clone(),
                    category_tier,
                    car_performance: team.car_performance,
                    budget: team.budget,
                    reputacao: team.reputacao,
                    papel_necessario: TeamRole::Numero2,
                    piloto_existente_id: None,
                });
            }
            (Some(existing), None) => vacancies.push(Vacancy {
                team_id: team.id.clone(),
                team_name: team.nome.clone(),
                categoria: team.categoria.clone(),
                category_tier,
                car_performance: team.car_performance,
                budget: team.budget,
                reputacao: team.reputacao,
                papel_necessario: TeamRole::Numero2,
                piloto_existente_id: Some(existing.clone()),
            }),
            (None, Some(existing)) => vacancies.push(Vacancy {
                team_id: team.id.clone(),
                team_name: team.nome.clone(),
                categoria: team.categoria.clone(),
                category_tier,
                car_performance: team.car_performance,
                budget: team.budget,
                reputacao: team.reputacao,
                papel_necessario: TeamRole::Numero1,
                piloto_existente_id: Some(existing.clone()),
            }),
            (Some(_), Some(_)) => {}
        }
    }

    Ok(vacancies)
}

fn load_max_license_levels(conn: &Connection) -> Result<HashMap<String, u8>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT piloto_id, MAX(CAST(nivel AS INTEGER))
             FROM licenses
             GROUP BY piloto_id",
        )
        .map_err(|e| format!("Falha ao preparar consulta de licencas: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Falha ao ler licencas: {e}"))?;
    let mut map = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("Falha ao iterar licencas: {e}"))?
    {
        let piloto_id: String = row.get(0).unwrap_or_default();
        let nivel: u8 = row.get::<_, i64>(1).unwrap_or(0) as u8;
        map.insert(piloto_id, nivel);
    }
    Ok(map)
}

fn find_available_drivers(
    conn: &Connection,
    standings_by_driver: &HashMap<String, DriverMarketContext>,
) -> Result<Vec<AvailableDriver>, String> {
    let active_contracts = contract_queries::get_all_active_contracts(conn)
        .map_err(|e| format!("Falha ao recarregar contratos ativos: {e}"))?;
    let contracted_ids: HashSet<String> = active_contracts
        .into_iter()
        .map(|contract| contract.piloto_id)
        .collect();

    let drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar pilotos disponiveis: {e}"))?;
    let license_levels = load_max_license_levels(conn)?;
    let mut available = Vec::new();

    for driver in drivers {
        if driver.status != DriverStatus::Ativo || contracted_ids.contains(&driver.id) {
            continue;
        }
        let context = standings_by_driver
            .get(&driver.id)
            .cloned()
            .unwrap_or_else(|| default_market_context(&driver));
        let visibility = calculate_visibility(
            &driver,
            context.posicao_campeonato,
            context.total_pilotos,
            context.category_tier,
            context.vitorias,
            context.titulos,
            context.poles,
            &context.papel,
            &context.categoria,
        );
        let max_license_level = license_levels.get(&driver.id).copied();
        available.push(AvailableDriver {
            driver,
            visibility,
            posicao_campeonato: context.posicao_campeonato,
            categoria_atual: context.categoria,
            category_tier: context.category_tier,
            max_license_level,
        });
    }

    Ok(available)
}

fn sign_driver_to_team(
    conn: &Connection,
    driver: &Driver,
    vacancy: &Vacancy,
    new_season_number: i32,
    salary: f64,
    duration: i32,
    role: TeamRole,
) -> Result<(), String> {
    let team = team_queries::get_team_by_id(conn, &vacancy.team_id)
        .map_err(|e| format!("Falha ao buscar equipe da assinatura: {e}"))?
        .ok_or_else(|| format!("Equipe '{}' nao encontrada", vacancy.team_id))?;
    let new_contract = Contract::new(
        next_id(conn, IdType::Contract)
            .map_err(|e| format!("Falha ao gerar ID de contrato: {e}"))?,
        driver.id.clone(),
        driver.nome.clone(),
        vacancy.team_id.clone(),
        team.nome.clone(),
        new_season_number,
        duration,
        salary,
        role,
        vacancy.categoria.clone(),
    );
    contract_queries::insert_contract(conn, &new_contract)
        .map_err(|e| format!("Falha ao inserir contratacao: {e}"))?;

    let mut updated_driver = driver.clone();
    updated_driver.categoria_atual = Some(vacancy.categoria.clone());
    driver_queries::update_driver(conn, &updated_driver).map_err(|e| {
        format!(
            "Falha ao atualizar piloto contratado '{}': {e}",
            driver.nome
        )
    })?;
    Ok(())
}

fn generate_player_proposals(
    conn: &Connection,
    season_id: &str,
    new_season_number: i32,
    vacancies: &[Vacancy],
    player_was_expiring: bool,
    standings_by_driver: &HashMap<String, DriverMarketContext>,
    rng: &mut impl Rng,
) -> Result<Vec<MarketProposal>, String> {
    let player = match driver_queries::get_player_driver(conn) {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };
    let player_active_contract = contract_queries::get_active_contract_for_pilot(conn, &player.id)
        .map_err(|e| format!("Falha ao buscar contrato do jogador: {e}"))?;
    let player_is_free = player_active_contract.is_none();
    if !player_is_free && !player_was_expiring {
        return Ok(Vec::new());
    }

    // Calcula visibilidade real do jogador com os mesmos dados usados pela IA.
    let context = standings_by_driver
        .get(&player.id)
        .cloned()
        .unwrap_or_else(|| default_market_context(&player));
    let visibility = calculate_visibility(
        &player,
        context.posicao_campeonato,
        context.total_pilotos,
        context.category_tier,
        context.vitorias,
        context.titulos,
        context.poles,
        &context.papel,
        &context.categoria,
    );

    // Usa is_jogador=false para que generate_team_proposals avalie o jogador
    // com os mesmos critérios de qualquer piloto IA. A flag existe apenas para
    // impedir que o loop principal de mercado proponha ao jogador — aqui é intencional.
    let license_levels = load_max_license_levels(conn)?;
    let max_license_level = license_levels.get(&player.id).copied();
    let mut player_as_driver = player.clone();
    player_as_driver.is_jogador = false;
    let player_available = AvailableDriver {
        driver: player_as_driver,
        visibility,
        posicao_campeonato: context.posicao_campeonato,
        categoria_atual: context.categoria,
        category_tier: context.category_tier,
        max_license_level,
    };

    let mut proposals = Vec::new();
    for vacancy in vacancies {
        let team_proposals =
            generate_team_proposals(vacancy, &[player_available.clone()], new_season_number, rng);
        for mut proposal in team_proposals {
            // Restaura o ID correto do jogador e gera ID de proposta único por temporada.
            proposal.piloto_id = player.id.clone();
            proposal.piloto_nome = player.nome.clone();
            proposal.id = format!(
                "MP-{}-{}-{}-{}",
                new_season_number,
                vacancy.team_id,
                player.id,
                vacancy.papel_necessario.as_str(),
            );
            persist_player_proposal(conn, season_id, &proposal)?;
            proposals.push(proposal);
        }
    }

    Ok(proposals)
}

fn persist_player_proposal(
    conn: &Connection,
    season_id: &str,
    proposal: &MarketProposal,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO market_proposals (
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
    )
    .map_err(|e| format!("Falha ao persistir proposta do jogador: {e}"))?;
    Ok(())
}

fn fill_remaining_vacancies_with_rookies(
    conn: &Connection,
    teams: &[crate::models::team::Team],
    new_season_number: i32,
    report: &mut MarketReport,
    rng: &mut impl Rng,
) -> Result<(), String> {
    loop {
        let current_drivers = driver_queries::get_all_drivers(conn)
            .map_err(|e| format!("Falha ao recarregar pilotos: {e}"))?;
        let current_by_id: HashMap<String, Driver> = current_drivers
            .iter()
            .cloned()
            .map(|driver| (driver.id.clone(), driver))
            .collect();
        sync_team_slots(conn, teams, &current_by_id)?;
        let vacancies = find_vacancies(conn)?;
        if vacancies.is_empty() {
            break;
        }

        let mut available = find_available_drivers(conn, &HashMap::new())?;
        for vacancy in vacancies {
            let rookie_index = available
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.driver.categoria_atual.is_none())
                .max_by(|(_, a), (_, b)| {
                    a.driver
                        .atributos
                        .skill
                        .total_cmp(&b.driver.atributos.skill)
                })
                .map(|(index, _)| index);

            if let Some(index) = rookie_index {
                let rookie = available.remove(index);
                sign_driver_to_team(
                    conn,
                    &rookie.driver,
                    &vacancy,
                    new_season_number,
                    calculate_emergency_salary(&vacancy, &rookie.driver),
                    1,
                    vacancy.papel_necessario.clone(),
                )?;
                report.rookies_placed += 1;
                report.new_signings.push(SigningInfo {
                    driver_id: rookie.driver.id.clone(),
                    driver_name: rookie.driver.nome.clone(),
                    team_id: vacancy.team_id.clone(),
                    team_name: vacancy.team_name.clone(),
                    categoria: vacancy.categoria.clone(),
                    papel: vacancy.papel_necessario.as_str().to_string(),
                    tipo: "rookie".to_string(),
                });
                continue;
            }

            let emergency = generate_emergency_rookie(conn, rng)?;
            sign_driver_to_team(
                conn,
                &emergency,
                &vacancy,
                new_season_number,
                calculate_emergency_salary(&vacancy, &emergency),
                1,
                vacancy.papel_necessario.clone(),
            )?;
            report.rookies_placed += 1;
            report.new_signings.push(SigningInfo {
                driver_id: emergency.id.clone(),
                driver_name: emergency.nome.clone(),
                team_id: vacancy.team_id.clone(),
                team_name: vacancy.team_name.clone(),
                categoria: vacancy.categoria.clone(),
                papel: vacancy.papel_necessario.as_str().to_string(),
                tipo: "rookie".to_string(),
            });
        }
    }

    Ok(())
}

fn calculate_emergency_salary(vacancy: &Vacancy, driver: &Driver) -> f64 {
    let tier_base = match vacancy.category_tier {
        0 => 9_000.0,
        1 => 18_000.0,
        2 => 35_000.0,
        3 => 70_000.0,
        4 => 120_000.0,
        _ => 50_000.0,
    };
    (tier_base * (driver.atributos.skill / 75.0).max(0.7)).max(5_000.0)
}

fn generate_emergency_rookie(conn: &Connection, rng: &mut impl Rng) -> Result<Driver, String> {
    let existing_drivers = driver_queries::get_all_drivers(conn)
        .map_err(|e| format!("Falha ao carregar nomes para rookie emergencial: {e}"))?;
    let mut names: HashSet<String> = existing_drivers
        .into_iter()
        .map(|driver| driver.nome)
        .collect();
    let mut rookies = crate::evolution::rookies::generate_rookies(1, &mut names, rng);
    let mut rookie = rookies
        .pop()
        .ok_or_else(|| "Falha ao gerar rookie emergencial".to_string())?;
    rookie.id = next_id(conn, IdType::Driver)
        .map_err(|e| format!("Falha ao gerar ID de rookie emergencial: {e}"))?;
    driver_queries::insert_driver(conn, &rookie)
        .map_err(|e| format!("Falha ao persistir rookie emergencial: {e}"))?;
    Ok(rookie)
}

fn refresh_team_hierarchy(
    conn: &Connection,
    teams: &[crate::models::team::Team],
    drivers_by_id: &HashMap<String, Driver>,
) -> Result<(), String> {
    for team in teams {
        let refreshed = team_queries::get_team_by_id(conn, &team.id)
            .map_err(|e| format!("Falha ao recarregar equipe '{}': {e}", team.nome))?
            .ok_or_else(|| format!("Equipe '{}' nao encontrada", team.id))?;
        let mut pilots = Vec::new();
        if let Some(pilot_id) = &refreshed.piloto_1_id {
            if let Some(driver) = drivers_by_id.get(pilot_id) {
                pilots.push(driver);
            }
        }
        if let Some(pilot_id) = &refreshed.piloto_2_id {
            if let Some(driver) = drivers_by_id.get(pilot_id) {
                pilots.push(driver);
            }
        }
        pilots.sort_by(|a, b| b.atributos.skill.total_cmp(&a.atributos.skill));
        let n1 = pilots.first().map(|driver| driver.id.as_str());
        let n2 = pilots.get(1).map(|driver| driver.id.as_str());
        team_queries::update_team_pilots(conn, &team.id, n1, n2).map_err(|e| {
            format!(
                "Falha ao atualizar pilotos finais da equipe '{}': {e}",
                team.nome
            )
        })?;
        team_queries::update_team_hierarchy(
            conn,
            &team.id,
            n1,
            n2,
            HierarchyStatus::Estavel.as_str(),
            0.0,
        )
        .map_err(|e| {
            format!(
                "Falha ao atualizar hierarquia da equipe '{}': {e}",
                team.nome
            )
        })?;
    }
    Ok(())
}

fn timestamp_now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};
    use rusqlite::Connection;

    use super::*;
    use crate::constants::teams::get_team_templates;
    use crate::db::migrations;
    use crate::db::queries::seasons as season_queries;
    use crate::models::season::Season;

    #[test]
    fn test_market_fills_all_vacancies() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(300);

        let report = run_market(&conn, 2, &mut rng).expect("market should run");

        assert_eq!(report.unresolved_vacancies, 0);
        assert!(find_vacancies(&conn).expect("vacancies").is_empty());
    }

    #[test]
    fn test_market_expired_contracts_processed() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(301);

        let report = run_market(&conn, 2, &mut rng).expect("market should run");

        assert!(report.contracts_expired >= 1);
        let status: String = conn
            .query_row(
                "SELECT status FROM contracts WHERE id = 'C002'",
                [],
                |row| row.get(0),
            )
            .expect("expired contract status");
        assert_eq!(status, "Expirado");
    }

    #[test]
    fn test_market_all_teams_have_two_pilots() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(302);

        run_market(&conn, 2, &mut rng).expect("market should run");

        let teams = team_queries::get_all_teams(&conn).expect("teams");
        assert!(teams
            .iter()
            .all(|team| team.piloto_1_id.is_some() && team.piloto_2_id.is_some()));
    }

    #[test]
    fn test_market_hierarchy_updated() {
        let conn = setup_market_fixture();
        let mut rng = StdRng::seed_from_u64(303);

        run_market(&conn, 2, &mut rng).expect("market should run");

        let teams = team_queries::get_all_teams(&conn).expect("teams");
        assert!(teams.iter().all(|team| team.hierarquia_n1_id.is_some()));
        assert!(teams.iter().all(|team| team.hierarquia_n2_id.is_some()));
        assert!(teams
            .iter()
            .all(|team| team.hierarquia_status == HierarchyStatus::Estavel.as_str()));
    }

    #[test]
    fn test_load_market_contexts_fails_on_corrupted_standings_row() {
        let conn = setup_market_fixture();
        conn.execute(
            "UPDATE standings
             SET categoria = CAST(X'00' AS BLOB)
             WHERE temporada_id = 'S001' AND piloto_id = 'P001'",
            [],
        )
        .expect("corrupt standings row");

        let drivers_by_id: HashMap<String, Driver> = driver_queries::get_all_drivers(&conn)
            .expect("drivers")
            .into_iter()
            .map(|driver| (driver.id.clone(), driver))
            .collect();
        let expiring_by_driver: HashMap<String, Contract> = HashMap::new();

        let result = load_market_contexts(
            &conn,
            Some("S001"),
            &drivers_by_id,
            &expiring_by_driver,
        );

        let err = result.expect_err("corrupted standings should fail");
        assert!(err.contains("Falha ao ler categoria do standings"));
    }

    fn setup_market_fixture() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        migrations::run_all(&conn).expect("schema");

        let previous = Season::new("S001".to_string(), 1, 2024);
        let next = Season::new("S002".to_string(), 2, 2025);
        season_queries::insert_season(&conn, &previous).expect("previous season");
        season_queries::finalize_season(&conn, &previous.id).expect("finalize previous");
        season_queries::insert_season(&conn, &next).expect("next season");

        let mut team_rng = StdRng::seed_from_u64(200);
        let team_a = sample_team("gt4", "T001", &mut team_rng);
        let team_b = sample_team("gt4", "T002", &mut team_rng);
        team_queries::insert_team(&conn, &team_a).expect("team a");
        team_queries::insert_team(&conn, &team_b).expect("team b");

        let driver_a = sample_driver("P001", "Piloto A", Some("gt4"), 78.0, DriverStatus::Ativo);
        let driver_b = sample_driver("P002", "Piloto B", Some("gt4"), 66.0, DriverStatus::Ativo);
        let driver_c = sample_driver(
            "P003",
            "Piloto C",
            Some("gt4"),
            62.0,
            DriverStatus::Aposentado,
        );
        let driver_d = sample_driver("P004", "Piloto D", Some("gt4"), 74.0, DriverStatus::Ativo);
        let driver_e = sample_driver("P005", "Piloto E", None, 59.0, DriverStatus::Ativo);
        let driver_f = sample_driver("P006", "Piloto F", Some("gt3"), 76.0, DriverStatus::Ativo);
        for driver in [
            &driver_a, &driver_b, &driver_c, &driver_d, &driver_e, &driver_f,
        ] {
            driver_queries::insert_driver(&conn, driver).expect("insert driver");
        }

        let contract_a = Contract::new(
            "C001".to_string(),
            driver_a.id.clone(),
            driver_a.nome.clone(),
            team_a.id.clone(),
            team_a.nome.clone(),
            1,
            2,
            140_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        let contract_b = Contract::new(
            "C002".to_string(),
            driver_b.id.clone(),
            driver_b.nome.clone(),
            team_a.id.clone(),
            team_a.nome.clone(),
            1,
            1,
            95_000.0,
            TeamRole::Numero2,
            "gt4".to_string(),
        );
        let contract_c = Contract::new(
            "C003".to_string(),
            driver_c.id.clone(),
            driver_c.nome.clone(),
            team_b.id.clone(),
            team_b.nome.clone(),
            1,
            2,
            85_000.0,
            TeamRole::Numero1,
            "gt4".to_string(),
        );
        contract_queries::insert_contract(&conn, &contract_a).expect("contract a");
        contract_queries::insert_contract(&conn, &contract_b).expect("contract b");
        contract_queries::insert_contract(&conn, &contract_c).expect("contract c");

        team_queries::update_team_pilots(&conn, &team_a.id, Some(&driver_a.id), Some(&driver_b.id))
            .expect("team a pilots");
        team_queries::update_team_pilots(&conn, &team_b.id, Some(&driver_c.id), None)
            .expect("team b pilots");

        insert_standing(
            &conn,
            &previous.id,
            &driver_a.id,
            &team_a.id,
            "gt4",
            1,
            120.0,
            3,
            2,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_b.id,
            &team_a.id,
            "gt4",
            4,
            72.0,
            1,
            1,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_c.id,
            &team_b.id,
            "gt4",
            6,
            40.0,
            0,
            0,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_d.id,
            &team_b.id,
            "gt4",
            2,
            96.0,
            2,
            1,
        );
        insert_standing(
            &conn,
            &previous.id,
            &driver_f.id,
            &team_a.id,
            "gt3",
            3,
            88.0,
            1,
            2,
        );

        conn.execute(
            "UPDATE meta SET value = '4' WHERE key = 'next_contract_id'",
            [],
        )
        .expect("contract counter");
        conn.execute(
            "UPDATE meta SET value = '7' WHERE key = 'next_driver_id'",
            [],
        )
        .expect("driver counter");

        conn
    }

    fn sample_team(category: &str, id: &str, rng: &mut StdRng) -> crate::models::team::Team {
        let template = get_team_templates(category)[0];
        crate::models::team::Team::from_template_with_rng(
            template,
            category,
            id.to_string(),
            2025,
            rng,
        )
    }

    fn sample_driver(
        id: &str,
        name: &str,
        category: Option<&str>,
        skill: f64,
        status: DriverStatus,
    ) -> Driver {
        let mut driver = Driver::new(
            id.to_string(),
            name.to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            24,
            2020,
        );
        driver.categoria_atual = category.map(str::to_string);
        driver.status = status;
        driver.atributos.skill = skill;
        driver.atributos.consistencia = 68.0;
        driver.stats_temporada.vitorias = 1;
        driver.stats_temporada.poles = 1;
        driver.stats_carreira.titulos = 1;
        driver
    }

    fn insert_standing(
        conn: &Connection,
        season_id: &str,
        driver_id: &str,
        team_id: &str,
        category: &str,
        position: i32,
        points: f64,
        wins: i32,
        poles: i32,
    ) {
        conn.execute(
            "INSERT INTO standings (
                temporada_id, piloto_id, equipe_id, categoria, posicao, pontos, vitorias, podios, poles, corridas
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![season_id, driver_id, team_id, category, position, points, wins, wins + 1, poles, 8],
        )
        .expect("insert standing");
    }
}
