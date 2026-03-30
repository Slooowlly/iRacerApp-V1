//! Templates de titulo e texto para noticias do vencedor da corrida.
//!
//! Placeholders: {name}, {track}, {grid}, {n} (milestone/rounds), {rival}, {team}

pub const LIMPA_TITULO: &[&str] = &[
    "{name} vence em {track}",
    "Vitoria de {name} em {track}",
    "{name} conquista a vitoria em {track}",
    "{name} triunfa em {track}",
    "{name} leva a melhor em {track}",
    "Vitoria para {name} em {track}",
    "{name} sobe ao topo do podio em {track}",
    "{name} cruza a linha na frente em {track}",
    "E de {name}! Vitoria em {track}",
    "{name} confirma boa fase e vence em {track}",
    "{name} fecha a corrida na frente em {track}",
    "{name} domina e vence em {track}",
];

pub const LIMPA_TEXTO: &[&str] = &[
    "{name} venceu a corrida em {track} partindo de P{grid}.",
    "Largando de P{grid}, {name} conquistou a vitoria em {track}.",
    "{name} fez uma corrida solida e venceu em {track} saindo de P{grid}.",
    "Partindo de P{grid}, {name} cruzou a linha em primeiro em {track}.",
    "Com uma performance consistente, {name} venceu em {track} largando de P{grid}.",
    "{name} levou a melhor em {track} apos largar da posicao {grid}.",
    "Saindo de P{grid}, {name} controlou a corrida e venceu em {track}.",
    "{name} mostrou ritmo forte e venceu a corrida em {track}, largando de P{grid}.",
];

pub const TURBULENTA_TITULO: &[&str] = &[
    "{name} vence corrida movimentada em {track}",
    "{name} sobrevive ao caos e vence em {track}",
    "Corrida agitada em {track}: {name} vence",
    "{name} supera corrida turbulenta em {track}",
    "Em corrida movimentada, {name} leva a vitoria em {track}",
    "{name} navega o caos e vence em {track}",
    "Corrida agitada: {name} sai vencedor em {track}",
    "{name} encontra caminho na confusao e vence em {track}",
    "Prova movimentada em {track} termina com vitoria de {name}",
    "{name} se destaca em corrida turbulenta em {track}",
];

pub const TURBULENTA_TEXTO: &[&str] = &[
    "Em uma corrida cheia de incidentes, {name} partiu de P{grid} e conquistou a vitoria em {track}.",
    "{name} venceu uma corrida movimentada em {track}, largando de P{grid}.",
    "A corrida em {track} foi marcada por incidentes, mas {name} manteve o foco e venceu saindo de P{grid}.",
    "Largando de P{grid}, {name} sobreviveu a uma prova agitada e cruzou a linha na frente em {track}.",
    "Apesar da corrida conturbada, {name} manteve a calma e venceu em {track} partindo de P{grid}.",
    "{name} navegou uma corrida cheia de acao e venceu em {track}, saindo de P{grid}.",
];

pub const CAOTICA_TITULO: &[&str] = &[
    "Prova caotica: {name} sobrevive em {track}",
    "Caos em {track}: {name} sai vencedor",
    "{name} vence em meio ao caos de {track}",
    "Corrida caotica em {track} termina com vitoria de {name}",
    "Caos total em {track}: {name} herda a vitoria",
    "{name} sobrevive a corrida caotica em {track}",
    "Destruicao em {track}: {name} e o ultimo de pe",
    "{name} emerge do caos para vencer em {track}",
    "Corrida de eliminacao em {track}: {name} triunfa",
    "Carnificina em {track}: vitoria de {name}",
];

