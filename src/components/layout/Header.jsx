import useCareerStore from "../../stores/useCareerStore";
import {
  categoryLabel,
  formatCompactDate,
  formatDate,
  formatNextRaceCountdown,
} from "../../utils/formatters";
import GlassButton from "../ui/GlassButton";
import TabNavigation from "./TabNavigation";

function Header({ activeTab, onTabChange }) {
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const season = useCareerStore((state) => state.season);
  const nextRace = useCareerStore((state) => state.nextRace);
  const temporalSummary = useCareerStore((state) => state.temporalSummary);
  const calendarDisplayDate = useCareerStore((state) => state.calendarDisplayDate);
  const displayDaysUntilNextEvent = useCareerStore((state) => state.displayDaysUntilNextEvent);
  const isCalendarAdvancing = useCareerStore((state) => state.isCalendarAdvancing);
  const showRaceBriefing = useCareerStore((state) => state.showRaceBriefing);
  const startCalendarAdvance = useCareerStore((state) => state.startCalendarAdvance);
  const closeRaceBriefing = useCareerStore((state) => state.closeRaceBriefing);
  const visibleDate = calendarDisplayDate ?? temporalSummary?.current_display_date;
  const visibleCountdown = displayDaysUntilNextEvent ?? temporalSummary?.days_until_next_event;

  function handleNextRace() {
    void Promise.resolve(startCalendarAdvance?.()).catch((error) => {
      console.error("Erro ao avancar calendario pelo header:", error);
    });
  }

  function handleBackToBriefingOrigin() {
    closeRaceBriefing?.();
  }

  return (
    <header className="relative z-20 flex flex-col">
      {/* Top bar: team name LEFT · tabs CENTERED · (space) RIGHT */}
      <div className="shrink-0 px-3 py-2 sm:px-4 lg:px-5 xl:px-6">
        <div className="mx-auto flex w-full max-w-[1680px] items-center">
          {/* Left — team name (hidden when in race briefing, center takes over) */}
          <div className="flex min-w-0 flex-1 items-center gap-2">
            {!showRaceBriefing && (
              <>
                <span
                  className="h-3 w-3 shrink-0 rounded-full"
                  style={{ backgroundColor: playerTeam?.cor_primaria ?? "#58a6ff" }}
                />
                <span className="truncate text-xs font-bold uppercase tracking-[0.14em] text-text-primary">
                  {playerTeam?.nome ?? "—"}
                </span>
              </>
            )}
          </div>

          {/* Center — tabs or team name when in race briefing */}
          {showRaceBriefing ? (
            <div className="flex items-center gap-3">
              <span
                className="h-4 w-4 shrink-0 rounded-full"
                style={{ backgroundColor: playerTeam?.cor_primaria ?? "#58a6ff" }}
              />
              <span className="text-3xl font-bold tracking-[-0.035em] text-text-primary">
                {playerTeam?.nome ?? "—"}
              </span>
            </div>
          ) : (
            <TabNavigation activeTab={activeTab} onTabChange={onTabChange} />
          )}

          {/* Right — date info + action button */}
          <div className="flex flex-1 justify-end">
            <div className="flex items-center gap-3 rounded-2xl border border-white/10 bg-white/[0.05] px-4 py-2 backdrop-blur-md">
              <div className="text-right">
                <p className="text-[10px] uppercase tracking-[0.18em] text-text-secondary">
                  Data {formatCompactDate(visibleDate)}
                </p>
                <p className="mt-1 text-xs font-semibold text-text-primary">
                  {formatNextRaceCountdown(visibleCountdown)}
                </p>
              </div>
              {showRaceBriefing ? (
                <GlassButton
                  variant="primary"
                  className="rounded-full px-5 py-2.5"
                  onClick={handleBackToBriefingOrigin}
                >
                  Voltar
                </GlassButton>
              ) : (
                <GlassButton
                  variant="primary"
                  disabled={!nextRace || isCalendarAdvancing}
                  className="rounded-full px-5 py-2.5"
                  onClick={handleNextRace}
                >
                  {isCalendarAdvancing ? "Avancando..." : "Avancar calendario"}
                </GlassButton>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Event banner — only on Pilotos tab */}
      {activeTab === "standings" && !showRaceBriefing && <div className="flex items-stretch h-[14vh] min-h-[110px]">
        <div className="mx-auto flex w-full max-w-[1680px] items-stretch px-3 sm:px-4 lg:px-5 xl:px-6">
          {nextRace ? (
            <div className="flex w-full items-center gap-6">
              {/* Track image with race badge overlay */}
              <TrackImage
                trackName={nextRace.track_name}
                rodada={nextRace.rodada}
                totalRodadas={season?.total_rodadas ?? "?"}
              />

              {/* Circuit info text */}
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-accent-primary">
                  Próximo Evento
                </p>
                <h2 className="mt-1 truncate text-3xl font-bold tracking-[-0.02em] text-text-primary sm:text-4xl">
                  {nextRace.track_name}
                </h2>
                {nextRace.display_date && (
                  <div className="mt-1.5 text-xs text-text-muted">
                    {formatDate(nextRace.display_date)}
                  </div>
                )}
                <div className="mt-1 flex items-center gap-3 text-sm text-text-secondary">
                  <span className="flex items-center gap-1.5">
                    <span className="opacity-60">🕐</span>
                    {nextRace.horario} Local
                  </span>
                  <span className="opacity-30">·</span>
                  <span>{categoryLabel(playerTeam?.categoria)}</span>
                </div>
              </div>

              {/* Right — clima */}
              <div className="shrink-0">
                <StatBlock
                  label="Clima"
                  value={weatherWithTemp(nextRace.clima, nextRace.temperatura)}
                  icon={weatherIcon(nextRace.clima)}
                />
              </div>
            </div>
          ) : (
            <p className="text-sm text-text-muted">
              {season
                ? `Temporada ${season.numero} · Ano ${season.ano} — Sem corrida pendente`
                : "Carregando..."}
            </p>
          )}
        </div>
      </div>}
    </header>
  );
}

function TrackImage({ trackName, rodada, totalRodadas }) {
  const src = `/tracks/${encodeURIComponent(trackName)}.png`;

  return (
    <div className="relative my-3 w-64 shrink-0 self-stretch overflow-hidden rounded-2xl border border-white/10 bg-white/5">
      <img
        src={src}
        alt={trackName}
        className="h-full w-full object-cover"
        onError={(e) => { e.currentTarget.style.display = "none"; }}
      />
      {/* Race badge — top-left corner */}
      <div className="absolute left-2 top-2 rounded border border-accent-primary/40 bg-[rgba(10,15,28,0.55)] px-2 py-0.5 backdrop-blur-[8px]">
        <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-accent-primary">
          Corrida {rodada}/{totalRodadas}
        </span>
      </div>
    </div>
  );
}

function StatBlock({ label, value, icon }) {
  return (
    <div className="text-right">
      <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
      <p className="mt-0.5 flex items-center justify-end gap-1 text-sm font-semibold text-text-secondary">
        {icon && <span>{icon}</span>}
        {value}
      </p>
    </div>
  );
}


function weatherWithTemp(clima, temperatura) {
  const label = weatherLabel(clima);
  if (temperatura == null) return label;
  return `${Math.round(temperatura)}° ${label}`;
}

function weatherLabel(value) {
  if (value === "HeavyRain") return "Chuva Forte";
  if (value === "Wet") return "Chuva";
  if (value === "Damp") return "Úmido";
  return "Parcialmente Nublado";
}

function weatherIcon(value) {
  if (value === "HeavyRain") return "⛈";
  if (value === "Wet") return "🌧";
  if (value === "Damp") return "🌦";
  return "⛅";
}

export default Header;
