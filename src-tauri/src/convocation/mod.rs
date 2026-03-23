pub mod eligibility;
pub mod pipeline;
pub mod quotas;
pub mod scoring;

pub use eligibility::{coletar_candidatos, Candidato, FonteConvocacao};
pub use pipeline::{
    advance_to_convocation_window, encerrar_bloco_especial, iniciar_bloco_especial,
    run_convocation_window, run_pos_especial, ConvocationResult, GridClasse, PosEspecialResult,
};
pub use quotas::{calcular_cotas, Cotas};
pub use scoring::calcular_score;
