use serde::{Deserialize, Serialize};

/// Caráter esportivo canônico da pista — determina que atributos do piloto
/// e do carro são mais relevantes na simulação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackCharacter {
    /// Larga, rápida, curvas amplas — skill e carro dominam.
    Flowing,
    /// Mista — equilíbrio (baseline neutro).
    Technical,
    /// Estreita, lenta, exige precisão — adaptabilidade e consistência dominam.
    Tight,
    /// Oval com setor interno — velocidade e carro dominam, pack mais fluido.
    Roval,
}

/// Dados de simulação por pista: caráter e multiplicadores de stress.
#[derive(Debug, Clone)]
pub struct TrackSimulationData {
    pub track_character: TrackCharacter,
    /// Multiplica a taxa de desgaste de pneu (> 1.0 = mais desgaste).
    pub tire_stress_multiplier: f64,
    /// Multiplica a taxa de desgaste físico (> 1.0 = mais fadiga).
    pub physical_stress_multiplier: f64,
}

impl TrackSimulationData {
    fn new(character: TrackCharacter, tire: f64, physical: f64) -> Self {
        Self {
            track_character: character,
            tire_stress_multiplier: tire,
            physical_stress_multiplier: physical,
        }
    }
}

/// Retorna os dados de simulação para uma pista específica (por track_id do iRacing).
/// Fallback para pistas desconhecidas: Technical, stress = 1.0.
pub fn get_track_simulation_data(track_id: u32) -> TrackSimulationData {
    use TrackCharacter::*;
    match track_id {
        // ── Rovais ──────────────────────────────────────────────────────────
        554 => TrackSimulationData::new(Roval, 0.95, 0.90), // Charlotte Roval
        45 => TrackSimulationData::new(Roval, 0.95, 0.90),  // Daytona Road
        185 => TrackSimulationData::new(Roval, 0.90, 0.88), // Indianapolis Road

        // ── Flowing ─────────────────────────────────────────────────────────
        188 => TrackSimulationData::new(Flowing, 1.25, 1.25), // Spa-Francorchamps
        52 => TrackSimulationData::new(Flowing, 1.10, 1.20),  // Road America
        106 => TrackSimulationData::new(Flowing, 1.20, 1.15), // Silverstone GP
        93 => TrackSimulationData::new(Flowing, 1.00, 1.00),  // Monza
        169 => TrackSimulationData::new(Flowing, 1.00, 1.05), // Philip Island
        393 => TrackSimulationData::new(Flowing, 1.05, 1.00), // Bahrain
        193 => TrackSimulationData::new(Flowing, 0.95, 0.95), // Hockenheim GP
        53 => TrackSimulationData::new(Flowing, 1.00, 1.00),  // Sonoma
        360 => TrackSimulationData::new(Flowing, 1.00, 1.00), // Paul Ricard
        373 => TrackSimulationData::new(Flowing, 1.00, 1.05), // Fuji
        389 => TrackSimulationData::new(Flowing, 1.00, 1.00), // Zandvoort
        516 => TrackSimulationData::new(Flowing, 1.00, 0.95), // Yas Marina

        // ── Tight ────────────────────────────────────────────────────────────
        196 => TrackSimulationData::new(Tight, 1.30, 1.45), // Nordschleife
        339 => TrackSimulationData::new(Tight, 1.10, 1.15), // Cadwell Park
        194 => TrackSimulationData::new(Tight, 1.05, 1.10), // Hungaroring
        325 => TrackSimulationData::new(Tight, 0.82, 0.80), // Tsukuba
        318 => TrackSimulationData::new(Tight, 1.05, 1.05), // Long Beach
        504 => TrackSimulationData::new(Tight, 1.05, 1.05), // Detroit
        261 => TrackSimulationData::new(Tight, 1.00, 1.05), // Oulton Park Fosters
        341 => TrackSimulationData::new(Tight, 1.00, 1.05), // Oulton Park Island
        8 => TrackSimulationData::new(Tight, 0.85, 0.82),   // Summit Point Jefferson

        // ── Technical (com stress diferenciado) ─────────────────────────────
        238 => TrackSimulationData::new(Technical, 1.35, 1.30), // Sebring
        528 => TrackSimulationData::new(Technical, 1.20, 1.60), // Nürb 24H Combined
        287 => TrackSimulationData::new(Technical, 1.20, 1.50), // Le Mans
        119 => TrackSimulationData::new(Technical, 1.20, 1.30), // Mount Panorama / Bathurst
        164 => TrackSimulationData::new(Technical, 1.15, 1.25), // Suzuka
        212 => TrackSimulationData::new(Technical, 1.20, 1.15), // COTA
        58 => TrackSimulationData::new(Technical, 1.25, 1.20),  // VIR Full
        67 => TrackSimulationData::new(Technical, 1.10, 1.20),  // Watkins Glen Boot
        14 => TrackSimulationData::new(Technical, 0.85, 0.82),  // Lime Rock
        9 => TrackSimulationData::new(Technical, 0.85, 0.85),   // Summit Point S
        195 => TrackSimulationData::new(Technical, 0.88, 0.85), // Hockenheim Short
        301 => TrackSimulationData::new(Technical, 0.88, 0.85), // Brands Hatch Indy
        520 => TrackSimulationData::new(Technical, 0.85, 0.85), // Autodromo di Modena

        // ── Technical — stress neutro (demais) ──────────────────────────────
        _ => TrackSimulationData::new(Technical, 1.00, 1.00),
    }
}

