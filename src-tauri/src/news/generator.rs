#[cfg(test)]
mod tests {
    use crate::evolution::growth::{AttributeChange, GrowthReport};
    use crate::evolution::pipeline::{
        EndOfSeasonResult, LicenseEarned, RetirementInfo, RookieInfo,
    };
    use crate::market::preseason::{MarketEvent, MarketEventType};
    use crate::promotion::{
        MovementType, PilotEffect, PilotEffectType, PromotionResult, TeamMovement,
    };
    use crate::simulation::race::{RaceDriverResult, RaceResult};

    use super::{
        generate_news_from_end_of_season, generate_news_from_market_events,
        generate_news_from_race, generate_player_rejection_news, generate_player_signing_news,
    };
    use crate::news::{NewsImportance, NewsType};

    #[test]
    fn test_market_event_generates_news() {
        let events = vec![MarketEvent {
            event_type: MarketEventType::ContractExpired,
            headline: "Piloto A deixa Team One".to_string(),
            description: "O contrato de Piloto A com Team One expirou.".to_string(),
            driver_id: Some("P001".to_string()),
            driver_name: Some("Piloto A".to_string()),
            team_id: Some("T001".to_string()),
            team_name: Some("Team One".to_string()),
            from_team: Some("Team One".to_string()),
            to_team: None,
            categoria: Some("gt4".to_string()),
        }];
        let mut next_id = id_gen();
        let mut timestamp = 100;

        let news = generate_news_from_market_events(&events, 2, 1, &mut next_id, &mut timestamp);

        assert_eq!(news.len(), 1);
        assert_eq!(news[0].tipo, NewsType::Mercado);
        assert_eq!(news[0].semana_pretemporada, Some(1));
        assert_eq!(news[0].rodada, Some(-1));
    }

    #[test]
    fn test_transfer_news_is_high_importance() {
        let events = vec![MarketEvent {
            event_type: MarketEventType::TransferCompleted,
            headline: "Piloto B assina com Team Two".to_string(),
            description: "Piloto B deixou o mercado livre e assinou.".to_string(),
            driver_id: Some("P002".to_string()),
            driver_name: Some("Piloto B".to_string()),
            team_id: Some("T002".to_string()),
            team_name: Some("Team Two".to_string()),
            from_team: None,
            to_team: Some("Team Two".to_string()),
            categoria: Some("gt3".to_string()),
        }];
        let mut next_id = id_gen();
        let mut timestamp = 50;

        let news = generate_news_from_market_events(&events, 3, 4, &mut next_id, &mut timestamp);

        assert_eq!(news[0].importancia, NewsImportance::Alta);
        assert_eq!(news[0].tipo, NewsType::Mercado);
    }

    #[test]
    fn test_player_proposal_news_is_destaque() {
        let events = vec![MarketEvent {
            event_type: MarketEventType::PlayerProposalReceived,
            headline: "Voce recebeu uma proposta de Team Three".to_string(),
            description: "Team Three oferece um assento N1.".to_string(),
            driver_id: Some("P007".to_string()),
            driver_name: Some("Jogador".to_string()),
            team_id: Some("T003".to_string()),
            team_name: Some("Team Three".to_string()),
            from_team: None,
            to_team: Some("Team Three".to_string()),
            categoria: Some("gt4".to_string()),
        }];
        let mut next_id = id_gen();
        let mut timestamp = 1;

        let news = generate_news_from_market_events(&events, 2, 6, &mut next_id, &mut timestamp);

        assert_eq!(news[0].importancia, NewsImportance::Destaque);
        assert_eq!(news[0].icone, "💼");
    }

    #[test]
    fn test_retirement_generates_news() {
        let result = sample_end_of_season();
        let mut next_id = id_gen();
        let mut timestamp = 10;

        let news = generate_news_from_end_of_season(&result, 1, &mut next_id, &mut timestamp);

        assert!(news.iter().any(|item| {
            item.tipo == NewsType::Aposentadoria && item.titulo.contains("Veterano")
        }));
    }

