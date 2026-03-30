#[cfg(test)]
mod tests {
    use crate::db::queries::race_history::StandingEntry;
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
        build_incident_news_item, build_injury_news_items, generate_news_from_end_of_season,
        generate_news_from_market_events, generate_news_from_pos_especial, generate_news_from_race,
        generate_player_rejection_news, generate_player_signing_news, select_primary_incident,
    };
    use crate::models::enums::InjuryType;
    use crate::models::injury::Injury;
    use crate::news::race_context::{
        RaceNarrativeContext, TeamPerformanceTier, WeatherNarrative, WinnerNarrativeContext,
    };
    use crate::news::{NewsImportance, NewsType};
    use crate::simulation::incidents::{IncidentResult, IncidentSeverity, IncidentType};

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

        let news = generate_news_from_market_events(
            &events,
            2,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

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

        let news = generate_news_from_market_events(
            &events,
            3,
            4,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

        assert_eq!(news[0].importancia, NewsImportance::Alta);
        assert_eq!(news[0].tipo, NewsType::Mercado);
    }

    #[test]
    fn test_player_proposal_news_is_destaque() {
        let events = vec![MarketEvent {
            event_type: MarketEventType::PlayerProposalReceived,
            headline: "Jogador recebe proposta de Team Three".to_string(),
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

        let news = generate_news_from_market_events(
            &events,
            2,
            6,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );

        assert_eq!(news[0].importancia, NewsImportance::Destaque);
        assert!(
            news[0].titulo.contains("Jogador") || news[0].titulo.contains("Team Three"),
            "proposal title should contain player or team name, got: {}",
            news[0].titulo
        );
    }

    #[test]
    fn test_retirement_generates_news() {
        let result = sample_end_of_season();
        let mut next_id = id_gen();
        let mut timestamp = 10;

        let news = generate_news_from_end_of_season(
            &result,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

        let retirement = news
            .iter()
            .find(|item| {
                item.tipo == NewsType::Aposentadoria && item.driver_id.as_deref() == Some("P200")
            })
            .expect("retirement news");
        assert!(retirement.titulo.contains("Veterano"));
        assert_eq!(retirement.categoria_id, Some("gt3".to_string()));
        assert_eq!(retirement.categoria_nome, Some("GT3".to_string()));
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

        let news = generate_news_from_end_of_season(
            &result,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

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
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
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
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
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
            main_incident_count: 0,
            notable_incident_pilot_ids: Vec::new(),
            most_positions_gained_id: None,
        };
        let mut next_id = id_gen();
        let mut timestamp = 200;

        let news = generate_news_from_race(
            &race_result,
            2,
            3,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

        let player_news = news
            .iter()
            .find(|item| {
                item.tipo == NewsType::Corrida
                    && item.driver_id.as_deref() == Some("P007")
                    && (item.titulo.contains("P2")
                        || item.titulo.contains("2º")
                        || item.texto.contains("P2"))
                    && item.texto.contains("18")
            })
            .expect("player race news");
        assert_eq!(player_news.importancia, NewsImportance::Media);
    }

    #[test]
    fn test_midfield_player_result_news_stays_low_importance() {
        let race_result = RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![
                RaceDriverResult {
                    pilot_id: "P007".to_string(),
                    pilot_name: "Jogador".to_string(),
                    team_id: "T001".to_string(),
                    team_name: "Equipe".to_string(),
                    grid_position: 10,
                    finish_position: 8,
                    positions_gained: 2,
                    best_lap_time_ms: 90_300.0,
                    total_race_time_ms: 3_610_000.0,
                    gap_to_winner_ms: 15_000.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 0,
                    incidents: Vec::new(),
                    has_fastest_lap: false,
                    points_earned: 4,
                    is_jogador: true,
                    laps_completed: 20,
                    final_tire_wear: 0.8,
                    final_physical: 0.9,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
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
                    total_race_time_ms: 3_595_000.0,
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
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
            ],
            pole_sitter_id: "P003".to_string(),
            winner_id: "P003".to_string(),
            fastest_lap_id: "P003".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Okayama".to_string(),
            total_incidents: 0,
            total_dnfs: 0,
            main_incident_count: 0,
            notable_incident_pilot_ids: Vec::new(),
            most_positions_gained_id: None,
        };
        let mut next_id = id_gen();
        let mut timestamp = 240;

        let news = generate_news_from_race(
            &race_result,
            2,
            4,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

        let player_news = news
            .iter()
            .find(|item| {
                item.tipo == NewsType::Corrida
                    && item.driver_id.as_deref() == Some("P007")
                    && (item.titulo.contains("P8")
                        || item.titulo.contains("8º")
                        || item.titulo.contains("8ª")
                        || item.texto.contains("P8"))
                    && item.texto.contains("4")
            })
            .expect("midfield player race news");
        assert_eq!(player_news.importancia, NewsImportance::Baixa);
        assert!(
            player_news.titulo.contains("Jogador")
                || player_news.texto.contains("Jogador"),
            "player race news should mention the pilot name",
        );
        assert!(
            !player_news.titulo.contains("VOC")
                && !player_news.titulo.contains("Voc")
                && !player_news.texto.contains("VOC")
                && !player_news.texto.contains("Voc")
                && !player_news.texto.contains("voc"),
            "player race news should not use second-person language",
        );
    }

    #[test]
    fn test_winner_is_not_duplicated_as_position_gainer() {
        let race_result = RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![RaceDriverResult {
                pilot_id: "P003".to_string(),
                pilot_name: "Vencedor".to_string(),
                team_id: "T002".to_string(),
                team_name: "Time Vencedor".to_string(),
                grid_position: 4,
                finish_position: 1,
                positions_gained: 3,
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
                classification_status: crate::simulation::race::ClassificationStatus::Finished,
                notable_incident: None,
                dnf_catalog_id: None,
                damage_origin_segment: None,
            }],
            pole_sitter_id: "P003".to_string(),
            winner_id: "P003".to_string(),
            fastest_lap_id: "P003".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Interlagos".to_string(),
            total_incidents: 0,
            total_dnfs: 0,
            main_incident_count: 0,
            notable_incident_pilot_ids: Vec::new(),
            most_positions_gained_id: Some("P003".to_string()),
        };
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_race(
            &race_result,
            1,
            1,
            "gt3",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

        assert_eq!(
            news.iter()
                .filter(|item| item.driver_id.as_deref() == Some("P003"))
                .count(),
            1
        );
    }

    #[test]
    fn test_generate_player_signing_news() {
        let news = generate_player_signing_news(
            "Jogador",
            "P007",
            "Team Nova",
            "T001",
            "gt4",
            "Numero1",
            2,
        );
        assert_eq!(news.tipo, NewsType::Mercado);
        assert_eq!(news.importancia, NewsImportance::Destaque);
        assert!(news.titulo.contains("Team Nova") || news.titulo.contains("Jogador"));
        assert!(!news.titulo.contains("VOC"));
        assert!(!news.texto.contains("Voc") && !news.texto.contains("voc"));
        assert_eq!(news.driver_id, Some("P007".to_string()));
        assert_eq!(news.team_id, Some("T001".to_string()));
    }

    #[test]
    fn test_generate_player_rejection_news() {
        let news = generate_player_rejection_news("Jogador", "P007", "Team Nova", "T001", 2);
        assert_eq!(news.tipo, NewsType::Mercado);
        assert_eq!(news.importancia, NewsImportance::Media);
        assert!(news.titulo.contains("Team Nova") || news.titulo.contains("Você"));
        assert_eq!(news.driver_id, Some("P007".to_string()));
        assert_eq!(news.team_id, Some("T001".to_string()));
    }

    #[test]
    fn test_generate_news_from_pos_especial_carries_driver_id() {
        let mut next_id = id_gen();
        let mut timestamp = 0i64;
        let news = generate_news_from_pos_especial(
            &[(
                "gt3".to_string(),
                "overall".to_string(),
                Some("Campeao".to_string()),
                Some("P777".to_string()),
            )],
            3,
            &mut next_id,
            &mut timestamp,
        );

        assert_eq!(news.len(), 1);
        assert_eq!(news[0].driver_id, Some("P777".to_string()));
        assert_eq!(news[0].categoria_id, Some("gt3".to_string()));
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
                categoria: Some("gt3".to_string()),
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
            licenses_earned: vec![
                LicenseEarned {
                    driver_id: "P100".to_string(),
                    driver_name: "Evolutivo".to_string(),
                    license_level: 1,
                    category: "gt4".to_string(),
                },
                LicenseEarned {
                    driver_id: "P101".to_string(),
                    driver_name: "Outro Piloto".to_string(),
                    license_level: 2,
                    category: "gt3".to_string(),
                },
            ],
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

    fn make_inc(
        id: &str,
        tipo: IncidentType,
        sev: IncidentSeverity,
        dnf: bool,
        linked: Option<&str>,
    ) -> IncidentResult {
        IncidentResult {
            pilot_id: id.to_string(),
            incident_type: tipo,
            severity: sev,
            segment: "Lap1".to_string(),
            positions_lost: 3,
            is_dnf: dnf,
            description: "test".to_string(),
            linked_pilot_id: linked.map(|s| s.to_string()),
            is_two_car_incident: linked.is_some(),
            injury_risk_multiplier: 0.0,
            narrative_importance_hint: 0,
            catalog_id: None,
            damage_origin_segment: None,
        }
    }

    fn make_inc_with_description(
        id: &str,
        tipo: IncidentType,
        sev: IncidentSeverity,
        dnf: bool,
        description: &str,
        linked: Option<&str>,
        damage_origin_segment: Option<&str>,
    ) -> IncidentResult {
        IncidentResult {
            pilot_id: id.to_string(),
            incident_type: tipo,
            severity: sev,
            segment: "Lap1".to_string(),
            positions_lost: if dnf { 0 } else { 3 },
            is_dnf: dnf,
            description: description.to_string(),
            linked_pilot_id: linked.map(|s| s.to_string()),
            is_two_car_incident: linked.is_some(),
            injury_risk_multiplier: 0.0,
            narrative_importance_hint: 0,
            catalog_id: None,
            damage_origin_segment: damage_origin_segment.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_select_primary_incident_prioritizes_critical_collision() {
        let inc_major_dnf = make_inc(
            "P002",
            IncidentType::Collision,
            IncidentSeverity::Major,
            true,
            None,
        );
        let inc_critical = make_inc(
            "P001",
            IncidentType::Collision,
            IncidentSeverity::Critical,
            false,
            None,
        );
        let incidents = [inc_major_dnf, inc_critical];
        let result = select_primary_incident(&incidents, &[], None, &[]);
        // Critical (priority 6) must beat Collision+DNF (priority 5)
        assert_eq!(result.map(|i| i.pilot_id.as_str()), Some("P001"));
    }

    #[test]
    fn test_select_primary_incident_player_tiebreak() {
        // Two incidents of same priority — player involvement wins tiebreak
        let inc_a = make_inc(
            "P001",
            IncidentType::DriverError,
            IncidentSeverity::Major,
            true,
            None,
        );
        let inc_b = make_inc(
            "P007",
            IncidentType::DriverError,
            IncidentSeverity::Major,
            true,
            None,
        );
        let incidents = [inc_a, inc_b];
        let result = select_primary_incident(&incidents, &[], Some("P007"), &[]);
        assert_eq!(result.map(|i| i.pilot_id.as_str()), Some("P007"));
    }

    // ── Testes de race_framing ─────────────────────────────────────────────────

    fn make_race_result_for_framing(
        main_incident_count: i32,
        total_dnfs: i32,
        total_incidents: i32,
    ) -> RaceResult {
        RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![RaceDriverResult {
                pilot_id: "P001".to_string(),
                pilot_name: "Vencedor".to_string(),
                team_id: "T001".to_string(),
                team_name: "Equipe".to_string(),
                grid_position: 1,
                finish_position: 1,
                positions_gained: 0,
                best_lap_time_ms: 89_000.0,
                total_race_time_ms: 3_590_000.0,
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
                final_tire_wear: 0.8,
                final_physical: 0.9,
                classification_status: crate::simulation::race::ClassificationStatus::Finished,
                notable_incident: None,
                dnf_catalog_id: None,
                damage_origin_segment: None,
            }],
            pole_sitter_id: "P001".to_string(),
            winner_id: "P001".to_string(),
            fastest_lap_id: "P001".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Interlagos".to_string(),
            total_incidents,
            total_dnfs,
            main_incident_count,
            notable_incident_pilot_ids: Vec::new(),
            most_positions_gained_id: None,
        }
    }

    #[test]
    fn test_race_framing_limpa() {
        let result = make_race_result_for_framing(0, 0, 0);
        let news = generate_news_from_race(
            &result,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut id_gen(),
            &mut 0i64,
            &std::collections::HashMap::new(),
            None,
        );
        let winner_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("P001"))
            .unwrap();
        assert!(
            winner_news.titulo.contains("Vencedor") && winner_news.titulo.contains("Interlagos"),
            "framing limpa deve mencionar piloto e pista, got: {}",
            winner_news.titulo
        );
        assert!(!winner_news.titulo.to_lowercase().contains("caot"));
        assert!(!winner_news.titulo.contains("movimentada"));
    }

    #[test]
    fn test_race_framing_turbulenta() {
        let result = make_race_result_for_framing(1, 0, 0);
        let news = generate_news_from_race(
            &result,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut id_gen(),
            &mut 0i64,
            &std::collections::HashMap::new(),
            None,
        );
        let winner_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("P001"))
            .unwrap();
        assert!(
            winner_news.titulo.contains("movimentada"),
            "framing turbulenta deve conter 'movimentada', got: {}",
            winner_news.titulo
        );
    }

    #[test]
    fn test_race_framing_caotica() {
        let result = make_race_result_for_framing(3, 0, 0);
        let news = generate_news_from_race(
            &result,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut id_gen(),
            &mut 0i64,
            &std::collections::HashMap::new(),
            None,
        );
        let winner_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("P001"))
            .unwrap();
        let title = winner_news.titulo.to_lowercase();
        assert!(
            title.contains("caot")
                || title.contains("caos")
                || title.contains("carnificina")
                || title.contains("destruicao"),
            "framing caotica deve soar caotico, got: {}",
            winner_news.titulo
        );
    }

    #[test]
    fn test_race_framing_thematic_slot_overrides_framing() {
        // Slot temático tem prioridade — framing caótica não deve alterar o slot
        let result = make_race_result_for_framing(5, 6, 10);
        let news = generate_news_from_race(
            &result,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::FinalDaTemporada,
            &mut id_gen(),
            &mut 0i64,
            &std::collections::HashMap::new(),
            None,
        );
        let winner_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("P001"))
            .unwrap();
        assert!(
            winner_news.titulo.contains("Grande Final"),
            "slot FinalDaTemporada deve ter prioridade, got: {}",
            winner_news.titulo
        );
    }

    #[test]
    fn test_build_injury_news_player_is_destaque() {
        let injury = Injury {
            id: "INJ-1".to_string(),
            pilot_id: "P007".to_string(),
            injury_type: InjuryType::Leve,
            modifier: 0.95,
            races_total: 2,
            races_remaining: 2,
            skill_penalty: 0.05,
            season: 1,
            race_occurred: "R001".to_string(),
            active: true,
        };
        let mut names = std::collections::HashMap::new();
        names.insert("P007".to_string(), "Jogador".to_string());
        let mut id = || "N001".to_string();
        let mut ts = 0i64;
        let items = build_injury_news_items(
            &[injury],
            Some("P007"),
            &names,
            "Interlagos",
            "gt4",
            3,
            1,
            &mut id,
            &mut ts,
            &std::collections::HashMap::new(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].tipo, NewsType::Lesao);
        assert_eq!(items[0].importancia, NewsImportance::Destaque);
    }

    #[test]
    fn test_incident_news_elite_ai_bumps_to_alta() {
        let incident = make_inc(
            "P900",
            IncidentType::Collision,
            IncidentSeverity::Major,
            false,
            None,
        );
        let mut next_id = id_gen();
        let mut timestamp = 0i64;
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("P900".to_string(), 90.0);

        let news = build_incident_news_item(
            &incident,
            "Interlagos",
            "Piloto Elite",
            None,
            false,
            false,
            "gt3",
            4,
            1,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
        );

        assert_eq!(news.importancia, NewsImportance::Alta);
    }

    #[test]
    fn test_incident_news_mechanical_uses_exact_description() {
        let incident = make_inc_with_description(
            "P901",
            IncidentType::Mechanical,
            IncidentSeverity::Major,
            true,
            "Piloto Tecnico abandona com problema no cambio - sincronizador da 3a marcha falhou",
            None,
            None,
        );
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = build_incident_news_item(
            &incident,
            "Interlagos",
            "Piloto Tecnico",
            None,
            false,
            false,
            "gt4",
            4,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
        );

        assert!(
            news.texto
                .contains("problema no cambio - sincronizador da 3a marcha falhou"),
            "mechanical news should preserve exact incident detail, got: {}",
            news.texto
        );
    }

    #[test]
    fn test_incident_news_uses_solo_collision_dnf_title_when_only_one_driver_abandoned() {
        let incident = make_inc_with_description(
            "P902",
            IncidentType::Collision,
            IncidentSeverity::Major,
            true,
            "Rodrigo abandona apos colisao com Aiden",
            Some("P903"),
            None,
        );
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = build_incident_news_item(
            &incident,
            "Charlotte",
            "Rodrigo",
            Some("Aiden"),
            false,
            false,
            "gt4",
            5,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
        );

        assert!(
            !news.titulo.contains("Aiden"),
            "single-DNF collision headline should not imply the linked pilot also retired, got: {}",
            news.titulo
        );
    }

    #[test]
    fn test_select_primary_incident_allows_mechanical_non_dnf() {
        let inc_error_major = make_inc(
            "P002",
            IncidentType::DriverError,
            IncidentSeverity::Major,
            false,
            None,
        );
        let inc_mechanical = make_inc(
            "P001",
            IncidentType::Mechanical,
            IncidentSeverity::Major,
            false,
            None,
        );
        let incidents = [inc_error_major, inc_mechanical];

        let result = select_primary_incident(&incidents, &[], None, &[]);
        assert_eq!(result.map(|i| i.pilot_id.as_str()), Some("P001"));
    }

    #[test]
    fn test_select_primary_incident_keeps_critical_collision_above_mechanical_non_dnf() {
        let inc_mechanical = make_inc(
            "P002",
            IncidentType::Mechanical,
            IncidentSeverity::Major,
            false,
            None,
        );
        let inc_critical = make_inc(
            "P001",
            IncidentType::Collision,
            IncidentSeverity::Critical,
            false,
            None,
        );
        let incidents = [inc_mechanical, inc_critical];

        let result = select_primary_incident(&incidents, &[], None, &[]);
        assert_eq!(result.map(|i| i.pilot_id.as_str()), Some("P001"));
    }

    #[test]
    fn test_generate_news_from_race_adds_incident_summary_for_chaotic_race() {
        let race_result = RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![
                RaceDriverResult {
                    pilot_id: "P001".to_string(),
                    pilot_name: "Vencedor".to_string(),
                    team_id: "T001".to_string(),
                    team_name: "Equipe 1".to_string(),
                    grid_position: 1,
                    finish_position: 1,
                    positions_gained: 0,
                    best_lap_time_ms: 89_000.0,
                    total_race_time_ms: 3_590_000.0,
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
                    final_tire_wear: 0.8,
                    final_physical: 0.9,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
                RaceDriverResult {
                    pilot_id: "P010".to_string(),
                    pilot_name: "Piloto A".to_string(),
                    team_id: "T010".to_string(),
                    team_name: "Equipe A".to_string(),
                    grid_position: 4,
                    finish_position: 18,
                    positions_gained: -14,
                    best_lap_time_ms: 90_500.0,
                    total_race_time_ms: 3_800_000.0,
                    gap_to_winner_ms: 210_000.0,
                    is_dnf: true,
                    dnf_reason: Some("abandono".to_string()),
                    dnf_segment: Some("MID".to_string()),
                    incidents_count: 1,
                    incidents: vec![make_inc_with_description(
                        "P010",
                        IncidentType::Mechanical,
                        IncidentSeverity::Major,
                        true,
                        "Piloto A abandona com problema no cambio - sincronizador da 3a marcha falhou",
                        None,
                        None,
                    )],
                    has_fastest_lap: false,
                    points_earned: 0,
                    is_jogador: false,
                    laps_completed: 11,
                    final_tire_wear: 0.6,
                    final_physical: 0.7,
                    classification_status: crate::simulation::race::ClassificationStatus::Dnf,
                    notable_incident: Some(
                        "Piloto A abandona com problema no cambio - sincronizador da 3a marcha falhou"
                            .to_string(),
                    ),
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
                RaceDriverResult {
                    pilot_id: "P011".to_string(),
                    pilot_name: "Piloto B".to_string(),
                    team_id: "T011".to_string(),
                    team_name: "Equipe B".to_string(),
                    grid_position: 7,
                    finish_position: 12,
                    positions_gained: -5,
                    best_lap_time_ms: 90_900.0,
                    total_race_time_ms: 3_640_000.0,
                    gap_to_winner_ms: 50_000.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 1,
                    incidents: vec![make_inc_with_description(
                        "P011",
                        IncidentType::Mechanical,
                        IncidentSeverity::Minor,
                        false,
                        "Piloto B perdeu rendimento com freios comprometidos",
                        None,
                        None,
                    )],
                    has_fastest_lap: false,
                    points_earned: 0,
                    is_jogador: false,
                    laps_completed: 20,
                    final_tire_wear: 0.7,
                    final_physical: 0.8,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
                RaceDriverResult {
                    pilot_id: "P012".to_string(),
                    pilot_name: "Piloto C".to_string(),
                    team_id: "T012".to_string(),
                    team_name: "Equipe C".to_string(),
                    grid_position: 9,
                    finish_position: 15,
                    positions_gained: -6,
                    best_lap_time_ms: 91_100.0,
                    total_race_time_ms: 3_680_000.0,
                    gap_to_winner_ms: 90_000.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 1,
                    incidents: vec![make_inc_with_description(
                        "P012",
                        IncidentType::Mechanical,
                        IncidentSeverity::Minor,
                        false,
                        "Piloto C perdeu posicoes por dano de colisao anterior",
                        None,
                        Some("EARLY"),
                    )],
                    has_fastest_lap: false,
                    points_earned: 0,
                    is_jogador: false,
                    laps_completed: 20,
                    final_tire_wear: 0.7,
                    final_physical: 0.8,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: Some("EARLY".to_string()),
                },
            ],
            pole_sitter_id: "P001".to_string(),
            winner_id: "P001".to_string(),
            fastest_lap_id: "P001".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Interlagos".to_string(),
            total_incidents: 5,
            total_dnfs: 1,
            main_incident_count: 2,
            notable_incident_pilot_ids: vec!["P010".to_string()],
            most_positions_gained_id: None,
        };
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_race(
            &race_result,
            1,
            4,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            None,
        );

        let summary = news
            .iter()
            .find(|item| {
                item.tipo == NewsType::Incidente
                    && item.texto.contains("sincronizador da 3a marcha falhou")
                    && item.texto.contains("freios comprometidos")
            })
            .expect("chaotic race incident summary news");
        assert!(
            summary.texto.contains("consequencia do contato")
                || summary.texto.contains("dano de colisao anterior"),
            "summary should preserve latent damage context, got: {}",
            summary.texto
        );
    }

    #[test]
    fn test_eos_player_license_gets_destaque_and_ai_is_grouped() {
        let result = sample_end_of_season();
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_end_of_season(
            &result,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            Some("P100"),
        );

        let player_license = news
            .iter()
            .find(|item| {
                item.tipo == NewsType::Milestone && item.driver_id.as_deref() == Some("P100")
            })
            .expect("player license news");
        assert_eq!(player_license.importancia, NewsImportance::Destaque);
        assert!(
            player_license.titulo.contains("licença") || player_license.titulo.contains("Licença")
        );
        assert!(player_license.texto.contains("GT4"));
        assert_eq!(player_license.categoria_id, Some("gt4".to_string()));
        assert_eq!(player_license.categoria_nome, Some("GT4".to_string()));

        let ai_group = news
            .iter()
            .find(|item| item.tipo == NewsType::Milestone && item.driver_id.is_none())
            .expect("ai license group");
        assert_eq!(ai_group.importancia, NewsImportance::Media);
        assert!(ai_group.titulo.contains("1") || ai_group.texto.contains("1"));
    }

    // ── Testes de visibilidade narrativa de mercado ───────────────────────────

    #[test]
    fn test_market_visibility_bonus_monotonic() {
        use crate::market::visibility::derive_market_visibility_profile;
        let baixa = derive_market_visibility_profile(10.0);
        let rel = derive_market_visibility_profile(40.0);
        let alta = derive_market_visibility_profile(70.0);
        let elite = derive_market_visibility_profile(90.0);
        assert!((super::market_news_visibility_bonus(&baixa) - 0.0).abs() < 1e-9);
        assert!(
            super::market_news_visibility_bonus(&rel) < super::market_news_visibility_bonus(&alta)
        );
        assert!(
            super::market_news_visibility_bonus(&alta)
                < super::market_news_visibility_bonus(&elite)
        );
        assert!((super::market_news_visibility_bonus(&elite) - 18.0).abs() < 1e-9);
    }

    #[test]
    fn test_market_news_editorial_score_elite_higher_than_baixa() {
        // Mesmo evento, Elite → score narrativo maior que Baixa
        let score_elite = super::market_news_editorial_score(40.0, Some(90.0));
        let score_baixa = super::market_news_editorial_score(40.0, Some(10.0));
        assert!(score_elite > score_baixa);
    }

    #[test]
    fn test_factual_relevance_dominant() {
        // TransferCompleted de piloto Baixa (65.0) supera ContractRenewed de piloto Elite (58.0).
        // Evento factualmente mais forte continua dominando sobre visibilidade pública.
        let score_transfer_baixa = super::market_news_editorial_score(65.0, Some(10.0)); // 65.0
        let score_renewal_elite = super::market_news_editorial_score(40.0, Some(90.0)); // 58.0
        assert!(score_transfer_baixa > score_renewal_elite);
    }

    #[test]
    fn test_elite_renewal_bumps_importance_to_alta() {
        // ContractRenewed com piloto Elite deve gerar NewsImportance::Alta (acima do padrão Media).
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("P001".to_string(), 90.0);
        let event = MarketEvent {
            event_type: MarketEventType::ContractRenewed,
            headline: "Piloto renova".to_string(),
            description: "Renovacao confirmada.".to_string(),
            driver_id: Some("P001".to_string()),
            driver_name: Some("Piloto Elite".to_string()),
            team_id: None,
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let mut next_id = id_gen();
        let mut timestamp = 100;
        let news = generate_news_from_market_events(
            &[event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            &std::collections::HashMap::new(),
        );
        assert_eq!(news[0].importancia, NewsImportance::Alta);
    }

    #[test]
    fn test_ineligible_events_unaffected_by_visibility() {
        // PlayerProposalReceived continua Destaque e HierarchyUpdated continua Baixa
        // independentemente do driver_midia.
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("P001".to_string(), 90.0);

        let proposal_event = MarketEvent {
            event_type: MarketEventType::PlayerProposalReceived,
            headline: "Proposta recebida".to_string(),
            description: "Jogador recebe uma proposta.".to_string(),
            driver_id: Some("P001".to_string()),
            driver_name: Some("Jogador".to_string()),
            team_id: None,
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let hierarchy_event = MarketEvent {
            event_type: MarketEventType::HierarchyUpdated,
            headline: "Hierarquia definida".to_string(),
            description: "Equipe definiu hierarquia.".to_string(),
            driver_id: Some("P001".to_string()),
            driver_name: Some("Piloto Elite".to_string()),
            team_id: None,
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        };

        let mut next_id = id_gen();
        let mut timestamp = 1;
        let news = generate_news_from_market_events(
            &[proposal_event, hierarchy_event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            &std::collections::HashMap::new(),
        );
        assert_eq!(news[0].importancia, NewsImportance::Destaque); // PlayerProposal inalterado
        assert_eq!(news[1].importancia, NewsImportance::Baixa); // HierarchyUpdated inalterado
    }

    #[test]
    fn test_missing_driver_midia_uses_factual_only() {
        // driver_id ausente do mapa → sem bônus narrativo → importância factual pura.
        // ContractRenewed base=40 → Media (sem boost).
        let event = MarketEvent {
            event_type: MarketEventType::ContractRenewed,
            headline: "Renovacao".to_string(),
            description: "Piloto renova.".to_string(),
            driver_id: Some("P999".to_string()), // não está no mapa
            driver_name: Some("Desconhecido".to_string()),
            team_id: None,
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let mut next_id = id_gen();
        let mut timestamp = 1;
        let news = generate_news_from_market_events(
            &[event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(), // mapa vazio — degradação segura
            &std::collections::HashMap::new(),
        );
        assert_eq!(news[0].importancia, NewsImportance::Media);
    }

    // ── Testes de team_presence_editorial_bonus / generate_news_from_market_events ──

    #[test]
    fn test_market_team_presence_elite_promotes_renewal() {
        // ContractRenewed base=40; team Elite bônus=+10 → score 50 → Alta
        use crate::public_presence::team::TeamPublicPresenceTier;
        let event = MarketEvent {
            event_type: MarketEventType::ContractRenewed,
            headline: "Renovacao".to_string(),
            description: "Piloto renova.".to_string(),
            driver_id: None,
            driver_name: None,
            team_id: Some("T_ELITE".to_string()),
            team_name: Some("Equipe Elite".to_string()),
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let mut team_presence = std::collections::HashMap::new();
        team_presence.insert("T_ELITE".to_string(), TeamPublicPresenceTier::Elite);
        let mut next_id = id_gen();
        let mut timestamp = 1;
        let news = generate_news_from_market_events(
            &[event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &team_presence,
        );
        assert_eq!(news[0].importancia, NewsImportance::Alta);
    }

    #[test]
    fn test_market_team_presence_baixa_stays_media() {
        // ContractRenewed base=40; team Baixa bônus=+0 → score 40 → Media
        use crate::public_presence::team::TeamPublicPresenceTier;
        let event = MarketEvent {
            event_type: MarketEventType::ContractRenewed,
            headline: "Renovacao".to_string(),
            description: "Piloto renova.".to_string(),
            driver_id: None,
            driver_name: None,
            team_id: Some("T_BAIXA".to_string()),
            team_name: Some("Equipe Baixa".to_string()),
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let mut team_presence = std::collections::HashMap::new();
        team_presence.insert("T_BAIXA".to_string(), TeamPublicPresenceTier::Baixa);
        let mut next_id = id_gen();
        let mut timestamp = 1;
        let news = generate_news_from_market_events(
            &[event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &team_presence,
        );
        assert_eq!(news[0].importancia, NewsImportance::Media);
    }

    #[test]
    fn test_market_team_presence_no_effect_on_ineligible() {
        // PlayerProposalReceived permanece Destaque mesmo com team Elite
        use crate::public_presence::team::TeamPublicPresenceTier;
        let event = MarketEvent {
            event_type: MarketEventType::PlayerProposalReceived,
            headline: "Proposta recebida".to_string(),
            description: "Jogador recebe proposta.".to_string(),
            driver_id: Some("P001".to_string()),
            driver_name: Some("Jogador".to_string()),
            team_id: Some("T_ELITE".to_string()),
            team_name: Some("Equipe Elite".to_string()),
            from_team: None,
            to_team: None,
            categoria: None,
        };
        let mut team_presence = std::collections::HashMap::new();
        team_presence.insert("T_ELITE".to_string(), TeamPublicPresenceTier::Elite);
        let mut next_id = id_gen();
        let mut timestamp = 1;
        let news = generate_news_from_market_events(
            &[event],
            1,
            1,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            &team_presence,
        );
        assert_eq!(news[0].importancia, NewsImportance::Destaque);
    }

    #[test]
    fn test_market_team_presence_driver_dominates_over_team() {
        // Elite driver (bônus=18) + Baixa team (bônus=0) = 40+18=58 → Alta
        // Relevante driver (bônus=5) + Elite team (bônus=10) = 40+5+10=55 → Alta
        // Ambos são Alta, mas garantimos que Elite driver isolado (sem team) ≥ Relevante driver + Elite team
        // I.e.: piloto continua sendo o vetor dominante
        use crate::public_presence::team::TeamPublicPresenceTier;
        let make_renewal = |driver_id: &str, team_id: &str| MarketEvent {
            event_type: MarketEventType::ContractRenewed,
            headline: "Renovacao".to_string(),
            description: "Piloto renova.".to_string(),
            driver_id: Some(driver_id.to_string()),
            driver_name: None,
            team_id: Some(team_id.to_string()),
            team_name: None,
            from_team: None,
            to_team: None,
            categoria: None,
        };

        // Elite driver + Baixa team
        let mut driver_midia_elite = std::collections::HashMap::new();
        driver_midia_elite.insert("D_ELITE".to_string(), 90.0_f64); // Elite tier → +18
        let mut team_baixa = std::collections::HashMap::new();
        team_baixa.insert("T_BAIXA".to_string(), TeamPublicPresenceTier::Baixa); // +0

        // Relevante driver + Elite team
        let mut driver_midia_rel = std::collections::HashMap::new();
        driver_midia_rel.insert("D_REL".to_string(), 40.0_f64); // Relevante tier → +5
        let mut team_elite = std::collections::HashMap::new();
        team_elite.insert("T_ELITE".to_string(), TeamPublicPresenceTier::Elite); // +10

        let mut next_id = id_gen();
        let mut ts = 1i64;
        let news1 = generate_news_from_market_events(
            &[make_renewal("D_ELITE", "T_BAIXA")],
            1,
            1,
            &mut next_id,
            &mut ts,
            &driver_midia_elite,
            &team_baixa,
        );
        let news2 = generate_news_from_market_events(
            &[make_renewal("D_REL", "T_ELITE")],
            1,
            1,
            &mut next_id,
            &mut ts,
            &driver_midia_rel,
            &team_elite,
        );
        // Sanidade numérica: Elite driver = 40+18=58 > Relevante+Elite = 40+5+10=55
        // Ambos Alta, mas score interno do piloto Elite puro é maior
        // O teste verifica que nenhum dos dois fica abaixo de Alta (regressão)
        assert_eq!(news1[0].importancia, NewsImportance::Alta);
        assert_eq!(news2[0].importancia, NewsImportance::Alta);
    }

    // ── Testes de promote_narrative_importance ────────────────────────────────

    #[test]
    fn test_promote_narrative_alta_tier_promotes_media() {
        // Alta tier (midia 70) → Media vira Alta
        let result = super::promote_narrative_importance(NewsImportance::Media, Some(70.0));
        assert_eq!(result, NewsImportance::Alta);
    }

    #[test]
    fn test_promote_narrative_elite_tier_promotes_media() {
        // Elite tier (midia 90) → Media vira Alta
        let result = super::promote_narrative_importance(NewsImportance::Media, Some(90.0));
        assert_eq!(result, NewsImportance::Alta);
    }

    #[test]
    fn test_promote_narrative_relevante_no_change() {
        // Relevante (midia 40) → Media permanece Media
        let result = super::promote_narrative_importance(NewsImportance::Media, Some(40.0));
        assert_eq!(result, NewsImportance::Media);
    }

    #[test]
    fn test_promote_narrative_baixa_no_change() {
        // Baixa (midia 10) → Media permanece Media
        let result = super::promote_narrative_importance(NewsImportance::Media, Some(10.0));
        assert_eq!(result, NewsImportance::Media);
    }

    #[test]
    fn test_promote_narrative_already_alta_unchanged() {
        // Cap: Alta + Elite → Alta (não vira Destaque)
        let result = super::promote_narrative_importance(NewsImportance::Alta, Some(90.0));
        assert_eq!(result, NewsImportance::Alta);
    }

    #[test]
    fn test_promote_narrative_none_midia_unchanged() {
        // None → sem efeito, Media permanece Media
        let result = super::promote_narrative_importance(NewsImportance::Media, None);
        assert_eq!(result, NewsImportance::Media);
    }

    // ── Testes de integração: corrida ─────────────────────────────────────────

    fn make_race_with_mover(mover_id: &str, positions_gained: i32) -> RaceResult {
        RaceResult {
            qualifying_results: Vec::new(),
            race_results: vec![
                RaceDriverResult {
                    pilot_id: "P001".to_string(),
                    pilot_name: "Vencedor".to_string(),
                    team_id: "T001".to_string(),
                    team_name: "Equipe".to_string(),
                    grid_position: 1,
                    finish_position: 1,
                    positions_gained: 0,
                    best_lap_time_ms: 89_000.0,
                    total_race_time_ms: 3_590_000.0,
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
                    final_tire_wear: 0.8,
                    final_physical: 0.9,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
                RaceDriverResult {
                    pilot_id: mover_id.to_string(),
                    pilot_name: "Piloto Mover".to_string(),
                    team_id: "T002".to_string(),
                    team_name: "Equipe B".to_string(),
                    grid_position: 10,
                    finish_position: 10 - positions_gained,
                    positions_gained,
                    best_lap_time_ms: 90_000.0,
                    total_race_time_ms: 3_600_000.0,
                    gap_to_winner_ms: 5_000.0,
                    is_dnf: false,
                    dnf_reason: None,
                    dnf_segment: None,
                    incidents_count: 0,
                    incidents: Vec::new(),
                    has_fastest_lap: false,
                    points_earned: 6,
                    is_jogador: false,
                    laps_completed: 20,
                    final_tire_wear: 0.75,
                    final_physical: 0.85,
                    classification_status: crate::simulation::race::ClassificationStatus::Finished,
                    notable_incident: None,
                    dnf_catalog_id: None,
                    damage_origin_segment: None,
                },
            ],
            pole_sitter_id: "P001".to_string(),
            winner_id: "P001".to_string(),
            fastest_lap_id: "P001".to_string(),
            total_laps: 20,
            weather: "Dry".to_string(),
            track_name: "Interlagos".to_string(),
            total_incidents: 0,
            total_dnfs: 0,
            main_incident_count: 0,
            notable_incident_pilot_ids: Vec::new(),
            most_positions_gained_id: Some(mover_id.to_string()),
        }
    }

    fn make_first_win_context() -> RaceNarrativeContext {
        RaceNarrativeContext {
            season_num: 1,
            round: 3,
            total_rounds: 12,
            category: "gt4".to_string(),
            track_name: "Interlagos".to_string(),
            thematic_slot: crate::models::enums::ThematicSlot::NaoClassificado,
            winner: WinnerNarrativeContext {
                pilot_id: "P001".to_string(),
                pilot_name: "Vencedor".to_string(),
                team_id: "T001".to_string(),
                team_name: "Equipe".to_string(),
                nationality: "br".to_string(),
                had_pole: false,
                had_fastest_lap: false,
                is_grand_slam: false,
                led_from_start: false,
                gap_to_second_ms: 2_500.0,
                is_dominant_win: false,
                is_photo_finish: false,
                positions_gained: 2,
                is_comeback_win: false,
                had_incidents: false,
                survived_collision: false,
                collision_with_names: Vec::new(),
                high_tire_wear: false,
                career_wins_before: 0,
                season_wins_before: 0,
                is_first_career_win: true,
                is_first_category_win: true,
                is_first_win_with_team: true,
                rounds_since_last_win: None,
                is_drought_end: false,
                previous_dnf_here: None,
                is_redemption: false,
                is_home_race: false,
                is_category_rookie: true,
                is_career_rookie: true,
                team_performance_tier: TeamPerformanceTier::Media,
                is_underdog_win: false,
                motivation: 70.0,
                beat_rival: false,
                rival_beaten_name: None,
                milestone_wins: None,
            },
            weather: WeatherNarrative::Dry,
            standings_before: vec![StandingEntry {
                pilot_id: "P001".to_string(),
                pilot_name: "Vencedor".to_string(),
                points: 18.0,
                position: 2,
            }],
        }
    }

    #[test]
    fn test_race_position_gainer_elite_bumps_to_alta() {
        // Position gainer com driver Elite (midia 90) → NewsImportance::Alta
        let race = make_race_with_mover("PMOVER", 5);
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("PMOVER".to_string(), 90.0);
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_race(
            &race,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            None,
        );

        let mover_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("PMOVER"))
            .expect("mover news");
        assert_eq!(mover_news.importancia, NewsImportance::Alta);
    }

    #[test]
    fn test_race_position_gainer_baixa_stays_media() {
        // Position gainer com driver Baixa (midia 10) → NewsImportance::Media
        let race = make_race_with_mover("PMOVER2", 5);
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("PMOVER2".to_string(), 10.0);
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_race(
            &race,
            1,
            1,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            None,
        );

        let mover_news = news
            .iter()
            .find(|n| n.driver_id.as_deref() == Some("PMOVER2"))
            .expect("mover news");
        assert_eq!(mover_news.importancia, NewsImportance::Media);
    }

    #[test]
    fn test_race_winner_combines_first_win_title_and_body() {
        let race = make_race_result_for_framing(0, 0, 0);
        let ctx = make_first_win_context();
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_race(
            &race,
            1,
            3,
            "gt4",
            crate::models::enums::ThematicSlot::NaoClassificado,
            &mut next_id,
            &mut timestamp,
            &std::collections::HashMap::new(),
            Some(&ctx),
        );

        let winner_news = news
            .iter()
            .find(|item| item.driver_id.as_deref() == Some("P001"))
            .expect("winner news");

        assert!(
            winner_news.titulo.to_lowercase().contains("primeira"),
            "winner title should use combined first-win narrative, got: {}",
            winner_news.titulo
        );
        assert!(
            winner_news
                .texto
                .to_lowercase()
                .contains("primeira vitoria na carreira"),
            "winner body should mention career first win, got: {}",
            winner_news.texto
        );
        assert!(
            winner_news
                .texto
                .to_lowercase()
                .contains("primeira vitoria na gt4"),
            "winner body should mention category first win, got: {}",
            winner_news.texto
        );
        assert!(
            winner_news
                .texto
                .to_lowercase()
                .contains("primeira vitoria pela equipe"),
            "winner body should mention team first win, got: {}",
            winner_news.texto
        );
    }

    // ── Testes de integração: fim de temporada ────────────────────────────────

    #[test]
    fn test_eos_decliner_elite_bumps_to_alta() {
        // Decliner com mídia Elite → NewsImportance::Alta (acima do padrão Media)
        use crate::evolution::growth::{AttributeChange, GrowthReport};
        let mut result = sample_end_of_season();
        result.growth_reports = vec![GrowthReport {
            driver_id: "PDEC".to_string(),
            driver_name: "Em Declinio".to_string(),
            changes: vec![AttributeChange {
                attribute: "skill".to_string(),
                old_value: 70,
                new_value: 65,
                delta: -5,
                reason: "Temporada fraca".to_string(),
            }],
            overall_delta: -5.0,
        }];
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("PDEC".to_string(), 90.0); // Elite
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_end_of_season(
            &result,
            1,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            None,
        );

        let dec_news = news
            .iter()
            .find(|n| n.tipo == NewsType::Evolucao && n.driver_id.as_deref() == Some("PDEC"))
            .expect("decliner news");
        assert_eq!(dec_news.importancia, NewsImportance::Alta);
    }

    #[test]
    fn test_eos_rookie_non_genio_elite_bumps_to_alta() {
        // Rookie não-Gênio com mídia Elite → NewsImportance::Alta
        use crate::evolution::pipeline::RookieInfo;
        let mut result = sample_end_of_season();
        result.rookies_generated = vec![RookieInfo {
            driver_id: "PROOKIE".to_string(),
            driver_name: "Novato Famoso".to_string(),
            nationality: "br".to_string(),
            age: 19,
            skill: 58,
            tipo: "Talento".to_string(), // não é Genio
        }];
        let mut driver_midia = std::collections::HashMap::new();
        driver_midia.insert("PROOKIE".to_string(), 90.0); // Elite
        let mut next_id = id_gen();
        let mut timestamp = 0i64;

        let news = generate_news_from_end_of_season(
            &result,
            1,
            &mut next_id,
            &mut timestamp,
            &driver_midia,
            None,
        );

        let rookie_news = news
            .iter()
            .find(|n| n.tipo == NewsType::Rookies && n.driver_id.as_deref() == Some("PROOKIE"))
            .expect("rookie news");
        assert_eq!(rookie_news.importancia, NewsImportance::Alta);
    }
}
use crate::evolution::pipeline::EndOfSeasonResult;
use crate::market::preseason::{MarketEvent, MarketEventType};
use crate::market::visibility::{
    derive_market_visibility_profile, MarketVisibilityProfile, MarketVisibilityTier,
};
use crate::models::enums::ThematicSlot;
use crate::models::injury::Injury;
use crate::news::flavour::first_win::{
    build_first_win_narrative, FirstWinContext, FirstWinNarrative,
};
use crate::news::flavour::{pick_and_format, pick_title_and_body, templates};
use crate::news::race_context::{RaceNarrativeContext, WeatherNarrative};
use crate::news::season_framing::SeasonalFramingSignal;
use crate::news::{NewsImportance, NewsItem, NewsType};
use crate::promotion::{MovementType, PilotEffectType};
use crate::public_presence::team::TeamPublicPresenceTier;
use crate::simulation::incidents::{IncidentResult, IncidentSeverity, IncidentType};
use crate::simulation::race::RaceResult;

/// Bônus de força editorial de notícia por tier de visibilidade pública.
///
/// Escala calibrada para score editorial de notícia (0–100). Não representa score
/// de mercado factual ou econômico — não deve ser reutilizado fora do news system.
/// Alinhado em proporção com helpers de mercado anteriores (Baixa=0, Elite=max).
fn market_news_visibility_bonus(profile: &MarketVisibilityProfile) -> f64 {
    match profile.tier {
        MarketVisibilityTier::Baixa => 0.0,
        MarketVisibilityTier::Relevante => 5.0,
        MarketVisibilityTier::Alta => 10.0,
        MarketVisibilityTier::Elite => 18.0,
    }
}

/// Score base de relevância editorial por tipo de evento de mercado.
///
/// Retorna None para eventos inelegíveis — cuja importância é fixada por razões
/// estruturais (PlayerProposalReceived → Destaque; HierarchyUpdated, PreSeasonComplete
/// → fixados editorialmente). Nesses casos, visibilidade pública não intervém.
fn market_base_narrative_score(event_type: &MarketEventType) -> Option<f64> {
    match event_type {
        MarketEventType::TransferCompleted => Some(65.0),
        MarketEventType::ContractRenewed => Some(40.0),
        MarketEventType::RookieSigned => Some(40.0),
        MarketEventType::ContractExpired => Some(35.0),
        MarketEventType::TransferRejected => Some(20.0),
        _ => None,
    }
}

/// Score editorial combinado para notícia de mercado: relevância factual + visibilidade pública.
///
/// `midia = None` (sem driver_id ou driver ausente do mapa) significa ausência de
/// modulação pública — não é erro. O evento continua avaliado pela relevância factual.
fn market_news_editorial_score(base: f64, midia: Option<f64>) -> f64 {
    let bonus = midia
        .map(|m| market_news_visibility_bonus(&derive_market_visibility_profile(m)))
        .unwrap_or(0.0);
    base + bonus
}

/// Mapeia score editorial para NewsImportance.
/// Cap em Alta — Destaque é reservado para eventos player-facing.
fn market_score_to_importance(score: f64) -> NewsImportance {
    if score >= 50.0 {
        NewsImportance::Alta
    } else if score >= 30.0 {
        NewsImportance::Media
    } else {
        NewsImportance::Baixa
    }
}

/// Promoção discreta de importância por visibilidade pública narrativa — v1.
///
/// Esta função é o mecanismo de "centralidade longitudinal" desta versão:
/// ao aplicar-se em geradores recorrentes (corrida, fim de temporada), faz com que
/// pilotos mais públicos apareçam com mais frequência em notícias Alta ao longo da
/// temporada. Não é um sistema global de ranking narrativo — é promoção local em pontos
/// de geração já existentes.
///
/// Só promove Media → Alta para pilotos Alta ou Elite — evitando banalização.
/// Cap via match explícito: Alta e Destaque são preservados sem depender de ord numérica.
/// `midia = None` → sem efeito (degradação segura, não é erro estrutural).
pub(crate) fn promote_narrative_importance(
    importance: NewsImportance,
    midia: Option<f64>,
) -> NewsImportance {
    match importance {
        NewsImportance::Alta | NewsImportance::Destaque => return importance,
        _ => {}
    }
    match midia.map(|m| derive_market_visibility_profile(m).tier) {
        Some(MarketVisibilityTier::Alta | MarketVisibilityTier::Elite) => NewsImportance::Alta,
        _ => importance,
    }
}

/// Bônus editorial de notícia por tier de presença pública de equipe.
///
/// Secundário ao bônus de visibilidade de piloto (Baixa=0/Relevante=5/Alta=10/Elite=18).
/// Escala deliberadamente menor: Elite equipe (10) < Elite piloto (18).
/// Garante dominância do indivíduo: Elite driver + Baixa team (18) > Relevante driver + Elite team (15).
/// Efeito máximo: +10 (Elite), suficiente para promover ContractRenewed 40→50 (Alta).
fn team_presence_editorial_bonus(tier: &TeamPublicPresenceTier) -> f64 {
    match tier {
        TeamPublicPresenceTier::Baixa => 0.0,
        TeamPublicPresenceTier::Relevante => 2.0,
        TeamPublicPresenceTier::Alta => 6.0,
        TeamPublicPresenceTier::Elite => 10.0,
    }
}

pub fn generate_news_from_market_events(
    events: &[MarketEvent],
    temporada: i32,
    semana: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
    driver_midia: &std::collections::HashMap<String, f64>,
    team_presence: &std::collections::HashMap<String, TeamPublicPresenceTier>,
) -> Vec<NewsItem> {
    events
        .iter()
        .map(|event| {
            let driver_name = event.driver_name.as_deref().unwrap_or("Piloto");
            let team_name = event.team_name.as_deref().unwrap_or("Equipe");
            let seed = format!(
                "mkt:{}:{}:{}:{}",
                event.driver_id.as_deref().unwrap_or(""),
                event.team_id.as_deref().unwrap_or(""),
                semana,
                temporada,
            );
            let rep = [("{name}", driver_name), ("{team}", team_name)];

            let (tipo, mut importancia, icone, titulo, texto) = match event.event_type {
                MarketEventType::ContractExpired => {
                    let (t, b) = pick_title_and_body(
                        templates::market::EXPIRED_TITULO,
                        templates::market::EXPIRED_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Mercado,
                        NewsImportance::Media,
                        "\u{1F4CB}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::ContractRenewed => {
                    let (t, b) = pick_title_and_body(
                        templates::market::RENEWED_TITULO,
                        templates::market::RENEWED_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Mercado,
                        NewsImportance::Media,
                        "\u{270D}\u{FE0F}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::TransferCompleted => {
                    let (t, b) = pick_title_and_body(
                        templates::market::TRANSFER_TITULO,
                        templates::market::TRANSFER_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Mercado,
                        NewsImportance::Alta,
                        "\u{1F4CB}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::TransferRejected => {
                    let (t, b) = pick_title_and_body(
                        templates::market::REJECTED_TITULO,
                        templates::market::REJECTED_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Mercado,
                        NewsImportance::Baixa,
                        "\u{1F5DE}\u{FE0F}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::RookieSigned => {
                    let (t, b) = pick_title_and_body(
                        templates::market::ROOKIE_SIGNED_TITULO,
                        templates::market::ROOKIE_SIGNED_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Rookies,
                        NewsImportance::Media,
                        "\u{1F393}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::PlayerProposalReceived => {
                    let (t, b) = pick_title_and_body(
                        templates::market::PROPOSAL_TITULO,
                        templates::market::PROPOSAL_TEXTO,
                        &seed,
                        &rep,
                    );
                    (
                        NewsType::Mercado,
                        NewsImportance::Destaque,
                        "\u{1F4BC}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::HierarchyUpdated => {
                    let (t, b) = pick_title_and_body(
                        templates::market::HIERARCHY_TITULO,
                        templates::market::HIERARCHY_TEXTO,
                        &seed,
                        &[("{team}", team_name)],
                    );
                    (
                        NewsType::Hierarquia,
                        NewsImportance::Baixa,
                        "\u{26A1}".to_string(),
                        t,
                        b,
                    )
                }
                MarketEventType::PreSeasonComplete => {
                    let (t, b) = pick_title_and_body(
                        templates::market::PRESEASON_TITULO,
                        templates::market::PRESEASON_TEXTO,
                        &seed,
                        &[],
                    );
                    (
                        NewsType::PreTemporada,
                        NewsImportance::Alta,
                        "\u{1F4F0}".to_string(),
                        t,
                        b,
                    )
                }
            };

            if let Some(base) = market_base_narrative_score(&event.event_type) {
                let midia = event
                    .driver_id
                    .as_deref()
                    .and_then(|id| driver_midia.get(id).copied());
                let team_bonus = event
                    .team_id
                    .as_deref()
                    .and_then(|tid| team_presence.get(tid))
                    .map(team_presence_editorial_bonus)
                    .unwrap_or(0.0);
                importancia = market_score_to_importance(
                    market_news_editorial_score(base, midia) + team_bonus,
                );
            }

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
    driver_midia: &std::collections::HashMap<String, f64>,
    player_id: Option<&str>,
) -> Vec<NewsItem> {
    let mut news = Vec::new();

    for retirement in &result.retirements {
        let seed = format!("eos:ret:{}:{}", retirement.driver_id, temporada);
        let age_str = retirement.age.to_string();
        let rep = [
            ("{name}", retirement.driver_name.as_str()),
            ("{age}", age_str.as_str()),
            ("{reason}", retirement.reason.as_str()),
        ];
        let (titulo, texto) = pick_title_and_body(
            templates::end_of_season::APOSENTA_TITULO,
            templates::end_of_season::APOSENTA_TEXTO,
            &seed,
            &rep,
        );
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Aposentadoria,
            NewsImportance::Alta,
            "\u{1F474}".to_string(),
            titulo,
            texto,
            Some(0),
            None,
            temporada,
            retirement.categoria.clone(),
            retirement.categoria.as_deref().map(format_category_name),
            Some(retirement.driver_id.clone()),
            None,
        ));
    }

    for movement in &result.promotion_result.movements {
        let target_name = format_category_name(&movement.to_category);
        let from_name = format_category_name(&movement.from_category);
        let seed = format!("eos:mov:{}:{}", movement.team_id, temporada);
        let rep = [
            ("{team}", movement.team_name.as_str()),
            ("{cat}", target_name.as_str()),
            ("{from}", from_name.as_str()),
            ("{reason}", movement.reason.as_str()),
        ];

        let (tipo, importancia, icone, titles, texts) = match movement.movement_type {
            MovementType::Promocao => (
                NewsType::Promocao,
                NewsImportance::Destaque,
                "\u{2B06}\u{FE0F}",
                templates::end_of_season::PROMOCAO_TITULO,
                templates::end_of_season::PROMOCAO_TEXTO,
            ),
            MovementType::Rebaixamento => (
                NewsType::Rebaixamento,
                NewsImportance::Alta,
                "\u{2B07}\u{FE0F}",
                templates::end_of_season::REBAIXA_TITULO,
                templates::end_of_season::REBAIXA_TEXTO,
            ),
        };

        let (titulo, texto) = pick_title_and_body(titles, texts, &seed, &rep);
        news.push(build_news_item(
            next_id,
            timestamp,
            tipo,
            importancia,
            icone.to_string(),
            titulo,
            texto,
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
        let seed = format!("eos:eff:{}:{}", effect.driver_id, temporada);
        let rep = [
            ("{name}", effect.driver_name.as_str()),
            ("{reason}", effect.reason.as_str()),
        ];
        match effect.effect {
            PilotEffectType::FreedNoLicense => {
                let (titulo, texto) = pick_title_and_body(
                    templates::end_of_season::FREED_AI_TITULO,
                    templates::end_of_season::FREED_AI_TEXTO,
                    &seed,
                    &rep,
                );
                news.push(build_news_item(
                    next_id,
                    timestamp,
                    NewsType::Mercado,
                    NewsImportance::Alta,
                    "\u{1F4CB}".to_string(),
                    titulo,
                    texto,
                    Some(0),
                    None,
                    temporada,
                    None,
                    None,
                    Some(effect.driver_id.clone()),
                    Some(effect.team_id.clone()),
                ));
            }
            PilotEffectType::FreedPlayerStays => {
                let (titulo, texto) = pick_title_and_body(
                    templates::end_of_season::FREED_PLAYER_TITULO,
                    templates::end_of_season::FREED_PLAYER_TEXTO,
                    &seed,
                    &rep,
                );
                news.push(build_news_item(
                    next_id,
                    timestamp,
                    NewsType::Mercado,
                    NewsImportance::Destaque,
                    "\u{1F4BC}".to_string(),
                    titulo,
                    texto,
                    Some(0),
                    None,
                    temporada,
                    None,
                    None,
                    Some(effect.driver_id.clone()),
                    Some(effect.team_id.clone()),
                ));
            }
            PilotEffectType::MovesWithTeam => {}
        }
    }

    for rookie in &result.rookies_generated {
        let seed = format!("eos:rook:{}:{}", rookie.driver_id, temporada);
        let age_str = rookie.age.to_string();
        let (titles, texts) = match rookie.tipo.as_str() {
            "Genio" => (
                templates::end_of_season::ROOKIE_GENIO_TITULO,
                templates::end_of_season::ROOKIE_GENIO_TEXTO,
            ),
            "Talento" => (
                templates::end_of_season::ROOKIE_TALENTO_TITULO,
                templates::end_of_season::ROOKIE_TALENTO_TEXTO,
            ),
            _ => (
                templates::end_of_season::ROOKIE_NORMAL_TITULO,
                templates::end_of_season::ROOKIE_NORMAL_TEXTO,
            ),
        };
        let rep = [
            ("{name}", rookie.driver_name.as_str()),
            ("{age}", age_str.as_str()),
        ];
        let importance = if rookie.tipo == "Genio" {
            NewsImportance::Alta
        } else {
            let midia = driver_midia.get(&rookie.driver_id).copied();
            promote_narrative_importance(NewsImportance::Media, midia)
        };
        let (titulo, texto) = pick_title_and_body(titles, texts, &seed, &rep);
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Rookies,
            importance,
            "\u{1F393}".to_string(),
            titulo,
            texto,
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
        for license in &result.licenses_earned {
            if Some(license.driver_id.as_str()) == player_id {
                let seed = format!("eos:lic:{}:{}", license.driver_id, temporada);
                let level_str = license.license_level.to_string();
                let cat_name = format_category_name(&license.category);
                let rep = [
                    ("{name}", license.driver_name.as_str()),
                    ("{n}", level_str.as_str()),
                    ("{cat}", cat_name.as_str()),
                ];
                let (titulo, texto) = pick_title_and_body(
                    templates::end_of_season::LICENSE_PLAYER_TITULO,
                    templates::end_of_season::LICENSE_PLAYER_TEXTO,
                    &seed,
                    &rep,
                );
                news.push(build_news_item(
                    next_id,
                    timestamp,
                    NewsType::Milestone,
                    NewsImportance::Destaque,
                    "\u{1F3C5}".to_string(),
                    titulo,
                    texto,
                    Some(0),
                    None,
                    temporada,
                    Some(license.category.clone()),
                    Some(cat_name),
                    Some(license.driver_id.clone()),
                    None,
                ));
            }
        }

        let ai_licenses: Vec<_> = result
            .licenses_earned
            .iter()
            .filter(|license| Some(license.driver_id.as_str()) != player_id)
            .collect();

        if !ai_licenses.is_empty() {
            let count = ai_licenses.len();
            let names: Vec<_> = ai_licenses
                .iter()
                .take(5)
                .map(|license| license.driver_name.clone())
                .collect();
            let names_str = if count > 5 {
                format!("{} e mais {} pilotos", names.join(", "), count - 5)
            } else {
                names.join(", ")
            };
            let seed = format!("eos:licgrp:{}", temporada);
            let count_str = count.to_string();
            let rep = [("{n}", count_str.as_str()), ("{names}", names_str.as_str())];
            let (titulo, texto) = pick_title_and_body(
                templates::end_of_season::LICENSE_GROUP_TITULO,
                templates::end_of_season::LICENSE_GROUP_TEXTO,
                &seed,
                &rep,
            );
            news.push(build_news_item(
                next_id,
                timestamp,
                NewsType::Milestone,
                NewsImportance::Media,
                "\u{1F3C5}".to_string(),
                titulo,
                texto,
                Some(0),
                None,
                temporada,
                None,
                None,
                None,
                None,
            ));
        }
    }

    if let Some(grower) = result
        .growth_reports
        .iter()
        .max_by(|a, b| a.overall_delta.total_cmp(&b.overall_delta))
        .filter(|report| report.overall_delta > 3.0)
    {
        let seed = format!("eos:grow:{}:{}", grower.driver_id, temporada);
        let rep = [("{name}", grower.driver_name.as_str())];
        let grower_midia = driver_midia.get(&grower.driver_id).copied();
        let importance = promote_narrative_importance(NewsImportance::Alta, grower_midia);
        let (titulo, texto) = pick_title_and_body(
            templates::end_of_season::GROWER_TITULO,
            templates::end_of_season::GROWER_TEXTO,
            &seed,
            &rep,
        );
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Evolucao,
            importance,
            "\u{1F4C8}".to_string(),
            titulo,
            texto,
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
        let seed = format!("eos:dec:{}:{}", decliner.driver_id, temporada);
        let rep = [("{name}", decliner.driver_name.as_str())];
        let dec_midia = driver_midia.get(&decliner.driver_id).copied();
        let importance = promote_narrative_importance(NewsImportance::Media, dec_midia);
        let (titulo, texto) = pick_title_and_body(
            templates::end_of_season::DECLINER_TITULO,
            templates::end_of_season::DECLINER_TEXTO,
            &seed,
            &rep,
        );
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Evolucao,
            importance,
            "\u{1F4C9}".to_string(),
            titulo,
            texto,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaceFraming {
    Limpa,
    Turbulenta,
    Caotica,
}

fn race_framing(result: &RaceResult) -> RaceFraming {
    if result.main_incident_count >= 2 || result.total_dnfs >= 4 || result.total_incidents >= 10 {
        RaceFraming::Caotica
    } else if result.main_incident_count >= 1
        || result.total_dnfs >= 2
        || result.total_incidents >= 4
    {
        RaceFraming::Turbulenta
    } else {
        RaceFraming::Limpa
    }
}

pub fn generate_news_from_race(
    race_result: &RaceResult,
    temporada: i32,
    rodada: i32,
    categoria: &str,
    thematic_slot: ThematicSlot,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
    driver_midia: &std::collections::HashMap<String, f64>,
    ctx: Option<&RaceNarrativeContext>,
) -> Vec<NewsItem> {
    let mut news = Vec::new();
    let category_name = format_category_name(categoria);
    let category_name_option = Some(category_name.clone());

    if let Some(winner) = race_result
        .race_results
        .iter()
        .find(|result| result.finish_position == 1)
    {
        let (titulo, texto, importancia_base) = if let Some(ctx) = ctx {
            winner_news_with_context(ctx, race_result, winner.grid_position, &category_name)
        } else {
            winner_fallback_with_templates(winner, race_result, thematic_slot, rodada, temporada)
        };

        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            importancia_base,
            "\u{1F3C6}".to_string(),
            titulo,
            texto,
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name_option.clone(),
            Some(winner.pilot_id.clone()),
            Some(winner.team_id.clone()),
        ));
    }

    if let Some(mover) = race_result
        .most_positions_gained_id
        .as_deref()
        .and_then(|id| race_result.race_results.iter().find(|r| r.pilot_id == id))
        .filter(|r| !r.is_dnf && r.positions_gained >= 3)
        .filter(|r| r.finish_position != 1)
    {
        let mover_midia = driver_midia.get(&mover.pilot_id).copied();
        let mover_importance = promote_narrative_importance(NewsImportance::Media, mover_midia);
        let seed = format!("gainer:{}:{}:{}", mover.pilot_id, rodada, temporada);
        let n_str = mover.positions_gained.to_string();
        let rep = [
            ("{name}", mover.pilot_name.as_str()),
            ("{n}", n_str.as_str()),
        ];
        let (titulo, texto) = pick_title_and_body(
            templates::race::GAINER_TITULO,
            templates::race::GAINER_TEXTO,
            &seed,
            &rep,
        );
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            mover_importance,
            "\u{1F4C8}".to_string(),
            titulo,
            texto,
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name_option.clone(),
            Some(mover.pilot_id.clone()),
            Some(mover.team_id.clone()),
        ));
    }

    if let Some(player_result) = race_result
        .race_results
        .iter()
        .find(|result| result.is_jogador)
    {
        let seed = format!("player:{}:{}:{}", player_result.pilot_id, rodada, temporada);
        let pos_str = player_result.finish_position.to_string();
        let grid_str = player_result.grid_position.to_string();
        let pts_str = player_result.points_earned.to_string();
        let rep = [
            ("{name}", player_result.pilot_name.as_str()),
            ("{track}", race_result.track_name.as_str()),
            ("{pos}", pos_str.as_str()),
            ("{grid}", grid_str.as_str()),
            ("{pts}", pts_str.as_str()),
        ];
        let (titulo, texto) = pick_title_and_body(
            templates::race::JOGADOR_TITULO,
            templates::race::JOGADOR_TEXTO,
            &seed,
            &rep,
        );
        news.push(build_news_item(
            next_id,
            timestamp,
            NewsType::Corrida,
            player_result_news_importance(player_result),
            "\u{1F3C1}".to_string(),
            titulo,
            texto,
            Some(rodada),
            None,
            temporada,
            Some(categoria.to_string()),
            category_name_option,
            Some(player_result.pilot_id.clone()),
            Some(player_result.team_id.clone()),
        ));
    }

    if let Some(summary_item) = build_chaotic_incident_summary_item(
        race_result,
        temporada,
        rodada,
        categoria,
        next_id,
        timestamp,
    ) {
        news.push(summary_item);
    }

    news
}

fn build_chaotic_incident_summary_item(
    race_result: &RaceResult,
    temporada: i32,
    rodada: i32,
    categoria: &str,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Option<NewsItem> {
    if !should_generate_chaotic_incident_summary(race_result) {
        return None;
    }

    let mut incidents: Vec<(&crate::simulation::race::RaceDriverResult, &IncidentResult)> =
        race_result
            .race_results
            .iter()
            .flat_map(|result| result.incidents.iter().map(move |incident| (result, incident)))
            .collect();

    if incidents.is_empty() {
        return None;
    }

    incidents.sort_by(|(_, a), (_, b)| {
        chaotic_summary_priority(b)
            .cmp(&chaotic_summary_priority(a))
            .then_with(|| b.pilot_id.cmp(&a.pilot_id))
    });

    let highlights: Vec<String> = incidents
        .into_iter()
        .take(4)
        .map(|(_, incident)| incident_summary_detail(incident))
        .collect();

    if highlights.is_empty() {
        return None;
    }

    let titulo = if race_result.total_dnfs >= 2 || race_result.total_incidents >= 6 {
        format!("Corrida caotica em {}", race_result.track_name)
    } else {
        format!("Incidentes marcam {}", race_result.track_name)
    };
    let texto = format!(
        "A prova teve varios incidentes relevantes: {}.",
        highlights.join("; ")
    );

    Some(build_news_item(
        next_id,
        timestamp,
        NewsType::Incidente,
        NewsImportance::Alta,
        "\u{1F6A8}".to_string(),
        titulo,
        texto,
        Some(rodada),
        None,
        temporada,
        Some(categoria.to_string()),
        Some(format_category_name(categoria)),
        None,
        None,
    ))
}

fn should_generate_chaotic_incident_summary(race_result: &RaceResult) -> bool {
    race_result.total_incidents >= 5
        || race_result.total_dnfs >= 2
        || (race_result.total_dnfs >= 1 && race_result.total_incidents >= 3)
}

fn chaotic_summary_priority(incident: &IncidentResult) -> u8 {
    match (&incident.incident_type, incident.is_dnf) {
        (_, true) => 3,
        (IncidentType::Mechanical, false) => 2,
        (IncidentType::Collision, false) => 1,
        _ => 0,
    }
}

fn incident_summary_detail(incident: &IncidentResult) -> String {
    if let Some(ref origin) = incident.damage_origin_segment {
        if *origin != incident.segment {
            return format!(
                "Consequencia do contato no {}: {}",
                origin, incident.description
            );
        }
    }

    incident.description.clone()
}

fn winner_news_with_context(
    ctx: &RaceNarrativeContext,
    race_result: &RaceResult,
    grid_position: i32,
    category_name: &str,
) -> (String, String, NewsImportance) {
    if let Some(first_win_narrative) =
        build_first_win_narrative_for_winner(ctx, grid_position, category_name)
    {
        let importance = if ctx.winner.is_first_career_win {
            NewsImportance::Destaque
        } else {
            NewsImportance::Alta
        };
        return (
            first_win_narrative.title,
            first_win_narrative.body,
            importance,
        );
    }

    let seed = format!(
        "winner:{}:{}:{}",
        ctx.winner.pilot_id, ctx.round, ctx.season_num
    );
    let grid_str = grid_position.to_string();
    let name = ctx.winner.pilot_name.as_str();
    let track = ctx.track_name.as_str();

    if let Some(milestone) = ctx.winner.milestone_wins {
        let n_str = milestone.to_string();
        return winner_pick_from_templates(
            templates::winner::MILESTONE_TITULO,
            templates::winner::MILESTONE_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
                ("{n}", n_str.as_str()),
            ],
            NewsImportance::Destaque,
        );
    }

    if ctx.winner.is_redemption {
        return winner_pick_from_templates(
            templates::winner::REDENCAO_TITULO,
            templates::winner::REDENCAO_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_drought_end {
        let n_str = ctx.winner.rounds_since_last_win.unwrap_or(10).to_string();
        return winner_pick_from_templates(
            templates::winner::JEJUM_TITULO,
            templates::winner::JEJUM_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
                ("{n}", n_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_home_race {
        return winner_pick_from_templates(
            templates::winner::CASA_TITULO,
            templates::winner::CASA_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_underdog_win {
        return winner_pick_from_templates(
            templates::winner::UNDERDOG_TITULO,
            templates::winner::UNDERDOG_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.survived_collision {
        return winner_pick_from_templates(
            templates::winner::COLISAO_SOBREVIVEU_TITULO,
            templates::winner::COLISAO_SOBREVIVEU_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_grand_slam {
        return winner_pick_from_templates(
            templates::winner::GRAND_SLAM_TITULO,
            templates::winner::GRAND_SLAM_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Destaque,
        );
    }

    if ctx.winner.is_photo_finish {
        return winner_pick_from_templates(
            templates::winner::PHOTO_FINISH_TITULO,
            templates::winner::PHOTO_FINISH_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_dominant_win {
        return winner_pick_from_templates(
            templates::winner::DOMINANTE_TITULO,
            templates::winner::DOMINANTE_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.is_comeback_win {
        return winner_pick_from_templates(
            templates::winner::COMEBACK_TITULO,
            templates::winner::COMEBACK_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.weather == WeatherNarrative::Wet {
        return winner_pick_from_templates(
            templates::winner::CHUVA_TITULO,
            templates::winner::CHUVA_TEXTO,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    if ctx.winner.beat_rival {
        if let Some(rival) = ctx.winner.rival_beaten_name.as_deref() {
            return winner_pick_from_templates(
                templates::winner::RIVAL_TITULO,
                templates::winner::RIVAL_TEXTO,
                &seed,
                &[
                    ("{name}", name),
                    ("{track}", track),
                    ("{grid}", grid_str.as_str()),
                    ("{rival}", rival),
                ],
                NewsImportance::Alta,
            );
        }
    }

    if let Some((titles, texts)) = winner_slot_templates(ctx.thematic_slot) {
        return winner_pick_from_templates(
            titles,
            texts,
            &seed,
            &[
                ("{name}", name),
                ("{track}", track),
                ("{grid}", grid_str.as_str()),
            ],
            NewsImportance::Alta,
        );
    }

    let (titles, texts) = winner_framing_templates(race_framing(race_result));
    winner_pick_from_templates(
        titles,
        texts,
        &seed,
        &[
            ("{name}", name),
            ("{track}", track),
            ("{grid}", grid_str.as_str()),
        ],
        NewsImportance::Alta,
    )
}

fn build_first_win_narrative_for_winner(
    ctx: &RaceNarrativeContext,
    grid_position: i32,
    category_name: &str,
) -> Option<FirstWinNarrative> {
    let first_win_ctx = FirstWinContext {
        is_career: ctx.winner.is_first_career_win,
        is_category: ctx.winner.is_first_category_win,
        is_with_team: ctx.winner.is_first_win_with_team,
    };
    let seed = format!(
        "{}:{}:{}:{}",
        ctx.season_num, ctx.round, ctx.category, ctx.winner.pilot_id
    );

    build_first_win_narrative(
        &first_win_ctx,
        &ctx.winner.pilot_name,
        &ctx.winner.team_name,
        category_name,
        &ctx.track_name,
        grid_position,
        &seed,
    )
}

fn winner_pick_from_templates(
    titles: &[&str],
    texts: &[&str],
    seed: &str,
    replacements: &[(&str, &str)],
    importance: NewsImportance,
) -> (String, String, NewsImportance) {
    let (titulo, texto) = pick_title_and_body(titles, texts, seed, replacements);
    (titulo, texto, importance)
}

fn winner_slot_templates(
    thematic_slot: ThematicSlot,
) -> Option<(&'static [&'static str], &'static [&'static str])> {
    match thematic_slot {
        ThematicSlot::AberturaDaTemporada => Some((
            templates::winner::ABERTURA_TITULO,
            templates::winner::ABERTURA_TEXTO,
        )),
        ThematicSlot::FinalDaTemporada => Some((
            templates::winner::FINAL_TITULO,
            templates::winner::FINAL_TEXTO,
        )),
        ThematicSlot::FinalEspecial => Some((
            templates::winner::FINAL_ESPECIAL_TITULO,
            templates::winner::FINAL_ESPECIAL_TEXTO,
        )),
        ThematicSlot::TensaoPreFinal => Some((
            templates::winner::TENSAO_TITULO,
            templates::winner::TENSAO_TEXTO,
        )),
        ThematicSlot::VisitanteRegional => Some((
            templates::winner::VISITANTE_TITULO,
            templates::winner::VISITANTE_TEXTO,
        )),
        ThematicSlot::MidpointPrestigio => Some((
            templates::winner::PRESTIGIO_TITULO,
            templates::winner::PRESTIGIO_TEXTO,
        )),
        _ => None,
    }
}

fn winner_framing_templates(
    framing: RaceFraming,
) -> (&'static [&'static str], &'static [&'static str]) {
    match framing {
        RaceFraming::Caotica => (
            templates::winner::CAOTICA_TITULO,
            templates::winner::CAOTICA_TEXTO,
        ),
        RaceFraming::Turbulenta => (
            templates::winner::TURBULENTA_TITULO,
            templates::winner::TURBULENTA_TEXTO,
        ),
        RaceFraming::Limpa => (
            templates::winner::LIMPA_TITULO,
            templates::winner::LIMPA_TEXTO,
        ),
    }
}

fn winner_fallback_with_templates(
    winner: &crate::simulation::race::RaceDriverResult,
    race_result: &RaceResult,
    thematic_slot: ThematicSlot,
    rodada: i32,
    temporada: i32,
) -> (String, String, NewsImportance) {
    let seed = format!("winner:{}:{}:{}", winner.pilot_id, rodada, temporada);
    let grid_str = winner.grid_position.to_string();
    let replacements = [
        ("{name}", winner.pilot_name.as_str()),
        ("{track}", race_result.track_name.as_str()),
        ("{grid}", grid_str.as_str()),
    ];

    if let Some((titles, texts)) = winner_slot_templates(thematic_slot) {
        return winner_pick_from_templates(
            titles,
            texts,
            &seed,
            &replacements,
            NewsImportance::Alta,
        );
    }

    let (titles, texts) = winner_framing_templates(race_framing(race_result));
    winner_pick_from_templates(titles, texts, &seed, &replacements, NewsImportance::Alta)
}

fn player_result_news_importance(
    result: &crate::simulation::race::RaceDriverResult,
) -> NewsImportance {
    if result.is_dnf {
        NewsImportance::Media
    } else if result.finish_position == 1 {
        NewsImportance::Alta
    } else if result.finish_position <= 3 {
        NewsImportance::Media
    } else {
        NewsImportance::Baixa
    }
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
        driver_id_secondary: None,
        team_id,
    };
    *timestamp += 1;
    item
}

pub(crate) fn format_category_name(category_id: &str) -> String {
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
    player_id: &str,
    team_name: &str,
    team_id: &str,
    categoria: &str,
    papel: &str,
    temporada: i32,
) -> NewsItem {
    let seed = format!("sign:{}:{}:{}", player_id, team_id, temporada);
    let cat_name = format_category_name(categoria);
    let role = if papel == "Numero1" {
        "piloto principal"
    } else {
        "segundo piloto"
    };
    let temp_str = temporada.to_string();
    let rep = [
        ("{name}", player_name),
        ("{team}", team_name),
        ("{cat}", cat_name.as_str()),
        ("{role}", role),
        ("{temp}", temp_str.as_str()),
    ];
    let (titulo, texto) = pick_title_and_body(
        templates::market::PLAYER_SIGN_TITULO,
        templates::market::PLAYER_SIGN_TEXTO,
        &seed,
        &rep,
    );
    NewsItem {
        id: String::new(),
        tipo: NewsType::Mercado,
        icone: "\u{270D}\u{FE0F}".to_string(),
        titulo,
        texto,
        rodada: Some(0),
        semana_pretemporada: None,
        temporada,
        categoria_id: Some(categoria.to_string()),
        categoria_nome: Some(cat_name),
        importancia: NewsImportance::Destaque,
        timestamp: 0,
        driver_id: Some(player_id.to_string()),
        driver_id_secondary: None,
        team_id: Some(team_id.to_string()),
    }
}

pub fn generate_player_rejection_news(
    player_name: &str,
    player_id: &str,
    team_name: &str,
    team_id: &str,
    temporada: i32,
) -> NewsItem {
    let seed = format!("rej:{}:{}:{}", player_id, team_id, temporada);
    let temp_str = temporada.to_string();
    let rep = [
        ("{name}", player_name),
        ("{team}", team_name),
        ("{temp}", temp_str.as_str()),
    ];
    let (_, texto) = pick_title_and_body(
        templates::market::PLAYER_REJECT_TITULO,
        templates::market::PLAYER_REJECT_TEXTO,
        &seed,
        &rep,
    );
    let titulo = format!("{} recusa proposta da {}", player_name, team_name);
    NewsItem {
        id: String::new(),
        tipo: NewsType::Mercado,
        icone: "\u{274C}".to_string(),
        titulo,
        texto,
        rodada: Some(0),
        semana_pretemporada: None,
        temporada,
        categoria_id: None,
        categoria_nome: None,
        importancia: NewsImportance::Media,
        timestamp: 0,
        driver_id: Some(player_id.to_string()),
        driver_id_secondary: None,
        team_id: Some(team_id.to_string()),
    }
}

pub fn generate_news_from_pos_especial(
    campeoes: &[(String, String, Option<String>, Option<String>)],
    temporada: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> Vec<NewsItem> {
    let mut items = Vec::new();

    for (categoria, classe, maybe_nome, maybe_driver_id) in campeoes {
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
                maybe_driver_id.clone(),
                None,
            ));
        }
    }

    items
}

// ── Incident / Injury news helpers ───────────────────────────────────────────

/// Priority levels for incident news selection (higher = more important).
/// Mirrors the ordering documented in the implementation plan.
fn incident_priority(inc: &IncidentResult) -> u8 {
    match (&inc.incident_type, &inc.severity, inc.is_dnf) {
        (IncidentType::Collision, IncidentSeverity::Critical, _) => 7,
        (IncidentType::Collision, _, true) => 6,
        (IncidentType::DriverError, _, true) => 5,
        (IncidentType::Mechanical, _, true) => 4,
        (IncidentType::Collision, IncidentSeverity::Major, false) => 3,
        (IncidentType::Mechanical, _, false) => 2,
        (IncidentType::DriverError, IncidentSeverity::Major, false) => 1,
        _ => 0,
    }
}

/// Selects the single most narratively relevant incident from a race.
///
/// Priority order (descending):
/// 1. Collision + Critical
/// 2. Collision + DNF
/// 3. DriverError + DNF
/// 4. Mechanical + DNF
/// 5. Collision + Major (no DNF)
/// 6. DriverError + Major (no DNF)
///
/// Tiebreakers: injury linked > player involved > deterministic stable order (pilot_id).
pub fn select_primary_incident<'a>(
    incidents: &'a [IncidentResult],
    new_injuries: &[Injury],
    player_id: Option<&str>,
    notable_pilot_ids: &[String],
) -> Option<&'a IncidentResult> {
    let injured_pilots: std::collections::HashSet<&str> = new_injuries
        .iter()
        .map(|inj| inj.pilot_id.as_str())
        .collect();

    incidents
        .iter()
        .filter(|inc| incident_priority(inc) > 0)
        .max_by(|a, b| {
            let pa = incident_priority(a);
            let pb = incident_priority(b);
            pa.cmp(&pb)
                .then_with(|| {
                    // Tiebreak 1: incident involving a freshly injured pilot
                    let a_injured = injured_pilots.contains(a.pilot_id.as_str())
                        || a.linked_pilot_id
                            .as_deref()
                            .map_or(false, |id| injured_pilots.contains(id));
                    let b_injured = injured_pilots.contains(b.pilot_id.as_str())
                        || b.linked_pilot_id
                            .as_deref()
                            .map_or(false, |id| injured_pilots.contains(id));
                    a_injured.cmp(&b_injured)
                })
                .then_with(|| {
                    // Tiebreak 2: notable pilot involvement (headline-level incidents from motor)
                    let notable_pilot = |inc: &&IncidentResult| {
                        notable_pilot_ids.contains(&inc.pilot_id)
                            || inc
                                .linked_pilot_id
                                .as_ref()
                                .map_or(false, |id| notable_pilot_ids.contains(id))
                    };
                    notable_pilot(a).cmp(&notable_pilot(b))
                })
                .then_with(|| {
                    // Tiebreak 3: player involvement
                    let a_player = player_id.map_or(false, |pid| {
                        a.pilot_id == pid || a.linked_pilot_id.as_deref() == Some(pid)
                    });
                    let b_player = player_id.map_or(false, |pid| {
                        b.pilot_id == pid || b.linked_pilot_id.as_deref() == Some(pid)
                    });
                    a_player.cmp(&b_player)
                })
                .then_with(|| {
                    // Tiebreak 4: deterministic stable order
                    b.pilot_id.cmp(&a.pilot_id)
                })
        })
}

/// Builds the single incident `NewsItem` for a race.
///
/// Text is generated from templates — no raw `description` string is exposed.
pub fn build_incident_news_item(
    incident: &IncidentResult,
    track_name: &str,
    pilot_name: &str,
    linked_pilot_name: Option<&str>,
    linked_pilot_is_dnf: bool,
    is_player: bool,
    category: &str,
    round: i32,
    season: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
    driver_midia: &std::collections::HashMap<String, f64>,
) -> NewsItem {
    let seed = format!("inc:{}:{}:{}", incident.pilot_id, round, season);
    let other_name = linked_pilot_name.unwrap_or("");
    let dnf_note = if incident.is_dnf {
        "Um dos pilotos abandonou."
    } else {
        ""
    };

    let (titulo, texto, mut importancia) =
        match (&incident.incident_type, &incident.severity, incident.is_dnf) {
            (IncidentType::Collision, IncidentSeverity::Critical, _) => {
                let t = pick_and_format(
                    templates::incidents::CRITICAL_TITULO,
                    &seed,
                    &[("{track}", track_name)],
                );
                let b = if linked_pilot_name.is_some() {
                    pick_and_format(
                        templates::incidents::CRITICAL_TEXTO_PAIR,
                        &seed,
                        &[
                            ("{a}", pilot_name),
                            ("{b}", other_name),
                            ("{track}", track_name),
                            ("{dnf_note}", dnf_note),
                        ],
                    )
                } else {
                    pick_and_format(
                        templates::incidents::CRITICAL_TEXTO_SOLO,
                        &seed,
                        &[("{name}", pilot_name), ("{track}", track_name)],
                    )
                };
                (t, b, NewsImportance::Alta)
            }
            (IncidentType::Collision, _, true) => {
                let t = if linked_pilot_name.is_some() && linked_pilot_is_dnf {
                    pick_and_format(
                        templates::incidents::COLISAO_DNF_TITULO_PAIR,
                        &seed,
                        &[
                            ("{a}", pilot_name),
                            ("{b}", other_name),
                            ("{track}", track_name),
                        ],
                    )
                } else {
                    pick_and_format(
                        templates::incidents::COLISAO_DNF_TITULO_SOLO,
                        &seed,
                        &[("{name}", pilot_name), ("{track}", track_name)],
                    )
                };
                let b = pick_and_format(
                    templates::incidents::COLISAO_DNF_TEXTO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                (t, b, NewsImportance::Alta)
            }
            (IncidentType::DriverError, _, true) => {
                let t = pick_and_format(
                    templates::incidents::ERRO_DNF_TITULO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                let b = pick_and_format(
                    templates::incidents::ERRO_DNF_TEXTO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                let imp = if is_player {
                    NewsImportance::Alta
                } else {
                    NewsImportance::Media
                };
                (t, b, imp)
            }
            (IncidentType::Mechanical, _, true) => {
                let t = pick_and_format(
                    templates::incidents::MECANICO_TITULO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                let b = incident.description.clone();
                (t, b, NewsImportance::Media)
            }
            (IncidentType::Mechanical, _, false) => {
                let t = format!("Problema mecanico atrapalha {}", pilot_name);
                let b = incident.description.clone();
                let imp = if is_player {
                    NewsImportance::Media
                } else {
                    NewsImportance::Baixa
                };
                (t, b, imp)
            }
            (IncidentType::Collision, IncidentSeverity::Major, false) => {
                let t = if linked_pilot_name.is_some() {
                    pick_and_format(
                        templates::incidents::COLISAO_MAJOR_TITULO_PAIR,
                        &seed,
                        &[
                            ("{a}", pilot_name),
                            ("{b}", other_name),
                            ("{track}", track_name),
                        ],
                    )
                } else {
                    pick_and_format(
                        templates::incidents::COLISAO_MAJOR_TITULO_SOLO,
                        &seed,
                        &[("{name}", pilot_name), ("{track}", track_name)],
                    )
                };
                let b = pick_and_format(
                    templates::incidents::COLISAO_MAJOR_TEXTO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                (t, b, NewsImportance::Media)
            }
            _ => {
                let t = pick_and_format(
                    templates::incidents::ERRO_MAJOR_TITULO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                let b = pick_and_format(
                    templates::incidents::ERRO_MAJOR_TEXTO,
                    &seed,
                    &[("{name}", pilot_name), ("{track}", track_name)],
                );
                let imp = if is_player {
                    NewsImportance::Media
                } else {
                    NewsImportance::Baixa
                };
                (t, b, imp)
            }
        };

    if !is_player {
        let midia = driver_midia.get(&incident.pilot_id).copied();
        importancia = promote_narrative_importance(importancia, midia);
    }

    let texto = if let Some(ref origin) = incident.damage_origin_segment {
        if *origin != incident.segment {
            format!("Consequencia do contato no {}: {}", origin, texto)
        } else {
            texto
        }
    } else {
        texto
    };

    build_news_item(
        next_id,
        timestamp,
        NewsType::Incidente,
        importancia,
        "\u{1F6A8}".to_string(),
        titulo,
        texto,
        Some(round),
        None,
        season,
        Some(category.to_string()),
        Some(format_category_name(category)),
        Some(incident.pilot_id.clone()),
        None,
    )
}

/// Builds one `NewsType::Lesao` `NewsItem` per injured pilot.
///
/// Importance: `Destaque` for the player, `Alta` for AI drivers.
pub fn build_injury_news_items(
    new_injuries: &[Injury],
    player_id: Option<&str>,
    pilot_names: &std::collections::HashMap<String, String>,
    track_name: &str,
    category: &str,
    round: i32,
    season: i32,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
    driver_midia: &std::collections::HashMap<String, f64>,
) -> Vec<NewsItem> {
    new_injuries
        .iter()
        .map(|injury| {
            let is_player = player_id.map_or(false, |pid| injury.pilot_id == pid);
            let importancia = if is_player {
                NewsImportance::Destaque
            } else {
                let midia = driver_midia.get(&injury.pilot_id).copied();
                promote_narrative_importance(NewsImportance::Alta, midia)
            };
            let pilot_name = pilot_names
                .get(&injury.pilot_id)
                .map(|s| s.as_str())
                .unwrap_or("Piloto");

            let seed = format!("inj:{}:{}:{}", injury.pilot_id, round, season);
            let n_str = injury.races_remaining.to_string();
            let rep = [
                ("{name}", pilot_name),
                ("{track}", track_name),
                ("{n}", n_str.as_str()),
            ];

            let (titles, texts) = match injury.injury_type {
                crate::models::enums::InjuryType::Leve => (
                    templates::injury::LEVE_TITULO,
                    templates::injury::LEVE_TEXTO,
                ),
                crate::models::enums::InjuryType::Moderada => (
                    templates::injury::MODERADA_TITULO,
                    templates::injury::MODERADA_TEXTO,
                ),
                crate::models::enums::InjuryType::Grave => (
                    templates::injury::GRAVE_TITULO,
                    templates::injury::GRAVE_TEXTO,
                ),
                crate::models::enums::InjuryType::Critica => (
                    templates::injury::CRITICA_TITULO,
                    templates::injury::CRITICA_TEXTO,
                ),
            };

            let (titulo, texto) = pick_title_and_body(titles, texts, &seed, &rep);

            build_news_item(
                next_id,
                timestamp,
                NewsType::Lesao,
                importancia,
                "\u{1F3E5}".to_string(),
                titulo,
                texto,
                Some(round),
                None,
                season,
                Some(category.to_string()),
                Some(format_category_name(category)),
                Some(injury.pilot_id.clone()),
                None,
            )
        })
        .collect()
}

pub fn build_seasonal_framing_news_item(
    signal: SeasonalFramingSignal,
    temporada: i32,
    rodada: i32,
    categoria_id: &str,
    next_id: &mut impl FnMut() -> String,
    timestamp: &mut i64,
) -> NewsItem {
    let categoria_nome = format_category_name(categoria_id);
    build_news_item(
        next_id,
        timestamp,
        NewsType::FramingSazonal,
        signal.importance,
        "📡".to_string(),
        signal.title,
        signal.body,
        Some(rodada),
        None,
        temporada,
        Some(categoria_id.to_string()),
        Some(categoria_nome),
        signal.driver_id,
        None,
    )
}
