use crate::models::driver::Driver;
use crate::models::enums::TeamRole;

pub fn calculate_visibility(
    driver: &Driver,
    posicao_campeonato: i32,
    total_pilotos: i32,
    category_tier: u8,
    vitorias: i32,
    titulos: i32,
    poles: i32,
    papel: &TeamRole,
    categoria: &str,
) -> f64 {
    let mut vis = 3.0;

    if posicao_campeonato <= 3 {
        vis += 4.0;
    } else if posicao_campeonato <= 5 {
        vis += 3.0;
    } else if posicao_campeonato <= 10 {
        vis += 2.0;
    } else if total_pilotos > 0 && posicao_campeonato <= total_pilotos / 2 {
        vis += 1.0;
    }

    vis += category_tier as f64 * 0.3;

    if driver.idade < 23 {
        vis += 2.0;
    } else if driver.idade <= 28 {
        vis += 1.0;
    } else if driver.idade > 35 {
        vis -= 1.0;
    }

    vis += (vitorias.max(0) as f64 * 0.5).min(1.5);
    vis += (titulos.max(0) as f64 * 2.0).min(4.0);
    vis += (poles.max(0) as f64 * 0.2).min(0.4);

    if *papel == TeamRole::Numero2 {
        vis -= 2.0;
    }

    if categoria == "mazda_rookie" || categoria == "toyota_rookie" {
        vis = vis.min(3.0);
    }

    vis.clamp(0.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_champion_high_visibility() {
        let driver = sample_driver(25);

        let visibility =
            calculate_visibility(&driver, 1, 20, 4, 5, 1, 3, &TeamRole::Numero1, "gt3");

        assert!(visibility >= 8.0);
    }

    #[test]
    fn test_rookie_category_capped_visibility() {
        let driver = sample_driver(19);

        let visibility = calculate_visibility(
            &driver,
            1,
            12,
            0,
            5,
            0,
            4,
            &TeamRole::Numero1,
            "mazda_rookie",
        );

        assert!(visibility <= 3.0);
    }

    #[test]
    fn test_young_driver_visibility_bonus() {
        let young = sample_driver(20);
        let prime = sample_driver(27);

        let young_visibility =
            calculate_visibility(&young, 6, 20, 2, 1, 0, 1, &TeamRole::Numero1, "bmw_m2");
        let prime_visibility =
            calculate_visibility(&prime, 6, 20, 2, 1, 0, 1, &TeamRole::Numero1, "bmw_m2");

        assert!(young_visibility > prime_visibility);
    }

    #[test]
    fn test_n2_visibility_penalty() {
        let driver = sample_driver(25);

        let n1 = calculate_visibility(&driver, 4, 20, 3, 1, 0, 0, &TeamRole::Numero1, "gt4");
        let n2 = calculate_visibility(&driver, 4, 20, 3, 1, 0, 0, &TeamRole::Numero2, "gt4");

        assert!(n1 > n2);
    }

    fn sample_driver(age: u32) -> Driver {
        Driver::new(
            "P001".to_string(),
            "Piloto".to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            age,
            2020,
        )
    }
}