    #[test]
    fn test_promotion_generates_destaque_news() {
        let mut result = sample_end_of_season();
        result.promotion_result.movements = vec![TeamMovement {
            team_id: "T010".to_string(),
            team_name: "Team Rise".to_string(),
            from_category: "mazda_rookie".to_string(),
            to_category: "mazda_amador".to_string(),
            movement_type: MovementType::Promocao,
            reason: "Campea de construtores".to_string(),
        }];
        let mut next_id = id_gen();
        let mut timestamp = 10;

        let news = generate_news_from_end_of_season(&result, 1, &mut next_id, &mut timestamp);

        let promotion = news
            .iter()
            .find(|item| item.tipo == NewsType::Promocao)
            .expect("promotion news");
        assert_eq!(promotion.importancia, NewsImportance::Destaque);
    }

    #[test]
    fn test_race_player_result_news() {
        let race_result = RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![
                RaceDriverResult {
                    pilot_id: "P007".to_string(),
                    pilot_name: "Jogador".to_string(),
                    team_id: "T001".to_string(),
                    team_name: "Equipe".to_string(),
                    grid_position: 5,
                    finish_position: 2,
                    positions_gained: 3,
                    best_lap_time_ms: 90_000.0,
                    total_race_time_ms: 3_600_000.0,
                    gap_to_winner_ms: 2_000.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 0,
                    incidents: Vec::new(),
                    has_fastest_lap: false,
                    points_earned: 18,
                    is_jogador: true,
                    laps_completed: 20,
                    final_tire_wear: 0.8,
                    final_physical: 0.9,
                },
                RaceDriverResult {
                    pilot_id: "P003".to_string(),
                    pilot_name: "Vencedor".to_string(),
                    team_id: "T002".to_string(),
                    team_name: "Time Vencedor".to_string(),
                    grid_position: 1,
                    finish_position: 1,
                    positions_gained: 0,
                    best_lap_time_ms: 89_500.0,
                    total_race_time_ms: 3_598_000.0,
                    gap_to_winner_ms: 0.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 0,
                    incidents: Vec::new(),
                    has_fastest_lap: true,
                    points_earned: 25,
                    is_jogador: false,
                    laps_completed: 20,
                    final_tire_wear: 0.82,
                    final_physical: 0.92,
                },
            ],
            pole_sitter_id: "P003".to_string(),
            winner_id: "P003".to_string(),
            fastest_lap_id: "P003".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Interlagos".to_string(),
            total_incidents: 0,
            total_dnfs: 0,
        };
        let mut next_id = id_gen();
        let mut timestamp = 200;

        let news = generate_news_from_race(&race_result, 2, 3, "gt4", crate::models::enums::ThematicSlot::NaoClassificado, &mut next_id, &mut timestamp);

