import { colors } from "./colors";

export const WIZARD_STEPS = [
  "Dificuldade",
  "Piloto",
  "Categoria",
  "Equipe",
  "Confirmacao",
];

export const DIFFICULTIES = [
  { id: "facil", name: "Facil", emoji: "😊", desc: "IA skill 20-60", accent: colors.VERDE },
  { id: "medio", name: "Medio", emoji: "😐", desc: "IA skill 30-80", accent: colors.AMARELO },
  { id: "dificil", name: "Dificil", emoji: "😤", desc: "IA skill 50-90", accent: colors.LARANJA },
  { id: "lendario", name: "Lendario", emoji: "💀", desc: "IA skill 70-100", accent: colors.VERMELHO },
];

export const NATIONALITIES = [
  { id: "br", label: "🇧🇷 Brasileiro", labelFem: "🇧🇷 Brasileira" },
  { id: "gb", label: "🇬🇧 Britanico", labelFem: "🇬🇧 Britanica" },
  { id: "de", label: "🇩🇪 Alemao", labelFem: "🇩🇪 Alema" },
  { id: "fr", label: "🇫🇷 Frances", labelFem: "🇫🇷 Francesa" },
  { id: "it", label: "🇮🇹 Italiano", labelFem: "🇮🇹 Italiana" },
  { id: "es", label: "🇪🇸 Espanhol", labelFem: "🇪🇸 Espanhola" },
  { id: "nl", label: "🇳🇱 Holandes", labelFem: "🇳🇱 Holandesa" },
  { id: "au", label: "🇦🇺 Australiano", labelFem: "🇦🇺 Australiana" },
  { id: "jp", label: "🇯🇵 Japones", labelFem: "🇯🇵 Japonesa" },
  { id: "us", label: "🇺🇸 Americano", labelFem: "🇺🇸 Americana" },
  { id: "mx", label: "🇲🇽 Mexicano", labelFem: "🇲🇽 Mexicana" },
  { id: "ar", label: "🇦🇷 Argentino", labelFem: "🇦🇷 Argentina" },
  { id: "fi", label: "🇫🇮 Finlandes", labelFem: "🇫🇮 Finlandesa" },
  { id: "be", label: "🇧🇪 Belga", labelFem: "🇧🇪 Belga" },
  { id: "pt", label: "🇵🇹 Portugues", labelFem: "🇵🇹 Portuguesa" },
  { id: "ca", label: "🇨🇦 Canadense", labelFem: "🇨🇦 Canadense" },
  { id: "at", label: "🇦🇹 Austriaco", labelFem: "🇦🇹 Austriaca" },
  { id: "ch", label: "🇨🇭 Suico", labelFem: "🇨🇭 Suica" },
  { id: "dk", label: "🇩🇰 Dinamarques", labelFem: "🇩🇰 Dinamarquesa" },
  { id: "se", label: "🇸🇪 Sueco", labelFem: "🇸🇪 Sueca" },
  { id: "no", label: "🇳🇴 Noruegues", labelFem: "🇳🇴 Norueguesa" },
  { id: "pl", label: "🇵🇱 Polones", labelFem: "🇵🇱 Polonesa" },
  { id: "cn", label: "🇨🇳 Chines", labelFem: "🇨🇳 Chinesa" },
];

export const STARTING_CATEGORIES = [
  {
    id: "mazda_rookie",
    name: "Mazda MX-5 Rookie Cup",
    car: "Mazda MX-5 2016",
    emoji: "🔴",
    teams: 6,
    races: 5,
    drivers: 12,
    description: "Categoria de entrada para aprender leitura de corrida, pista e consistencia.",
  },
  {
    id: "toyota_rookie",
    name: "Toyota GR86 Rookie Cup",
    car: "Toyota GR86",
    emoji: "⚪",
    teams: 6,
    races: 5,
    drivers: 12,
    description: "Categoria de entrada com o mesmo tamanho de grid e foco total em fundamentos.",
  },
];

export const TEAM_PREVIEWS = {
  mazda_rookie: [
    { index: 0, name: "Racing Academy Red", shortName: "RAR", primaryColor: "#cc0000", secondaryColor: "#ffffff", country: "🇺🇸 EUA", performanceRating: 68 },
    { index: 1, name: "Rolling Thunder Academy", shortName: "RTA", primaryColor: "#1a1a2e", secondaryColor: "#e94560", country: "🇺🇸 EUA", performanceRating: 74 },
    { index: 2, name: "Grid Start Racing School", shortName: "GSR", primaryColor: "#ff6b00", secondaryColor: "#000000", country: "🇬🇧 Reino Unido", performanceRating: 60 },
    { index: 3, name: "First Gear Motorsport", shortName: "FGM", primaryColor: "#00457c", secondaryColor: "#f0f0f0", country: "🇩🇪 Alemanha", performanceRating: 71 },
    { index: 4, name: "Apex Academy Racing", shortName: "AAR", primaryColor: "#2d6a4f", secondaryColor: "#ffffff", country: "🇫🇷 Franca", performanceRating: 78 },
    { index: 5, name: "Rookie Squad Racing", shortName: "RSQ", primaryColor: "#7b2d8b", secondaryColor: "#e0e0e0", country: "🇯🇵 Japao", performanceRating: 64 },
  ],
  toyota_rookie: [
    { index: 0, name: "TRD Driver Academy", shortName: "TDA", primaryColor: "#eb0a1e", secondaryColor: "#ffffff", country: "🇯🇵 Japao", performanceRating: 74 },
    { index: 1, name: "GR Racing School", shortName: "GRS", primaryColor: "#1c3557", secondaryColor: "#c8a84b", country: "🇯🇵 Japao", performanceRating: 78 },
    { index: 2, name: "Open Road Academy", shortName: "ORA", primaryColor: "#0b5563", secondaryColor: "#ffffff", country: "🇺🇸 EUA", performanceRating: 64 },
    { index: 3, name: "Speed Lab Rookies", shortName: "SLR", primaryColor: "#ff4500", secondaryColor: "#111111", country: "🇺🇸 EUA", performanceRating: 68 },
    { index: 4, name: "Rising Stars Motorsport", shortName: "RSM", primaryColor: "#ffd700", secondaryColor: "#333333", country: "🇬🇧 Reino Unido", performanceRating: 71 },
    { index: 5, name: "Fresh Start Racing", shortName: "FSR", primaryColor: "#228b22", secondaryColor: "#ffffff", country: "🇩🇪 Alemanha", performanceRating: 60 },
  ],
};

export const LOADING_MESSAGES = [
  "Criando pilotos...",
  "Gerando equipes...",
  "Montando contratos...",
  "Preparando calendario...",
  "Finalizando save...",
];
