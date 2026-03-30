use rand::Rng;

use crate::constants::categories::get_category_config;
use crate::market::proposals::{MarketProposal, ProposalStatus, Vacancy};
use crate::market::visibility::{
    derive_market_visibility_profile, MarketVisibilityProfile, MarketVisibilityTier,
};
use crate::models::driver::Driver;

#[derive(Debug, Clone)]
pub struct AvailableDriver {
    pub driver: Driver,
    pub visibility: f64,
    pub posicao_campeonato: i32,
    pub categoria_atual: String,
    pub category_tier: u8,
    /// Nível máximo de licença que o piloto possui. None = sem nenhuma licença.
    pub max_license_level: Option<u8>,
}

pub fn generate_team_proposals(
    vacancy: &Vacancy,
    available_drivers: &[AvailableDriver],
    current_season: i32,
    rng: &mut impl Rng,
) -> Vec<MarketProposal> {
    let required_license =
        get_category_config(&vacancy.categoria).and_then(|config| config.licenca_necessaria);

    let mut candidates: Vec<&AvailableDriver> = available_drivers
        .iter()
        .filter(|available| {
            let license_ok = match required_license {
                None => true,
                Some(required) => available
                    .max_license_level
                    .map_or(false, |level| level >= required),
            };
            available.visibility >= 4.0
                && available.driver.status.as_str() != "Aposentado"
                && !available.driver.is_jogador
                && available.category_tier.abs_diff(vacancy.category_tier) <= 1
                && license_ok
        })
        .collect();

    candidates.sort_by(|a, b| {
        // 1º: mérito esportivo principal + bônus público marginal
        // 2º: prioridade coarse por tier de visibilidade pública
        // 3º: refinamento fino dentro do mesmo tier (marketability_bias contínuo)
        let pa = derive_market_visibility_profile(a.driver.atributos.midia);
        let pb = derive_market_visibility_profile(b.driver.atributos.midia);
        candidate_score(b)
            .total_cmp(&candidate_score(a))
            .then_with(|| shortlist_public_priority(&pb).total_cmp(&shortlist_public_priority(&pa)))
            .then_with(|| proposal_attention_weight(&pb).total_cmp(&proposal_attention_weight(&pa)))
    });

    candidates
        .into_iter()
        .take(3)
        .map(|candidate| MarketProposal {
            id: format!(
                "TMP-{}-{}-{}",
                vacancy.team_id, candidate.driver.id, current_season
            ),
            equipe_id: vacancy.team_id.clone(),
            equipe_nome: vacancy.team_name.clone(),
            piloto_id: candidate.driver.id.clone(),
            piloto_nome: candidate.driver.nome.clone(),
            categoria: vacancy.categoria.clone(),
            papel: vacancy.papel_necessario.clone(),
            salario_oferecido: calculate_offer_salary(vacancy, &candidate.driver, rng),
            duracao_anos: match vacancy.category_tier {
                0..=1 => rng.gen_range(1..=2),
                2..=3 => rng.gen_range(1..=3),
                _ => rng.gen_range(2..=3),
            },
            status: ProposalStatus::Pendente,
            motivo_recusa: None,
        })
        .collect()
}

/// Bônus de apelo público de mercado para score de candidatos.
///
/// Política operacional local de seleção — traduz MarketVisibilityTier em
/// um diferencial de score pequeno e auditável. Máximo: 0.12 (vs. componente
/// de skill que pode chegar a 40.0). Fator de desempate marginal, não medida
/// geral de valor do piloto.
///
/// Nesta v1, o bônus depende apenas do tier (não da posição fina dentro da
/// faixa), o que o torna previsível e semanticamente controlado.
fn market_public_visibility_bonus(profile: &MarketVisibilityProfile) -> f64 {
    match profile.tier {
        MarketVisibilityTier::Baixa => 0.0,
        MarketVisibilityTier::Relevante => 0.03,
        MarketVisibilityTier::Alta => 0.07,
        MarketVisibilityTier::Elite => 0.12,
    }
}

/// Prioridade pública secundária para ordenação da shortlist.
///
/// Critério de desempate na sort da shortlist — não é score principal.
/// Só tem efeito quando `candidate_score()` empata. Escala deliberadamente
/// menor que `market_public_visibility_bonus()` (máx 0.06 vs. máx 0.12)
/// para evitar dupla amplificação de mídia.
///
/// Semântica: entre candidatos esportivamente equivalentes, o de maior
/// apelo público sobe levemente na shortlist. Não desbloqueio — apenas ordenação.
fn shortlist_public_priority(profile: &MarketVisibilityProfile) -> f64 {
    match profile.tier {
        MarketVisibilityTier::Baixa => 0.0,
        MarketVisibilityTier::Relevante => 0.01,
        MarketVisibilityTier::Alta => 0.03,
        MarketVisibilityTier::Elite => 0.06,
    }
}