pub const CAOTICA_TEXTO: &[&str] = &[
    "Em uma corrida marcada por abandonos e incidentes, {name} partiu de P{grid} e conseguiu vencer em {track}.",
    "{name} sobreviveu a uma prova caotica em {track} e cruzou a linha em primeiro, largando de P{grid}.",
    "A corrida em {track} foi um campo de batalha, mas {name} manteve o carro inteiro e venceu saindo de P{grid}.",
    "Com varios pilotos abandonando, {name} aproveitou a oportunidade e venceu em {track} partindo de P{grid}.",
    "Largando de P{grid}, {name} foi um dos poucos a completar a corrida e saiu vitorioso em {track}.",
    "O caos dominou {track}, mas {name} manteve a compostura e garantiu a vitoria, partindo de P{grid}.",
];

pub const MILESTONE_TITULO: &[&str] = &[
    "{name} alcanca {n} vitorias na carreira!",
    "Historico: {name} chega a {n} vitorias!",
    "{n}a vitoria para {name}!",
    "Marco historico: {name} soma {n} vitorias",
    "{name} celebra sua {n}a vitoria em {track}",
];

pub const MILESTONE_TEXTO: &[&str] = &[
    "{name} alcancou a marca de {n} vitorias na carreira ao vencer em {track}, largando de P{grid}.",
    "Com a vitoria em {track}, {name} chega a {n} vitorias na carreira. Um marco impressionante.",
    "{name} celebrou sua {n}a vitoria em {track}, partindo de P{grid}. Uma conquista para a historia.",
    "A vitoria em {track} marcou a {n}a conquista de {name} na carreira, largando de P{grid}.",
];

pub const REDENCAO_TITULO: &[&str] = &[
    "Redencao! {name} vence em {track} apos drama anterior",
    "{name} supera o passado e vence em {track}",
    "De volta por cima: {name} triunfa em {track}",
    "{name} exorciza os demonios de {track}",
    "Redencao em {track}: {name} transforma dor em vitoria",
    "{name} acerta as contas com {track} e vence",
];

pub const REDENCAO_TEXTO: &[&str] = &[
    "{name} venceu em {track} partindo de P{grid}, superando o abandono sofrido anteriormente nesta mesma pista.",
    "Apos um abandono marcante em {track}, {name} voltou para vencer. Largou de P{grid} e cruzou a linha na frente.",
    "{name} transformou um historico negativo em {track} em vitoria, largando de P{grid} e dominando a corrida.",
    "A pista de {track} ja havia sido cenario de drama para {name}, mas desta vez ele largou de P{grid} e saiu vitorioso.",
];

pub const JEJUM_TITULO: &[&str] = &[
    "{name} quebra jejum de {n} corridas em {track}!",
    "Fim da seca: {name} volta a vencer apos {n} corridas",
    "{name} reencontra a vitoria depois de {n} provas",
    "Jejum encerrado: {name} vence em {track}",
    "{name} volta ao topo apos {n} corridas sem vencer",
    "Fim do jejum! {name} triunfa em {track}",
];

pub const JEJUM_TEXTO: &[&str] = &[
    "{name} encerrou um jejum de {n} corridas ao vencer em {track}, largando de P{grid}.",
    "Depois de {n} corridas sem vitoria, {name} finalmente voltou ao lugar mais alto do podio em {track}, saindo de P{grid}.",
    "{name} quebrou uma seca de {n} provas e conquistou a vitoria em {track}, partindo de P{grid}.",
    "Largando de P{grid}, {name} encerrou {n} corridas de espera e voltou a vencer em {track}.",
];

pub const CASA_TITULO: &[&str] = &[
    "{name} vence em casa em {track}!",
    "Alegria da torcida: {name} triunfa em {track}",
    "{name} presenteia a torcida local em {track}",
    "Vitoria em casa para {name} em {track}!",
    "{name} brilha diante da torcida em {track}",
    "A torcida vai ao delirio: {name} vence em {track}",
];

pub const CASA_TEXTO: &[&str] = &[
    "{name} venceu diante de sua torcida em {track}, largando de P{grid}. Emocao garantida.",
    "Correndo em casa, {name} fez a alegria dos fas ao vencer em {track} partindo de P{grid}.",
    "{name} nao decepcionou a torcida local e venceu em {track}, saindo de P{grid}.",
    "Largando de P{grid}, {name} dominou a corrida em {track} e celebrou a vitoria com sua torcida.",
];

