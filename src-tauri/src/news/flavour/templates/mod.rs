pub mod end_of_season;
pub mod incidents;
pub mod injury;
pub mod market;
pub mod race;
pub mod rivalry;
pub mod winner;

#[cfg(test)]
mod tests {
    use super::{end_of_season, incidents, injury, market, race, rivalry};

    #[test]
    fn test_catalog_includes_latest_race_templates() {
        assert_eq!(race::WINNER_TITULO[0], "{name} vence em {track}!");
        assert_eq!(race::WINNER_PLAYER_TITULO[0], "{name} vence em {track}!");
        assert_eq!(race::PODIUM_TITULO[0], "{name} conquista P{pos} em {track}");
        assert_eq!(
            race::CONSTRUCTOR_CHAMPION_TITULO[0],
            "{team} conquista título de construtores!"
        );
    }

    #[test]
    fn test_catalog_includes_latest_incident_templates() {
        assert_eq!(incidents::CRITICAL_TITULO[0], "Acidente grave em {track}!");
        assert_eq!(
            incidents::MECANICO_TITULO[0],
            "{name} abandona com problemas mecânicos"
        );
    }

    #[test]
    fn test_catalog_includes_latest_injury_templates() {
        assert_eq!(injury::LEVE_TITULO[0], "{name} sofre lesão leve em {track}");
        assert_eq!(
            injury::CRITICA_TITULO[0],
            "URGENTE: {name} sofre lesão crítica"
        );
    }

    #[test]
    fn test_catalog_includes_latest_market_templates() {
        assert_eq!(market::PROPOSAL_TITULO[0], "{team} quer {name}!");
        assert_eq!(market::PLAYER_SIGN_TITULO[0], "{name} assinou com a {team}!");
    }

    #[test]
    fn test_catalog_includes_latest_end_of_season_templates() {
        assert_eq!(
            end_of_season::APOSENTA_TITULO[0],
            "{name} anuncia aposentadoria"
        );
        assert_eq!(
            end_of_season::LICENSE_PLAYER_TITULO[0],
            "{name} conquistou licença nível {n}!"
        );
    }

    #[test]
    fn test_catalog_includes_latest_rivalry_templates() {
        assert_eq!(
            rivalry::COMP_INTENSA_TITULO[0],
            "Guerra interna: {a} vs {b}!"
        );
        assert_eq!(
            rivalry::COL_INTENSA_TITULO[0],
            "Inimigos declarados: {a} e {b}"
        );
    }
}