        assert!(news.iter().any(|item| {
            item.tipo == NewsType::Corrida && item.titulo.contains("Voce terminou em P2")
        }));
    }

    #[test]
    fn test_generate_player_signing_news() {
        let news = generate_player_signing_news("Jogador", "Team Nova", "gt4", "Numero1", 2);
        assert_eq!(news.tipo, NewsType::Mercado);
        assert_eq!(news.importancia, NewsImportance::Destaque);
        assert!(news.titulo.contains("Team Nova"));
    }

    #[test]
    fn test_generate_player_rejection_news() {
        let news = generate_player_rejection_news("Jogador", "Team Nova", 2);
        assert_eq!(news.tipo, NewsType::Mercado);
        assert_eq!(news.importancia, NewsImportance::Media);
        assert!(news.titulo.contains("Team Nova"));
    }

    fn id_gen() -> impl FnMut() -> String {
        let mut counter = 1;
        move || {
            let id = format!("N{:03}", counter);
            counter += 1;
            id
        }
    }

    fn sample_end_of_season() -> EndOfSeasonResult {
        EndOfSeasonResult {
            growth_reports: vec![GrowthReport {
                driver_id: "P100".to_string(),
                driver_name: "Evolutivo".to_string(),
                changes: vec![AttributeChange {
                    attribute: "skill".to_string(),
                    old_value: 60,
                    new_value: 65,
                    delta: 5,
                    reason: "Temporada forte".to_string(),
                }],
                overall_delta: 5.0,
            }],
            motivation_reports: Vec::new(),
            retirements: vec![RetirementInfo {
                driver_id: "P200".to_string(),
                driver_name: "Veterano".to_string(),
                age: 41,
                reason: "Aposentou-se aos 41 anos".to_string(),
            }],
            rookies_generated: vec![RookieInfo {
                driver_id: "P300".to_string(),
                driver_name: "Jovem Talento".to_string(),
                nationality: "br".to_string(),
                age: 18,
                skill: 64,
                tipo: "Genio".to_string(),
            }],
            new_season_id: "S002".to_string(),
            new_year: 2025,
            licenses_earned: vec![LicenseEarned {
                driver_id: "P100".to_string(),
                driver_name: "Evolutivo".to_string(),
                license_level: 1,
                category: "gt4".to_string(),
            }],
            promotion_result: PromotionResult {
                movements: vec![TeamMovement {
                    team_id: "T001".to_string(),
                    team_name: "Team Rise".to_string(),
                    from_category: "gt4".to_string(),
                    to_category: "gt3".to_string(),
                    movement_type: MovementType::Promocao,
                    reason: "Top 3 do campeonato".to_string(),
                }],
                pilot_effects: vec![
                    PilotEffect {
                        driver_id: "P500".to_string(),
                        driver_name: "Sem Licenca".to_string(),
                        team_id: "T001".to_string(),
                        effect: PilotEffectType::FreedNoLicense,
                        reason: "Sem licenca para a nova categoria".to_string(),
                    },
                    PilotEffect {
                        driver_id: "P007".to_string(),
                        driver_name: "Jogador".to_string(),
                        team_id: "T001".to_string(),
                        effect: PilotEffectType::FreedPlayerStays,
                        reason: "Jogador sem licenca, fica na categoria".to_string(),
                    },
                ],
                attribute_deltas: Vec::new(),
                errors: Vec::new(),
            },
            preseason_initialized: true,
            preseason_total_weeks: 7,
        }
    }
}
use crate::evolution::pipeline::EndOfSeasonResult;
use crate::market::preseason::{MarketEvent, MarketEventType};
use crate::models::enums::ThematicSlot;
use crate::news::{NewsImportance, NewsItem, NewsType};
use crate::promotion::{MovementType, PilotEffectType};
use crate::simulation::race::RaceResult;

