use std::hash::{Hash, Hasher};

use crate::news::{NewsItem, NewsType};

pub(crate) const EDITORIAL_DECK_VARIANT_COUNT: usize = 16;
pub(crate) const EDITORIAL_BLOCK_VARIANT_COUNT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EditorialStoryType {
    Corrida,
    Incidente,
    Piloto,
    Equipe,
    Mercado,
    Estrutural,
}

#[cfg(test)]
mod position_variant_guard_tests {
    use super::*;

    #[test]
    fn test_corrida_impacto_position_variants_are_skipped_when_position_phrase_is_unavailable() {
        let item = NewsItem {
            id: "POSITION001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        let extras = EditorialExtras {
            driver_position: Some(16),
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };

        for variation in [0, 4] {
            let blocks = build_editorial_blocks(
                EditorialStoryType::Corrida,
                [0, variation, 0],
                &item,
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
                &extras,
            );

            assert!(
                !blocks[1].contains("Como ,"),
                "corrida impacto variation {variation} should skip position-only template when phrase is unavailable: {}",
                blocks[1],
            );
            assert!(
                !blocks[1].contains("â€”  â€”"),
                "corrida impacto variation {variation} should skip empty position wrapper when phrase is unavailable: {}",
                blocks[1],
            );
        }
    }

    #[test]
    fn test_piloto_pressao_position_variants_are_skipped_when_position_phrase_is_unavailable() {
        let item = NewsItem {
            id: "POSITION002".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        let extras = EditorialExtras {
            driver_position: Some(16),
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };

        for variation in [1, 4] {
            let blocks = build_editorial_blocks(
                EditorialStoryType::Piloto,
                [0, variation, 0],
                &item,
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
                &extras,
            );

            assert!(
                !blocks[1].contains("Como ,"),
                "piloto pressao variation {variation} should skip position-only template when phrase is unavailable: {}",
                blocks[1],
            );
            assert!(
                !blocks[1].contains("â€”  â€”"),
                "piloto pressao variation {variation} should skip empty position wrapper when phrase is unavailable: {}",
                blocks[1],
            );
        }
    }

    #[test]
    fn test_corrida_and_piloto_position_variants_use_clean_top_five_wording() {
        let item = NewsItem {
            id: "POSITION003".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        let extras = EditorialExtras {
            driver_position: Some(3),
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };

        let corrida_blocks = build_editorial_blocks(
            EditorialStoryType::Corrida,
            [0, 4, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert_eq!(
            corrida_blocks[1],
            "Carlos Mendes entra em Okayama sob a pressão típica de quem chega hoje como o terceiro colocado."
        );

        let piloto_blocks = build_editorial_blocks(
            EditorialStoryType::Piloto,
            [0, 4, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        assert_eq!(
            piloto_blocks[1],
            "Carlos Mendes entra em Okayama sob a pressão típica de quem chega hoje como o terceiro colocado."
        );
    }

    #[test]
    fn test_position_variants_do_not_emit_placeholder_copy_when_driver_position_is_none() {
        let item = NewsItem {
            id: "POSITION004".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        let extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };

        let corrida_blocks = build_editorial_blocks(
            EditorialStoryType::Corrida,
            [0, 0, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        let piloto_blocks = build_editorial_blocks(
            EditorialStoryType::Piloto,
            [0, 1, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );
        let joined = format!("{} {}", corrida_blocks[1], piloto_blocks[1]).to_lowercase();

        for forbidden in ["o nome em foco", "nome no top-10", "segunda metade do grid"] {
            assert!(
                !joined.contains(forbidden),
                "position-guarded variants should not leak placeholder `{forbidden}` when driver_position is None: {joined}",
            );
        }
    }

    #[test]
    fn test_mercado_preseason_variant_only_appears_when_preseason_week_exists() {
        let item = NewsItem {
            id: "MARKET001".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(3),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        let regular_extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };
        let regular_blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 1],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &regular_extras,
        );
        assert!(
            !regular_blocks[2].contains("pré-temporada"),
            "market pp1 should be skipped for regular-season stories: {}",
            regular_blocks[2],
        );

        let preseason_extras = EditorialExtras {
            preseason_week: Some(2),
            ..regular_extras
        };
        let preseason_blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 1],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &preseason_extras,
        );
        assert!(
            preseason_blocks[2].contains("pré-temporada"),
            "market pp1 should remain available for preseason stories: {}",
            preseason_blocks[2],
        );
    }

    #[test]
    fn test_mercado_pp1_uses_expected_preseason_phrase_for_each_phase() {
        let item = NewsItem {
            id: "MARKET001B".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(3),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };

        for (week, expected) in [
            (
                1,
                "A movimentação acontece na abertura da pré-temporada, o que muda o ritmo esperado de definição nesta fase.",
            ),
            (
                2,
                "A movimentação acontece no meio da pré-temporada, o que muda o ritmo esperado de definição nesta fase.",
            ),
            (
                5,
                "A movimentação acontece nas semanas finais de pré-temporada, o que muda o ritmo esperado de definição nesta fase.",
            ),
        ] {
            let extras = EditorialExtras {
                driver_position: None,
                team_position: None,
                driver_secondary_label: None,
                preseason_week: Some(week),
                presence_tier: None,
                subject_is_team: false,
                event_label: "Okayama",
            };
            let blocks = build_editorial_blocks(
                EditorialStoryType::Mercado,
                [0, 0, 1],
                &item,
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
                &extras,
            );

            assert_eq!(blocks[2], expected);
        }
    }

    #[test]
    fn test_mercado_presence_variant_only_appears_for_team_centered_stories() {
        let item = NewsItem {
            id: "MARKET002".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(3),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: None,
            team_id: Some("T001".to_string()),
        };

        let driver_centered_extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: Some("alta"),
            subject_is_team: false,
            event_label: "Okayama",
        };
        let driver_centered_blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 6],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &driver_centered_extras,
        );
        assert!(
            !driver_centered_blocks[2].contains("Para uma equipe"),
            "market pp6 should be skipped for driver-centered stories: {}",
            driver_centered_blocks[2],
        );

        let team_centered_extras = EditorialExtras {
            subject_is_team: true,
            ..driver_centered_extras
        };
        let team_centered_blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 6],
            &item,
            "Equipe Solaris",
            "Mazda MX-5 Rookie Cup",
            &team_centered_extras,
        );
        assert!(
            team_centered_blocks[2].contains("Para uma equipe"),
            "market pp6 should remain available for team-centered stories: {}",
            team_centered_blocks[2],
        );
        assert_eq!(
            team_centered_blocks[2],
            "Para uma equipe com visibilidade expressiva, cada passo deste movimento altera o peso público e esportivo da pauta."
        );
    }

