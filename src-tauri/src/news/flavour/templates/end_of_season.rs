// ══════════════════════════════════════════════════════════════════════════════
// APOSENTADORIA
// ══════════════════════════════════════════════════════════════════════════════

pub const APOSENTA_TITULO: &[&str] = &[
    "{name} anuncia aposentadoria",
    "Adeus às pistas: {name} se aposenta",
    "{name} encerra carreira",
    "Aposentadoria de {name}",
    "{name} diz adeus ao automobilismo",
];

pub const APOSENTA_TEXTO: &[&str] = &[
    "{name}, aos {age} anos, anunciou sua aposentadoria. {reason}",
    "Fim de uma era: {name} se despede das pistas aos {age} anos. {reason}",
    "Aos {age} anos, {name} decidiu encerrar sua carreira. {reason}",
    "{name} pendura o capacete aos {age} anos. {reason}",
    "Carreira encerrada: {name}, {age} anos, se aposenta. {reason}",
];

// ══════════════════════════════════════════════════════════════════════════════
// PROMOÇÃO
// ══════════════════════════════════════════════════════════════════════════════

pub const PROMOCAO_TITULO: &[&str] = &[
    "{team} promovida para {cat}!",
    "PROMOÇÃO: {team} sobe para {cat}",
    "{team} conquista vaga na {cat}",
    "Subiu! {team} vai para {cat}",
    "{team} garante promoção à {cat}",
];

pub const PROMOCAO_TEXTO: &[&str] = &[
    "A {team} foi promovida da {from} para a {cat}. {reason}",
    "Grande conquista: a {team} sobe da {from} para a {cat}. {reason}",
    "Promoção garantida: {team} deixa a {from} rumo à {cat}. {reason}",
    "A {team} se despede da {from} e chega à {cat}. {reason}",
    "Subiu! A {team} vai da {from} para a {cat}. {reason}",
];

// ══════════════════════════════════════════════════════════════════════════════
// REBAIXAMENTO
// ══════════════════════════════════════════════════════════════════════════════

pub const REBAIXA_TITULO: &[&str] = &[
    "{team} rebaixada para {cat}",
    "REBAIXAMENTO: {team} cai para {cat}",
    "{team} desce para {cat}",
    "Queda: {team} vai para {cat}",
    "{team} sofre rebaixamento para {cat}",
];

pub const REBAIXA_TEXTO: &[&str] = &[
    "A {team} foi rebaixada da {from} para a {cat}. {reason}",
    "Momento difícil: a {team} cai da {from} para a {cat}. {reason}",
    "Rebaixamento confirmado: {team} deixa a {from} para a {cat}. {reason}",
    "A {team} não conseguiu evitar a queda da {from} para a {cat}. {reason}",
    "Desceu: a {team} vai da {from} para a {cat}. {reason}",
];

// ══════════════════════════════════════════════════════════════════════════════
// PILOTOS LIBERADOS
// ══════════════════════════════════════════════════════════════════════════════

pub const FREED_AI_TITULO: &[&str] = &[
    "{name} fica sem equipe",
    "{name} liberado pelo time",
    "Sem vaga: {name} disponível",
    "{name} busca nova equipe",
    "Mercado: {name} livre",
];

pub const FREED_AI_TEXTO: &[&str] = &[
    "{name} ficou sem equipe e está disponível no mercado. {reason}",
    "Após mudanças, {name} está livre para negociar. {reason}",
    "{name} busca nova oportunidade após ser liberado. {reason}",
    "Piloto livre: {name} procura uma vaga para a próxima temporada. {reason}",
    "{name} está no mercado após ficar sem equipe. {reason}",
];

pub const FREED_PLAYER_TITULO: &[&str] = &[
    "{name} precisa de uma nova equipe!",
    "Atenção: {name} ficou sem vaga",
    "{name} busca um novo contrato",
    "Mudança no grid deixa {name} sem equipe",
    "{name} está livre no mercado!",
];

pub const FREED_PLAYER_TEXTO: &[&str] = &[
    "Devido às mudanças, {name} precisa encontrar uma nova equipe. {reason}",
    "A situação de {name} mudou: é hora de buscar um novo contrato. {reason}",
    "{name} está livre no mercado e pode negociar com as equipes disponíveis. {reason}",
    "Momento decisivo para {name}: definir a próxima equipe. {reason}",
    "As mudanças deixaram {name} sem vaga; agora é hora de negociar. {reason}",
];

// ══════════════════════════════════════════════════════════════════════════════
// ROOKIES
// ══════════════════════════════════════════════════════════════════════════════

pub const ROOKIE_GENIO_TITULO: &[&str] = &[
    "Prodígio {name} chega ao grid!",
    "Gênio da pilotagem: {name}",
    "{name}: o novo fenômeno",
    "Talento raro: {name} estreia",
    "{name}, o prodígio, está aqui!",
];

