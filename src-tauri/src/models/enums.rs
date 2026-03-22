use serde::{Deserialize, Serialize};

// ── Status do piloto ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriverStatus {
    Ativo,
    Lesionado,
    Aposentado,
    Suspenso,
}

impl DriverStatus {
    pub fn as_str(&self) -> &str {
        match self {
            DriverStatus::Ativo => "Ativo",
            DriverStatus::Lesionado => "Lesionado",
            DriverStatus::Aposentado => "Aposentado",
            DriverStatus::Suspenso => "Suspenso",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Lesionado" => DriverStatus::Lesionado,
            "Aposentado" => DriverStatus::Aposentado,
            "Suspenso" => DriverStatus::Suspenso,
            _ => DriverStatus::Ativo,
        }
    }
}

// ── Personalidade primária ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryPersonality {
    Ambicioso,
    Consolidador,
    Mercenario,
    Leal,
}

impl PrimaryPersonality {
    pub fn as_str(&self) -> &str {
        match self {
            PrimaryPersonality::Ambicioso => "Ambicioso",
            PrimaryPersonality::Consolidador => "Consolidador",
            PrimaryPersonality::Mercenario => "Mercenario",
            PrimaryPersonality::Leal => "Leal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Ambicioso" => PrimaryPersonality::Ambicioso,
            "Consolidador" | "Tecnico" | "Consistente" => PrimaryPersonality::Consolidador,
            "Mercenario" | "Agressivo" => PrimaryPersonality::Mercenario,
            "Leal" | "Calmo" => PrimaryPersonality::Leal,
            _ => PrimaryPersonality::Ambicioso,
        }
    }
}

// ── Personalidade secundária ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecondaryPersonality {
    CabecaQuente,
    SangueFrio,
    Apostador,
    Calculista,
    Showman,
    TeamPlayer,
    Solitario,
    Estudioso,
}

impl SecondaryPersonality {
    pub fn as_str(&self) -> &str {
        match self {
            SecondaryPersonality::CabecaQuente => "CabecaQuente",
            SecondaryPersonality::SangueFrio => "SangueFrio",
            SecondaryPersonality::Apostador => "Apostador",
            SecondaryPersonality::Calculista => "Calculista",
            SecondaryPersonality::Showman => "Showman",
            SecondaryPersonality::TeamPlayer => "TeamPlayer",
            SecondaryPersonality::Solitario => "Solitario",
            SecondaryPersonality::Estudioso => "Estudioso",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CabecaQuente" => SecondaryPersonality::CabecaQuente,
            "SangueFrio" | "Sensivel" => SecondaryPersonality::SangueFrio,
            "Apostador" | "Competitivo" => SecondaryPersonality::Apostador,
            "Calculista" => SecondaryPersonality::Calculista,
            "Showman" | "Lider" => SecondaryPersonality::Showman,
            "TeamPlayer" | "Trabalhador" => SecondaryPersonality::TeamPlayer,
            "Solitario" => SecondaryPersonality::Solitario,
            "Estudioso" | "Inteligente" => SecondaryPersonality::Estudioso,
            _ => SecondaryPersonality::Calculista,
        }
    }
}

// ── Status do contrato ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    Ativo,
    Expirado,
    Rescindido,
    Pendente,
}

impl ContractStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ContractStatus::Ativo => "Ativo",
            ContractStatus::Expirado => "Expirado",
            ContractStatus::Rescindido => "Rescindido",
            ContractStatus::Pendente => "Pendente",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "Expirado" => ContractStatus::Expirado,
            "Rescindido" => ContractStatus::Rescindido,
            "Pendente" => ContractStatus::Pendente,
            _ => ContractStatus::Ativo,
        }
    }
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Papel do piloto na equipe ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamRole {
    Numero1,
    Numero2,
}