    #[test]
    fn test_mercado_pp6_reads_naturally_for_low_public_presence_tier() {
        let item = NewsItem {
            id: "MARKET002B".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(3),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: Some("T001".to_string()),
        };

        let extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: Some("baixa"),
            subject_is_team: true,
            event_label: "Okayama",
        };
        let blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 6],
            &item,
            "Equipe Solaris",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );

        assert_eq!(
            blocks[2],
            "Para uma equipe de circulação mais discreta, cada passo deste movimento altera o peso público e esportivo da pauta."
        );
    }

    #[test]
    fn test_mercado_presence_variant_is_skipped_without_presence_tier_and_team_subject() {
        let item = NewsItem {
            id: "MARKET003".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(3),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: None,
            team_id: Some("T001".to_string()),
        };

        let extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };
        let blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 6],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );

        assert!(
            !blocks[2].contains("Para uma equipe"),
            "market pp6 should be skipped when there is no public presence tier and no team-centered subject: {}",
            blocks[2],
        );
    }

    #[test]
    fn test_public_presence_phrase_uses_neutral_copy_for_all_supported_tiers() {
        assert_eq!(
            public_presence_phrase(Some("elite")),
            Some("de alto impacto público")
        );
        assert_eq!(
            public_presence_phrase(Some("alta")),
            Some("com visibilidade expressiva")
        );
        assert_eq!(
            public_presence_phrase(Some("relevante")),
            Some("com presença reconhecida no paddock")
        );
        assert_eq!(
            public_presence_phrase(Some("baixa")),
            Some("de circulação mais discreta")
        );
        assert_eq!(public_presence_phrase(Some("desconhecida")), None);
        assert_eq!(public_presence_phrase(None), None);
    }

    #[test]
    fn test_preseason_phase_phrase_is_optional_and_maps_expected_ranges() {
        assert_eq!(
            preseason_phase_phrase(Some(1)),
            Some("na abertura da pré-temporada")
        );
        assert_eq!(
            preseason_phase_phrase(Some(2)),
            Some("no meio da pré-temporada")
        );
        assert_eq!(
            preseason_phase_phrase(Some(5)),
            Some("nas semanas finais de pré-temporada")
        );
        assert_eq!(preseason_phase_phrase(None), None);
    }

    #[test]
    fn test_mercado_deck_revised_variants_avoid_race_framing() {
        for variation in [6usize, 11, 13] {
            let deck = build_story_deck(
                EditorialStoryType::Mercado,
                variation,
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
                "Okayama",
            );

            assert!(
                !deck.contains("Okayama"),
                "revised market deck variation {variation} should not lean on the next race label: {deck}"
            );
            assert!(
                !deck.contains("próxima prova"),
                "revised market deck variation {variation} should avoid race-framed copy: {deck}"
            );
            assert!(
                !deck.contains("rodada seguinte"),
                "revised market deck variation {variation} should avoid race-framed sequencing: {deck}"
            );
        }
    }

    #[test]
    fn test_incidente_c11_requires_secondary_driver_label() {
        let item = NewsItem {
            id: "INCIDENT001".to_string(),
            tipo: NewsType::Incidente,
            icone: "I".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(4),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: None,
            team_id: None,
        };

        let extras_without_secondary = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };
        let blocks_without_secondary = build_editorial_blocks(
            EditorialStoryType::Incidente,
            [0, 11, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras_without_secondary,
        );
        assert!(
            !blocks_without_secondary[1].contains("o outro envolvido"),
            "incident c11 should be skipped when no secondary driver exists: {}",
            blocks_without_secondary[1],
        );

        let extras_with_secondary = EditorialExtras {
            driver_secondary_label: Some("Marco Ferreira"),
            ..extras_without_secondary
        };
        let blocks_with_secondary = build_editorial_blocks(
            EditorialStoryType::Incidente,
            [0, 11, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras_with_secondary,
        );
        assert_eq!(
            blocks_with_secondary[1],
            "O incidente envolveu Carlos Mendes e Marco Ferreira: os dois chegam a Okayama em chave de resposta."
        );
    }

    #[test]
    fn test_incidente_c7_uses_direct_secondary_fragment() {
        let item = NewsItem {
            id: "INCIDENT002".to_string(),
            tipo: NewsType::Incidente,
            icone: "I".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(4),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: Some("P002".to_string()),
            team_id: None,
        };

        let extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: Some("Marco Ferreira"),
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };
        let blocks = build_editorial_blocks(
            EditorialStoryType::Incidente,
            [0, 7, 0],
            &item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &extras,
        );

        assert_eq!(
            blocks[1],
            "O dano recai sobre Carlos Mendes e Marco Ferreira, deixando a semana marcada por reparo e espera."
        );
    }

    #[test]
    fn test_missing_context_fallbacks_do_not_expose_editorial_placeholder_labels() {
        let corrida_item = NewsItem {
            id: "PLACEHOLDER001".to_string(),
            tipo: NewsType::Corrida,
            icone: "R".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(1),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: None,
            driver_id_secondary: None,
            team_id: None,
        };
        let mercado_item = NewsItem {
            id: "PLACEHOLDER002".to_string(),
            tipo: NewsType::Mercado,
            icone: "M".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(2),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: None,
            team_id: Some("T001".to_string()),
        };
        let incidente_item = NewsItem {
            id: "PLACEHOLDER003".to_string(),
            tipo: NewsType::Incidente,
            icone: "I".to_string(),
            titulo: "Titulo de teste".to_string(),
            texto: "Texto".to_string(),
            rodada: Some(4),
            semana_pretemporada: None,
            temporada: 1,
            categoria_id: Some("mazda_rookie".to_string()),
            categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
            importancia: crate::news::NewsImportance::Alta,
            timestamp: 1,
            driver_id: Some("P001".to_string()),
            driver_id_secondary: None,
            team_id: None,
        };

        let neutral_extras = EditorialExtras {
            driver_position: None,
            team_position: None,
            driver_secondary_label: None,
            preseason_week: None,
            presence_tier: None,
            subject_is_team: false,
            event_label: "Okayama",
        };

        let corrida_blocks = build_editorial_blocks(
            EditorialStoryType::Corrida,
            [0, 0, 0],
            &corrida_item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &neutral_extras,
        );
        let piloto_blocks = build_editorial_blocks(
            EditorialStoryType::Piloto,
            [0, 1, 3],
            &corrida_item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &neutral_extras,
        );
        let mercado_blocks = build_editorial_blocks(
            EditorialStoryType::Mercado,
            [0, 0, 6],
            &mercado_item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &neutral_extras,
        );
        let incidente_blocks = build_editorial_blocks(
            EditorialStoryType::Incidente,
            [0, 11, 0],
            &incidente_item,
            "Carlos Mendes",
            "Mazda MX-5 Rookie Cup",
            &neutral_extras,
        );

        let joined = [
            corrida_blocks.join(" "),
            piloto_blocks.join(" "),
            mercado_blocks.join(" "),
            incidente_blocks.join(" "),
        ]
        .join(" ")
        .to_lowercase();

        for forbidden in ["o nome em foco", "o piloto", "a equipe", "o outro envolvido"] {
            assert!(
                !joined.contains(forbidden),
                "guarded editorial fallbacks should not expose placeholder label `{forbidden}`: {joined}",
            );
        }
    }

    fn editorial_review_fixture(
        kind: EditorialStoryType,
    ) -> (NewsItem, EditorialExtras<'static>, &'static str, &'static str) {
        match kind {
            EditorialStoryType::Corrida => (
                NewsItem {
                    id: "DUMP_CORRIDA".to_string(),
                    tipo: NewsType::Corrida,
                    icone: "R".to_string(),
                    titulo: "Carlos Mendes chega pressionado a Okayama".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: None,
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: Some("T001".to_string()),
                },
                EditorialExtras {
                    driver_position: Some(3),
                    team_position: None,
                    driver_secondary_label: None,
                    preseason_week: None,
                    presence_tier: None,
                    subject_is_team: false,
                    event_label: "Okayama",
                },
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
            ),
            EditorialStoryType::Incidente => (
                NewsItem {
                    id: "DUMP_INCIDENTE".to_string(),
                    tipo: NewsType::Incidente,
                    icone: "I".to_string(),
                    titulo: "Carlos Mendes e Marco Ferreira se tocam na pista".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: None,
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: Some("P002".to_string()),
                    team_id: None,
                },
                EditorialExtras {
                    driver_position: Some(3),
                    team_position: None,
                    driver_secondary_label: Some("Marco Ferreira"),
                    preseason_week: None,
                    presence_tier: None,
                    subject_is_team: false,
                    event_label: "Okayama",
                },
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
            ),
            EditorialStoryType::Piloto => (
                NewsItem {
                    id: "DUMP_PILOTO".to_string(),
                    tipo: NewsType::Rivalidade,
                    icone: "P".to_string(),
                    titulo: "Carlos Mendes cresce no momento decisivo".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: None,
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                EditorialExtras {
                    driver_position: Some(3),
                    team_position: None,
                    driver_secondary_label: None,
                    preseason_week: None,
                    presence_tier: None,
                    subject_is_team: false,
                    event_label: "Okayama",
                },
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
            ),
            EditorialStoryType::Equipe => (
                NewsItem {
                    id: "DUMP_EQUIPE".to_string(),
                    tipo: NewsType::Hierarquia,
                    icone: "E".to_string(),
                    titulo: "Equipe Solaris reorganiza a estrutura".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: None,
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: Some("T001".to_string()),
                },
                EditorialExtras {
                    driver_position: None,
                    team_position: Some(2),
                    driver_secondary_label: None,
                    preseason_week: None,
                    presence_tier: Some("alta"),
                    subject_is_team: true,
                    event_label: "Okayama",
                },
                "Equipe Solaris",
                "Mazda MX-5 Rookie Cup",
            ),
            EditorialStoryType::Mercado => (
                NewsItem {
                    id: "DUMP_MERCADO".to_string(),
                    tipo: NewsType::Mercado,
                    icone: "M".to_string(),
                    titulo: "Equipe Solaris observa reforcos no paddock".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: Some(2),
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: None,
                    driver_id_secondary: None,
                    team_id: Some("T001".to_string()),
                },
                EditorialExtras {
                    driver_position: None,
                    team_position: None,
                    driver_secondary_label: None,
                    preseason_week: Some(2),
                    presence_tier: Some("alta"),
                    subject_is_team: true,
                    event_label: "Okayama",
                },
                "Equipe Solaris",
                "Mazda MX-5 Rookie Cup",
            ),
            EditorialStoryType::Estrutural => (
                NewsItem {
                    id: "DUMP_ESTRUTURAL".to_string(),
                    tipo: NewsType::FramingSazonal,
                    icone: "S".to_string(),
                    titulo: "Carlos Mendes entra em nova fase no campeonato".to_string(),
                    texto: "Texto".to_string(),
                    rodada: Some(3),
                    semana_pretemporada: None,
                    temporada: 1,
                    categoria_id: Some("mazda_rookie".to_string()),
                    categoria_nome: Some("Mazda MX-5 Rookie Cup".to_string()),
                    importancia: crate::news::NewsImportance::Alta,
                    timestamp: 1,
                    driver_id: Some("P001".to_string()),
                    driver_id_secondary: None,
                    team_id: None,
                },
                EditorialExtras {
                    driver_position: Some(3),
                    team_position: None,
                    driver_secondary_label: None,
                    preseason_week: None,
                    presence_tier: None,
                    subject_is_team: false,
                    event_label: "Okayama",
                },
                "Carlos Mendes",
                "Mazda MX-5 Rookie Cup",
            ),
        }
    }

    fn dump_editorial_blocks_for_review(kind: EditorialStoryType) {
        let (item, extras, subject, category) = editorial_review_fixture(kind);
        let labels = editorial_block_labels(kind);
        let kind_label = match kind {
            EditorialStoryType::Corrida => "CORRIDA",
            EditorialStoryType::Incidente => "INCIDENTE",
            EditorialStoryType::Piloto => "PILOTO",
            EditorialStoryType::Equipe => "EQUIPE",
            EditorialStoryType::Mercado => "MERCADO",
            EditorialStoryType::Estrutural => "ESTRUTURAL",
        };
        let pos_label = extras
            .driver_position
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "=== {kind_label} / subject={subject} / event={} / pos={pos_label} ===",
            extras.event_label
        );

        for variation in 0..EDITORIAL_BLOCK_VARIANT_COUNT {
            let blocks = build_editorial_blocks(
                kind,
                [variation, variation, variation],
                &item,
                subject,
                category,
                &extras,
            );

            assert_eq!(blocks.len(), 3);
            println!();
            println!("{}[{variation}]: {}", labels[0], blocks[0]);
            println!("{}[{variation}]: {}", labels[1], blocks[1]);
            println!("{}[{variation}]: {}", labels[2], blocks[2]);
        }
    }

    #[test]
    #[ignore]
    fn dump_corrida_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Corrida);
    }

    #[test]
    #[ignore]
    fn dump_piloto_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Piloto);
    }

    #[test]
    #[ignore]
    fn dump_mercado_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Mercado);
    }

    #[test]
    #[ignore]
    fn dump_incidente_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Incidente);
    }

    #[test]
    #[ignore]
    fn dump_equipe_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Equipe);
    }

    #[test]
    #[ignore]
    fn dump_estrutural_blocks_for_editorial_review() {
        dump_editorial_blocks_for_review(EditorialStoryType::Estrutural);
    }

    #[test]
    #[ignore]
    fn dump_position_sensitive_variants_matrix() {
        let positions: &[Option<i32>] = &[
            Some(1),
            Some(2),
            Some(3),
            Some(5),
            Some(6),
            Some(10),
            Some(15),
            None,
        ];

        let (corrida_item, _, corrida_subject, corrida_category) =
            editorial_review_fixture(EditorialStoryType::Corrida);
        let (piloto_item, _, piloto_subject, piloto_category) =
            editorial_review_fixture(EditorialStoryType::Piloto);

        println!("\n=== POSITION MATRIX ===");

        for pos in positions {
            let pos_label = pos
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string());

            println!("\npos={pos_label}");

            let corrida_extras = EditorialExtras {
                driver_position: *pos,
                team_position: None,
                driver_secondary_label: None,
                preseason_week: None,
                presence_tier: None,
                subject_is_team: false,
                event_label: "Okayama",
            };

            let piloto_extras = EditorialExtras {
                driver_position: *pos,
                team_position: None,
                driver_secondary_label: None,
                preseason_week: None,
                presence_tier: None,
                subject_is_team: false,
                event_label: "Okayama",
            };

            let corrida_i0 = build_editorial_blocks(
                EditorialStoryType::Corrida,
                [0, 0, 0],
                &corrida_item,
                corrida_subject,
                corrida_category,
                &corrida_extras,
            );
            let corrida_i4 = build_editorial_blocks(
                EditorialStoryType::Corrida,
                [0, 4, 0],
                &corrida_item,
                corrida_subject,
                corrida_category,
                &corrida_extras,
            );
            let corrida_i8 = build_editorial_blocks(
                EditorialStoryType::Corrida,
                [0, 8, 0],
                &corrida_item,
                corrida_subject,
                corrida_category,
                &corrida_extras,
            );
            let piloto_p1 = build_editorial_blocks(
                EditorialStoryType::Piloto,
                [0, 1, 0],
                &piloto_item,
                piloto_subject,
                piloto_category,
                &piloto_extras,
            );
            let piloto_p4 = build_editorial_blocks(
                EditorialStoryType::Piloto,
                [0, 4, 0],
                &piloto_item,
                piloto_subject,
                piloto_category,
                &piloto_extras,
            );
            let piloto_p8 = build_editorial_blocks(
                EditorialStoryType::Piloto,
                [0, 8, 0],
                &piloto_item,
                piloto_subject,
                piloto_category,
                &piloto_extras,
            );

            println!("  Corrida i0: {}", corrida_i0[1]);
            println!("  Corrida i4: {}", corrida_i4[1]);
            println!("  Corrida i8: {}", corrida_i8[1]);
            println!("  Piloto  p1: {}", piloto_p1[1]);
            println!("  Piloto  p4: {}", piloto_p4[1]);
            println!("  Piloto  p8: {}", piloto_p8[1]);

            // assertions leves: nenhum placeholder de label deve vazar
            for (label, text) in [
                ("Corrida i0", &corrida_i0[1]),
                ("Corrida i4", &corrida_i4[1]),
                ("Corrida i8", &corrida_i8[1]),
                ("Piloto  p1", &piloto_p1[1]),
                ("Piloto  p4", &piloto_p4[1]),
                ("Piloto  p8", &piloto_p8[1]),
            ] {
                for forbidden in ["o nome em foco", "nome no top-10", "segunda metade do grid"] {
                    assert!(
                        !text.contains(forbidden),
                        "pos={pos_label} [{label}] contém placeholder proibido `{forbidden}`: {text}",
                    );
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn dump_mercado_public_presence_matrix() {
        let (item, _, subject, category) =
            editorial_review_fixture(EditorialStoryType::Mercado);

        // (tier, subject_is_team)
        let cases: &[(&str, Option<&str>, bool)] = &[
            ("elite / team=true",     Some("elite"),     true),
            ("alta / team=true",      Some("alta"),      true),
            ("relevante / team=true", Some("relevante"), true),
            ("baixa / team=true",     Some("baixa"),     true),
            ("None / team=true",      None,              true),
            ("alta / team=false",     Some("alta"),      false),
            ("None / team=false",     None,              false),
        ];

        println!("\n=== MERCADO PRESENCE MATRIX ===");

        for (label, tier, is_team) in cases {
            let extras = EditorialExtras {
                driver_position: None,
                team_position: None,
                driver_secondary_label: None,
                preseason_week: None,
                presence_tier: *tier,
                subject_is_team: *is_team,
                event_label: "Okayama",
            };

            // variant 6 do bloco "Próximo passo" (block index 2)
            let blocks = build_editorial_blocks(
                EditorialStoryType::Mercado,
                [0, 0, 6],
                &item,
                subject,
                category,
                &extras,
            );
            let proximo_passo = &blocks[2];

            println!("tier={label}: {proximo_passo}");

            // assertions leves
            if !is_team {
                assert!(
                    !proximo_passo.contains("Para uma equipe"),
                    "team=false não deve conter 'Para uma equipe' [tier={label}]: {proximo_passo}",
                );
            }
            if tier.is_none() {
                assert!(
                    !proximo_passo.contains("Para uma equipe"),
                    "tier=None não deve conter 'Para uma equipe' [label={label}]: {proximo_passo}",
                );
            }
            if *is_team && tier.is_some() {
                assert!(
                    proximo_passo.contains("Para uma equipe"),
                    "team=true + tier={:?} deve conter 'Para uma equipe' [label={label}]: {proximo_passo}",
                    tier,
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn dump_mercado_preseason_matrix() {
        let weeks: &[Option<i32>] = &[Some(1), Some(2), Some(3), Some(5), None];

        let (item, _, subject, category) =
            editorial_review_fixture(EditorialStoryType::Mercado);

        println!("\n=== MERCADO PRESEASON MATRIX ===");

        for week in weeks {
            let week_label = week
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string());

            let extras = EditorialExtras {
                driver_position: None,
                team_position: None,
                driver_secondary_label: None,
                preseason_week: *week,
                presence_tier: Some("alta"),
                subject_is_team: true,
                event_label: "Okayama",
            };

            // variante 1 do bloco "Próximo passo" (block index 2)
            let blocks = build_editorial_blocks(
                EditorialStoryType::Mercado,
                [0, 0, 1],
                &item,
                subject,
                category,
                &extras,
            );
            let proximo_passo = &blocks[2];

            println!("week={week_label}: {proximo_passo}");

            // assertions leves
            match week {
                None => {
                    assert!(
                        !proximo_passo.contains("pré-temporada"),
                        "week=None não deve conter 'pré-temporada': {proximo_passo}",
                    );
                }
                Some(1) => {
                    assert!(
                        proximo_passo.contains("na abertura da pré-temporada"),
                        "week=1 deve conter 'na abertura da pré-temporada': {proximo_passo}",
                    );
                }
                Some(2) | Some(3) => {
                    assert!(
                        proximo_passo.contains("no meio da pré-temporada"),
                        "week={week_label} deve conter 'no meio da pré-temporada': {proximo_passo}",
                    );
                }
                Some(5) => {
                    assert!(
                        proximo_passo.contains("nas semanas finais de pré-temporada"),
                        "week=5 deve conter 'nas semanas finais de pré-temporada': {proximo_passo}",
                    );
                }
                _ => {}
            }
        }
    }
}

pub(crate) fn classify_editorial_story_type(item: &NewsItem) -> EditorialStoryType {
    match item.tipo {
        NewsType::Corrida => EditorialStoryType::Corrida,
        NewsType::Incidente | NewsType::Lesao => EditorialStoryType::Incidente,
        NewsType::Mercado | NewsType::PreTemporada => EditorialStoryType::Mercado,
        NewsType::Hierarquia => EditorialStoryType::Equipe,
        NewsType::Promocao
        | NewsType::Rebaixamento
        | NewsType::Aposentadoria
        | NewsType::FramingSazonal => EditorialStoryType::Estrutural,
        NewsType::Rivalidade | NewsType::Milestone | NewsType::Rookies | NewsType::Evolucao => {
            EditorialStoryType::Piloto
        }
    }
}

pub(crate) fn editorial_block_labels(kind: EditorialStoryType) -> [&'static str; 3] {
    match kind {
        EditorialStoryType::Corrida => ["Resumo", "Impacto", "Leitura"],
        EditorialStoryType::Incidente => ["Ocorrido", "Consequência", "Estado"],
        EditorialStoryType::Piloto => ["Momento", "Pressão", "Sinal"],
        EditorialStoryType::Equipe => ["Movimento", "Resposta", "Panorama"],
        EditorialStoryType::Mercado => ["Movimento", "Impacto", "Próximo passo"],
        EditorialStoryType::Estrutural => ["Mudança", "Efeito", "Panorama"],
    }
}

pub(crate) fn editorial_variant_index(
    item: &NewsItem,
    kind: EditorialStoryType,
    lane: &str,
    modulo: usize,
) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    item.id.hash(&mut hasher);
    item.timestamp.hash(&mut hasher);
    kind.hash(&mut hasher);
    lane.hash(&mut hasher);
    (hasher.finish() as usize) % modulo
}

pub(crate) fn editorial_slot_indexes(
    item: &NewsItem,
    kind: EditorialStoryType,
) -> (usize, [usize; 3]) {
    (
        editorial_variant_index(item, kind, "deck", EDITORIAL_DECK_VARIANT_COUNT),
        [
            editorial_variant_index(item, kind, "block-0", EDITORIAL_BLOCK_VARIANT_COUNT),
            editorial_variant_index(item, kind, "block-1", EDITORIAL_BLOCK_VARIANT_COUNT),
            editorial_variant_index(item, kind, "block-2", EDITORIAL_BLOCK_VARIANT_COUNT),
        ],
    )
}

fn select_variant<const N: usize>(index: usize, variants: [String; N]) -> String {
    select_valid_variant(index, variants, |_| true)
}

/// Como `select_variant`, mas pula variantes inválidas para o contexto atual.
///
/// Começa em `preferred_index` e avança ciclicamente até encontrar um índice
/// que `validity(idx)` aprove. Se nenhum aprovar, usa `preferred_index % N`
/// como fallback incondicional (não deve ocorrer se pelo menos um variant for
/// sempre válido).
fn select_valid_variant<const N: usize>(
    preferred_index: usize,
    variants: [String; N],
    validity: impl Fn(usize) -> bool,
) -> String {
    let variants = Vec::from(variants);

    for offset in 0..N {
        let candidate = (preferred_index + offset) % N;
        if validity(candidate) {
            return variants[candidate].clone();
        }
    }

    variants[preferred_index % N].clone()
}

/// Dados de contexto extras passados para os blocos editoriais.
/// Vêm de `EditorialStoryContext` em `news_tab.rs` e enriquecem os templates
/// com informações competitivas e de temporada que não estavam disponíveis antes.
pub(crate) struct EditorialExtras<'a> {
    /// Posição atual do piloto principal no campeonato
    pub driver_position: Option<i32>,
    /// Posição atual da equipe principal no campeonato
    pub team_position: Option<i32>,
    /// Label do piloto secundário (rival, outro envolvido no incidente, etc.)
    pub driver_secondary_label: Option<&'a str>,
    /// Semana de pré-temporada em que a notícia aconteceu (None se for temporada normal)
    pub preseason_week: Option<i32>,
    /// Tier de presença pública da equipe envolvida ("elite", "alta", "relevante", "baixa")
    pub presence_tier: Option<&'a str>,
    /// Indica se o assunto principal da story está ancorado em equipe.
    pub subject_is_team: bool,
    /// Label da próxima corrida da categoria, quando o template realmente precisa disso
    pub event_label: &'a str,
}

/// Retorna uma frase posicional editorial para uso nos templates de bloco.
///
/// Retorna `None` quando a posição é desconhecida — o template deve usar um
/// variant sem frase posicional (o guard em `select_valid_variant` garante isso).
pub(crate) fn position_pressure_phrase(position: Option<i32>) -> Option<String> {
    match position {
        Some(1) => Some("o líder do campeonato".to_string()),
        Some(2) => Some("o vice-líder".to_string()),
        Some(3) => Some("o terceiro colocado".to_string()),
        Some(4) => Some("o quarto colocado".to_string()),
        Some(5) => Some("o quinto colocado".to_string()),
        Some(_) => None,
        None => None,
    }
}

/// "na abertura da pré-temporada", "no meio da pré-temporada", etc.
/// Usado em blocos de Mercado para dar fase à movimentação pré-temporada.
pub(crate) fn preseason_phase_phrase(week: Option<i32>) -> Option<&'static str> {
    match week {
        Some(1) => Some("na abertura da pré-temporada"),
        Some(2) | Some(3) => Some("no meio da pré-temporada"),
        Some(_) => Some("nas semanas finais de pré-temporada"),
        None => None,
    }
}

/// "de alto impacto público", "com visibilidade expressiva", etc.
/// Usado em blocos editoriais quando existe uma leitura pública real para a equipe.
pub(crate) fn public_presence_phrase(tier: Option<&str>) -> Option<&'static str> {
    match tier {
        Some("elite") => Some("de alto impacto público"),
        Some("alta") => Some("com visibilidade expressiva"),
        Some("relevante") => Some("com presença reconhecida no paddock"),
        Some("baixa") | Some("discreta") => Some("de circulação mais discreta"),
        _ => None,
    }
}

pub(crate) fn build_story_deck(
    kind: EditorialStoryType,
    variation: usize,
    subject: &str,
    category: &str,
    event_label: &str,
) -> String {
    match kind {
        EditorialStoryType::Corrida => select_variant(variation, [
            format!("{subject} leva {category} para um recorte mais urgente antes de {event_label}."),
            format!("O resultado empurra {subject} para o centro de {category} e muda a conversa para {event_label}."),
            format!("{category} chega a {event_label} com nova referência competitiva ao redor de {subject}."),
            format!("A pauta de pista ganha peso porque {subject} reabre a disputa em {category}."),
            format!("{subject} transforma a semana de {category} em um teste mais direto para {event_label}."),
            format!("O campeonato passa a ler {subject} como ponto de pressão real na aproximação de {event_label}."),
            format!("A edição de {category} muda de tom quando {subject} desloca a atenção para a próxima prova."),
            format!("{subject} recoloca a rodada de {category} em movimento e deixa {event_label} menos previsível."),
            format!("{category} ganha nova temperatura quando {subject} encurta a margem de tranquilidade do grid."),
            format!("O fim de semana de {category} chega a {event_label} com menos conforto e mais cobrança sobre {subject}."),
            format!("{subject} muda a leitura da categoria ao transformar a próxima largada em validação imediata."),
            format!("{category} entra em nova faixa de tensão depois que {subject} altera a ordem natural da conversa."),
            format!("A rodada seguinte passa a existir sob outra luz porque {subject} elevou o nível de cobrança em {category}."),
            format!("{subject} deixa {category} mais exposta à disputa aberta e tira a sensação de roteiro estável."),
            format!("A manchete de {category} fica mais viva porque {subject} empurra o debate direto para {event_label}."),
            format!("{subject} redistribui a atenção do campeonato e faz {category} chegar mais pressionada à próxima prova."),
        ]),
        EditorialStoryType::Incidente => select_variant(variation, [
            format!("O incidente deixa {subject} no centro do dano mais sensível desta faixa de {category}."),
            format!("{subject} chega a {event_label} com um episódio recente ainda pesando na leitura de {category}."),
            format!("A semana de {category} endurece porque o caso envolvendo {subject} ainda não saiu do foco."),
            format!("O recorte atual de {category} passa por entender quanto do dano de {subject} segue aberto."),
            format!("A notícia de incidente muda o ritmo de {category} e empurra {subject} para resposta imediata."),
            format!("{subject} vira referência de reparo esportivo e narrativo na aproximação de {event_label}."),
            format!("O episódio atravessa a pauta de {category} e obriga {subject} a responder em prazo curto."),
            format!("A categoria absorve um choque relevante e deixa {subject} sob observação antes de {event_label}."),
            format!("O caso recente força {category} a tratar {subject} como nome em estado de recuperação competitiva."),
            format!("{subject} entra na próxima etapa com o peso de um dano ainda não encerrado em {category}."),
            format!("A conversa do paddock endurece porque {subject} ainda simboliza o custo mais visível do incidente."),
            format!("{category} segue lendo o caso de {subject} como ferida recente, não como página virada."),
            format!("A ocorrência muda o tom da semana e faz {subject} carregar o foco mais delicado de {category}."),
            format!("A rodada seguinte de {category} nasce marcada pelo dano que ainda acompanha {subject}."),
            format!("{subject} vira referência do reparo mais urgente desta fase do campeonato."),
            format!("O incidente mantém {category} em estado de alerta e deixa {subject} no centro da cobrança."),
        ]),
        EditorialStoryType::Piloto => select_variant(variation, [
            format!("{subject} entra no radar principal de {category} com um momento que pede leitura menos superficial."),
            format!("O foco individual da semana recai sobre {subject}, nome que ganhou outra escala em {category}."),
            format!("{subject} deixa de ser nota lateral e vira termômetro do momento atual em {category}."),
            format!("A pauta de piloto cresce porque {subject} aparece como resposta pendente antes de {event_label}."),
            format!("{category} passa a medir o próximo passo de {subject} com menos paciência e mais atenção."),
            format!("A fase de {subject} ganha peso de manchete e reorganiza a expectativa para {event_label}."),
            format!("{subject} se torna um dos nomes que explicam o momento competitivo de {category}."),
            format!("O recorte atual empurra {subject} para a frente da conversa e encurta a margem para resposta."),
            format!("{subject} assume papel central na narrativa recente e faz {category} olhar além da tabela."),
            format!("O paddock trata {subject} como termômetro de fase e leva essa leitura para {event_label}."),
            format!("{category} volta a discutir o tamanho real da fase de {subject} à luz do que vem pela frente."),
            format!("{subject} aparece como síntese do momento esportivo que hoje define a conversa em {category}."),
            format!("A discussão sobre piloto ganha outro peso quando {subject} deixa de ser coadjuvante na pauta."),
            format!("{subject} puxa a leitura individual mais forte desta edição e reposiciona a expectativa do grid."),
            format!("O nome de {subject} cresce porque {category} já cobra impacto imediato na próxima rodada."),
            format!("{subject} entra no centro editorial da semana e passa a organizar a conversa de {category}."),
        ]),
        EditorialStoryType::Equipe => select_variant(variation, [
            format!("{subject} vira referência coletiva em {category} e puxa o box para o centro da edição."),
            format!("A equipe de {subject} muda a leitura do paddock e amplia o peso operacional de {event_label}."),
            format!("O bloco coletivo da semana passa por {subject}, que redefine o tom de {category}."),
            format!("{category} ganha um recorte menos individual quando {subject} entra no eixo principal da pauta."),
            format!("{subject} transforma a conversa de {category} em um teste de estrutura, não só de brilho isolado."),
            format!("A engrenagem de {subject} passa a importar mais porque {event_label} cobra resposta de conjunto."),
            format!("O box de {subject} assume peso renovado e reorganiza a pauta coletiva do campeonato."),
            format!("{subject} recoloca a leitura de equipe no centro de {category} antes da próxima resposta em pista."),
            format!("{category} volta a falar de execução coletiva quando {subject} deixa de ser pano de fundo."),
            format!("O paddock trata {subject} como exemplo de estrutura que pode mudar a próxima rodada."),
            format!("{subject} leva a discussão de {category} para um campo mais operacional e menos individual."),
            format!("A equipe de {subject} passa a explicar mais da rodada do que apenas o desempenho de um nome."),
            format!("{category} reencontra a pauta de box quando {subject} vira referência de consistência ou falha."),
            format!("{subject} empurra a leitura coletiva para a frente e recoloca a execução no centro da edição."),
            format!("O grid passa a medir a estrutura de {subject} com atenção ampliada antes de {event_label}."),
            format!("{subject} transforma o debate de {category} em análise de conjunto e capacidade de resposta."),
        ]),
        EditorialStoryType::Mercado => select_variant(variation, [
            format!("O movimento envolvendo {subject} reposiciona a conversa de {category} e abre um recorte de mercado mais direto."),
            format!("{subject} empurra a pauta de mercado para a frente e muda o equilíbrio de interesse em {category}."),
            format!("A notícia mexe com o paddock porque {subject} passa a concentrar expectativa esportiva e contratual."),
            format!("O caso ligado a {subject} tira o mercado da periferia e cola o tema no centro de {category}."),
            format!("{category} entra em uma semana de especulação mais concreta a partir do nome de {subject}."),
            format!("{subject} faz o paddock olhar para o mercado como pauta de curto prazo, não só de fundo."),
            format!("A movimentação em torno de {subject} altera a temperatura de {category} ainda nos bastidores."),
            format!("O grid passa a tratar {subject} como pivô de uma conversa de mercado com peso real."),
            format!("{subject} deixa o mercado mais visível e obriga {category} a reagir em público e nos bastidores."),
            format!("A pauta de contratos ganha urgência porque {subject} movimenta expectativa e valor em {category}."),
            format!("{category} encurta a distância entre rumor e definição quando {subject} entra nesse eixo."),
            format!("{subject} recoloca o mercado no centro da semana e muda o foco do paddock para o que vem a seguir."),
            format!("O nome de {subject} transforma o ambiente de {category} em uma vitrine de expectativa contratual."),
            format!("As próximas semanas já passam a ser acompanhadas pela conversa de mercado que gira ao redor de {subject}."),
            format!("{subject} aproxima negociação e desempenho em um recorte que pesa mais do que o habitual."),
            format!("O paddock trata {subject} como gatilho de uma nova fase de mercado em {category}."),
        ]),
        EditorialStoryType::Estrutural => select_variant(variation, [
            format!("A mudança ligada a {subject} altera a estrutura de {category} e pede uma leitura mais ampla do momento."),
            format!("{subject} passa a representar uma virada de desenho dentro de {category}."),
            format!("O enquadramento de {category} muda porque {subject} deixa de ocupar o mesmo lugar na hierarquia."),
            format!("A pauta estrutural ganha corpo quando {subject} desloca o eixo do campeonato."),
            format!("{category} entra numa fase menos estática e {subject} vira uma das chaves dessa transição."),
            format!("A reorganização em torno de {subject} redefine o mapa de expectativas para {event_label}."),
            format!("{subject} concentra uma mudança que repercute além da rodada e afeta o desenho da temporada."),
            format!("A notícia abre uma frente estrutural em {category} e coloca {subject} no centro desse movimento."),
            format!("{subject} passa a simbolizar uma troca de patamar na forma como {category} se organiza."),
            format!("A temporada muda de desenho quando {subject} altera o lugar que ocupava em {category}."),
            format!("{category} entra em nova configuração estrutural e {subject} aparece como uma das chaves do processo."),
            format!("O paddock passa a tratar {subject} como referência de uma mudança que vai além do resultado imediato."),
            format!("A hierarquia recente deixa de parecer fixa porque {subject} move o eixo estrutural da categoria."),
            format!("O campeonato ganha outro desenho quando {subject} deixa de ocupar o mesmo papel de antes."),
            format!("{subject} concentra uma transição que muda o enquadramento de {category} por várias semanas."),
            format!("A reorganização em {category} assume rosto claro quando {subject} altera a lógica do campeonato."),
        ]),
    }
}

pub(crate) fn build_editorial_blocks(
    kind: EditorialStoryType,
    variations: [usize; 3],
    item: &NewsItem,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let title = item.titulo.as_str();
    match kind {
        EditorialStoryType::Corrida => build_corrida_blocks_v2(variations, title, subject, category, extras),
        EditorialStoryType::Incidente => build_incidente_blocks_v2(variations, title, subject, category, extras),
        EditorialStoryType::Piloto => build_piloto_blocks_v2(variations, title, subject, category, extras),
        EditorialStoryType::Equipe => build_equipe_blocks_v2(variations, title, subject, category, extras),
        EditorialStoryType::Mercado => build_mercado_blocks_v2(variations, title, subject, category, extras),
        EditorialStoryType::Estrutural => build_estrutural_blocks_v2(variations, title, subject, category, extras),
    }
}

fn build_corrida_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    let resumo = select_variant(
        variations[0],
        [
            format!("A reorganização do centro da rodada passa por {title} e recoloca {category} em tom mais competitivo."),
            format!("O ponto principal desta corrida está em {title}."),
            format!("A prova fica menos previsível a partir de {title}."),
            format!("A notícia de pista parte de {title}, assunto que reposiciona a hierarquia de {category}."),
            format!("A rodada vai para um eixo mais agressivo de leitura a partir de {title}."),
            format!("Não se trata apenas de quem apareceu na frente: a mudança de enquadramento passa por {title}."),
            format!("O recorte que combina desempenho, tensão e resposta curta do campeonato começa em {title}."),
            format!("A conversa competitiva de {category} não se fecha; ela se reabre em {title}."),
            format!("A prova ganha outra escala quando o marcador da rodada passa a ser {title}."),
            format!("A mudança clara de tom dentro de {category} aparece em {title}."),
            format!("O centro editorial desta prova está em {title}, porque foi ali que a hierarquia saiu do lugar."),
            format!("A sensação de rodada menos estável e mais aberta nasce em {title}."),
        ],
    );
    let position_phrase = position_pressure_phrase(extras.driver_position);
    let has_position_phrase = position_phrase.is_some();
    // unwrap_or("") seguro: o guard garante que i0/i4 só são selecionados quando has_position=true
    let impacto = select_valid_variant(
        variations[1],
        [
            format!(
                "Como {}, {subject} chega a {event_ref} com margem de tolerância menor.",
                position_phrase.as_deref().unwrap_or("")
            ),
            format!("{subject} chega a {event_ref} com margem de tolerância menor."),
            format!("A próxima largada passa a cobrar de {subject} uma resposta à altura do impacto desta corrida."),
            format!("O grid passa a medir {subject} por aquilo que esta corrida provocou no desenho competitivo da categoria."),
            format!(
                "{subject} entra em {event_ref} sob a pressão típica de quem chega hoje como {}.",
                position_phrase.as_deref().unwrap_or("")
            ),
            format!("{subject} entra em {event_ref} sob cobrança mais visível."),
            format!("O resultado amplia a atenção sobre {subject} e encurta a margem para uma rodada neutra."),
            format!("O grid passa a observar {subject} pelo que esta prova alterou, não só pela posição final."),
            format!(
                "A condição de {} faz {subject} chegar a {event_ref} com cobrança ampliada.",
                position_phrase.as_deref().map(|s| s.trim_start_matches("o ")).unwrap_or("")
            ),
            format!("A próxima etapa amplia a cobrança esportiva sobre {subject}."),
            format!("O peso esportivo agora recai em como {subject} administra a nova leitura que surgiu após a prova."),
            format!("{subject} vira parâmetro do que a categoria espera ver confirmado já no próximo fim de semana."),
        ],
        // i0, i4 e i8 usam position_phrase — só válidos quando a posição é conhecida
        |idx| !matches!(idx, 0 | 4 | 8) || has_position_phrase,
    );
    let leitura = select_variant(
        variations[2],
        [
            format!("A leitura aqui é direta: {category} chega mais exposta, com menos conforto e mais disputa aberta."),
            format!("O que fica desta prova é um campeonato mais sensível à confirmação imediata do que aconteceu na pista."),
            format!("Para a sequência, o ponto central é saber se {subject} transforma esse impacto em ritmo ou se a pauta esfria já na próxima largada."),
            format!("O peso desta história está em como ela troca a segurança do roteiro por uma rodada seguinte com margem menor para erro."),
            format!("A próxima etapa vai dizer se o campeonato absorve a mudança ou se a disputa realmente entrou em outro patamar."),
            format!("Mais do que o resultado bruto, esta prova deixa um clima de validação imediata para a sequência."),
            format!("A categoria sai desta corrida em estado de atenção, porque a hierarquia ficou menos protegida."),
            format!("O paddock passa a ler a próxima prova como resposta editorial e esportiva ao que aconteceu aqui."),
            format!("O tema central agora é continuidade: ou o impacto ganha corpo, ou vira uma oscilação curta."),
            format!("A corrida muda a conversa porque tira a sensação de estabilidade e devolve dúvida ao grid."),
            format!("O que pesa de verdade é a forma como a etapa seguinte vai confirmar ou desmentir esta virada."),
            format!("A história cresce porque o campeonato já não parece tão confortável ao se aproximar da próxima rodada."),
        ],
    );
    [resumo, impacto, leitura]
}

