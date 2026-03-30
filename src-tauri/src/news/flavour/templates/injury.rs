// ══════════════════════════════════════════════════════════════════════════════
// LESÃO LEVE
// ══════════════════════════════════════════════════════════════════════════════

pub const LEVE_TITULO: &[&str] = &[
    "{name} sofre lesão leve em {track}",
    "Lesão menor para {name}",
    "{name} machucado mas estável",
    "Susto para {name} em {track}",
    "{name} escapa de lesão grave",
];

pub const LEVE_TEXTO: &[&str] = &[
    "{name} sofreu uma lesão leve e ficará de fora por {n} corrida(s).",
    "Apesar do susto, {name} teve apenas uma lesão leve. Retorno previsto em {n} corrida(s).",
    "{name} está bem mas precisará de {n} corrida(s) para se recuperar.",
    "Lesão leve para {name}: ausência de {n} corrida(s) esperada.",
    "{name} se machucou levemente e perderá {n} corrida(s).",
];

// ══════════════════════════════════════════════════════════════════════════════
// LESÃO MODERADA
// ══════════════════════════════════════════════════════════════════════════════

pub const MODERADA_TITULO: &[&str] = &[
    "{name} sofre lesão em {track}",
    "Lesão preocupa equipe de {name}",
    "{name} ficará afastado por lesão",
    "Lesão tira {name} das próximas corridas",
    "{name} se machuca em {track}",
];

pub const MODERADA_TEXTO: &[&str] = &[
    "{name} sofreu uma lesão moderada e ficará de fora por {n} corrida(s).",
    "A lesão de {name} vai tirá-lo da competição por {n} corrida(s).",
    "{name} precisará de tempo para se recuperar: {n} corrida(s) de ausência.",
    "Lesão moderada para {name}: retorno esperado após {n} corrida(s).",
    "{name} está fora das próximas {n} corrida(s) devido à lesão.",
];

// ══════════════════════════════════════════════════════════════════════════════
// LESÃO GRAVE
// ══════════════════════════════════════════════════════════════════════════════

pub const GRAVE_TITULO: &[&str] = &[
    "{name} sofre lesão grave em {track}",
    "Lesão séria preocupa {name}",
    "Grave: {name} se machuca feio",
    "{name} hospitalizado após {track}",
    "Lesão grave afasta {name}",
];

pub const GRAVE_TEXTO: &[&str] = &[
    "{name} sofreu uma lesão grave e ficará de fora por {n} corrida(s). Desejamos rápida recuperação.",
    "Momento difícil para {name}, que terá uma longa recuperação de {n} corrida(s).",
    "A lesão grave de {name} significa ausência de {n} corrida(s).",
    "{name} enfrenta uma recuperação longa: {n} corrida(s) fora.",
    "Todos desejam força a {name}, que ficará {n} corrida(s) afastado por lesão grave.",
];

// ══════════════════════════════════════════════════════════════════════════════
// LESÃO CRÍTICA
// ══════════════════════════════════════════════════════════════════════════════

pub const CRITICA_TITULO: &[&str] = &[
    "URGENTE: {name} sofre lesão crítica",
    "Lesão crítica para {name} em {track}",
    "{name} em estado sério após acidente",
    "Preocupação máxima com {name}",
    "Lesão crítica: {name} hospitalizado",
];

pub const CRITICA_TEXTO: &[&str] = &[
    "{name} sofreu uma lesão crítica e ficará de fora por {n} corrida(s). Toda a comunidade torce por sua recuperação.",
    "Momento muito difícil: {name} enfrenta uma lesão crítica e longa recuperação de {n} corrida(s).",
    "A lesão crítica de {name} é preocupante. Previsão de ausência de {n} corrida(s).",
    "{name} luta para se recuperar de lesão crítica. Retorno apenas após {n} corrida(s).",
    "Nossos pensamentos estão com {name}, afastado por {n} corrida(s) com lesão crítica.",
];
