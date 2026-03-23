use std::collections::HashSet;

use chrono::{Datelike, Local};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::constants::{scoring, skill_ranges};
use crate::generators::names::generate_pilot_identity;
use crate::models::enums::{DriverStatus, PrimaryPersonality, SecondaryPersonality};

// TODO(migration): o schema atual já cobre todos os campos persistidos do Módulo 10.
// Se no futuro tags visíveis ou metadados de geração forem persistidos, novas colunas seriam necessárias.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagLevel {
    DefeitoGrave,
    Defeito,
    Qualidade,
    QualidadeAlta,
    Elite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeTag {
    pub attribute_name: &'static str,
    pub tag_text: &'static str,
    pub level: TagLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverAttributes {
    pub skill: f64,
    pub consistencia: f64,
    pub racecraft: f64,
    pub defesa: f64,
    pub ritmo_classificacao: f64,
    pub gestao_pneus: f64,
    pub habilidade_largada: f64,
    pub adaptabilidade: f64,
    pub fator_chuva: f64,
    pub fitness: f64,
    pub experiencia: f64,
    pub desenvolvimento: f64,
    pub aggression: f64,
    pub smoothness: f64,
    pub midia: f64,
    pub mentalidade: f64,
    pub confianca: f64,
}

impl Default for DriverAttributes {
    fn default() -> Self {
        Self {
            skill: 50.0,
            consistencia: 50.0,
            racecraft: 50.0,
            defesa: 50.0,
            ritmo_classificacao: 50.0,
            gestao_pneus: 50.0,
            habilidade_largada: 50.0,
            adaptabilidade: 50.0,
            fator_chuva: 50.0,
            fitness: 50.0,
            experiencia: 50.0,
            desenvolvimento: 50.0,
            aggression: 50.0,
            smoothness: 50.0,
            midia: 50.0,
            mentalidade: 50.0,
            confianca: 50.0,
        }
    }
}

impl DriverAttributes {
    pub fn entries(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("skill", self.skill),
            ("consistencia", self.consistencia),
            ("racecraft", self.racecraft),
            ("defesa", self.defesa),
            ("ritmo_classificacao", self.ritmo_classificacao),
            ("gestao_pneus", self.gestao_pneus),
            ("habilidade_largada", self.habilidade_largada),
            ("adaptabilidade", self.adaptabilidade),
            ("fator_chuva", self.fator_chuva),
            ("fitness", self.fitness),
            ("experiencia", self.experiencia),
            ("desenvolvimento", self.desenvolvimento),
            ("aggression", self.aggression),
            ("smoothness", self.smoothness),
            ("midia", self.midia),
            ("mentalidade", self.mentalidade),
            ("confianca", self.confianca),
        ]
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriverSeasonStats {
    pub pontos: f64,
    pub vitorias: u32,
    pub podios: u32,
    pub poles: u32,
    pub corridas: u32,
    pub dnfs: u32,
    pub posicao_media: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriverCareerStats {
    pub pontos_total: f64,
    pub vitorias: u32,
    pub podios: u32,
    pub poles: u32,
    pub corridas: u32,
    pub temporadas: u32,
    pub titulos: u32,
    pub dnfs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Driver {
    pub id: String,
    pub nome: String,
    pub is_jogador: bool,
    pub idade: u32,
    pub nacionalidade: String,
    pub genero: String,
    pub categoria_atual: Option<String>,
    #[serde(default)]
    pub categoria_especial_ativa: Option<String>,
    pub status: DriverStatus,
    pub personalidade_primaria: Option<PrimaryPersonality>,
    pub personalidade_secundaria: Option<SecondaryPersonality>,
    pub ano_inicio_carreira: u32,
    #[serde(default)]
    pub atributos: DriverAttributes,
    #[serde(default)]
    pub stats_temporada: DriverSeasonStats,
    #[serde(default)]
    pub stats_carreira: DriverCareerStats,
    #[serde(default = "default_motivation")]
    pub motivacao: f64,
    #[serde(default = "default_track_history")]
    pub historico_circuitos: serde_json::Value,
    #[serde(default = "default_recent_results")]
    pub ultimos_resultados: serde_json::Value,
    pub melhor_resultado_temp: Option<u32>,
    pub temporadas_na_categoria: u32,
    pub corridas_na_categoria: u32,
    pub temporadas_motivacao_baixa: u32,
}

impl Driver {
    pub fn new(
        id: String,
        nome: String,
        nacionalidade: String,
        genero: String,
        idade: u32,
        ano_inicio_carreira: u32,
    ) -> Self {
        Self {
            id,
            nome,
            is_jogador: false,
            idade,
            nacionalidade,
            genero,
            categoria_atual: None,
            categoria_especial_ativa: None,
            status: DriverStatus::Ativo,
            personalidade_primaria: None,
            personalidade_secundaria: None,
            ano_inicio_carreira,
            atributos: DriverAttributes::default(),
            stats_temporada: DriverSeasonStats::default(),
            stats_carreira: DriverCareerStats::default(),
            motivacao: default_motivation(),
            historico_circuitos: default_track_history(),
            ultimos_resultados: default_recent_results(),
            melhor_resultado_temp: None,
            temporadas_na_categoria: 0,
            corridas_na_categoria: 0,
            temporadas_motivacao_baixa: 0,
        }
    }

    pub fn new_player(
        id: String,
        nome: String,
        nacionalidade: String,
        idade: u32,
        current_year: u32,
    ) -> Self {
        let mut driver = Self::new(
            id,
            nome,
            nacionalidade,
            "M".to_string(),
            idade,
            current_year.saturating_sub(idade.saturating_sub(16)),
        );
        driver.is_jogador = true;
        driver.personalidade_primaria = None;
        driver.personalidade_secundaria = None;
        driver.atributos = DriverAttributes::default();
        driver.motivacao = 70.0;
        driver
    }

    pub fn create_player(id: String, nome: String, nacionalidade: String, idade: i32) -> Self {
        Self::new_player(
            id,
            nome,
            nacionalidade,
            idade.clamp(16, 60) as u32,
            current_year(),
        )
    }

    pub fn generate_for_category(
        category_id: &str,
        category_tier: u8,
        difficulty: &str,
        count: usize,
        existing_names: &mut HashSet<String>,
        rng: &mut impl Rng,
    ) -> Vec<Self> {
        let mut generated = 1_usize;
        Self::generate_for_category_with_id_factory(
            category_id,
            category_tier,
            difficulty,
            count,
            existing_names,
            &mut || {
                let id = format!("PGEN-{}-{:03}", category_id, generated);
                generated += 1;
                id
            },
            rng,
        )
    }

    pub(crate) fn generate_for_category_with_id_factory<F, R>(
        category_id: &str,
        category_tier: u8,
        difficulty: &str,
        count: usize,
        existing_names: &mut HashSet<String>,
        id_factory: &mut F,
        rng: &mut R,
    ) -> Vec<Self>
    where
        F: FnMut() -> String,
        R: Rng,
    {
        let normalized_tier = category_tier.min(4);
        let skill_range =
            skill_ranges::get_skill_range_by_tier(normalized_tier).unwrap_or_else(|| {
                skill_ranges::get_skill_range_by_tier(4).expect("skill range tier 4")
            });

        let difficulty_id = normalize_difficulty_id(difficulty);
        let difficulty_config = scoring::get_difficulty_config(difficulty_id)
            .or_else(|| scoring::get_difficulty_config("medio"))
            .expect("difficulty config should exist");

        let mut drivers = Vec::with_capacity(count);
        let guaranteed_prodigies = if normalized_tier == 0 {
            count.min(2)
        } else {
            0
        };

        for index in 0..count {
            let rookie_prodigy = index < guaranteed_prodigies;
            let identity = generate_pilot_identity(existing_names, rng);
            existing_names.insert(identity.nome_completo.clone());

            let idade = if rookie_prodigy {
                rng.gen_range(17..=19)
            } else {
                let (min_age, max_age) = tier_age_range(normalized_tier);
                rng.gen_range(min_age..=max_age)
            };

            let (skill_min, skill_max) =
                effective_skill_bounds(skill_range, difficulty_config, rookie_prodigy);
            let skill = if rookie_prodigy {
                roll_stat(rng, 60, 70)
            } else {
                roll_stat(rng, skill_min, skill_max)
            };

            let consistencia = correlated_stat(rng, skill, 10);
            let racecraft = correlated_stat(rng, skill, 8);
            let defesa = correlated_stat(rng, skill, 8);
            let ritmo_classificacao = correlated_stat(rng, skill, 12);
            let gestao_pneus = roll_stat(rng, 40, 70);
            let habilidade_largada = roll_stat(rng, 40, 70);
            let adaptabilidade = roll_stat(rng, 40, 70);
            let fator_chuva = roll_stat(rng, 30, 70);
            let fitness = fitness_for_age(rng, idade);
            let experiencia = experience_for_profile(rng, idade, normalized_tier, rookie_prodigy);
            let desenvolvimento = development_for_profile(rng, idade, skill, rookie_prodigy);
            let aggression = roll_stat(rng, 30, 70);
            let smoothness = inverse_correlated_stat(rng, aggression);
            let midia = roll_stat(rng, 30, 70);
            let mentalidade = roll_stat(rng, 40, 70);
            let confianca = roll_stat(rng, 50, 70);

            let current_year = current_year();
            let ano_inicio = current_year.saturating_sub(idade.saturating_sub(16));
            let mut driver = Self::new(
                id_factory(),
                identity.nome_completo,
                identity.nacionalidade_label,
                identity.genero,
                idade,
                ano_inicio,
            );
            driver.categoria_atual = Some(category_id.to_string());
            driver.personalidade_primaria = Some(random_primary(rng));
            driver.personalidade_secundaria = Some(random_secondary(rng));
            driver.motivacao = roll_stat(rng, 50, 80) as f64;
            driver.atributos = DriverAttributes {
                skill: skill as f64,
                consistencia: consistencia as f64,
                racecraft: racecraft as f64,
                defesa: defesa as f64,
                ritmo_classificacao: ritmo_classificacao as f64,
                gestao_pneus: gestao_pneus as f64,
                habilidade_largada: habilidade_largada as f64,
                adaptabilidade: adaptabilidade as f64,
                fator_chuva: fator_chuva as f64,
                fitness: fitness as f64,
                experiencia: experiencia as f64,
                desenvolvimento: desenvolvimento as f64,
                aggression: aggression as f64,
                smoothness: smoothness as f64,
                midia: midia as f64,
                mentalidade: mentalidade as f64,
                confianca: confianca as f64,
            };
            drivers.push(driver);
        }

        drivers
    }

    pub fn get_visible_tags(&self) -> Vec<AttributeTag> {
        self.atributos
            .entries()
            .into_iter()
            .filter_map(|(attribute_name, value)| get_attribute_tag(attribute_name, value))
            .collect()
    }

    pub fn get_all_tags(&self) -> Vec<AttributeTag> {
        self.get_visible_tags()
    }

    pub fn attribute_tag(attribute_name: &'static str, value: f64) -> Option<AttributeTag> {
        get_attribute_tag(attribute_name, value)
    }

    pub fn reset_season_stats(&mut self) {
        self.stats_temporada = DriverSeasonStats::default();
        self.melhor_resultado_temp = None;
    }

    pub fn accumulate_career_stats(&mut self) {
        let season = &self.stats_temporada;
        let career = &mut self.stats_carreira;
        career.pontos_total += season.pontos;
        career.vitorias += season.vitorias;
        career.podios += season.podios;
        career.poles += season.poles;
        career.corridas += season.corridas;
        career.dnfs += season.dnfs;
        career.temporadas += 1;
    }
}

fn default_motivation() -> f64 {
    75.0
}

fn default_track_history() -> serde_json::Value {
    serde_json::json!({})
}

fn default_recent_results() -> serde_json::Value {
    serde_json::json!([])
}

fn current_year() -> u32 {
    Local::now().year().max(0) as u32
}

fn get_attribute_tag(attribute_name: &'static str, value: f64) -> Option<AttributeTag> {
    let rounded = value.round() as u8;
    let (level, index) = if rounded <= 10 {
        (TagLevel::DefeitoGrave, 0)
    } else if rounded <= 25 {
        (TagLevel::Defeito, 1)
    } else if rounded <= 74 {
        return None;
    } else if rounded <= 84 {
        (TagLevel::Qualidade, 2)
    } else if rounded <= 94 {
        (TagLevel::QualidadeAlta, 3)
    } else {
        (TagLevel::Elite, 4)
    };

    let tag_text = tag_text_for(attribute_name, index)?;
    Some(AttributeTag {
        attribute_name,
        tag_text,
        level,
    })
}

fn tag_text_for(attribute_name: &str, index: usize) -> Option<&'static str> {
    let tags = match attribute_name {
        "skill" => ["Lento", "Abaixo do Ritmo", "Veloz", "Super Veloz", "Alien"],
        "consistencia" => [
            "Totalmente Imprevisível",
            "Inconsistente",
            "Consistente",
            "Muito Consistente",
            "Máquina de Regularidade",
        ],
        "racecraft" => [
            "Perigo nas Rodas",
            "Roda-a-roda Fraco",
            "Bom Disputador",
            "Mestre em Disputas",
            "Racecraft de Elite",
        ],
        "defesa" => [
            "Porta Aberta",
            "Defesa Fraca",
            "Bom Defensor",
            "Muro na Pista",
            "Inultrapassável",
        ],
        "ritmo_classificacao" => [
            "Péssimo em Quali",
            "Lento na Classificação",
            "Forte na Classificação",
            "Especialista em Quali",
            "Rei da Pole",
        ],
        "gestao_pneus" => [
            "Destruidor de Pneus",
            "Gestão de Pneus Fraca",
            "Bom com Pneus",
            "Excelente Gestão",
            "Smooth Operator",
        ],
        "habilidade_largada" => [
            "Péssimo nas Largadas",
            "Ruim de Largada",
            "Boas Largadas",
            "Excelente nas Largadas",
            "Foguete na Largada",
        ],
        "adaptabilidade" => [
            "Inflexível",
            "Lento para Adaptar",
            "Adaptável",
            "Muito Adaptável",
            "Camaleão",
        ],
        "fator_chuva" => [
            "Terrível na Chuva",
            "Dificuldade na Chuva",
            "Bom na Chuva",
            "Especialista de Chuva",
            "Mestre da Chuva",
        ],
        "fitness" => [
            "Doente",
            "Fora de Forma",
            "Boa Forma Física",
            "Atleta",
            "Forma Física de Elite",
        ],
        "experiencia" => [
            "Calouro",
            "Inexperiente",
            "Experiente",
            "Muito Experiente",
            "Veterano Sábio",
        ],
        "desenvolvimento" => [
            "Estagnado",
            "Desenvolvimento Lento",
            "Em Ascensão",
            "Evolução Rápida",
            "Prodígio",
        ],
        "aggression" => [
            "Passivo Demais",
            "Muito Cauteloso",
            "Agressivo",
            "Muito Agressivo",
            "Kamikaze",
        ],
        "smoothness" => [
            "Pilotagem Bruta",
            "Pouco Suave",
            "Pilotagem Suave",
            "Muito Suave",
            "Pilotagem de Seda",
        ],
        "midia" => [
            "Invisível",
            "Discreto",
            "Carismático",
            "Queridinho da Mídia",
            "Estrela",
        ],
        "mentalidade" => [
            "Frágil sob Pressão",
            "Mentalidade Fraca",
            "Boa Mentalidade",
            "Mentalidade de Campeão",
            "Gelo nas Veias",
        ],
        "confianca" => [
            "Sem Confiança",
            "Inseguro",
            "Confiante",
            "Muito Confiante",
            "Inabalável",
        ],
        _ => return None,
    };

    Some(tags[index])
}

fn normalize_difficulty_id(input: &str) -> &'static str {
    match input.trim() {
        "facil" | "Facil" | "Fácil" => "facil",
        "medio" | "médio" | "Medio" | "Médio" | "Normal" | "normal" => "medio",
        "dificil" | "Difícil" | "Dificil" => "dificil",
        "lendario" | "lendário" | "Lendario" | "Lendário" | "Elite" | "elite" => "lendario",
        _ => "medio",
    }
}

fn effective_skill_bounds(
    range: &skill_ranges::SkillRangeConfig,
    difficulty: &scoring::DifficultyConfig,
    rookie_prodigy: bool,
) -> (u8, u8) {
    if rookie_prodigy {
        return (60, 70);
    }

    let min = range.skill_min.max(difficulty.skill_min_ia);
    let max = range.skill_max.min(difficulty.skill_max_ia);
    if min <= max {
        (min, max)
    } else {
        (range.skill_min, range.skill_max)
    }
}

fn roll_stat(rng: &mut impl Rng, min: u8, max: u8) -> u8 {
    rng.gen_range(min..=max)
}

fn correlated_stat(rng: &mut impl Rng, base: u8, variance: i16) -> u8 {
    let offset = rng.gen_range(-variance..=variance);
    clamp_stat(base as i16 + offset)
}

fn inverse_correlated_stat(rng: &mut impl Rng, aggression: u8) -> u8 {
    let offset = rng.gen_range(-10_i16..=10_i16);
    clamp_stat(100 - aggression as i16 + offset)
}

fn clamp_stat(value: i16) -> u8 {
    value.clamp(0, 100) as u8
}

fn tier_age_range(tier: u8) -> (u32, u32) {
    match tier {
        0 => (18, 24),
        1 => (20, 28),
        2 => (22, 31),
        3 => (24, 35),
        _ => (26, 40),
    }
}

fn fitness_for_age(rng: &mut impl Rng, age: u32) -> u8 {
    match age {
        0..=22 => roll_stat(rng, 70, 85),
        23..=32 => roll_stat(rng, 60, 75),
        33..=37 => roll_stat(rng, 50, 68),
        _ => roll_stat(rng, 40, 60),
    }
}

fn experience_for_profile(rng: &mut impl Rng, age: u32, tier: u8, rookie_prodigy: bool) -> u8 {
    let age_bonus = ((age.saturating_sub(17)) * 2) as i16;
    let tier_bonus = (tier as i16) * 10;
    let random_bonus = rng.gen_range(0_i16..=12_i16);
    let prodigy_bonus = if rookie_prodigy { 10 } else { 0 };
    clamp_stat(8 + age_bonus + tier_bonus + random_bonus + prodigy_bonus)
}

fn development_for_profile(rng: &mut impl Rng, age: u32, skill: u8, rookie_prodigy: bool) -> u8 {
    if rookie_prodigy || (age <= 21 && skill >= 60) {
        roll_stat(rng, 70, 90)
    } else if age >= 33 {
        roll_stat(rng, 20, 50)
    } else {
        roll_stat(rng, 40, 60)
    }
}

fn random_primary(rng: &mut impl Rng) -> PrimaryPersonality {
    match rng.gen_range(0_u8..4_u8) {
        0 => PrimaryPersonality::Ambicioso,
        1 => PrimaryPersonality::Consolidador,
        2 => PrimaryPersonality::Mercenario,
        _ => PrimaryPersonality::Leal,
    }
}

fn random_secondary(rng: &mut impl Rng) -> SecondaryPersonality {
    match rng.gen_range(0_u8..8_u8) {
        0 => SecondaryPersonality::CabecaQuente,
        1 => SecondaryPersonality::SangueFrio,
        2 => SecondaryPersonality::Apostador,
        3 => SecondaryPersonality::Calculista,
        4 => SecondaryPersonality::Showman,
        5 => SecondaryPersonality::TeamPlayer,
        6 => SecondaryPersonality::Solitario,
        _ => SecondaryPersonality::Estudioso,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_get_visible_tags_returns_only_extreme_attributes() {
        let mut driver = Driver::new(
            "DRV-1".to_string(),
            "Teste".to_string(),
            "Brasil".to_string(),
            "M".to_string(),
            22,
            2024,
        );
        driver.atributos.skill = 97.0;
        driver.atributos.consistencia = 20.0;

        let tags = driver.get_visible_tags();
        assert_eq!(tags.len(), 2);
        assert!(tags
            .iter()
            .any(|tag| tag.attribute_name == "skill" && tag.tag_text == "Alien"));
        assert!(tags
            .iter()
            .any(|tag| tag.attribute_name == "consistencia" && tag.tag_text == "Inconsistente"));
    }

    #[test]
    fn test_player_has_personalities_none() {
        let player = Driver::new_player(
            "PLY-1".to_string(),
            "Jogador".to_string(),
            "Brasil".to_string(),
            20,
            2024,
        );
        assert!(player.personalidade_primaria.is_none());
        assert!(player.personalidade_secundaria.is_none());
        assert_eq!(player.atributos.skill, 50.0);
        assert_eq!(player.motivacao, 70.0);
        assert!(player
            .atributos
            .entries()
            .into_iter()
            .all(|(_, value)| value == 50.0));
    }

    #[test]
    fn test_generate_for_category_uses_real_names() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut existing_names = HashSet::new();
        let drivers =
            Driver::generate_for_category("gt4", 3, "medio", 8, &mut existing_names, &mut rng);

        assert!(!drivers.is_empty());
        assert!(drivers
            .iter()
            .all(|driver| driver.nome.split_whitespace().count() >= 2));
        assert!(drivers.iter().all(|driver| driver.nome != "IA"));
    }

    #[test]
    fn test_generate_for_category_no_name_collisions() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut existing_names = HashSet::new();
        let drivers =
            Driver::generate_for_category("gt3", 4, "medio", 40, &mut existing_names, &mut rng);

        let unique_names: HashSet<_> = drivers.iter().map(|driver| driver.nome.clone()).collect();
        assert_eq!(unique_names.len(), drivers.len());
    }

    #[test]
    fn test_generate_for_category_rookie_has_prodigies() {
        let mut rng = StdRng::seed_from_u64(99);
        let mut existing_names = HashSet::new();
        let drivers = Driver::generate_for_category(
            "mazda_rookie",
            0,
            "medio",
            12,
            &mut existing_names,
            &mut rng,
        );

        let prodigies = drivers
            .iter()
            .filter(|driver| driver.atributos.skill >= 60.0 && driver.idade <= 19)
            .count();
        assert!(prodigies >= 2);
    }

    #[test]
    fn test_generate_for_category_skill_within_range() {
        let mut rng = StdRng::seed_from_u64(17);
        let mut existing_names = HashSet::new();
        let drivers =
            Driver::generate_for_category("gt4", 3, "medio", 25, &mut existing_names, &mut rng);

        let range = skill_ranges::get_skill_range_by_tier(3).expect("tier 3 range should exist");
        assert!(drivers.iter().all(|driver| {
            driver.atributos.skill >= range.skill_min as f64
                && driver.atributos.skill <= range.skill_max as f64
        }));
    }

    #[test]
    fn test_create_player_attributes_at_50() {
        let player = Driver::create_player(
            "P001".to_string(),
            "Jogador Teste".to_string(),
            "BR".to_string(),
            20,
        );

        assert!(player.is_jogador);
        assert!(player.personalidade_primaria.is_none());
        assert!(player.personalidade_secundaria.is_none());
        assert_eq!(player.motivacao, 70.0);
        assert!(player
            .atributos
            .entries()
            .into_iter()
            .all(|(_, value)| value == 50.0));
    }
}