fn build_incidente_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    let secondary_fragment = extras
        .driver_secondary_label
        .as_ref()
        .map(|name| format!(" e {name}"))
        .unwrap_or_default();
    let secondary = extras.driver_secondary_label.unwrap_or("");
    let ocorrido = select_variant(
        variations[0],
        [
            format!("{title} concentra o episódio que mais alterou o fluxo recente de {category}."),
            format!("O incidente central desta pauta está em {title}, ponto de quebra da sequência normal da rodada."),
            format!("{title} deixa de ser detalhe de prova e vira o ocorrido que muda a conversa da semana."),
            format!("O fato dominante deste recorte cabe em {title}, sem precisar de enfeite para pesar."),
            format!("{title} resume a ruptura que tirou a rodada do curso esperado."),
            format!("A ocorrência principal desta semana pode ser lida a partir de {title}."),
            format!("{title} vira o centro do noticiário porque foi ali que a prova mudou de tom."),
            format!("O episódio mais sensível de {category} se organiza ao redor de {title}."),
            format!("{title} concentra o ponto exato em que a rodada perdeu estabilidade."),
            format!("O paddock reconhece em {title} o caso que mais bagunçou a leitura recente da categoria."),
            format!("{title} transforma um incidente de pista em assunto central da edição."),
            format!("O ocorrido que reorienta a semana de {category} está resumido em {title}."),
        ],
    );
    let has_secondary = extras.driver_secondary_label.is_some();
    let consequencia = select_valid_variant(
        variations[1],
        [
            format!("O dano imediato recai sobre {subject}, que sai desta história com margem esportiva menor."),
            format!("{subject} precisa administrar uma perda que mexe ao mesmo tempo com resultado, confiança e narrativa."),
            format!("A consequência maior é obrigar {subject} a voltar para {event_ref} em chave de resposta, não de continuidade."),
            format!("{subject} deixa de ser apenas citado e vira o foco principal do dano que a categoria ainda está absorvendo."),
            format!("O episódio empurra {subject} para uma fase de reação curta e cobrança aberta."),
            format!("O prejuízo se concentra em {subject}, agora obrigado a recuperar pista, confiança e contexto."),
            format!("{subject} entra em um recorte mais frágil porque o dano transbordou a bandeirada."),
            format!("O dano recai sobre {subject}{secondary_fragment}, deixando a semana marcada por reparo e espera."),
            format!("A categoria passa a medir {subject} não só pelo incidente, mas pelo custo esportivo que ele deixou."),
            format!("O impacto mais visível é sobre {subject}, que perde conforto e ganha urgência na sequência."),
            format!("{subject} sai deste caso com menos margem emocional e competitiva para a etapa seguinte."),
            format!("O incidente envolveu {subject} e {secondary}: os dois chegam a {event_ref} em chave de resposta."),
        ],
        // c7 e c11 nomeiam explicitamente o piloto secundário — só válidos quando ele existe
        |idx| !matches!(idx, 7 | 11) || has_secondary,
    );
    let estado = select_variant(
        variations[2],
        [
            format!("O estado do caso ainda é de reparo: {event_ref} chega como a primeira chance real de medir recuperação."),
            format!("Em {category}, o episódio segue aberto e o paddock ainda trata a pauta mais como dano pendente do que como página virada."),
            format!("A categoria entra na sequência sob tensão, porque o efeito do incidente ainda pesa na leitura de quem volta ao grid."),
            format!("O quadro atual é de resposta curta: o que aconteceu não terminou na bandeirada e ainda condiciona a leitura esportiva de {subject}."),
            format!("A próxima etapa funciona como teste inicial de recuperação, não como retorno à normalidade."),
            format!("O paddock ainda lê o caso como ferida recente, com reflexos que seguem vivos na sequência."),
            format!("O ambiente permanece sensível porque a história ainda não encontrou um desfecho esportivo claro."),
            format!("A leitura atual é de instabilidade: o dano existe, segue em aberto e cobra resposta imediata."),
            format!("A situação continua pendente e deixa {category} em estado de observação antes da próxima largada."),
            format!("O estado desta pauta ainda é de reparo e contenção, não de encerramento."),
            format!("A categoria absorve o caso com cautela porque seus efeitos seguem atravessando o paddock."),
            format!("O cenário ainda é de repercussão ativa, com recuperação e resposta convivendo na mesma pauta."),
        ],
    );
    [ocorrido, consequencia, estado]
}