pub fn generate_news_from_market_events(
    events: &[MarketEvent],
    temporada: i32,
    semana: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Vec<NewsItem> {
    events
        .iter()
        .map(|event| {
            let (tipo, importancia, icone, titulo, texto) = match event.event_type {
                MarketEventType::ContractExpired => (
                    NewsType::Mercado,
                    NewsImportance::Media,
                    "📋".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::ContractRenewed => (
                    NewsType::Mercado,
                    NewsImportance::Media,
                    "✍️".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::TransferCompleted => (
                    NewsType::Mercado,
                    NewsImportance::Alta,
                    "📋".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::TransferRejected => (
                    NewsType::Mercado,
                    NewsImportance::Baixa,
                    "🗞️".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::RookieSigned => (
                    NewsType::Rookies,
                    NewsImportance::Media,
                    "🎓".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::PlayerProposalReceived => (
                    NewsType::Mercado,
                    NewsImportance::Destaque,
                    "💼".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::HierarchyUpdated => (
                    NewsType::Hierarquia,
                    NewsImportance::Baixa,
                    "⚡".to_string(),
                    event.headline.clone(),
                    event.description.clone(),
                ),
                MarketEventType::PreSeasonComplete => (
                    NewsType::PreTemporada,
                    NewsImportance::Alta,
                    "📰".to_string(),
                    "Pre-temporada encerrada!".to_string(),
                    "Todas as movimentacoes foram concluidas. A nova temporada esta prestes a comecar."
                        .to_string(),
                ),
            };

            build_news_item(
                next_id,
                timestamp,
                tipo,
                importancia,
                icone,
                titulo,
                texto,
                Some(-semana),
                Some(semana),
                temporada,
                event.categoria.clone(),
                event.categoria.as_deref().map(format_category_name),
                event.driver_id.clone(),
                event.team_id.clone(),
            )
        })
        .collect()
}

pub fn generate_news_from_end_of_season(
    result: &EndOfSeasonResult,
    temporada: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Vec<NewsItem> {
    let mut news = Vec::new();

    for retirement in &result.retirements {
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Aposentadoria,
            NewsImportance::Alta,
            "👴".to_string(),
            format!("{} se aposenta", retirement.driver_name),
            format!(
                "{} encerra sua carreira aos {} anos. {}",
                retirement.driver_name, retirement.age, retirement.reason
            ),
            Some(0),
            None,
            temporada,
            None,
            None,
            Some(retirement.driver_id.clone()),
            None,
        ));
    }

    for movement in &result.promotion_result.movements {
        let (tipo, importancia, icone, verb) = match movement.movement_type {
            MovementType::Promocao => (
                NewsType::Promocao,
                NewsImportance::Destaque,
                "⬆️",
                "promovida para",
            ),
            MovementType::Rebaixamento => (
                NewsType::Rebaixamento,
                NewsImportance::Alta,
                "⬇️",
                "rebaixada para",
            ),
        };
        let target_name = format_category_name(&movement.to_category);
        news.push(build_news_item(
            next_id,
            timestamp,
            tipo,
            importancia,
            icone.to_string(),
            format!("{} {} {}", movement.team_name, verb, target_name),
            format!(
                "{} {}. Sai de {}.",
                movement.team_name,
                movement.reason,
                format_category_name(&movement.from_category)
            ),
            Some(0),
            None,
            temporada,
            Some(movement.to_category.clone()),
            Some(target_name),
            None,
            Some(movement.team_id.clone()),
        ));
    }

    for effect in &result.promotion_result.pilot_effects {
        match effect.effect {
            PilotEffectType::FreedNoLicense => news.push(build_news_item(
                next_id,
                timestamp,
                NewsType::Mercado,
                NewsImportance::Alta,
                "📋".to_string(),
                format!("{} fica sem equipe", effect.driver_name),
                format!("{}: {}", effect.driver_name, effect.reason),
                Some(0),
                None,
                temporada,
                None,
                None,
                Some(effect.driver_id.clone()),
                Some(effect.team_id.clone()),
            )),
            PilotEffectType::FreedPlayerStays => news.push(build_news_item(
                next_id,
                timestamp,
                NewsType::Mercado,
                NewsImportance::Destaque,
                "💼".to_string(),
                "Voce ficou sem equipe!".to_string(),
                effect.reason.clone(),
                Some(0),
                None,
                temporada,
                None,
                None,
                Some(effect.driver_id.clone()),
                Some(effect.team_id.clone()),
            )),
            PilotEffectType::MovesWithTeam => {}
        }
    }

    for rookie in &result.rookies_generated {
        let tipo_label = match rookie.tipo.as_str() {
            "Genio" => "prodigio",
            "Talento" => "jovem talento",
            _ => "novo piloto",
        };
        let importance = if rookie.tipo == "Genio" {
            NewsImportance::Alta
        } else {
            NewsImportance::Media
        };
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Rookies,
            importance,
            "🎓".to_string(),
            format!("{} e o novo {} do grid", rookie.driver_name, tipo_label),
            format!(
                "{}, {} anos, entra no grid como {}.",
                rookie.driver_name, rookie.age, tipo_label
            ),
            Some(0),
            None,
            temporada,
            None,
            None,
            Some(rookie.driver_id.clone()),
            None,
        ));
    }

    if !result.licenses_earned.is_empty() {
        let count = result.licenses_earned.len();
        let names: Vec<_> = result
            .licenses_earned
            .iter()
            .take(5)
            .map(|license| license.driver_name.clone())
            .collect();
        let names_str = if count > 5 {
            format!("{} e mais {} pilotos", names.join(", "), count - 5)
        } else {
            names.join(", ")
        };
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Milestone,
            NewsImportance::Media,
            "🏅".to_string(),
            format!("{} pilotos conquistaram novas licencas", count),
            format!("Licencas conquistadas: {}", names_str),
            Some(0),
            None,
            temporada,
            None,
            None,
            None,
            None,
        ));
    }

    if let Some(grower) = result
        .growth_reports
        .iter()
        .max_by(|a, b| a.overall_delta.total_cmp(&b.overall_delta))
        .filter(|report| report.overall_delta > 3.0)
    {
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Evolucao,
            NewsImportance::Alta,
            "📈".to_string(),
            format!("{} teve evolucao impressionante", grower.driver_name),
            format!(
                "{} mostrou grande evolucao na pre-temporada.",
                grower.driver_name
            ),
            Some(0),
            None,
            temporada,
            None,
            None,
            Some(grower.driver_id.clone()),
            None,
        ));
    }

    if let Some(decliner) = result
        .growth_reports
        .iter()
        .min_by(|a, b| a.overall_delta.total_cmp(&b.overall_delta))
        .filter(|report| report.overall_delta < -3.0)
    {
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Evolucao,
            NewsImportance::Media,
            "📉".to_string(),
            format!("{} em declinio", decliner.driver_name),
            format!("{} apresentou queda de desempenho.", decliner.driver_name),
            Some(0),
            None,
            temporada,
            None,
            None,
            Some(decliner.driver_id.clone()),
            None,
        ));
    }

    news
}