/// Peso de atenção pública para priorização final de proposta.
///
/// Critério terciário de ordenação — só atua quando `candidate_score()` e
/// `shortlist_public_priority()` já empataram (mesmo tier de visibilidade).
/// Usa `marketability_bias` (contínuo 0.0–1.0) para discriminação fina
/// dentro do mesmo tier.
///
/// Dois pilotos Elite com midia=85 e midia=99 são indistinguíveis pelo
/// tier-coarse de `shortlist_public_priority`. Este critério diferencia:
/// o de maior apelo público bruto é levemente preferido na proposta.
///
/// Distinto das camadas anteriores: não é um bônus de score nem uma
/// prioridade tier-coarse — é refinamento contínuo dentro do tier.
fn proposal_attention_weight(profile: &MarketVisibilityProfile) -> f64 {
    profile.marketability_bias
}

fn candidate_score(available: &AvailableDriver) -> f64 {
    let age_bonus = if available.driver.idade < 24 {
        80.0
    } else if available.driver.idade <= 30 {
        100.0
    } else {
        50.0
    };

    // Apelo público de mercado — bônus deliberadamente pequeno.
    // `calculate_visibility()` mede visibilidade esportiva (posição/vitórias/títulos).
    // Este bônus mede apelo público persistente (midia). São complementares, não equivalentes.
    let media_profile = derive_market_visibility_profile(available.driver.atributos.midia);
    let media_bonus = market_public_visibility_bonus(&media_profile);

    available.driver.atributos.skill * 0.4
        + available.driver.atributos.consistencia * 0.2
        + (available.visibility * 10.0) * 0.2
        + age_bonus * 0.2
        + media_bonus
}