fn build_piloto_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    // unwrap_or("") seguro: o guard garante que p1/p4 só são selecionados quando a posição é conhecida
    let position_phrase = position_pressure_phrase(extras.driver_position);
    let has_position_phrase = position_phrase.is_some();
    let momento = select_variant(
        variations[0],
        [
            format!("O centro desta leitura individual está em {title}, com {subject} no foco principal."),
            format!("O recorte individual mais importante desta semana pode ser lido em {title}."),
            format!("O momento do piloto é o centro da pauta: {title}."),
            format!("A reordenação da percepção do paddock em torno desse nome começa em {title}."),
            format!("O eixo claro da conversa desta semana passa por {title} e pelo momento de {subject}."),
            format!("O recorte individual começa em {title}, mas o assunto maior é a fase de {subject}."),
            format!("A leitura mais fina sobre {subject} nasce em {title}."),
            format!("Fica claro em {title} por que {subject} saiu da periferia da pauta."),
            format!("O nome em foco desta edição é {subject}, e {title} explica o motivo."),
            format!("{subject} volta ao centro do noticiário da categoria a partir de {title}."),
            format!("A leitura individual mais forte desta semana nasce em {title}."),
            format!("Depois de {title}, fica difícil tratar {subject} como nome lateral na narrativa atual."),
        ],
    );
    let pressao = select_valid_variant(
        variations[1],
        [
            format!("A pressão cresce porque {category} já não permite semanas neutras antes de {event_ref}."),
            format!(
                "Como {}, {subject} entra em {event_ref} sem espaço para uma rodada neutra.",
                position_phrase.as_deref().unwrap_or("")
            ),
            format!("{subject} chega a {event_ref} com margem de tolerância menor."),
            format!("Em {category}, a resposta pedida agora é proporcional à visibilidade recente desse nome."),
            format!(
                "{subject} entra em {event_ref} sob a pressão típica de quem chega hoje como {}.",
                position_phrase.as_deref().unwrap_or("")
            ),
            format!("{subject} entra em {event_ref} sob cobrança mais visível."),
            format!("A próxima largada chega com cobrança esportiva mais visível sobre {subject}."),
            format!("A fase atual comprime a margem e eleva o nível de exigência na sequência."),
            format!(
                "A condição de {} encurta a margem de {subject} antes de {event_ref}.",
                position_phrase.as_deref().map(|s| s.trim_start_matches("o ")).unwrap_or("")
            ),
            format!("A próxima etapa amplia a cobrança esportiva sobre {subject}."),
            format!("O grid passa a exigir de {subject} uma resposta coerente com o peso da pauta."),
            format!("A leitura desta semana encurta a margem de erro e amplia o estado de atenção no campeonato."),
        ],
        // p1, p4 e p8 usam position_phrase — só válidos quando a posição é conhecida
        |idx| !matches!(idx, 1 | 4 | 8) || has_position_phrase,
    );
    let sinal = select_variant(
        variations[2],
        [
            format!("O sinal mais claro está em como esse nome passa a concentrar parte maior da atenção do grid."),
            format!("A categoria passa a reorganizar a conversa ao redor deste piloto e espera impacto já na próxima resposta."),
            format!("O paddock trata este caso como indício de fase, não como aparição circunstancial antes de {event_ref}."),
            format!("O sinal final é que {subject} deixou de ser assunto lateral e entrou no bloco de nomes que explicam o momento da temporada."),
            format!("O tema central passa a ser se essa fase ganha corpo ou se perde força logo adiante."),
            format!("O grid lê este momento como indicador real de fase, e não como ruído passageiro."),
            format!("A conversa muda porque esse desempenho passou a representar mais do que um resultado isolado."),
            format!("O sinal mais forte é a mudança de status: agora a categoria olha para {subject} com outra régua."),
            format!("A leitura final aponta que este nome já influencia a percepção do momento competitivo."),
            format!("O caso deixa um indicativo claro de fase, pressão e expectativa acumulada."),
            format!("A semana sugere que {subject} entrou de vez na faixa de nomes que organizam a conversa."),
            format!("O recado editorial é claro: {subject} já não cabe mais como nota de rodapé nesta temporada."),
        ],
    );
    [momento, pressao, sinal]
}

