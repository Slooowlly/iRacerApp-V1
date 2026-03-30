use crate::event_interest::InterestTier;
use crate::models::enums::ThematicSlot;
use crate::news::NewsImportance;

// ── Tipos públicos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SeasonalFramingKind {
    SeasonHeatUp,
    SpotlightDriver,
}

#[derive(Debug, Clone)]
pub struct SeasonalFramingSignal {
    pub kind: SeasonalFramingKind,
    pub importance: NewsImportance,
    pub title: String,
    pub body: String,
    pub driver_id: Option<String>,
    pub driver_name: Option<String>,
}

// ── Função principal ────────────────────────────────────────────────────────

/// Tenta gerar um único item de framing sazonal para o trigger de pós-corrida.
///
/// Regras:
/// - slot deve ser "quente" (abertura, midpoint de prestígio, tensão, finais)
/// - interest_tier deve ser Alto, MuitoAlto ou EventoPrincipal
/// - se houver vencedor com mídia ≥ 60 → SpotlightDriver (lastro = vitória real)
/// - senão → SeasonHeatUp
/// - nunca emite mais de 1 item; nunca chega a Destaque
///
/// `winner_id` e `winner_name` vêm do race_result já resolvido no call site.
pub fn try_generate_seasonal_framing(
    thematic_slot: &ThematicSlot,
    interest_tier: &InterestTier,
    winner_id: Option<&str>,
    winner_name: Option<&str>,
    winner_media: Option<f64>,
) -> Option<SeasonalFramingSignal> {
    if !is_hot_slot(thematic_slot) {
        return None;
    }
    if !is_high_interest(interest_tier) {
        return None;
    }

    let has_spotlight = winner_media.map_or(false, |m| m >= 60.0) && winner_id.is_some();

    if has_spotlight {
        let id = winner_id.unwrap().to_string();
        let name = winner_name.unwrap_or("O vencedor");
        let importance = if winner_media.unwrap_or(0.0) >= 85.0 {
            NewsImportance::Alta
        } else {
            NewsImportance::Media
        };
        Some(SeasonalFramingSignal {
            kind: SeasonalFramingKind::SpotlightDriver,
            importance,
            title: format!("{} assume o centro da narrativa", name),
            body: format!(
                "{} vence e consolida posição de destaque público nesta fase da temporada.",
                name
            ),
            driver_id: Some(id),
            driver_name: Some(name.to_string()),
        })
    } else {
        let (title, body) = heatup_texts(thematic_slot);
        Some(SeasonalFramingSignal {
            kind: SeasonalFramingKind::SeasonHeatUp,
            importance: NewsImportance::Alta,
            title: title.to_string(),
            body: body.to_string(),
            driver_id: None,
            driver_name: None,
        })
    }
}

// ── Helpers privados ────────────────────────────────────────────────────────

fn is_hot_slot(slot: &ThematicSlot) -> bool {
    matches!(
        slot,
        ThematicSlot::AberturaDaTemporada
            | ThematicSlot::MidpointPrestigio
            | ThematicSlot::TensaoPreFinal
            | ThematicSlot::FinalDaTemporada
            | ThematicSlot::FinalEspecial
            | ThematicSlot::AberturaEspecial
    )
}

fn is_high_interest(tier: &InterestTier) -> bool {
    matches!(
        tier,
        InterestTier::Alto | InterestTier::MuitoAlto | InterestTier::EventoPrincipal
    )
}

fn heatup_texts(slot: &ThematicSlot) -> (&'static str, &'static str) {
    match slot {
        ThematicSlot::AberturaDaTemporada => (
            "Temporada arranca com holofotes máximos",
            "A abertura da temporada concentra a atenção do paddock e da mídia especializada.",
        ),
        ThematicSlot::MidpointPrestigio => (
            "Temporada entra em fase decisiva",
            "A corrida de meio de temporada marca a virada narrativa do campeonato.",
        ),
        ThematicSlot::TensaoPreFinal => (
            "Tensão antes da grande decisão",
            "O paddock vive expectativa máxima às vésperas da rodada final.",
        ),
        ThematicSlot::FinalDaTemporada => (
            "Grande Final concentra os olhares do paddock",
            "A corrida decisiva da temporada atrai atenção máxima de pilotos, equipes e imprensa.",
        ),
        ThematicSlot::FinalEspecial => (
            "Encerramento especial sob holofotes do público",
            "O bloco especial chega ao fim com atenção redobrada sobre o campeonato.",
        ),
        ThematicSlot::AberturaEspecial => (
            "Bloco especial começa sob atenção total",
            "O início do bloco especial traz renovado interesse público para a temporada.",
        ),
        _ => (
            "Temporada vive momento de alta atenção",
            "O campeonato entra em fase de grande interesse público.",
        ),
    }
}

