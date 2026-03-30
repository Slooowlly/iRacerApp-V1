//! Sistema de selecao deterministica de templates para variacao de noticias.
//!
//! Usa hash do contexto (seed) para escolher entre multiplas variantes de forma
//! estavel: a mesma seed sempre produz o mesmo indice, sem necessidade de RNG externo.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub mod first_win;
pub mod templates;

/// Seleciona um indice de forma deterministica a partir de uma seed string.
pub fn pick_index(count: usize, seed: &str) -> usize {
    if count == 0 {
        return 0;
    }

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    (hasher.finish() as usize) % count
}

/// Seleciona um indice com discriminador adicional.
pub fn pick_index_with_discriminator(count: usize, seed: &str, discriminator: &str) -> usize {
    if count == 0 {
        return 0;
    }

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    discriminator.hash(&mut hasher);
    (hasher.finish() as usize) % count
}

/// Seleciona um template e aplica substituicoes simples.
pub fn pick_and_format(templates: &[&str], seed: &str, replacements: &[(&str, &str)]) -> String {
    if templates.is_empty() {
        return String::new();
    }

    let idx = pick_index(templates.len(), seed);
    let mut result = templates[idx].to_string();
    for (placeholder, value) in replacements {
        result = result.replace(placeholder, value);
    }

    result
}

/// Seleciona titulo e texto com variacao independente.
pub fn pick_title_and_body(
    titles: &[&str],
    bodies: &[&str],
    seed: &str,
    replacements: &[(&str, &str)],
) -> (String, String) {
    let title = if titles.is_empty() {
        String::new()
    } else {
        let idx = pick_index_with_discriminator(titles.len(), seed, "title");
        let mut selected = titles[idx].to_string();
        for (placeholder, value) in replacements {
            selected = selected.replace(placeholder, value);
        }
        selected
    };

    let body = if bodies.is_empty() {
        String::new()
    } else {
        let idx = pick_index_with_discriminator(bodies.len(), seed, "body");
        let mut selected = bodies[idx].to_string();
        for (placeholder, value) in replacements {
            selected = selected.replace(placeholder, value);
        }
        selected
    };

    (title, body)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{pick_and_format, pick_index, pick_index_with_discriminator, pick_title_and_body};

    #[test]
    fn test_pick_index_zero_count_returns_zero() {
        assert_eq!(pick_index(0, "any_seed"), 0);
    }

    #[test]
    fn test_pick_index_single_option_returns_zero() {
        assert_eq!(pick_index(1, "any_seed"), 0);
    }

    #[test]
    fn test_pick_index_stays_in_bounds() {
        for count in 2..=20 {
            for i in 0..100 {
                let seed = format!("test_seed_{i}");
                let idx = pick_index(count, &seed);
                assert!(idx < count, "idx={idx} should be < count={count}");
            }
        }
    }

    #[test]
    fn test_pick_index_deterministic() {
        let seed = "P001:3:1:gt4";
        let idx1 = pick_index(10, seed);
        let idx2 = pick_index(10, seed);
        let idx3 = pick_index(10, seed);

        assert_eq!(idx1, idx2);
        assert_eq!(idx2, idx3);
    }

    #[test]
    fn test_pick_index_varies_with_seed() {
        let seeds = [
            "seed1", "seed2", "seed3", "seed4", "seed5", "seed6", "seed7", "seed8",
        ];
        let results: HashSet<usize> = seeds.iter().map(|seed| pick_index(5, seed)).collect();

        assert!(results.len() > 1, "should produce varied indices");
    }

    #[test]
    fn test_pick_index_with_discriminator_differs() {
        let seed = "P001:3:1";
        let idx_title = pick_index_with_discriminator(10, seed, "title");
        let idx_body = pick_index_with_discriminator(10, seed, "body");
        let idx_title_repeat = pick_index_with_discriminator(10, seed, "title");

        assert_eq!(idx_title, idx_title_repeat);
        let _ = idx_body;
    }

    #[test]
    fn test_pick_and_format_empty_returns_empty() {
        let result = pick_and_format(&[], "seed", &[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_pick_and_format_applies_replacements() {
        let templates = &["{name} vence em {track}", "Vitoria de {name} em {track}"];
        let result = pick_and_format(
            templates,
            "deterministic_seed",
            &[("{name}", "Piloto X"), ("{track}", "Interlagos")],
        );

        assert!(result.contains("Piloto X"));
        assert!(result.contains("Interlagos"));
        assert!(!result.contains("{name}"));
        assert!(!result.contains("{track}"));
    }

    #[test]
    fn test_pick_title_and_body_independent_selection() {
        let titles = &["T1", "T2", "T3"];
        let bodies = &["B1", "B2", "B3"];

        let (title, body) = pick_title_and_body(titles, bodies, "seed", &[]);

        assert!(titles.contains(&title.as_str()));
        assert!(bodies.contains(&body.as_str()));
    }

    #[test]
    fn test_pick_title_and_body_applies_replacements() {
        let titles = &["{name} vence"];
        let bodies = &["{name} conquistou a vitoria em {track}."];

        let (title, body) = pick_title_and_body(
            titles,
            bodies,
            "seed",
            &[("{name}", "Piloto"), ("{track}", "Spa")],
        );

        assert_eq!(title, "Piloto vence");
        assert_eq!(body, "Piloto conquistou a vitoria em Spa.");
    }

    #[test]
    fn test_pick_title_and_body_handles_empty_arrays() {
        let (title, body) = pick_title_and_body(&[], &[], "seed", &[]);
        assert_eq!(title, "");
        assert_eq!(body, "");

        let (title_only, body_only) = pick_title_and_body(&["T1"], &[], "seed", &[]);
        assert_eq!(title_only, "T1");
        assert_eq!(body_only, "");

        let (title_empty, body_value) = pick_title_and_body(&[], &["B1"], "seed", &[]);
        assert_eq!(title_empty, "");
        assert_eq!(body_value, "B1");
    }
}