fn build_equipe_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    let movimento = select_variant(
        variations[0],
        [
            format!("O movimento coletivo mais importante de {category} nesta edição pode ser lido em {title}."),
            format!("O movimento coletivo que colocou {subject} no centro da pauta está resumido em {title}."),
            format!("O box volta ao centro de um bloco que era mais individual semanas atrás a partir de {title}."),
            format!("A estrutura vira pauta e referência de movimento dentro de {category} em {title}."),
            format!("A equipe volta ao foco principal desta semana a partir de {title}."),
            format!("O recorte coletivo desta edição começa em {title}."),
            format!("A conversa sai do piloto e volta para o conjunto em {title}."),
            format!("Fica claro em {title} por que a leitura de equipe reapareceu com força em {category}."),
            format!("{subject} vai para o centro do debate sobre execução e estrutura a partir de {title}."),
            format!("O movimento principal do box nesta semana pode ser lido em {title}."),
            format!("O momento coletivo mais importante da categoria ganha rosto em {title}."),
            format!("A razão de a narrativa recente de {category} ter ficado menos individual está resumida em {title}."),
        ],
    );
    let resposta = select_variant(
        variations[1],
        [
            format!("{subject} precisa responder com consistência porque {event_ref} amplifica qualquer oscilação."),
            format!("A resposta esperada passa por manter ritmo, leitura de prova e controle operacional na sequência."),
            format!("{subject} deixa de ser pano de fundo e passa a ser parte do que define a rodada seguinte."),
            format!("{subject} entra em uma cobrança menos individual e mais ligada à execução de conjunto antes da próxima aparição."),
            format!("A próxima aparição já exige que {subject} devolva estabilidade e leitura mais limpa."),
            format!("A cobrança agora é coletiva: execução, ritmo e capacidade de reação do conjunto."),
            format!("{subject} passa a ser avaliada pela forma como sustenta o momento quando a pressão sobe."),
            format!("A resposta da equipe precisa aparecer em pista, estratégia e controle de corrida."),
            format!("O paddock já cobra de {subject} uma atuação coesa na próxima oportunidade."),
            format!("O foco se desloca para a capacidade de {subject} responder como estrutura, não só como destaque isolado."),
            format!("A etapa seguinte vira prova de consistência para {subject}."),
            format!("A expectativa agora é de controle operacional e repetição do nível coletivo apresentado."),
        ],
    );
    let panorama = select_variant(
        variations[2],
        [
            format!("O panorama é claro: o box entra mais forte na narrativa da rodada."),
            format!("Em {category}, o grid passa a medir com mais atenção a força coletiva e não só o brilho de um nome."),
            format!("A semana sugere que a estrutura da equipe vai pesar mais do que o normal na leitura competitiva."),
            format!("O paddock trata a pauta como sinal de que a engrenagem coletiva voltou a interferir diretamente no rumo da categoria."),
            format!("A categoria volta a olhar para o box como parte decisiva do próximo capítulo."),
            format!("O panorama aponta para uma rodada em que a estrutura pode valer tanto quanto o talento isolado."),
            format!("A leitura atual sugere que a execução coletiva seguirá pesando nas próximas semanas."),
            format!("O grid trata a força de conjunto como tema real desta fase do campeonato."),
            format!("A narrativa recente indica que o box retomou espaço na explicação da rodada."),
            format!("O panorama da semana mostra uma categoria mais atenta à solidez da equipe."),
            format!("A próxima etapa deve aprofundar a leitura sobre força coletiva e capacidade de entrega."),
            format!("O quadro geral deixa claro que a estrutura voltou a influenciar diretamente a história da temporada."),
        ],
    );
    [movimento, resposta, panorama]
}

