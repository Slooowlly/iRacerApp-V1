// ══════════════════════════════════════════════════════════════════════════════
// CONTRATO EXPIRADO
// ══════════════════════════════════════════════════════════════════════════════

pub const EXPIRED_TITULO: &[&str] = &[
    "{name} deixa {team}",
    "Contrato de {name} com {team} encerra",
    "{name} livre no mercado",
    "Fim da parceria {name} e {team}",
    "{name} não renova com {team}",
];

pub const EXPIRED_TEXTO: &[&str] = &[
    "O contrato de {name} com a {team} chegou ao fim e o piloto está livre no mercado.",
    "{name} encerra sua passagem pela {team} e busca novos desafios.",
    "Após o término do contrato, {name} deixa a {team} e avalia opções.",
    "A {team} não renovou com {name}, que agora está disponível.",
    "{name} e {team} seguem caminhos diferentes após fim de contrato.",
];

// ══════════════════════════════════════════════════════════════════════════════
// CONTRATO RENOVADO
// ══════════════════════════════════════════════════════════════════════════════

pub const RENEWED_TITULO: &[&str] = &[
    "{name} renova com {team}",
    "Renovação: {name} segue na {team}",
    "{team} mantém {name}",
    "Acordo renovado: {name} e {team}",
    "{name} assina novo contrato com {team}",
];