impl TeamRole {
    pub fn as_str(&self) -> &str {
        match self {
            TeamRole::Numero1 => "Numero1",
            TeamRole::Numero2 => "Numero2",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "Numero1" | "N1" | "Titular" => TeamRole::Numero1,
            "Numero2" | "N2" | "Reserva" | "Junior" => TeamRole::Numero2,
            _ => TeamRole::Numero2,
        }
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Hierarquia da equipe (N1/N2) ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HierarchyStatus {
    N1,
    N2,
    Independente,
}

impl HierarchyStatus {
    pub fn as_str(&self) -> &str {
        match self {
            HierarchyStatus::N1 => "N1",
            HierarchyStatus::N2 => "N2",
            HierarchyStatus::Independente => "Independente",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "N1" => HierarchyStatus::N1,
            "N2" => HierarchyStatus::N2,
            _ => HierarchyStatus::Independente,
        }
    }
}

// ── Condição climática ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RainGroup {
    Dry,
    Normal,
    Rainy,
}

impl RainGroup {
    pub fn as_str(&self) -> &str {
        match self {
            RainGroup::Dry => "Dry",
            RainGroup::Normal => "Normal",
            RainGroup::Rainy => "Rainy",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Dry" => RainGroup::Dry,
            "Rainy" => RainGroup::Rainy,
            _ => RainGroup::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherCondition {
    Dry,
    Damp,
    Wet,
    HeavyRain,
}

impl WeatherCondition {
    pub fn as_str(&self) -> &str {
        match self {
            WeatherCondition::Dry => "Dry",
            WeatherCondition::Damp => "Damp",
            WeatherCondition::Wet => "Wet",
            WeatherCondition::HeavyRain => "HeavyRain",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Damp" => WeatherCondition::Damp,
            "Wet" => WeatherCondition::Wet,
            "HeavyRain" => WeatherCondition::HeavyRain,
            _ => WeatherCondition::Dry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackType {
    Road,
    Roval,
}

impl TrackType {
    pub fn as_str(&self) -> &str {
        match self {
            TrackType::Road => "Road",
            TrackType::Roval => "Roval",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Roval" => TrackType::Roval,
            _ => TrackType::Road,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeasonStatus {
    EmAndamento,
    Finalizada,
}

impl SeasonStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SeasonStatus::EmAndamento => "EmAndamento",
            SeasonStatus::Finalizada => "Finalizada",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Finalizada" => SeasonStatus::Finalizada,
            "Ativa" | "EmAndamento" => SeasonStatus::EmAndamento,
            _ => SeasonStatus::EmAndamento,
        }
    }
}

impl std::fmt::Display for SeasonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaceStatus {
    Pendente,
    Concluida,
}

impl RaceStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RaceStatus::Pendente => "Pendente",
            RaceStatus::Concluida => "Concluida",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Concluida" => RaceStatus::Concluida,
            _ => RaceStatus::Pendente,
        }
    }
}

impl std::fmt::Display for RaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Tipo de incidente ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IncidentType {
    Colisao,
    Toque,
    SpinOut,
    FalhaMecanica,
    ErroPit,
}

impl IncidentType {
    pub fn as_str(&self) -> &str {
        match self {
            IncidentType::Colisao => "Colisao",
            IncidentType::Toque => "Toque",
            IncidentType::SpinOut => "SpinOut",
            IncidentType::FalhaMecanica => "FalhaMecanica",
            IncidentType::ErroPit => "ErroPit",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Colisao" => IncidentType::Colisao,
            "Toque" => IncidentType::Toque,
            "SpinOut" => IncidentType::SpinOut,
            "FalhaMecanica" => IncidentType::FalhaMecanica,
            _ => IncidentType::ErroPit,
        }
    }
}

// ── Severidade do incidente ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IncidentSeverity {
    Leve,
    Moderado,
    Grave,
}

impl IncidentSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            IncidentSeverity::Leve => "Leve",
            IncidentSeverity::Moderado => "Moderado",
            IncidentSeverity::Grave => "Grave",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Moderado" => IncidentSeverity::Moderado,
            "Grave" => IncidentSeverity::Grave,
            _ => IncidentSeverity::Leve,
        }
    }
}

// ── Segmento de corrida ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RaceSegment {
    Largada,
    Abertura,
    Desenvolvimento,
    RetaFinal,
    Conclusao,
}