pub fn generate_news_from_race(
    race_result: &RaceResult,
    temporada: i32,
    rodada: i32,
    categoria: &str,
    thematic_slot: ThematicSlot,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Vec<NewsItem> {
    let mut news = Vec::new();
    let category_name = Some(format_category_name(categoria));

    if let Some(winner) = race_result
        .race_results
        .iter()
        .find(|result| result.finish_position == 1)
    {
        // Tom narrativo baseado no papel temático da corrida
        let (titulo, importancia_base) = match thematic_slot {
            ThematicSlot::FinalDaTemporada => (
                format!("Grande Final: {} vence em {}", winner.pilot_name, race_result.track_name),
                NewsImportance::Alta,
            ),
            ThematicSlot::FinalEspecial => (
                format!("Encerramento especial: {} vence em {}", winner.pilot_name, race_result.track_name),
                NewsImportance::Alta,
            ),
            ThematicSlot::AberturaDaTemporada => (
                format!("Temporada abre em {}: {} vence", race_result.track_name, winner.pilot_name),
                NewsImportance::Alta,
            ),
            ThematicSlot::TensaoPreFinal => (
                format!("Tensão antes da decisão: {} vence em {}", winner.pilot_name, race_result.track_name),
                NewsImportance::Alta,
            ),
            ThematicSlot::VisitanteRegional => (
                format!("Visita especial a {}: {} vence", race_result.track_name, winner.pilot_name),
                NewsImportance::Alta,
            ),
            ThematicSlot::MidpointPrestigio => (
                format!("{} vence na pista de prestígio em {}", winner.pilot_name, race_result.track_name),
                NewsImportance::Alta,
            ),
            // Slots regulares, especiais sem distinção ou não classificado: tom neutro
            _ => (
                format!("{} vence em {}", winner.pilot_name, race_result.track_name),
                NewsImportance::Alta,
            ),
        };

        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            importancia_base,
            "🏆".to_string(),
            titulo,
            format!(
                "{} venceu a corrida em {} partindo de P{}.",
                winner.pilot_name, race_result.track_name, winner.grid_position
            ),
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name.clone(),
            Some(winner.pilot_id.clone()),
            Some(winner.team_id.clone()),
        ));
    }

    if let Some(mover) = race_result
        .race_results
        .iter()
        .filter(|result| !result.is_dnf)
        .max_by_key(|result| result.positions_gained)
        .filter(|result| result.positions_gained >= 3)
    {
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            NewsImportance::Media,
            "📈".to_string(),
            format!(
                "{} ganha {} posicoes!",
                mover.pilot_name, mover.positions_gained
            ),
            format!(
                "{} avancou {} posicoes durante a corrida.",
                mover.pilot_name, mover.positions_gained
            ),
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name.clone(),
            Some(mover.pilot_id.clone()),
            Some(mover.team_id.clone()),
        ));
    }

    if let Some(player_result) = race_result
        .race_results
        .iter()
        .find(|result| result.is_jogador)
    {
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            NewsImportance::Destaque,
            "🏁".to_string(),
            format!("Voce terminou em P{}", player_result.finish_position),
            format!(
                "Largou P{}, terminou P{}. {} pontos conquistados.",
                player_result.grid_position,
                player_result.finish_position,
                player_result.points_earned
            ),
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name,
            Some(player_result.pilot_id.clone()),
            Some(player_result.team_id.clone()),
        ));
    }

    news
}

