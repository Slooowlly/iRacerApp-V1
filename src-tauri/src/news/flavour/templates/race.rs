//! Templates para noticias secundarias e destaques de corrida.
//!
//! Placeholders: {name}, {track}, {n}, {pos}, {grid}, {pts}, {cat}, {team}

// ── Position gainer ──────────────────────────────────────────────────────────

pub const GAINER_TITULO: &[&str] = &[
    "{name} ganha {n} posicoes!",
    "Remontada: {name} avanca {n} posicoes",
    "{name} escala {n} posicoes no grid",
    "Grande recuperacao de {name}: +{n} posicoes",
    "{name} sobe {n} posicoes na corrida",
    "Corrida de recuperacao: {name} ganha {n} posicoes",
    "{name} avanca {n} posicoes e impressiona",
    "+{n} posicoes para {name} na corrida",
    "{name} faz corrida agressiva e ganha {n} posicoes",
    "Destaque da corrida: {name} com +{n} posicoes",
];

pub const GAINER_TEXTO: &[&str] = &[
    "{name} avancou {n} posicoes durante a corrida.",
    "Performance impressionante de {name}, que subiu {n} posicoes.",
    "{name} fez uma corrida de recuperacao e ganhou {n} posicoes.",
    "{name} mostrou garra ao avancar {n} posicoes ao longo da corrida.",
    "Com ultrapassagens decisivas, {name} escalou {n} posicoes na corrida.",
    "{name} nao se conformou com a posicao de largada e ganhou {n} posicoes.",
];

// ══════════════════════════════════════════════════════════════════════════════
// VENCEDOR DA CORRIDA
// ══════════════════════════════════════════════════════════════════════════════

pub const WINNER_TITULO: &[&str] = &[
    "{name} vence em {track}!",
    "Vitória de {name} em {track}",
    "{name} triunfa no GP de {track}",
    "Domínio de {name} em {track}",
    "{name} conquista vitória em {track}",
];

pub const WINNER_TEXTO: &[&str] = &[
    "{name} cruzou a linha de chegada em primeiro lugar no GP de {track}, conquistando {pts} pontos para o campeonato.",
    "Excelente performance de {name} que dominou a corrida em {track} e somou {pts} pontos importantes.",
    "Com uma corrida impecável, {name} garantiu a vitória em {track} e adicionou {pts} pontos à sua conta.",
    "{name} mostrou toda sua habilidade ao vencer em {track}, levando {pts} pontos para casa.",
    "O piloto {name} foi imbatível em {track}, conquistando mais {pts} pontos no campeonato.",
];

pub const WINNER_PLAYER_TITULO: &[&str] = &[
    "{name} vence em {track}!",
    "Vitória épica de {name} em {track}!",
    "Triunfo de {name} no GP de {track}!",
    "{name} domina em {track}!",
    "Que corrida! {name} vence em {track}!",
];

pub const WINNER_PLAYER_TEXTO: &[&str] = &[
    "{name} cruzou a linha de chegada em primeiro e conquistou {pts} pontos preciosos.",
    "Que performance incrível de {name}! A vitória em {track} adiciona {pts} pontos ao campeonato.",
    "{name} dominou a corrida do início ao fim e garantiu {pts} pontos com esta vitória.",
    "Uma corrida memorável: a vitória de {name} em {track} rende {pts} pontos importantes.",
    "{name} ficou com o lugar mais alto do pódio e somou {pts} pontos com esta vitória.",
];

pub const WINNER_COMEBACK_TITULO: &[&str] = &[
    "{name} vem de trás e vence em {track}!",
    "Remontada histórica de {name} em {track}",
    "{name} supera adversidades e triunfa",
    "De P{grid} à vitória: {name} brilha em {track}",
    "Corrida de recuperação perfeita de {name}",
];

pub const WINNER_COMEBACK_TEXTO: &[&str] = &[
    "Largando em P{grid}, {name} fez uma corrida espetacular de recuperação para vencer em {track}.",
    "{name} não se deixou abater pela posição de largada e conquistou uma vitória improvável.",
    "Uma das melhores corridas da temporada: {name} saiu de P{grid} para o lugar mais alto do pódio.",
    "Nada segurou {name} hoje. Mesmo largando em P{grid}, o piloto dominou e venceu em {track}.",
    "Performance memorável de {name}, que transformou uma largada difícil em vitória.",
];

// ══════════════════════════════════════════════════════════════════════════════
// PÓDIO (2º e 3º)
// ══════════════════════════════════════════════════════════════════════════════

pub const PODIUM_TITULO: &[&str] = &[
    "{name} conquista P{pos} em {track}",
    "Pódio para {name} em {track}",
    "{name} termina em {pos}º no GP de {track}",
    "Resultado sólido de {name} em {track}",
    "{name} garante pódio em {track}",
];

pub const PODIUM_TEXTO: &[&str] = &[
    "{name} fez uma corrida consistente e garantiu o {pos}º lugar em {track}, somando {pts} pontos.",
    "Mais um pódio para {name}, que terminou em P{pos} e levou {pts} pontos importantes.",
    "Resultado positivo para {name} em {track}: {pos}º lugar e {pts} pontos no campeonato.",
    "Com estratégia e habilidade, {name} assegurou a {pos}ª posição e {pts} pontos.",
    "{name} manteve a consistência e subiu ao pódio em {pos}º, conquistando {pts} pontos.",
];