fn build_mercado_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    let preseason_phrase = preseason_phase_phrase(extras.preseason_week);
    let presence_phrase = public_presence_phrase(extras.presence_tier);
    let movimento = select_variant(
        variations[0],
        [
            format!("O movimento de mercado que mais mexe com {category} agora está resumido em {title}."),
            format!("O movimento desta pauta tem ponto de partida em {title} e coloca {subject} em evidência."),
            format!("O recorte de mercado mais objetivo desta edição começa em {title}."),
            format!("O caso principal desta pauta pode ser lido assim: {title}."),
            format!("O mercado sai da periferia e vai para o centro da semana em {title}."),
            format!("Fica claro em {title} por que o paddock voltou a olhar para contratos e reposicionamentos."),
            format!("O ponto de partida da conversa mais quente de mercado neste momento está em {title}."),
            format!("O movimento mais relevante desta edição está condensado em {title}."),
            format!("A mudança de temperatura que hoje cerca o mercado da categoria aparece em {title}."),
            format!("A história de mercado desta semana ganha forma a partir de {title}."),
            format!("Bastidor vira notícia central dentro de {category} em {title}."),
            format!("O principal eixo de mercado que atravessa o paddock nesta fase pode ser lido em {title}."),
        ],
    );
    let impacto = select_variant(
        variations[1],
        [
            format!("{subject} altera a conversa competitiva e de imagem em um momento de atenção ampliada."),
            format!("O impacto já aparece no jeito como {category} redistribui atenção, interesse e pressão."),
            format!("{subject} passa a ocupar mais espaço fora da pista sem sair do centro esportivo da categoria."),
            format!("A hierarquia de interesse no paddock muda porque {subject} reorganiza expectativa e resposta no grid."),
            format!("O efeito imediato está na forma como o paddock reordena valor, atenção e projeção."),
            format!("O caso reposiciona {subject} e altera a leitura esportiva do ambiente."),
            format!("{subject} muda a lógica do interesse no grid e encurta a distância entre pista e bastidor."),
            format!("O paddock passa a tratar {subject} como tema competitivo e contratual ao mesmo tempo."),
            format!("O impacto ultrapassa o rumor e já afeta a forma como a categoria mede seus próximos passos."),
            format!("{subject} reorganiza a pauta da semana ao influenciar imagem, expectativa e pressão."),
            format!("A conversa de mercado muda de escala porque {subject} passa a interferir no debate esportivo."),
            format!("O caso deixa marcas imediatas no mapa de atenção que o grid distribui neste momento."),
        ],
    );
    let has_preseason = preseason_phrase.is_some();
    let has_presence = presence_phrase.is_some();
    let proximo_passo = select_valid_variant(
        variations[2],
        [
            format!("O próximo passo é medir se esse movimento chega à próxima etapa com consequência prática."),
            format!(
                "A movimentação acontece {}, o que muda o ritmo esperado de definição nesta fase.",
                preseason_phrase.unwrap_or("")
            ),
            format!("O recorte seguinte é observar quem reage primeiro a essa mudança antes de {event_ref}."),
            format!("A pauta continua aberta enquanto o grid tenta descobrir se o caso esfria, acelera ou muda de alvo."),
            format!("O próximo capítulo é entender se a conversa avança para definição ou volta ao campo da especulação."),
            format!("O paddock agora observa sinais concretos de resposta, não apenas novos comentários."),
            format!(
                "Para uma equipe {}, cada passo deste movimento altera o peso público e esportivo da pauta.",
                presence_phrase.unwrap_or("")
            ),
            format!("O tema segue aberto até que a categoria transforme rumor em consequência observável."),
            format!("O próximo passo é medir se a história atravessa a próxima rodada com mais força ou menos ruído."),
            format!("A leitura seguinte passa por entender se o mercado produz ação ou só prolonga expectativa."),
            format!("O caso só muda de patamar se encontrar resposta concreta nas próximas semanas."),
            format!("O paddock agora espera definição, reação ou mudança clara de alvo para esta conversa."),
        ],
        |idx| match idx {
            // pp1: fase de pré-temporada — só faz sentido quando a notícia tem semana de pré-temporada
            1 => has_preseason,
            // pp6: presença pública da equipe — só faz sentido quando a story é centrada em equipe
            6 => has_presence && extras.subject_is_team,
            _ => true,
        },
    );
    [movimento, impacto, proximo_passo]
}

