// ══════════════════════════════════════════════════════════════════════════════
// INCIDENTES CRÍTICOS
// ══════════════════════════════════════════════════════════════════════════════

pub const CRITICAL_TITULO: &[&str] = &[
    "Acidente grave em {track}!",
    "Incidente sério durante GP de {track}",
    "Bandeira vermelha em {track}",
    "Acidente preocupante em {track}",
    "Momento tenso em {track}",
];

pub const CRITICAL_TEXTO_PAIR: &[&str] = &[
    "{a} e {b} se envolveram em um acidente grave em {track}. {dnf_note}",
    "Colisão séria entre {a} e {b} durante o GP de {track}. {dnf_note}",
    "Acidente envolvendo {a} e {b} causou tensão em {track}. {dnf_note}",
    "{a} e {b} colidiram violentamente em {track}. {dnf_note}",
    "Momento dramático: {a} e {b} protagonizaram acidente em {track}. {dnf_note}",
];

pub const CRITICAL_TEXTO_SOLO: &[&str] = &[
    "{name} sofreu um acidente grave durante o GP de {track}.",
    "Momento preocupante para {name} após incidente sério em {track}.",
    "{name} se envolveu em um acidente grave na corrida de {track}.",
    "Acidente sério tirou {name} da corrida em {track}.",
    "{name} teve um incidente grave que causou preocupação em {track}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// COLISÃO COM DNF
// ══════════════════════════════════════════════════════════════════════════════

pub const COLISAO_DNF_TITULO_PAIR: &[&str] = &[
    "{a} e {b} abandonam após colisão",
    "Colisão tira {a} e {b} da corrida",
    "Abandono duplo: {a} e {b} colidem",
    "{a} e {b} fora após toque",
    "Colisão elimina {a} e {b}",
];

pub const COLISAO_DNF_TITULO_SOLO: &[&str] = &[
    "{name} abandona após colisão",
    "Colisão tira {name} da corrida",
    "DNF para {name} após incidente",
    "{name} fora após colisão",
    "Abandono: {name} sofre colisão",
];

pub const COLISAO_DNF_TEXTO: &[&str] = &[
    "{name} não conseguiu continuar após a colisão em {track}.",
    "O incidente em {track} encerrou prematuramente a corrida de {name}.",
    "Infelizmente, {name} teve que abandonar após o contato em {track}.",
    "A colisão custou caro: {name} abandonou a corrida em {track}.",
    "{name} viu sua corrida terminar cedo após o incidente em {track}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// ERRO DO PILOTO COM DNF
// ══════════════════════════════════════════════════════════════════════════════

pub const ERRO_DNF_TITULO: &[&str] = &[
    "{name} abandona após erro",
    "Erro custoso para {name}",
    "{name} sai de pista e abandona",
    "Abandono: erro de {name}",
    "{name} perde corrida com erro",
];

pub const ERRO_DNF_TEXTO: &[&str] = &[
    "{name} cometeu um erro e foi forçado a abandonar em {track}.",
    "Um erro de pilotagem tirou {name} da corrida em {track}.",
    "Momento difícil para {name}, que abandonou após erro próprio em {track}.",
    "{name} perdeu o controle e não conseguiu continuar em {track}.",
    "Erro custoso: {name} viu sua corrida terminar prematuramente em {track}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// PROBLEMAS MECÂNICOS
// ══════════════════════════════════════════════════════════════════════════════

pub const MECANICO_TITULO: &[&str] = &[
    "{name} abandona com problemas mecânicos",
    "Mecânica trai {name}",
    "DNF mecânico para {name}",
    "{name} para nos boxes e abandona",
    "Problemas no carro de {name}",
];

pub const MECANICO_TEXTO: &[&str] = &[
    "{name} foi forçado a abandonar devido a problemas mecânicos em {track}.",
    "A mecânica não colaborou: {name} teve que deixar a corrida em {track}.",
    "Problemas no carro tiraram {name} da disputa em {track}.",
    "{name} abandonou após falha mecânica durante o GP de {track}.",
    "Dia frustrante para {name}, que viu o carro falhar em {track}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// COLISÃO MAJOR (SEM DNF)
// ══════════════════════════════════════════════════════════════════════════════

pub const COLISAO_MAJOR_TITULO_PAIR: &[&str] = &[
    "Toque entre {a} e {b} em {track}",
    "{a} e {b} se tocam durante corrida",
    "Incidente entre {a} e {b}",
    "Contato: {a} versus {b}",
    "{a} e {b} colidem mas seguem",
];

pub const COLISAO_MAJOR_TITULO_SOLO: &[&str] = &[
    "{name} sofre incidente em {track}",
    "Toque prejudica {name}",
    "Incidente atrapalha {name}",
    "{name} perde tempo com incidente",
    "Contato complica corrida de {name}",
];

pub const COLISAO_MAJOR_TEXTO: &[&str] = &[
    "{name} conseguiu seguir em frente após o incidente em {track}, mas perdeu tempo.",
    "O contato prejudicou {name}, que ainda assim continuou na corrida em {track}.",
    "Apesar do incidente, {name} manteve o carro na pista em {track}.",
    "{name} sofreu com o toque mas não desistiu da corrida em {track}.",
    "Momento complicado para {name}, que perdeu posições após o incidente em {track}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// ERRO MAJOR (SEM DNF)
// ══════════════════════════════════════════════════════════════════════════════

pub const ERRO_MAJOR_TITULO: &[&str] = &[
    "{name} comete erro em {track}",
    "Erro de {name} durante corrida",
    "{name} escapa mas continua",
    "Momento difícil para {name}",
    "{name} perde tempo com escapada",
];

pub const ERRO_MAJOR_TEXTO: &[&str] = &[
    "{name} cometeu um erro em {track} mas conseguiu manter o carro na corrida.",
    "Apesar da escapada, {name} recuperou e seguiu competindo em {track}.",
    "Erro de {name} custou algumas posições em {track}.",
    "{name} teve um momento complicado mas não desistiu da corrida em {track}.",
    "A escapada de {name} em {track} custou tempo precioso.",
];