pub const UNDERDOG_TITULO: &[&str] = &[
    "Zebra! {name} vence com carro inferior em {track}",
    "{name} supera a diferenca de equipamento e vence em {track}",
    "Talento puro: {name} vence sem o melhor carro em {track}",
    "{name} prova que piloto faz diferenca em {track}",
    "Surpresa: {name} desbanca os favoritos em {track}",
    "{name} desafia as probabilidades e vence em {track}",
];

pub const UNDERDOG_TEXTO: &[&str] = &[
    "Mesmo sem o carro mais competitivo, {name} largou de P{grid} e conquistou a vitoria em {track}.",
    "{name} mostrou que talento supera equipamento ao vencer em {track}, partindo de P{grid}.",
    "Com um carro considerado inferior, {name} surpreendeu e venceu em {track} saindo de P{grid}.",
    "Largando de P{grid}, {name} desafiou todas as expectativas e venceu em {track} com um carro abaixo dos rivais.",
];

pub const COLISAO_SOBREVIVEU_TITULO: &[&str] = &[
    "{name} sobrevive a toque e vence em {track}",
    "De colisao a vitoria: {name} em {track}",
    "{name} se envolve em toque e ainda vence em {track}",
    "Resiliencia: {name} supera contato e vence em {track}",
    "{name} nao desiste apos toque e leva a vitoria em {track}",
    "Toque nao impede {name} de vencer em {track}",
];

pub const COLISAO_SOBREVIVEU_TEXTO: &[&str] = &[
    "{name} se envolveu em um incidente durante a corrida mas manteve o foco e venceu em {track}, largando de P{grid}.",
    "Apesar de um contato na corrida, {name} conseguiu se recuperar e cruzar a linha em primeiro em {track}, saindo de P{grid}.",
    "{name} sobreviveu a um toque e mostrou resiliencia ao vencer em {track}, partindo de P{grid}.",
    "Largando de P{grid}, {name} superou um incidente em pista e conquistou a vitoria em {track}.",
];

pub const GRAND_SLAM_TITULO: &[&str] = &[
    "Dominio total: {name} faz o Grand Slam em {track}",
    "{name} conquista o Grand Slam em {track}!",
    "Perfeito: {name} domina do inicio ao fim em {track}",
    "Grand Slam para {name} em {track}!",
    "{name} faz pole, volta mais rapida e vence em {track}",
];

pub const GRAND_SLAM_TEXTO: &[&str] = &[
    "{name} completou o Grand Slam em {track}: pole position, vitoria e volta mais rapida.",
    "Dominio absoluto de {name} em {track}. Pole, vitoria e volta mais rapida partindo de P{grid}.",
    "{name} nao deu chances aos rivais em {track}: pole, vitoria e melhor volta.",
    "Performance perfeita de {name} em {track}. Conquistou pole, vitoria e volta mais rapida.",
];

pub const PHOTO_FINISH_TITULO: &[&str] = &[
    "Photo finish: {name} vence por milesimos em {track}!",
    "{name} vence no limite em {track}!",
    "Final eletrizante: {name} por um triz em {track}",
    "No fio da navalha: {name} vence em {track}",
    "Diferenca minima: {name} leva a melhor em {track}",
    "{name} vence por margem infima em {track}!",
];

pub const PHOTO_FINISH_TEXTO: &[&str] = &[
    "{name} venceu por uma diferenca minima em {track}, em um final que tirou o folego de todos. Largou de P{grid}.",
    "A vitoria de {name} em {track} foi decidida por milesimos. Largando de P{grid}, ele segurou a pressao ate o fim.",
    "{name} cruzou a linha na frente por uma margem quase imperceptivel em {track}, partindo de P{grid}.",
    "Largando de P{grid}, {name} protagonizou um final dramatico e venceu por milesimos em {track}.",
];