fn build_estrutural_blocks_v2(
    variations: [usize; 3],
    title: &str,
    subject: &str,
    category: &str,
    extras: &EditorialExtras<'_>,
) -> [String; 3] {
    let event_ref = extras.event_label;
    let mudanca = select_variant(
        variations[0],
        [
            format!("A mudança estrutural mais clara deste trecho de {category} aparece em {title}."),
            format!("A mudança central desta pauta é resumida por {title}."),
            format!("O ponto de virada para a leitura mais ampla da categoria está em {title}."),
            format!("O que mudou de verdade nesta semana cabe em {title}."),
            format!("Fica claro em {title} por que a estrutura da categoria já não parece a mesma."),
            format!("A transformação mais relevante deste momento pode ser lida a partir de {title}."),
            format!("A frente estrutural desta edição se abre em {title}."),
            format!("Em {title}, fica visível como a hierarquia recente entrou em nova fase."),
            format!("O recorte estrutural mais forte da semana está condensado em {title}."),
            format!("A sensação de transição que já não cabe no fundo da pauta volta à categoria em {title}."),
            format!("A síntese da mudança de desenho que atravessa {category} está em {title}."),
            format!("A leitura da principal virada estrutural desta etapa do campeonato se organiza em {title}."),
        ],
    );
    let efeito = select_variant(
        variations[1],
        [
            format!("O efeito imediato cai sobre {subject}, que passa a ocupar outro lugar no desenho da temporada."),
            format!("O efeito em {category} não é pontual; ele reposiciona referência, expectativa e peso competitivo."),
            format!("{subject} vira referência inevitável para medir o tamanho real dessa reorganização."),
            format!("O efeito sobre {subject} extrapola uma rodada e mexe no mapa de força que a categoria vinha sustentando."),
            format!("A mudança altera o lugar de {subject} e força o paddock a rever o tamanho desta transição."),
            format!("O impacto estrutural reposiciona o mapa da categoria e mexe na régua usada até aqui."),
            format!("{subject} passa a servir de medida para entender a profundidade desta reorganização."),
            format!("A categoria precisa reler sua própria hierarquia depois do deslocamento gerado por esta mudança."),
            format!("O efeito mais forte está na maneira como a temporada passa a ser interpretada daqui em diante."),
            format!("A mudança redireciona expectativa, comparação e referência dentro de {category}."),
            format!("{subject} deixa de ocupar o mesmo papel e isso altera o mapa competitivo do campeonato."),
            format!("O efeito vai além da rodada porque modifica o desenho de força que parecia mais estável."),
        ],
    );
    let panorama = select_variant(
        variations[2],
        [
            format!("O panorama ganha peso porque a próxima etapa recebe uma categoria menos estática."),
            format!("A sequência indica que a reorganização vai seguir em pauta mesmo depois de {event_ref}."),
            format!("A temporada entra em fase nova porque a estrutura deixa de parecer fixa."),
            format!("O panorama final aponta uma categoria ainda em transição, com efeitos que devem atravessar as próximas semanas."),
            format!("A leitura geral aponta para continuidade da reorganização, não para um ajuste isolado."),
            format!("O campeonato passa a ser lido em outra chave porque a estrutura ganhou movimento real."),
            format!("O panorama desta fase é de transição aberta e impacto prolongado."),
            format!("A categoria segue em rearranjo, com reflexos que devem ultrapassar a próxima rodada."),
            format!("O paddock trata a mudança como parte de um ciclo novo, não como exceção curta."),
            format!("A estrutura deixa de parecer fixa e isso muda a forma de ler a temporada inteira."),
            format!("A sequência da temporada tende a ser atravessada por esta reorganização estrutural."),
            format!("O quadro geral ainda é de adaptação, com o campeonato absorvendo uma nova configuração."),
        ],
    );
    [mudanca, efeito, panorama]
}