pub const RENEWED_TEXTO: &[&str] = &[
    "{name} renovou seu contrato com a {team} e continuará defendendo a equipe.",
    "A {team} anunciou a renovação de {name} para a próxima temporada.",
    "{name} e {team} chegaram a um acordo para extensão de contrato.",
    "Boa notícia para a {team}: {name} permanece na equipe.",
    "Continuidade garantida: {name} seguirá como piloto da {team}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// TRANSFERÊNCIA COMPLETADA
// ══════════════════════════════════════════════════════════════════════════════

pub const TRANSFER_TITULO: &[&str] = &[
    "{name} é o novo piloto da {team}!",
    "OFICIAL: {name} assina com {team}",
    "Transferência: {name} vai para {team}",
    "{team} anuncia {name}",
    "Confirmado: {name} na {team}",
];

pub const TRANSFER_TEXTO: &[&str] = &[
    "{name} foi oficialmente anunciado como novo piloto da {team}.",
    "A {team} confirmou a contratação de {name} para seu lineup.",
    "Grande movimentação no mercado: {name} defenderá a {team}.",
    "{name} assinou contrato com a {team} e fará parte da equipe.",
    "A {team} reforça seu time com a chegada de {name}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// TRANSFERÊNCIA REJEITADA
// ══════════════════════════════════════════════════════════════════════════════

pub const REJECTED_TITULO: &[&str] = &[
    "{name} recusa proposta da {team}",
    "Negociação fracassa: {name} e {team}",
    "{name} diz não à {team}",
    "{team} não consegue {name}",
    "Proposta rejeitada por {name}",
];

pub const REJECTED_TEXTO: &[&str] = &[
    "{name} decidiu recusar a proposta da {team} e buscar outras opções.",
    "As negociações entre {name} e {team} não avançaram.",
    "A {team} teve sua proposta recusada por {name}.",
    "{name} optou por não aceitar a oferta da {team}.",
    "Não houve acordo: {name} rejeitou a proposta da {team}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// ROOKIE CONTRATADO
// ══════════════════════════════════════════════════════════════════════════════

pub const ROOKIE_SIGNED_TITULO: &[&str] = &[
    "{team} aposta em {name}",
    "Rookie {name} assina com {team}",
    "{name}: nova promessa da {team}",
    "{team} contrata jovem {name}",
    "Estreante {name} vai para {team}",
];

pub const ROOKIE_SIGNED_TEXTO: &[&str] = &[
    "A {team} apostou no jovem talento {name} para integrar seu elenco.",
    "{name} foi contratado pela {team} após se destacar nas categorias de base.",
    "Nova aposta: a {team} anunciou o rookie {name} em seu lineup.",
    "O estreante {name} terá sua chance na {team}.",
    "A {team} deu uma vaga ao promissor {name}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// PROPOSTA PARA O JOGADOR
// ══════════════════════════════════════════════════════════════════════════════

pub const PROPOSAL_TITULO: &[&str] = &[
    "{team} quer {name}!",
    "Proposta recebida da {team}!",
    "Oferta: {team} quer {name}",
    "A {team} fez uma proposta a {name}!",
    "Oportunidade: {team} procura {name}",
];

pub const PROPOSAL_TEXTO: &[&str] = &[
    "A {team} enviou uma proposta de contrato para {name}. Agora a decisão está nas mãos do piloto.",
    "{name} recebeu uma proposta da {team} e precisa analisar os termos antes de decidir.",
    "A {team} demonstrou interesse em {name} com uma proposta formal.",
    "Nova oportunidade no mercado: a {team} quer contratar {name}.",
    "A {team} colocou uma oferta na mesa para {name}; o próximo passo depende da resposta do piloto.",
];

// ══════════════════════════════════════════════════════════════════════════════
// HIERARQUIA ATUALIZADA
// ══════════════════════════════════════════════════════════════════════════════

pub const HIERARCHY_TITULO: &[&str] = &[
    "{team} redefine hierarquia",
    "Mudança de piloto principal na {team}",
    "Nova ordem na {team}",
    "{team} ajusta prioridades",
    "Hierarquia alterada na {team}",
];

pub const HIERARCHY_TEXTO: &[&str] = &[
    "A {team} reorganizou a hierarquia de pilotos para a próxima fase.",
    "Mudanças internas na {team}: a hierarquia foi ajustada.",
    "A {team} definiu novas prioridades entre seus pilotos.",
    "Reorganização na {team}: hierarquia de pilotos foi alterada.",
    "A ordem dos pilotos na {team} foi revista pela direção.",
];

// ══════════════════════════════════════════════════════════════════════════════
// PRÉ-TEMPORADA COMPLETA
// ══════════════════════════════════════════════════════════════════════════════

pub const PRESEASON_TITULO: &[&str] = &[
    "Pré-temporada encerrada!",
    "Fim da pré-temporada",
    "Equipes prontas para a temporada",
    "Preparações concluídas",
    "Pré-temporada chega ao fim",
];

pub const PRESEASON_TEXTO: &[&str] = &[
    "A pré-temporada chegou ao fim. As equipes estão prontas para a primeira corrida!",
    "Todos os preparativos foram concluídos. A temporada está prestes a começar!",
    "Fim da pré-temporada: os grids estão definidos e a ação começa em breve.",
    "As equipes finalizaram suas preparações. É hora de competir!",
    "Pré-temporada encerrada com sucesso. A primeira corrida se aproxima!",
];

// ══════════════════════════════════════════════════════════════════════════════
// JOGADOR ASSINA
// ══════════════════════════════════════════════════════════════════════════════

pub const PLAYER_SIGN_TITULO: &[&str] = &[
    "{name} assinou com a {team}!",
    "Contrato fechado: {name} é da {team}!",
    "Bem-vindo à {team}!",
    "Oficial: {name} na {team}!",
    "Acordo selado com a {team}!",
];

pub const PLAYER_SIGN_TEXTO: &[&str] = &[
    "{name} assinou com a {team} como {role} para a temporada {temp} na {cat}.",
    "{name} é oficialmente o {role} da {team} na {cat} para a temporada {temp}.",
    "O contrato de {name} com a {team} foi fechado: o piloto será {role} na {cat} em {temp}.",
    "Bem-vindo à {team}: {name} disputará a {cat} como {role} na temporada {temp}.",
    "Acordo selado: {name} defenderá a {team} como {role} na {cat}, temporada {temp}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// JOGADOR REJEITA
// ══════════════════════════════════════════════════════════════════════════════

pub const PLAYER_REJECT_TITULO: &[&str] = &[
    "{name} recusou a {team}",
    "Proposta da {team} rejeitada",
    "{name} disse não à {team}",
    "Decisão tomada: {name} rejeita a {team}",
    "{team} recebe a recusa de {name}",
];

pub const PLAYER_REJECT_TEXTO: &[&str] = &[
    "{name} decidiu recusar a proposta da {team} para a temporada {temp}.",
    "A oferta da {team} foi rejeitada, e {name} segue avaliando outras oportunidades.",
    "{name} optou por não assinar com a {team} para {temp}.",
    "A {team} foi informada de que {name} não aceitaria a proposta.",
    "Proposta da {team} recusada; o mercado continua aberto para {name}.",
];