pub const DOMINANTE_TITULO: &[&str] = &[
    "{name} domina e vence com folga em {track}",
    "Passeio de {name} em {track}",
    "Dominio: {name} vence sem ser ameacado em {track}",
    "{name} nao da chance e vence facil em {track}",
    "Vitoria tranquila para {name} em {track}",
    "{name} controla a corrida e vence com margem em {track}",
];

pub const DOMINANTE_TEXTO: &[&str] = &[
    "{name} dominou a corrida de ponta a ponta em {track} e venceu com grande margem, largando de P{grid}.",
    "Largando de P{grid}, {name} abriu vantagem cedo e nunca foi ameacado em {track}.",
    "{name} fez uma corrida impecavel em {track} e cruzou a linha com folga sobre o segundo colocado, saindo de P{grid}.",
    "Com uma performance dominante, {name} venceu com margem confortavel em {track}, partindo de P{grid}.",
];

pub const COMEBACK_TITULO: &[&str] = &[
    "Remontada historica: {name} vence vindo de P{grid}",
    "{name} sai de P{grid} e vence em {track}!",
    "Incrivel: {name} escala o grid e vence em {track}",
    "{name} faz corrida de recuperacao e vence em {track}",
    "De P{grid} para a vitoria: {name} em {track}",
    "Remontada: {name} supera largada ruim e vence em {track}",
    "{name} mostra garra e vence vindo de P{grid} em {track}",
    "Recuperacao impressionante de {name} em {track}",
];

pub const COMEBACK_TEXTO: &[&str] = &[
    "{name} largou de P{grid} e escalou o grid ate conquistar a vitoria em {track}. Remontada impressionante.",
    "Partindo de P{grid}, {name} fez uma corrida de recuperacao memoravel e venceu em {track}.",
    "{name} transformou uma largada em P{grid} em vitoria em {track}, ultrapassando rivais ao longo da corrida.",
    "Largando de P{grid}, {name} nao se abateu e foi escalando posicoes ate chegar a vitoria em {track}.",
    "A remontada de {name} em {track} foi uma das mais impressionantes da temporada. Saiu de P{grid} e venceu.",
];

pub const CHUVA_TITULO: &[&str] = &[
    "{name} brilha na chuva e vence em {track}",
    "Mestre da chuva: {name} domina pista molhada em {track}",
    "{name} danca na chuva e vence em {track}",
    "Chuva em {track}: {name} se destaca e vence",
    "{name} mostra habilidade na chuva em {track}",
    "Pista molhada, vitoria de {name} em {track}",
];

pub const CHUVA_TEXTO: &[&str] = &[
    "{name} mostrou habilidade excepcional na pista molhada de {track} e venceu a corrida, largando de P{grid}.",
    "A chuva em {track} nao impediu {name} de dominar a corrida e conquistar a vitoria partindo de P{grid}.",
    "{name} se adaptou perfeitamente as condicoes de chuva em {track} e venceu com autoridade, saindo de P{grid}.",
    "Largando de P{grid} sob chuva, {name} fez uma corrida brilhante e venceu em {track}.",
];

pub const RIVAL_TITULO: &[&str] = &[
    "{name} supera {rival} e vence em {track}",
    "Duelo vencido: {name} leva a melhor sobre {rival} em {track}",
    "{name} derrota rival e vence em {track}",
    "{name} deixa {rival} para tras e vence em {track}",
    "Rivalidade: {name} supera {rival} com vitoria em {track}",
    "{name} vence e manda recado para {rival} em {track}",
];

pub const RIVAL_TEXTO: &[&str] = &[
    "{name} venceu em {track} partindo de P{grid} e, de quebra, deixou seu rival {rival} para tras.",
    "Largando de P{grid}, {name} nao so venceu em {track} como tambem superou {rival} no confronto direto.",
    "A vitoria de {name} em {track} teve sabor especial: alem de cruzar a linha em primeiro saindo de P{grid}, ele bateu o rival {rival}.",
    "{name} venceu em {track} e reafirmou superioridade sobre {rival}, largando de P{grid}.",
];