pub const ROOKIE_GENIO_TEXTO: &[&str] = &[
    "{name}, {age} anos, é considerado um prodígio e chega com grandes expectativas.",
    "Aos {age} anos, {name} é visto como um dos maiores talentos de sua geração.",
    "O jovem gênio {name}, {age} anos, promete agitar o grid.",
    "{name} ({age} anos) é a grande promessa que todos estão de olho.",
    "Com apenas {age} anos, {name} já é chamado de fenômeno.",
];

pub const ROOKIE_TALENTO_TITULO: &[&str] = &[
    "Talento {name} entra no grid",
    "{name}: jovem promessa",
    "Estreante {name} chama atenção",
    "{name} começa carreira",
    "Revelação: {name} estreia",
];

pub const ROOKIE_TALENTO_TEXTO: &[&str] = &[
    "{name}, {age} anos, é uma jovem promessa que desperta interesse das equipes.",
    "O talentoso {name} ({age} anos) faz sua estreia no grid.",
    "Aos {age} anos, {name} começa sua jornada como piloto profissional.",
    "{name} ({age}) chega ao grid como uma das revelações da temporada.",
    "Jovem talento: {name}, {age} anos, inicia sua carreira.",
];

pub const ROOKIE_NORMAL_TITULO: &[&str] = &[
    "{name} estreia no grid",
    "Novo piloto: {name}",
    "{name} começa carreira",
    "Estreante {name} chega",
    "{name} entra para o grid",
];

pub const ROOKIE_NORMAL_TEXTO: &[&str] = &[
    "{name}, {age} anos, faz sua estreia como piloto profissional.",
    "Aos {age} anos, {name} começa sua carreira no automobilismo.",
    "{name} ({age}) inicia sua trajetória como piloto.",
    "Bem-vindo ao grid: {name}, {age} anos, estreia nesta temporada.",
    "O piloto {name} ({age} anos) faz sua primeira temporada.",
];

// ══════════════════════════════════════════════════════════════════════════════
// LICENÇAS
// ══════════════════════════════════════════════════════════════════════════════

pub const LICENSE_PLAYER_TITULO: &[&str] = &[
    "{name} conquistou licença nível {n}!",
    "Nova licença: nível {n}!",
    "Licença {cat} desbloqueada!",
    "{name} alcança licença nível {n}!",
    "Licença alcançada por {name}: nível {n}!",
];

pub const LICENSE_PLAYER_TEXTO: &[&str] = &[
    "{name} conquistou a licença nível {n} para a {cat}, abrindo novas oportunidades.",
    "A licença de {name} na {cat} subiu para o nível {n}.",
    "Com a licença nível {n}, {name} agora pode competir na {cat}.",
    "Nível {n} de licença alcançado na {cat}: {name} segue evoluindo.",
    "A licença nível {n} foi liberada para {name} na {cat}.",
];

pub const LICENSE_GROUP_TITULO: &[&str] = &[
    "{n} pilotos conquistam novas licenças",
    "Licenças concedidas a {n} pilotos",
    "Grupo de {n} pilotos avança",
    "{n} pilotos sobem de nível",
    "Novas licenças para {n} pilotos",
];

pub const LICENSE_GROUP_TEXTO: &[&str] = &[
    "{n} pilotos conquistaram novas licenças nesta temporada: {names}.",
    "As licenças foram concedidas a {n} pilotos, incluindo: {names}.",
    "Grupo de {n} pilotos avançou de nível: {names}.",
    "{names} estão entre os {n} pilotos que subiram de licença.",
    "Nesta temporada, {n} pilotos alcançaram novas licenças: {names}.",
];

// ══════════════════════════════════════════════════════════════════════════════
// EVOLUÇÃO / DECLÍNIO
// ══════════════════════════════════════════════════════════════════════════════

pub const GROWER_TITULO: &[&str] = &[
    "{name} teve grande evolução",
    "Evolução notável de {name}",
    "{name} cresceu muito esta temporada",
    "Destaque: evolução de {name}",
    "{name} mostrou grande progresso",
];

pub const GROWER_TEXTO: &[&str] = &[
    "{name} apresentou uma evolução impressionante ao longo da temporada.",
    "A temporada foi de grande crescimento para {name}.",
    "{name} surpreendeu com sua evolução e maturidade.",
    "Analistas destacam o progresso de {name} nesta temporada.",
    "{name} evoluiu significativamente e promete ainda mais.",
];

pub const DECLINER_TITULO: &[&str] = &[
    "{name} teve temporada difícil",
    "Declínio de {name} preocupa",
    "{name} não manteve nível",
    "Temporada complicada para {name}",
    "{name} apresentou queda de rendimento",
];

pub const DECLINER_TEXTO: &[&str] = &[
    "{name} teve uma temporada abaixo das expectativas e apresentou declínio.",
    "O desempenho de {name} caiu ao longo da temporada.",
    "{name} não conseguiu manter o nível e preocupa para o futuro.",
    "Analistas notam o declínio de {name} nesta temporada.",
    "Temporada difícil para {name}, que viu seu rendimento cair.",
];