impl RaceSegment {
    pub fn as_str(&self) -> &str {
        match self {
            RaceSegment::Largada => "Largada",
            RaceSegment::Abertura => "Abertura",
            RaceSegment::Desenvolvimento => "Desenvolvimento",
            RaceSegment::RetaFinal => "RetaFinal",
            RaceSegment::Conclusao => "Conclusao",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Largada" => RaceSegment::Largada,
            "Abertura" => RaceSegment::Abertura,
            "Desenvolvimento" => RaceSegment::Desenvolvimento,
            "RetaFinal" => RaceSegment::RetaFinal,
            _ => RaceSegment::Conclusao,
        }
    }
}

// ── Tipo de lesão ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InjuryType {
    Leve,
    Moderada,
    Grave,
    Critica,
}

impl InjuryType {
    pub fn as_str(&self) -> &str {
        match self {
            InjuryType::Leve => "Leve",
            InjuryType::Moderada => "Moderada",
            InjuryType::Grave => "Grave",
            InjuryType::Critica => "Critica",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Moderada" => InjuryType::Moderada,
            "Grave" => InjuryType::Grave,
            "Critica" => InjuryType::Critica,
            _ => InjuryType::Leve,
        }
    }
}

// ── Status da proposta de mercado ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pendente,
    Aceita,
    Recusada,
    Expirada,
}

impl ProposalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ProposalStatus::Pendente => "Pendente",
            ProposalStatus::Aceita => "Aceita",
            ProposalStatus::Recusada => "Recusada",
            ProposalStatus::Expirada => "Expirada",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Aceita" => ProposalStatus::Aceita,
            "Recusada" => ProposalStatus::Recusada,
            "Expirada" => ProposalStatus::Expirada,
            _ => ProposalStatus::Pendente,
        }
    }
}

// ── Motivo de recusa de proposta ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RefusalReason {
    SalarioBaixo,
    EquipeFraca,
    CategoriaErrada,
    BloqueioHierarquico,
    PreferenciaPessoal,
}

impl RefusalReason {
    pub fn as_str(&self) -> &str {
        match self {
            RefusalReason::SalarioBaixo => "SalarioBaixo",
            RefusalReason::EquipeFraca => "EquipeFraca",
            RefusalReason::CategoriaErrada => "CategoriaErrada",
            RefusalReason::BloqueioHierarquico => "BloqueioHierarquico",
            RefusalReason::PreferenciaPessoal => "PreferenciaPessoal",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "SalarioBaixo" => RefusalReason::SalarioBaixo,
            "EquipeFraca" => RefusalReason::EquipeFraca,
            "CategoriaErrada" => RefusalReason::CategoriaErrada,
            "BloqueioHierarquico" => RefusalReason::BloqueioHierarquico,
            _ => RefusalReason::PreferenciaPessoal,
        }
    }
}

// ── Tipo de notícia ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NewsType {
    Contratacao,
    Corrida,
    Lesao,
    Aposentadoria,
    Promocao,
    Rivalidade,
    Titulo,
}

impl NewsType {
    pub fn as_str(&self) -> &str {
        match self {
            NewsType::Contratacao => "Contratacao",
            NewsType::Corrida => "Corrida",
            NewsType::Lesao => "Lesao",
            NewsType::Aposentadoria => "Aposentadoria",
            NewsType::Promocao => "Promocao",
            NewsType::Rivalidade => "Rivalidade",
            NewsType::Titulo => "Titulo",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Contratacao" => NewsType::Contratacao,
            "Lesao" => NewsType::Lesao,
            "Aposentadoria" => NewsType::Aposentadoria,
            "Promocao" => NewsType::Promocao,
            "Rivalidade" => NewsType::Rivalidade,
            "Titulo" => NewsType::Titulo,
            _ => NewsType::Corrida,
        }
    }
}

// ── Dificuldade ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Facil,
    Medio,
    Dificil,
    Lendario,
}

impl Difficulty {
    pub fn as_str(&self) -> &str {
        match self {
            Difficulty::Facil => "Facil",
            Difficulty::Medio => "Medio",
            Difficulty::Dificil => "Dificil",
            Difficulty::Lendario => "Lendario",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Facil" => Difficulty::Facil,
            "Dificil" => Difficulty::Dificil,
            "Lendario" => Difficulty::Lendario,
            _ => Difficulty::Medio,
        }
    }
}