fn build_news_item(
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
    tipo: NewsType,
    importancia: NewsImportance,
    icone: String,
    titulo: String,
    texto: String,
    rodada: Option<i32>,
    semana_pretemporada: Option<i32>,
    temporada: i32,
    categoria_id: Option<String>,
    categoria_nome: Option<String>,
    driver_id: Option<String>,
    team_id: Option<String>,
) -> NewsItem {
    let item = NewsItem {
        id: next_id(),
        tipo,
        icone,
        titulo,
        texto,
        rodada,
        semana_pretemporada,
        temporada,
        categoria_id,
        categoria_nome,
        importancia,
        timestamp: *timestamp,
        driver_id,
        team_id,
    };
    *timestamp += 1;
    item
}

fn format_category_name(category_id: &str) -> String {
    match category_id {
        "mazda_rookie" => "Mazda Rookie".to_string(),
        "toyota_rookie" => "Toyota Rookie".to_string(),
        "mazda_amador" => "Mazda Championship".to_string(),
        "toyota_amador" => "Toyota Cup".to_string(),
        "bmw_m2" => "BMW M2".to_string(),
        "production_challenger" => "Production Challenger".to_string(),
        "gt4" => "GT4".to_string(),
        "gt3" => "GT3".to_string(),
        "endurance" => "Endurance".to_string(),
        other => other.to_string(),
    }
}

pub fn generate_player_signing_news(
    player_name: &str,
    team_name: &str,
    categoria: &str,
    papel: &str,
    temporada: i32,
) -> NewsItem {
    NewsItem {
        id: String::new(),
        tipo: NewsType::Mercado,
        icone: "✍️".to_string(),
        titulo: format!("Voce assinou com {}!", team_name),
        texto: format!(
            "{} e o novo {} da {} para a temporada {} em {}.",
            player_name,
            if papel == "Numero1" {
                "piloto principal"
            } else {
                "segundo piloto"
            },
            team_name,
            temporada,
            format_category_name(categoria),
        ),
        rodada: Some(0),
        semana_pretemporada: None,
        temporada,
        categoria_id: Some(categoria.to_string()),
        categoria_nome: Some(format_category_name(categoria)),
        importancia: NewsImportance::Destaque,
        timestamp: 0,
        driver_id: None,
        team_id: None,
    }
}

pub fn generate_player_rejection_news(
    player_name: &str,
    team_name: &str,
    temporada: i32,
) -> NewsItem {
    NewsItem {
        id: String::new(),
        tipo: NewsType::Mercado,
        icone: "❌".to_string(),
        titulo: format!("Voce recusou proposta de {}", team_name),
        texto: format!(
            "{} declinou a oferta de {} para a temporada {}.",
            player_name, team_name, temporada
        ),
        rodada: Some(0),
        semana_pretemporada: None,
        temporada,
        categoria_id: None,
        categoria_nome: None,
        importancia: NewsImportance::Media,
        timestamp: 0,
        driver_id: None,
        team_id: None,
    }
}

/// Gera notícias de encerramento do bloco especial.
///
/// `campeoes` é uma lista de `(categoria, classe, Option<driver_nome>)` representando
/// o piloto com mais pontos por classe. Se `driver_nome` for None, nenhuma notícia
/// de campeão é gerada para aquela classe.
pub fn generate_news_from_pos_especial(
    campeoes: &[(String, String, Option<String>)],
    temporada: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Vec<NewsItem> {
    let mut items = Vec::new();

    for (categoria, classe, maybe_nome) in campeoes {
        if let Some(nome) = maybe_nome {
            let cat_nome = format_category_name(categoria);
            let titulo = format!("Campeao do {} — {}", cat_nome, classe.to_uppercase());
            let texto = format!(
                "{} se sagra campeao da classe {} no {} da temporada {}.",
                nome,
                classe.to_uppercase(),
                cat_nome,
                temporada,
            );
            items.push(build_news_item(
                next_id,
                timestamp,
                NewsType::Corrida,
                NewsImportance::Destaque,
                "🏆".to_string(),
                titulo,
                texto,
                None,
                None,
                temporada,
                Some(categoria.clone()),
                Some(cat_nome),
                None,
                None,
            ));
        }
    }

    items
}