pub const ABERTURA_TITULO: &[&str] = &[
    "Temporada abre em {track}: {name} vence",
    "{name} inaugura a temporada com vitoria em {track}",
    "Abertura em {track}: {name} larga na frente",
    "{name} comeca a temporada vencendo em {track}",
    "Primeira corrida, primeira vitoria: {name} em {track}",
];

pub const ABERTURA_TEXTO: &[&str] = &[
    "{name} abriu a temporada com o pe direito ao vencer em {track}, largando de P{grid}.",
    "Na abertura da temporada em {track}, {name} partiu de P{grid} e conquistou a vitoria.",
    "{name} inaugurou a temporada com uma vitoria em {track}, saindo de P{grid}.",
];

pub const FINAL_TITULO: &[&str] = &[
    "Grande Final: {name} vence em {track}",
    "{name} vence a grande decisao em {track}",
    "Final eletrizante: {name} triunfa em {track}",
    "{name} encerra a temporada com vitoria em {track}",
    "Ultima corrida: {name} vence em {track}",
];

pub const FINAL_TEXTO: &[&str] = &[
    "{name} encerrou a temporada com chave de ouro ao vencer a ultima corrida em {track}, largando de P{grid}.",
    "Na grande final em {track}, {name} partiu de P{grid} e conquistou a vitoria.",
    "{name} coroou a temporada com uma vitoria na corrida final em {track}, saindo de P{grid}.",
];

pub const FINAL_ESPECIAL_TITULO: &[&str] = &[
    "Encerramento especial: {name} vence em {track}",
    "{name} fecha o bloco com vitoria em {track}",
    "Noite especial em {track}: {name} vence",
    "{name} brilha no encerramento especial em {track}",
];

pub const FINAL_ESPECIAL_TEXTO: &[&str] = &[
    "{name} venceu a corrida especial em {track}, largando de P{grid}.",
    "No encerramento especial em {track}, {name} partiu de P{grid} e levou a vitoria.",
    "{name} marcou presenca na corrida especial em {track} com uma vitoria partindo de P{grid}.",
];

pub const TENSAO_TITULO: &[&str] = &[
    "Tensao antes da decisao: {name} vence em {track}",
    "{name} vence sob pressao em {track}",
    "Clima de decisao em {track}: {name} leva a melhor",
    "{name} mostra sangue frio e vence em {track}",
];

pub const TENSAO_TEXTO: &[&str] = &[
    "Com a decisao do campeonato se aproximando, {name} venceu em {track} largando de P{grid}.",
    "{name} lidou com a pressao da reta final e conquistou a vitoria em {track}, saindo de P{grid}.",
    "Na penultima etapa em {track}, {name} partiu de P{grid} e garantiu a vitoria.",
];

pub const VISITANTE_TITULO: &[&str] = &[
    "Visita especial a {track}: {name} vence",
    "{name} brilha na visita a {track}",
    "Etapa especial em {track}: vitoria de {name}",
    "{name} marca presenca em {track} com vitoria",
];

pub const VISITANTE_TEXTO: &[&str] = &[
    "{name} venceu na visita especial a {track}, largando de P{grid}.",
    "Na etapa especial em {track}, {name} partiu de P{grid} e conquistou a vitoria.",
    "{name} deixou sua marca na pista de {track} com uma vitoria partindo de P{grid}.",
];

pub const PRESTIGIO_TITULO: &[&str] = &[
    "{name} vence na pista de prestigio em {track}",
    "Prestigio em {track}: {name} conquista a vitoria",
    "{name} domina a classica de {track}",
    "Vitoria de prestigio para {name} em {track}",
];

pub const PRESTIGIO_TEXTO: &[&str] = &[
    "{name} venceu na classica pista de {track}, uma das mais prestigiadas do calendario, largando de P{grid}.",
    "Na pista de prestigio de {track}, {name} partiu de P{grid} e conquistou uma vitoria de peso.",
    "{name} brilhou em uma das pistas mais importantes do calendario ao vencer em {track}, saindo de P{grid}.",
];