fn calculate_offer_salary(vacancy: &Vacancy, driver: &Driver, rng: &mut impl Rng) -> f64 {
    let tier_base = match vacancy.category_tier {
        0 => 10_000.0,
        1 => 25_000.0,
        2 => 50_000.0,
        3 => 100_000.0,
        4 => 200_000.0,
        _ => 150_000.0,
    };

    let skill_modifier = driver.atributos.skill / 70.0;
    let budget_modifier = (vacancy.budget / 70.0).min(1.5);
    let variance = rng.gen_range(0.85..=1.15);

    (tier_base * skill_modifier * budget_modifier * variance).max(5_000.0)
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::models::driver::Driver;
    use crate::models::enums::TeamRole;

    #[test]
    fn test_generate_proposals_for_vacancy() {
        let vacancy = sample_vacancy(3);
        let available = vec![
            sample_available_driver("P001", "gt4", 3, 6.5, 72.0),
            sample_available_driver("P002", "bmw_m2", 2, 5.5, 68.0),
            sample_available_driver("P003", "gt3", 4, 7.0, 74.0),
        ];
        let mut rng = StdRng::seed_from_u64(1);

        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert!(!proposals.is_empty());
        assert!(proposals.len() <= 3);
    }

    #[test]
    fn test_proposals_respect_tier_limit() {
        let vacancy = sample_vacancy(2);
        let available = vec![
            sample_available_driver("P001", "mazda_amador", 1, 6.0, 65.0),
            sample_available_driver("P002", "endurance", 5, 8.0, 82.0),
        ];
        let mut rng = StdRng::seed_from_u64(2);

        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert!(proposals
            .iter()
            .all(|proposal| proposal.piloto_id != "P002"));
    }

    #[test]
    fn test_proposals_salary_scales_with_tier() {
        let low = sample_vacancy(2);
        let high = sample_vacancy(3);
        let available = vec![sample_available_driver("P001", "gt4", 3, 7.0, 72.0)];
        let mut rng_low = StdRng::seed_from_u64(3);
        let mut rng_high = StdRng::seed_from_u64(3);

        let low_offer = generate_team_proposals(&low, &available, 2, &mut rng_low);
        let high_offer = generate_team_proposals(&high, &available, 2, &mut rng_high);

        assert!(high_offer[0].salario_oferecido > low_offer[0].salario_oferecido);
    }

    fn sample_vacancy(tier: u8) -> Vacancy {
        Vacancy {
            team_id: "T001".to_string(),
            team_name: "Equipe".to_string(),
            categoria: "gt4".to_string(),
            category_tier: tier,
            car_performance: 8.0,
            budget: 75.0,
            reputacao: 70.0,
            papel_necessario: TeamRole::Numero1,
            piloto_existente_id: None,
        }
    }

    fn sample_available_driver(
        id: &str,
        category: &str,
        tier: u8,
        visibility: f64,
        skill: f64,
    ) -> AvailableDriver {
        let mut driver = Driver::new(
            id.to_string(),
            format!("Piloto {id}"),
            "Brasil".to_string(),
            "M".to_string(),
            24,
            2020,
        );
        driver.atributos.skill = skill;
        driver.atributos.consistencia = 65.0;
        AvailableDriver {
            driver,
            visibility,
            posicao_campeonato: 3,
            categoria_atual: category.to_string(),
            category_tier: tier,
            // nível alto para não bloquear testes que não testam licença
            max_license_level: Some(10),
        }
    }

    // ── Testes de shortlist_public_priority ──────────────────────────────────

    #[test]
    fn test_shortlist_priority_breaks_tie_by_media() {
        // Mesmo score esportivo, mídia diferente → mais midiático vem primeiro
        let vacancy = sample_vacancy(3);
        let mut low_media = sample_available_driver("P001", "gt4", 3, 6.5, 70.0);
        low_media.driver.atributos.midia = 10.0; // Baixa

        let mut high_media = sample_available_driver("P002", "gt4", 3, 6.5, 70.0);
        high_media.driver.atributos.midia = 90.0; // Elite

        let available = vec![low_media, high_media];
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert_eq!(proposals[0].piloto_id, "P002");
    }

    #[test]
    fn test_shortlist_priority_does_not_override_sporting_merit() {
        // Candidato A: esportivamente superior, mídia mínima
        // Candidato B: esportivamente inferior, mídia máxima
        // A deve vir primeiro independente da ordem de entrada
        let vacancy = sample_vacancy(3);
        let mut sporting_better = sample_available_driver("P001", "gt4", 3, 6.5, 75.0);
        sporting_better.driver.atributos.midia = 5.0; // Baixa

        let mut media_better = sample_available_driver("P002", "gt4", 3, 6.5, 65.0);
        media_better.driver.atributos.midia = 100.0; // Elite

        // ordem invertida na entrada para não depender de ordenação inicial
        let available = vec![media_better, sporting_better];
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert_eq!(proposals[0].piloto_id, "P001");
    }

    #[test]
    fn test_ineligible_driver_stays_out_despite_elite_media() {
        // Piloto com visibility < 4.0 não deve entrar, mesmo com mídia Elite
        let vacancy = sample_vacancy(3);
        let mut ineligible = sample_available_driver("P001", "gt4", 3, 3.5, 70.0);
        ineligible.driver.atributos.midia = 100.0; // Elite — não deve desbloquear

        let available = vec![ineligible];
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert!(proposals.is_empty());
    }

    #[test]
    fn test_shortlist_priority_monotonic() {
        let profile_baixa = derive_market_visibility_profile(10.0);
        let profile_relevante = derive_market_visibility_profile(40.0);
        let profile_alta = derive_market_visibility_profile(70.0);
        let profile_elite = derive_market_visibility_profile(90.0);

        assert!(
            shortlist_public_priority(&profile_baixa)
                < shortlist_public_priority(&profile_relevante)
        );
        assert!(
            shortlist_public_priority(&profile_relevante)
                < shortlist_public_priority(&profile_alta)
        );
        assert!(
            shortlist_public_priority(&profile_alta) < shortlist_public_priority(&profile_elite)
        );
    }

    // ── Testes de proposal_attention_weight ──────────────────────────────────

    #[test]
    fn test_proposal_attention_breaks_same_tier_tie() {
        // Dois Elite, mesmo candidate_score esportivo, midia raw diferente dentro do tier.
        // shortlist_public_priority empata (ambos Elite → 0.06).
        // proposal_attention_weight decide: bias=0.99 > bias=0.85.
        let vacancy = sample_vacancy(3);
        let mut low_raw = sample_available_driver("P001", "gt4", 3, 6.5, 70.0);
        low_raw.driver.atributos.midia = 85.0; // Elite, bias=0.85

        let mut high_raw = sample_available_driver("P002", "gt4", 3, 6.5, 70.0);
        high_raw.driver.atributos.midia = 99.0; // Elite, bias=0.99

        let available = vec![low_raw, high_raw];
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert_eq!(proposals[0].piloto_id, "P002");
    }

    #[test]
    fn test_proposal_attention_does_not_override_sporting_merit() {
        // Candidato A: esportivamente superior, midia mínima.
        // Candidato B: esportivamente inferior, midia máxima.
        // candidate_score decide no 1º critério — mérito esportivo domina.
        let vacancy = sample_vacancy(3);
        let mut sporting_better = sample_available_driver("P001", "gt4", 3, 6.5, 75.0);
        sporting_better.driver.atributos.midia = 5.0; // Baixa

        let mut media_better = sample_available_driver("P002", "gt4", 3, 6.5, 65.0);
        media_better.driver.atributos.midia = 100.0; // Elite máximo

        let available = vec![media_better, sporting_better];
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert_eq!(proposals[0].piloto_id, "P001");
    }

    #[test]
    fn test_proposal_attention_does_not_override_tier_priority() {
        // Candidato A: tier Alta (midia=80, shortlist_priority=0.03).
        // Candidato B: tier Relevante (midia=58, shortlist_priority=0.01, bias=0.58).
        // Mesmo candidate_score → 2º critério (tier) decide.
        // bias alto de B (0.58) não supera tier superior de A — 3º critério não chega a ser chamado.
        let vacancy = sample_vacancy(3);
        let mut tier_alta = sample_available_driver("P001", "gt4", 3, 6.5, 70.0);
        tier_alta.driver.atributos.midia = 80.0; // Alta

        let mut tier_relevante = sample_available_driver("P002", "gt4", 3, 6.5, 70.0);
        tier_relevante.driver.atributos.midia = 58.0; // Relevante

        let available = vec![tier_relevante, tier_alta]; // ordem invertida
        let mut rng = StdRng::seed_from_u64(42);
        let proposals = generate_team_proposals(&vacancy, &available, 2, &mut rng);

        assert_eq!(proposals[0].piloto_id, "P001");
    }

    #[test]
    fn test_proposal_attention_weight_monotonic() {
        let p0 = derive_market_visibility_profile(0.0);
        let p50 = derive_market_visibility_profile(50.0);
        let p85 = derive_market_visibility_profile(85.0);
        let p99 = derive_market_visibility_profile(99.0);

        assert!(proposal_attention_weight(&p0) < proposal_attention_weight(&p50));
        assert!(proposal_attention_weight(&p50) < proposal_attention_weight(&p85));
        assert!(proposal_attention_weight(&p85) < proposal_attention_weight(&p99));
    }

    // ── Testes semânticos de media_bonus em candidate_score ───────────────────

    #[test]
    fn test_media_bonus_breaks_tie_between_similar_candidates() {
        // Dois candidatos esportivamente idênticos, midia diferente →
        // o mais midiático deve ter score maior.
        let mut low_media = sample_available_driver("P001", "gt4", 3, 6.5, 65.0);
        low_media.driver.atributos.midia = 10.0; // Baixa → bônus 0.0

        let mut high_media = sample_available_driver("P002", "gt4", 3, 6.5, 65.0);
        high_media.driver.atributos.midia = 90.0; // Elite → bônus 0.12

        assert!(candidate_score(&high_media) > candidate_score(&low_media));
    }

    #[test]
    fn test_media_bonus_same_tier_same_bonus() {
        // Nesta v1, bônus depende apenas do tier, não da posição fina dentro da faixa.
        // Dois pilotos no mesmo tier Elite com midia diferente → bônus idêntico.
        let mut a = sample_available_driver("P001", "gt4", 3, 6.5, 65.0);
        a.driver.atributos.midia = 85.0; // Elite (limite inferior)

        let mut b = sample_available_driver("P002", "gt4", 3, 6.5, 65.0);
        b.driver.atributos.midia = 99.0; // Elite (quase máximo)

        let diff = (candidate_score(&a) - candidate_score(&b)).abs();
        assert!(
            diff < 1e-9,
            "pilotos no mesmo tier devem ter bônus idêntico: diff={diff}"
        );
    }

    #[test]
    fn test_media_bonus_does_not_override_sporting_merit() {
        // Candidato A: skill moderadamente superior, midia mínima.
        let mut sporting_better = sample_available_driver("P001", "gt4", 3, 6.5, 72.0);
        sporting_better.driver.atributos.midia = 5.0; // Baixa → 0.0

        // Candidato B: skill inferior, midia máxima.
        let mut media_better = sample_available_driver("P002", "gt4", 3, 6.5, 64.0);
        media_better.driver.atributos.midia = 100.0; // Elite → 0.12

        // Diferença de skill (72-64)*0.4 = 3.2 >> bônus máximo (0.12).
        // Diferença moderada mas clara deve superar o bônus máximo de mídia.
        assert!(
            candidate_score(&sporting_better) > candidate_score(&media_better),
            "diferença esportiva moderada deve superar bônus máximo de mídia"
        );
    }
}
