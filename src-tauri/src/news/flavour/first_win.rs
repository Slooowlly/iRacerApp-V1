use crate::news::flavour::pick_index_with_discriminator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstWinContext {
    pub is_career: bool,
    pub is_category: bool,
    pub is_with_team: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstWinNarrative {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstWinLabel {
    CareerOnly,
    CategoryOnly,
    TeamOnly,
    CareerCategory,
    CareerTeam,
    CategoryTeam,
    Complete,
}

/// Builds a single deterministic narrative for all first-win combinations.
pub fn build_first_win_narrative(
    ctx: &FirstWinContext,
    pilot_name: &str,
    team_name: &str,
    category_name: &str,
    track_name: &str,
    grid_position: i32,
    seed: &str,
) -> Option<FirstWinNarrative> {
    let label = classify_first_win(ctx)?;
    let achievements = build_achievement_list(ctx, team_name, category_name);
    let achievements_text = join_with_e(&achievements);
    let variant = pick_variant(seed, label, 3);

    let title = match (label, variant) {
        (FirstWinLabel::CareerOnly, 0) => {
            format!("Primeira vitoria! {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CareerOnly, 1) => {
            format!("{pilot_name} conquista a primeira da carreira em {track_name}")
        }
        (FirstWinLabel::CareerOnly, _) => {
            format!("Primeira na carreira: {pilot_name} triunfa em {track_name}")
        }
        (FirstWinLabel::CategoryOnly, 0) => {
            format!("{pilot_name} conquista primeira vitoria na {category_name} em {track_name}")
        }
        (FirstWinLabel::CategoryOnly, 1) => {
            format!("Primeira na {category_name}: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CategoryOnly, _) => {
            format!("Primeira na {category_name}: {pilot_name} abre a conta em {track_name}")
        }
        (FirstWinLabel::TeamOnly, 0) => {
            format!("{pilot_name} conquista primeira vitoria pela {team_name} em {track_name}")
        }
        (FirstWinLabel::TeamOnly, 1) => {
            format!("Primeira pela {team_name}: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::TeamOnly, _) => {
            format!("Primeira pela {team_name}: {pilot_name} estreia no topo em {track_name}")
        }
        (FirstWinLabel::CareerCategory, 0) => {
            format!("Primeira vitoria dupla: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CareerCategory, 1) => format!(
            "{pilot_name} conquista a primeira da carreira e da {category_name} em {track_name}"
        ),
        (FirstWinLabel::CareerCategory, _) => {
            format!("Primeira na carreira e na {category_name}: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CareerTeam, 0) => {
            format!("Primeira vitoria com impacto imediato: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CareerTeam, 1) => format!(
            "{pilot_name} conquista a primeira da carreira e da {team_name} em {track_name}"
        ),
        (FirstWinLabel::CareerTeam, _) => {
            format!("Primeira na carreira e pela {team_name}: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::CategoryTeam, 0) => {
            format!(
                "Primeira na {category_name} e pela {team_name}: {pilot_name} estreia em destaque"
            )
        }
        (FirstWinLabel::CategoryTeam, 1) => format!(
            "Primeira na {category_name} e pela {team_name}: {pilot_name} vence em {track_name}"
        ),
        (FirstWinLabel::CategoryTeam, _) => format!(
            "Primeira na {category_name} e pela {team_name}: {pilot_name} vence em {track_name}"
        ),
        (FirstWinLabel::Complete, 0) => {
            format!("Primeira vitoria completa: {pilot_name} vence em {track_name}")
        }
        (FirstWinLabel::Complete, 1) => format!(
            "Primeira completa: {pilot_name} junta carreira, categoria e equipe em {track_name}"
        ),
        (FirstWinLabel::Complete, _) => {
            format!("Primeira completa: {pilot_name} conquista a tripla estreia em {track_name}")
        }
    };

    let body = match variant {
        0 => format!(
            "{pilot_name} conquistou {achievements_text} e venceu em {track_name}, largando de P{grid_position}."
        ),
        1 => format!(
            "{pilot_name} venceu em {track_name}, largando de P{grid_position}, e marcou {achievements_text}."
        ),
        _ => format!(
            "{pilot_name} venceu em {track_name}, largando de P{grid_position}, e celebrou {achievements_text}."
        ),
    };

    Some(FirstWinNarrative { title, body })
}

fn classify_first_win(ctx: &FirstWinContext) -> Option<FirstWinLabel> {
    match (ctx.is_career, ctx.is_category, ctx.is_with_team) {
        (false, false, false) => None,
        (true, false, false) => Some(FirstWinLabel::CareerOnly),
        (false, true, false) => Some(FirstWinLabel::CategoryOnly),
        (false, false, true) => Some(FirstWinLabel::TeamOnly),
        (true, true, false) => Some(FirstWinLabel::CareerCategory),
        (true, false, true) => Some(FirstWinLabel::CareerTeam),
        (false, true, true) => Some(FirstWinLabel::CategoryTeam),
        (true, true, true) => Some(FirstWinLabel::Complete),
    }
}

fn build_achievement_list(
    ctx: &FirstWinContext,
    team_name: &str,
    category_name: &str,
) -> Vec<String> {
    let mut achievements = Vec::with_capacity(3);

    if ctx.is_career {
        achievements.push("sua primeira vitoria na carreira".to_string());
    }
    if ctx.is_category {
        achievements.push(format!("sua primeira vitoria na {category_name}"));
    }
    if ctx.is_with_team {
        achievements.push(format!("sua primeira vitoria pela {team_name}"));
    }

    achievements
}

fn join_with_e(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} e {second}"),
        [first, second, third] => format!("{first}, {second} e {third}"),
        _ => {
            let last_index = parts.len() - 1;
            format!("{} e {}", parts[..last_index].join(", "), parts[last_index])
        }
    }
}

fn pick_variant(seed: &str, label: FirstWinLabel, variants: usize) -> usize {
    let discriminator = format!("{label:?}");
    pick_index_with_discriminator(variants, seed, &discriminator)
}

#[cfg(test)]
mod tests {
    use super::{build_first_win_narrative, FirstWinContext};

    #[test]
    fn test_build_first_win_narrative_returns_none_without_flags() {
        let narrative = build_first_win_narrative(
            &FirstWinContext {
                is_career: false,
                is_category: false,
                is_with_team: false,
            },
            "Piloto X",
            "Equipe Y",
            "GT4",
            "Interlagos",
            3,
            "seed-none",
        );

        assert!(narrative.is_none());
    }

    #[test]
    fn test_build_first_win_narrative_covers_all_seven_combinations() {
        let cases = [
            (
                "career_only",
                FirstWinContext {
                    is_career: true,
                    is_category: false,
                    is_with_team: false,
                },
                &[
                    "primeira vitoria na carreira",
                    "venceu em interlagos",
                    "largando de p3",
                ][..],
            ),
            (
                "category_only",
                FirstWinContext {
                    is_career: false,
                    is_category: true,
                    is_with_team: false,
                },
                &[
                    "primeira vitoria na gt4",
                    "venceu em interlagos",
                    "largando de p3",
                ][..],
            ),
            (
                "team_only",
                FirstWinContext {
                    is_career: false,
                    is_category: false,
                    is_with_team: true,
                },
                &[
                    "primeira vitoria pela equipe y",
                    "venceu em interlagos",
                    "largando de p3",
                ][..],
            ),
            (
                "career_category",
                FirstWinContext {
                    is_career: true,
                    is_category: true,
                    is_with_team: false,
                },
                &[
                    "primeira vitoria na carreira",
                    "primeira vitoria na gt4",
                    "venceu em interlagos",
                ][..],
            ),
            (
                "career_team",
                FirstWinContext {
                    is_career: true,
                    is_category: false,
                    is_with_team: true,
                },
                &[
                    "primeira vitoria na carreira",
                    "primeira vitoria pela equipe y",
                    "venceu em interlagos",
                ][..],
            ),
            (
                "category_team",
                FirstWinContext {
                    is_career: false,
                    is_category: true,
                    is_with_team: true,
                },
                &[
                    "primeira vitoria na gt4",
                    "primeira vitoria pela equipe y",
                    "venceu em interlagos",
                ][..],
            ),
            (
                "complete",
                FirstWinContext {
                    is_career: true,
                    is_category: true,
                    is_with_team: true,
                },
                &[
                    "primeira vitoria na carreira",
                    "primeira vitoria na gt4",
                    "primeira vitoria pela equipe y",
                ][..],
            ),
        ];

        for (seed, ctx, expected_fragments) in cases {
            let narrative = build_first_win_narrative(
                &ctx,
                "Piloto X",
                "Equipe Y",
                "GT4",
                "Interlagos",
                3,
                seed,
            )
            .unwrap_or_else(|| panic!("expected Some narrative for {seed}"));

            let body = narrative.body.to_lowercase();
            for fragment in expected_fragments {
                assert!(
                    body.contains(fragment),
                    "body for {seed} should contain '{fragment}', got: {}",
                    narrative.body
                );
            }

            assert!(
                narrative.title.to_lowercase().contains("primeira"),
                "title for {seed} should mention first win, got: {}",
                narrative.title
            );
        }
    }
}
