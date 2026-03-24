use crate::calendar::CalendarEntry;
use crate::models::driver::Driver;
use crate::models::enums::WeatherCondition;
use crate::models::team::Team;

#[derive(Debug, Clone)]
pub struct SimulationContext {
    pub category_id: String,
    pub category_tier: u8,
    pub track_id: u32,
    pub track_name: String,
    pub weather: WeatherCondition,
    pub temperature: f64,
    pub total_laps: i32,
    pub race_duration_minutes: i32,
    pub is_championship_deciding: bool,
    pub base_lap_time_ms: f64,
    pub tire_degradation_rate: f64,
    pub physical_degradation_rate: f64,
    pub incidents_enabled: bool,
}

impl SimulationContext {
    pub fn from_calendar_entry(
        entry: &CalendarEntry,
        category_tier: u8,
        is_championship_deciding: bool,
    ) -> Self {
        Self {
            category_id: entry.categoria.clone(),
            category_tier,
            track_id: entry.track_id,
            track_name: entry.track_name.clone(),
            weather: entry.clima,
            temperature: entry.temperatura,
            total_laps: entry.voltas,
            race_duration_minutes: entry.duracao_corrida_min,
            is_championship_deciding,
            base_lap_time_ms: 90_000.0,
            tire_degradation_rate: 0.02,
            physical_degradation_rate: 0.01,
            incidents_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimDriver {
    pub id: String,
    pub nome: String,
    pub is_jogador: bool,
    pub skill: u8,
    pub consistencia: u8,
    pub racecraft: u8,
    pub defesa: u8,
    pub ritmo_classificacao: u8,
    pub gestao_pneus: u8,
    pub habilidade_largada: u8,
    pub adaptabilidade: u8,
    pub fator_chuva: u8,
    pub fitness: u8,
    pub experiencia: u8,
    pub aggression: u8,
    pub smoothness: u8,
    pub mentalidade: u8,
    pub confianca: u8,
    pub car_performance: f64,
    pub car_reliability: f64,
    pub team_id: String,
    pub team_name: String,
    pub corridas_na_categoria: i32,
}

impl SimDriver {
    pub fn from_driver_and_team(driver: &Driver, team: &Team) -> Self {
        Self {
            id: driver.id.clone(),
            nome: driver.nome.clone(),
            is_jogador: driver.is_jogador,
            skill: as_u8(driver.atributos.skill),
            consistencia: as_u8(driver.atributos.consistencia),
            racecraft: as_u8(driver.atributos.racecraft),
            defesa: as_u8(driver.atributos.defesa),
            ritmo_classificacao: as_u8(driver.atributos.ritmo_classificacao),
            gestao_pneus: as_u8(driver.atributos.gestao_pneus),
            habilidade_largada: as_u8(driver.atributos.habilidade_largada),
            adaptabilidade: as_u8(driver.atributos.adaptabilidade),
            fator_chuva: as_u8(driver.atributos.fator_chuva),
            fitness: as_u8(driver.atributos.fitness),
            experiencia: as_u8(driver.atributos.experiencia),
            aggression: as_u8(driver.atributos.aggression),
            smoothness: as_u8(driver.atributos.smoothness),
            mentalidade: as_u8(driver.atributos.mentalidade),
            confianca: as_u8(driver.atributos.confianca),
            car_performance: team.car_performance,
            car_reliability: team.confiabilidade,
            team_id: team.id.clone(),
            team_name: team.nome.clone(),
            corridas_na_categoria: driver.corridas_na_categoria as i32,
        }
    }
}

fn as_u8(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use crate::calendar::CalendarEntry;
    use crate::models::driver::Driver;
    use crate::models::enums::{DriverStatus, RaceStatus, WeatherCondition};
    use crate::models::team::placeholder_team_from_db;

    use super::*;

    #[test]
    fn test_context_from_calendar_entry() {
        let entry = CalendarEntry {
            id: "R001".to_string(),
            season_id: "S001".to_string(),
            categoria: "gt3".to_string(),
            rodada: 1,
            nome: "Rodada 1 - Spa".to_string(),
            track_id: 100,
            track_name: "Spa".to_string(),
            track_config: "Full".to_string(),
            clima: WeatherCondition::Wet,
            temperatura: 18.5,
            voltas: 20,
            duracao_corrida_min: 45,
            duracao_classificacao_min: 15,
            status: RaceStatus::Pendente,
            horario: "14:00".to_string(),
            week_of_year: 5,
            season_phase: crate::models::enums::SeasonPhase::BlocoRegular,
            display_date: "2024-02-03".to_string(),
            thematic_slot: crate::models::enums::ThematicSlot::NaoClassificado,
        };

        let ctx = SimulationContext::from_calendar_entry(&entry, 4, true);

        assert_eq!(ctx.category_id, "gt3");
        assert_eq!(ctx.category_tier, 4);
        assert_eq!(ctx.track_name, "Spa");
        assert_eq!(ctx.weather, WeatherCondition::Wet);
        assert!(ctx.is_championship_deciding);
        assert!(ctx.incidents_enabled);
    }

    #[test]
    fn test_sim_driver_from_driver_and_team() {
        let mut driver = Driver::create_player(
            "P001".to_string(),
            "Joao Silva".to_string(),
            "🇧🇷 Brasileiro".to_string(),
            20,
        );
        driver.is_jogador = true;
        driver.status = DriverStatus::Ativo;
        driver.corridas_na_categoria = 7;
        driver.atributos.skill = 82.0;
        driver.atributos.gestao_pneus = 61.0;
        driver.atributos.ritmo_classificacao = 77.0;

        let mut team = placeholder_team_from_db(
            "T001".to_string(),
            "Team Test".to_string(),
            "gt3".to_string(),
            "2026-01-01T00:00:00".to_string(),
        );
        team.car_performance = 12.5;

        let sim_driver = SimDriver::from_driver_and_team(&driver, &team);

        assert_eq!(sim_driver.id, "P001");
        assert_eq!(sim_driver.team_id, "T001");
        assert_eq!(sim_driver.skill, 82);
        assert_eq!(sim_driver.gestao_pneus, 61);
        assert_eq!(sim_driver.car_reliability, team.confiabilidade);
        assert_eq!(sim_driver.corridas_na_categoria, 7);
    }
}