// ── Testes ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heatup_hot_slot_high_interest_no_media_driver() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::FinalDaTemporada,
            &InterestTier::Alto,
            None,
            None,
            None,
        );
        let signal = result.expect("deve gerar framing");
        assert_eq!(signal.kind, SeasonalFramingKind::SeasonHeatUp);
        assert_eq!(signal.importance, NewsImportance::Alta);
        assert!(signal.driver_id.is_none());
    }

    #[test]
    fn test_spotlight_driver_hot_slot_high_interest_high_media_winner() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::FinalDaTemporada,
            &InterestTier::EventoPrincipal,
            Some("driver-42"),
            Some("Max Verstappen"),
            Some(90.0),
        );
        let signal = result.expect("deve gerar framing");
        assert_eq!(signal.kind, SeasonalFramingKind::SpotlightDriver);
        assert_eq!(signal.importance, NewsImportance::Alta);
        assert_eq!(signal.driver_id.as_deref(), Some("driver-42"));
    }

    #[test]
    fn test_spotlight_driver_media_tier_media_when_below_85() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::MidpointPrestigio,
            &InterestTier::MuitoAlto,
            Some("driver-7"),
            Some("Piloto X"),
            Some(70.0),
        );
        let signal = result.expect("deve gerar framing");
        assert_eq!(signal.kind, SeasonalFramingKind::SpotlightDriver);
        assert_eq!(signal.importance, NewsImportance::Media);
    }

    #[test]
    fn test_no_framing_regular_slot_high_interest() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::RodadaRegular,
            &InterestTier::EventoPrincipal,
            Some("driver-1"),
            Some("Piloto Y"),
            Some(95.0),
        );
        assert!(result.is_none(), "slot regular não deve gerar framing");
    }

    #[test]
    fn test_no_framing_hot_slot_low_interest() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::FinalDaTemporada,
            &InterestTier::Baixo,
            Some("driver-1"),
            Some("Piloto Y"),
            Some(95.0),
        );
        assert!(result.is_none(), "interesse baixo não deve gerar framing");
    }

    #[test]
    fn test_no_framing_hot_slot_moderate_interest() {
        let result = try_generate_seasonal_framing(
            &ThematicSlot::TensaoPreFinal,
            &InterestTier::Moderado,
            Some("driver-1"),
            Some("Piloto Y"),
            Some(95.0),
        );
        assert!(
            result.is_none(),
            "interesse moderado não deve gerar framing"
        );
    }

    #[test]
    fn test_importance_never_destaque() {
        for (slot, tier, media) in [
            (
                ThematicSlot::FinalDaTemporada,
                InterestTier::EventoPrincipal,
                Some(100.0_f64),
            ),
            (ThematicSlot::AberturaDaTemporada, InterestTier::Alto, None),
        ] {
            if let Some(signal) = try_generate_seasonal_framing(
                &slot,
                &tier,
                Some("driver-x"),
                Some("Piloto Z"),
                media,
            ) {
                assert!(
                    signal.importance < NewsImportance::Destaque,
                    "framing nunca deve atingir Destaque"
                );
            }
        }
    }

    #[test]
    fn test_heatup_when_winner_media_below_threshold() {
        // vencedor com mídia 40 não deve virar spotlight — deve cair em heat-up
        let result = try_generate_seasonal_framing(
            &ThematicSlot::FinalEspecial,
            &InterestTier::MuitoAlto,
            Some("driver-low"),
            Some("Piloto Baixa Mídia"),
            Some(40.0),
        );
        let signal = result.expect("deve gerar framing");
        assert_eq!(signal.kind, SeasonalFramingKind::SeasonHeatUp);
    }
}