#[cfg(test)]
mod tests {
    use super::{position_pressure_phrase, select_valid_variant};

    #[test]
    fn test_select_valid_variant_keeps_preferred_index_when_allowed() {
        let selected = select_valid_variant(
            1,
            ["zero".to_string(), "one".to_string(), "two".to_string()],
            |idx| idx != 0,
        );

        assert_eq!(selected, "one");
    }

    #[test]
    fn test_select_valid_variant_falls_forward_until_it_finds_allowed_variant() {
        let selected = select_valid_variant(
            0,
            ["zero".to_string(), "one".to_string(), "two".to_string()],
            |idx| idx == 2,
        );

        assert_eq!(selected, "two");
    }

    #[test]
    fn test_select_valid_variant_falls_back_to_preferred_index_when_none_are_allowed() {
        let selected = select_valid_variant(
            2,
            ["zero".to_string(), "one".to_string(), "two".to_string()],
            |_| false,
        );

        assert_eq!(selected, "two");
    }

    #[test]
    fn test_position_pressure_phrase_only_returns_editorial_ranges_that_read_well() {
        assert_eq!(
            position_pressure_phrase(Some(1)),
            Some("o líder do campeonato".to_string())
        );
        assert_eq!(position_pressure_phrase(Some(6)), None);
        assert_eq!(position_pressure_phrase(Some(11)), None);
        assert_eq!(position_pressure_phrase(Some(16)), None);
        assert_eq!(position_pressure_phrase(None), None);
    }
}