// ══════════════════════════════════════════════════════════════════════════════
// RESULTADO DO JOGADOR
// ══════════════════════════════════════════════════════════════════════════════

pub const JOGADOR_TITULO: &[&str] = &[
    "{name} termina em P{pos} em {track}",
    "Resultado de {name}: {pos}º lugar no GP",
    "Fim de corrida para {name}: P{pos}",
    "{name} fecha a prova em {pos}ª posição",
    "{name} cruza a linha em {pos}º lugar",
];

pub const JOGADOR_TEXTO: &[&str] = &[
    "Largando de P{grid}, {name} cruzou a linha em P{pos} e conquistou {pts} pontos.",
    "Da {grid}ª posição de largada até o {pos}º lugar: {pts} pontos para {name}.",
    "A estratégia de {name} rendeu o {pos}º lugar partindo de P{grid}. Total: {pts} pontos.",
    "{name} completou a corrida em P{pos} (largada: P{grid}). Pontuação: {pts}.",
    "De P{grid} para P{pos}: {name} marcou {pts} pontos nesta corrida.",
];

// ══════════════════════════════════════════════════════════════════════════════
// ULTRAPASSAGENS NOTÁVEIS
// ══════════════════════════════════════════════════════════════════════════════

pub const OVERTAKE_TITULO: &[&str] = &[
    "{name} faz corrida de recuperação",
    "Show de ultrapassagens de {name}",
    "{name} ganha {n} posições em {track}",
    "Corrida agressiva de {name}",
    "{name} impressiona com {n} posições ganhas",
];

pub const OVERTAKE_TEXTO: &[&str] = &[
    "Largando de P{grid}, {name} terminou em P{pos} após ganhar {n} posições durante a corrida.",
    "{name} mostrou toda sua habilidade ao subir {n} posições, chegando em P{pos}.",
    "Uma das melhores performances do dia: {name} avançou {n} posições até o {pos}º lugar.",
    "Corrida de gala de {name}, que transformou a {grid}ª posição em {pos}º lugar.",
    "{name} não parou de ultrapassar: {n} posições ganhas para terminar em P{pos}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// CAMPEÃO / TÍTULOS
// ══════════════════════════════════════════════════════════════════════════════

pub const CHAMPION_TITULO: &[&str] = &[
    "{name} é CAMPEÃO da {cat}!",
    "TÍTULO para {name} na {cat}!",
    "{name} conquista o campeonato da {cat}",
    "Campeão! {name} vence a {cat}",
    "{name} coroa temporada com título da {cat}",
];

pub const CHAMPION_TEXTO: &[&str] = &[
    "{name} garantiu matematicamente o título da {cat} com {pts} pontos, uma temporada brilhante!",
    "Com uma campanha consistente, {name} conquistou o campeonato da {cat} somando {pts} pontos.",
    "O título da {cat} é de {name}! O piloto encerra a disputa com {pts} pontos.",
    "Ninguém mais pode alcançá-lo: {name} é o novo campeão da {cat} com {pts} pontos.",
    "Temporada perfeita coroada com o título: {name} dominou a {cat} com {pts} pontos.",
];

pub const CHAMPION_PLAYER_TITULO: &[&str] = &[
    "{name} é campeão da {cat}!",
    "Título conquistado por {name} na {cat}!",
    "Campeão! {name} vence a {cat}!",
    "{name} leva o campeonato da {cat}!",
    "O título da {cat} fica com {name}!",
];

pub const CHAMPION_PLAYER_TEXTO: &[&str] = &[
    "{name} conquistou o título da {cat} com {pts} pontos em uma temporada inesquecível.",
    "{name} é o novo campeão da {cat}! {pts} pontos e muito suor levaram a esse momento.",
    "Com {pts} pontos, {name} dominou a {cat} e entrou para a história com o título.",
    "A jornada de {name} na {cat} culminou neste título com {pts} pontos.",
    "A coroa ficou com {name}: {pts} pontos e o título da {cat} coroam a temporada.",
];

// ══════════════════════════════════════════════════════════════════════════════
// CONSTRUTORES
// ══════════════════════════════════════════════════════════════════════════════

pub const CONSTRUCTOR_CHAMPION_TITULO: &[&str] = &[
    "{team} conquista título de construtores!",
    "Campeonato de Construtores é da {team}!",
    "{team} vence entre as equipes",
    "Título de equipes para {team}",
    "{team} é campeã de construtores!",
];

pub const CONSTRUCTOR_CHAMPION_TEXTO: &[&str] = &[
    "A {team} conquistou o campeonato de construtores da {cat} com {pts} pontos.",
    "Com trabalho de equipe impecável, a {team} garantiu o título de construtores com {pts} pontos.",
    "O campeonato de construtores da {cat} fica com a {team}, que somou {pts} pontos.",
    "Equipe campeã! A {team} dominou a {cat} com {pts} pontos no campeonato de construtores.",
    "A {team} encerra a temporada no topo entre os construtores, com {pts} pontos.",
];