/// Factor de densidade do pelotão baseado no comprimento da pista.
/// Pistas curtas concentram o pack, aumentando o risco de colisão.
pub fn pack_density_factor(comprimento_km: f64) -> f64 {
    if comprimento_km < 2.5 {
        1.40
    } else if comprimento_km < 4.0 {
        1.10
    } else if comprimento_km < 6.0 {
        1.00
    } else if comprimento_km < 10.0 {
        0.90
    } else {
        0.75
    }
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_track_is_technical_default() {
        let data = get_track_simulation_data(99999);
        assert_eq!(data.track_character, TrackCharacter::Technical);
        assert_eq!(data.tire_stress_multiplier, 1.0);
        assert_eq!(data.physical_stress_multiplier, 1.0);
    }

    #[test]
    fn test_sebring_high_tire_stress() {
        let data = get_track_simulation_data(238);
        assert!(
            data.tire_stress_multiplier >= 1.30,
            "Sebring tire stress={} should be >= 1.30",
            data.tire_stress_multiplier
        );
    }

    #[test]
    fn test_tsukuba_low_tire_stress() {
        let data = get_track_simulation_data(325);
        assert!(
            data.tire_stress_multiplier <= 0.85,
            "Tsukuba tire stress={} should be <= 0.85",
            data.tire_stress_multiplier
        );
    }

    #[test]
    fn test_le_mans_high_physical_stress() {
        let data = get_track_simulation_data(287);
        assert!(
            data.physical_stress_multiplier >= 1.45,
            "Le Mans physical stress={} should be >= 1.45",
            data.physical_stress_multiplier
        );
    }

    #[test]
    fn test_lime_rock_low_physical_stress() {
        let data = get_track_simulation_data(14);
        assert!(
            data.physical_stress_multiplier <= 0.85,
            "Lime Rock physical stress={} should be <= 0.85",
            data.physical_stress_multiplier
        );
    }

    #[test]
    fn test_tight_has_higher_overtaking_diff_than_flowing() {
        // Tight → 1.15, Flowing → 0.90 (via overtaking_difficulty_for na profile.rs)
        // Aqui só verificamos que Hungaroring é Tight e Spa é Flowing
        let hungaroring = get_track_simulation_data(194);
        let spa = get_track_simulation_data(188);
        assert_eq!(hungaroring.track_character, TrackCharacter::Tight);
        assert_eq!(spa.track_character, TrackCharacter::Flowing);
    }

    #[test]
    fn test_pack_density_short_track() {
        // Summit Jefferson (8) tem 1.6 km → deve ser >= 1.35
        let factor = pack_density_factor(1.6);
        assert!(
            factor >= 1.35,
            "pack_density for 1.6km = {} should be >= 1.35",
            factor
        );
    }

    #[test]
    fn test_pack_density_le_mans() {
        // Le Mans = 13.6 km → deve ser <= 0.80
        let factor = pack_density_factor(13.6);
        assert!(
            factor <= 0.80,
            "pack_density for 13.6km = {} should be <= 0.80",
            factor
        );
    }

    #[test]
    fn test_nordschleife_is_tight_with_high_stress() {
        let data = get_track_simulation_data(196);
        assert_eq!(data.track_character, TrackCharacter::Tight);
        assert!(data.tire_stress_multiplier >= 1.25);
        assert!(data.physical_stress_multiplier >= 1.40);
    }

    #[test]
    fn test_roval_character() {
        let charlotte = get_track_simulation_data(554);
        let daytona = get_track_simulation_data(45);
        assert_eq!(charlotte.track_character, TrackCharacter::Roval);
        assert_eq!(daytona.track_character, TrackCharacter::Roval);
    }
}
