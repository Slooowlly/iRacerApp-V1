import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassButton from "../../components/ui/GlassButton";
import GlassCard from "../../components/ui/GlassCard";
import LoadingOverlay from "../../components/ui/LoadingOverlay";
import useCareerStore from "../../stores/useCareerStore";
import { buildFavoriteExpectationSelection, recentResults } from "./nextRaceBriefing";
import {
  buildEditorialCopy,
  classifyChampionshipState,
  classifyWeekendState,
} from "./nextRaceEditorial";

function NextRaceTab() {
  const [error, setError] = useState("");
  const [exportNotice, setExportNotice] = useState("");
  const [hasExistingPreseason, setHasExistingPreseason] = useState(false);
  const [driverStandings, setDriverStandings] = useState([]);
  const [teamStandings, setTeamStandings] = useState([]);
  const [briefingPhraseHistory, setBriefingPhraseHistory] = useState({ season_number: 0, entries: [] });
  const [isLoadingBriefing, setIsLoadingBriefing] = useState(true);
  const [briefingError, setBriefingError] = useState("");

  const player = useCareerStore((state) => state.player);
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const nextRace = useCareerStore((state) => state.nextRace);
  const nextRaceBriefing = useCareerStore((state) => state.nextRaceBriefing);
  const season = useCareerStore((state) => state.season);
  const isSimulating = useCareerStore((state) => state.isSimulating);
  const isAdvancing = useCareerStore((state) => state.isAdvancing);
  const careerId = useCareerStore((state) => state.careerId);
  const simulateRace = useCareerStore((state) => state.simulateRace);
  const advanceSeason = useCareerStore((state) => state.advanceSeason);
  const enterPreseason = useCareerStore((state) => state.enterPreseason);

  useEffect(() => {
    let active = true;

    async function detectPreseason() {
      if (!careerId || nextRace) return;

      try {
        await invoke("get_preseason_state", { careerId });
        if (active) {
          setHasExistingPreseason(true);
          await enterPreseason();
        }
      } catch (_error) {
        if (active) {
          setHasExistingPreseason(false);
        }
      }
    }

    detectPreseason();

    return () => {
      active = false;
    };
  }, [careerId, enterPreseason, nextRace]);

  useEffect(() => {
    let active = true;

    async function loadBriefingContext() {
      if (!careerId || !nextRace || !playerTeam?.categoria) {
        if (active) {
          setDriverStandings([]);
          setTeamStandings([]);
          setBriefingPhraseHistory({ season_number: 0, entries: [] });
          setIsLoadingBriefing(false);
        }
        return;
      }

      setIsLoadingBriefing(true);
      setBriefingError("");

      try {
        const [drivers, teams, phraseHistory] = await Promise.all([
          invoke("get_drivers_by_category", {
            careerId,
            category: playerTeam.categoria,
          }),
          invoke("get_teams_standings", {
            careerId,
            category: playerTeam.categoria,
          }),
          invoke("get_briefing_phrase_history", {
            careerId,
          }).catch(() => ({ season_number: 0, entries: [] })),
        ]);

        if (!active) return;

        setDriverStandings(Array.isArray(drivers) ? drivers : []);
        setTeamStandings(Array.isArray(teams) ? teams : []);
        setBriefingPhraseHistory(
          phraseHistory && Array.isArray(phraseHistory.entries)
            ? phraseHistory
            : { season_number: 0, entries: [] },
        );
      } catch (invokeError) {
        if (!active) return;

        setBriefingError(
          typeof invokeError === "string"
            ? invokeError
            : invokeError?.toString?.() ?? "Nao foi possivel montar o briefing.",
        );
      } finally {
        if (active) {
          setIsLoadingBriefing(false);
        }
      }
    }

    loadBriefingContext();

    return () => {
      active = false;
    };
  }, [careerId, nextRace, playerTeam?.categoria]);

  const briefing = useMemo(
    () =>
      buildBriefingContext({
        player,
        playerTeam,
        season,
        nextRace,
        nextRaceBriefing,
        driverStandings,
        teamStandings,
        briefingPhraseHistory,
      }),
    [
      player,
      playerTeam,
      season,
      nextRace,
      nextRaceBriefing,
      driverStandings,
      teamStandings,
      briefingPhraseHistory,
    ],
  );

  useEffect(() => {
    let active = true;

    async function persistBriefingPhrases() {
      if (!careerId || !season?.numero || !nextRace?.rodada || briefing.favorites.length === 0) {
        return;
      }

      const entries = briefing.favorites
        .map((driver) => ({
          round_number: nextRace.rodada,
          driver_id: driver.id,
          bucket_key: driver.expectationBucketKey,
          phrase_id: driver.expectationPhraseId,
        }))
        .filter((entry) => entry.bucket_key && entry.phrase_id);

      if (entries.length === 0) {
        return;
      }

      const allPersisted = entries.every((entry) =>
        briefingPhraseHistory.entries.some(
          (saved) =>
            saved.season_number === season.numero &&
            saved.round_number === entry.round_number &&
            saved.driver_id === entry.driver_id &&
            saved.bucket_key === entry.bucket_key &&
            saved.phrase_id === entry.phrase_id,
        ),
      );

      if (allPersisted) {
        return;
      }

      try {
        const updatedHistory = await invoke("save_briefing_phrase_history", {
          careerId,
          seasonNumber: season.numero,
          entries,
        });

        if (!active) return;
        if (updatedHistory && Array.isArray(updatedHistory.entries)) {
          setBriefingPhraseHistory(updatedHistory);
        }
      } catch (_error) {
        // Silencioso: a variacao recente melhora a imersao, mas nao deve quebrar o briefing.
      }
    }

    persistBriefingPhrases();

    return () => {
      active = false;
    };
  }, [
    briefing.favorites,
    briefingPhraseHistory.entries,
    careerId,
    nextRace?.rodada,
    season?.numero,
  ]);

  async function handleSimulate() {
    setError("");
    setExportNotice("");

    try {
      await simulateRace();
    } catch (invokeError) {
      setError(
        typeof invokeError === "string"
          ? invokeError
          : invokeError?.toString?.() ?? "Nao foi possivel simular a corrida.",
      );
    }
  }

  async function handleSeasonAdvance() {
    setError("");
    setExportNotice("");

    try {
      if (hasExistingPreseason) {
        await enterPreseason();
        return;
      }

      await advanceSeason();
    } catch (invokeError) {
      setError(
        typeof invokeError === "string"
          ? invokeError
          : invokeError?.toString?.() ?? "Nao foi possivel avancar para a pre-temporada.",
      );
    }
  }

  function handleExport() {
    setExportNotice("Exportacao para o iRacing chega em breve.");
  }

  if (!nextRace) {
    return (
      <div className="relative">
        <LoadingOverlay
          open={isAdvancing}
          title="Virando a temporada"
          message="Evolucao, aposentadorias, promocoes e preparacao da pre-temporada em andamento."
        />

        <GlassCard hover={false} className="rounded-[28px] p-10">
          <div className="py-6 text-center">
            <div className="text-6xl">PQ</div>
            <p className="mt-4 text-sm uppercase tracking-[0.22em] text-accent-primary">
              Proxima corrida
            </p>
            <h2 className="mt-3 text-3xl font-semibold text-text-primary">
              Temporada finalizada
            </h2>
            <p className="mt-3 text-sm text-text-secondary">
              {hasExistingPreseason
                ? "A pre-temporada ja foi iniciada. Voce pode voltar direto para o mercado semanal."
                : "Todas as corridas da temporada atual ja foram disputadas."}
            </p>
            <div className="mt-6">
              <GlassButton
                variant="primary"
                disabled={isAdvancing}
                onClick={() => void handleSeasonAdvance()}
              >
                {isAdvancing
                  ? "Processando..."
                  : hasExistingPreseason
                    ? "Continuar pre-temporada"
                    : "Avancar para pre-temporada"}
              </GlassButton>
            </div>
            {error ? <p className="mt-4 text-sm text-status-red">{error}</p> : null}
          </div>
        </GlassCard>
      </div>
    );
  }

  return (
    <div className="relative space-y-5">
      <LoadingOverlay
        open={isSimulating}
        title="Simulando corrida"
        message="Classificacao, corrida e atualizacao do campeonato em andamento."
      />

      <div className="grid gap-5 xl:grid-cols-[1.08fr_0.92fr]">
        <div className="space-y-5">
          <GlassCard
            hover={false}
            className="relative overflow-hidden rounded-[30px] border-white/8 bg-[linear-gradient(145deg,rgba(7,15,27,0.96)_0%,rgba(11,24,42,0.93)_54%,rgba(18,23,34,0.92)_100%)] p-5"
          >
            <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(88,166,255,0.22),transparent_24%),radial-gradient(circle_at_bottom_left,rgba(255,183,77,0.12),transparent_22%)]" />
            <div className="relative">
              <p className="text-[10px] uppercase tracking-[0.22em] text-accent-primary">
                Resumo do evento
              </p>

              <div className="mt-4 flex flex-col gap-5 border-b border-white/8 pb-5 md:flex-row md:items-start md:justify-between">
                <div>
                  <div className="mt-2 flex items-center gap-3">
                    <p className="text-[40px] font-semibold leading-none tracking-[-0.06em] text-text-primary">
                      {briefing.eventDateShort}
                    </p>
                    <p className="text-[12px] uppercase tracking-[0.14em] text-text-muted">
                      Etapa {nextRace.rodada} de {season?.total_rodadas ?? "?"}
                    </p>
                  </div>
                  <p className="mt-3 text-[22px] font-semibold tracking-[-0.03em] text-text-primary">
                    {nextRace.track_name}
                  </p>
                </div>

                <div className="text-left md:text-right">
                  <p className="text-[11px] uppercase tracking-[0.14em] text-text-muted">
                    Horario local
                  </p>
                  <p className="mt-2 text-[28px] font-semibold tracking-[-0.04em] text-text-primary">
                    {nextRace.horario}
                  </p>
                  <p className="mt-2 text-[12px] uppercase tracking-[0.08em] text-text-secondary">
                    {briefing.timePeriodPrefix}
                    <strong className="ml-1 text-[14px] tracking-[0.12em] text-text-primary">
                      {briefing.timePeriodHighlight}
                    </strong>
                  </p>
                </div>
              </div>

              <div className="mt-5 grid gap-3 md:grid-cols-3">
                <EventSummaryBox
                  label="Publico"
                  value={formatAudience(briefing.audienceEstimate)}
                  meta={briefing.audienceRankLabel}
                />
                <EventSummaryBox
                  label={briefing.broadcastLabel}
                  value={briefing.broadcastValue}
                  featured={briefing.broadcastLabel === "Cobertura"}
                />
                <EventSummaryBox
                  label="Historico"
                  value={briefing.historyValue}
                  meta={briefing.historyMeta}
                />
              </div>
            </div>
          </GlassCard>

          <GlassCard hover={false} className="rounded-[28px] border-white/8 bg-black/15 p-5">
            <p className="text-[10px] uppercase tracking-[0.22em] text-accent-primary">
              Previa da corrida
            </p>

            <div className="relative mt-4 overflow-hidden rounded-[26px] border border-white/8 bg-[linear-gradient(115deg,rgba(255,123,114,0.08),transparent_30%),linear-gradient(180deg,rgba(255,255,255,0.04),rgba(255,255,255,0.02))] px-5 py-5">
              <div className="pointer-events-none absolute -right-8 -top-8 h-36 w-36 rounded-full bg-[radial-gradient(circle,rgba(240,195,107,0.16),transparent_65%)]" />
              <div className="relative">
                <p className="text-[11px] uppercase tracking-[0.24em] text-accent-gold">
                  Chamada da etapa
                </p>
                <h3 className="mt-3 max-w-4xl text-[32px] font-semibold leading-[1.05] tracking-[-0.05em] text-text-primary">
                  {briefing.headline}
                </h3>
                <p className="mt-4 max-w-3xl text-[15px] leading-7 text-text-secondary">
                  {briefing.paragraphs[0] ?? briefing.attendanceNarrative}
                </p>
              </div>
            </div>

            <div className="mt-4 grid gap-4 lg:grid-cols-2">
              <BriefingPreviewPanel
                title="O que esta em jogo"
                body={briefing.paragraphs[1] ?? briefing.paragraphs[0] ?? briefing.actionHint}
              />
              <BriefingPreviewPanel
                title="Leitura do paddock"
                accentLabel={briefing.teamVoiceLabel}
                accentColor={playerTeam?.cor_primaria}
                body={briefing.quote}
                support={briefing.paddockSupport}
              />
            </div>
          </GlassCard>

          <GlassCard hover={false} className="rounded-[24px] border-white/8 bg-black/15 p-5">
            <p className="text-[10px] uppercase tracking-[0.22em] text-accent-primary">Condicoes</p>
            <div className="mt-4 space-y-3">
              <ConditionReportRow
                badge={briefing.weatherIcon}
                label="Tempo"
                value={briefing.weatherSummary}
                meta={briefing.weatherNarrative}
              />
              <ConditionReportRow
                badge="🌡"
                label="Temperatura"
                value={briefing.trackTemperatureLabel}
                meta={briefing.temperatureNarrative}
              />
              <ConditionReportRow
                badge="📻"
                label="Leitura do box"
                value={briefing.trackConditionLabel}
                meta={briefing.boxNarrative}
              />
            </div>
          </GlassCard>

        </div>

        <div className="space-y-5">
          <GlassCard hover={false} className="rounded-[28px]">
            <SectionTitle
              eyebrow="Sobre o grid"
              title="Favoritos ao podio"
              meta={
                isLoadingBriefing
                  ? "Montando analise..."
                  : `${briefing.favorites.length} pilotos em destaque`
              }
            />

            {isLoadingBriefing ? (
              <p className="mt-5 text-sm text-text-secondary">
                Montando leitura de forma e ritmo do grid.
              </p>
            ) : briefingError ? (
              <p className="mt-5 text-sm text-status-red">{briefingError}</p>
            ) : (
              <div className="mt-5 overflow-hidden rounded-[24px] border border-white/8">
                <div className="grid gap-3 bg-accent-primary/8 px-4 py-3 text-[11px] uppercase tracking-[0.16em] text-text-muted md:grid-cols-[72px_0.95fr_0.85fr_1.35fr]">
                  <div>Pos.</div>
                  <div>Piloto</div>
                  <div>Forma recente</div>
                  <div>Expectativa</div>
                </div>
                {briefing.favorites.map((driver, index) => (
                  <FavoriteRow key={driver.id} driver={driver} index={index} />
                ))}
              </div>
            )}
          </GlassCard>

          <GlassCard hover={false} className="rounded-[28px]">
            <SectionTitle
              eyebrow="Campeonato"
              title="Tabela de pilotos"
              meta={`Etapa ${nextRace.rodada} de ${season?.total_rodadas ?? "?"}`}
            />

            {briefing.championshipTable.length > 0 ? (
              <div className="mt-5 rounded-[22px] border border-white/8 bg-white/[0.03] p-3">
                <table
                  aria-label="Tabela do campeonato"
                  className="w-full table-fixed border-separate border-spacing-y-1.5"
                >
                  <thead>
                    <tr className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
                      <th className="w-9 px-2 text-left font-medium">#</th>
                      <th className="px-2 text-left font-medium">Piloto</th>
                      <th className="w-[46px] px-2 text-right font-medium">Pts</th>
                    </tr>
                  </thead>
                  <tbody>
                    {briefing.championshipTable.map((driver) => (
                      <ChampionshipTableRow key={driver.id} driver={driver} />
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="mt-5 text-sm text-text-secondary">
                Classificacao do campeonato indisponivel no momento.
              </p>
            )}
          </GlassCard>
        </div>
      </div>

      {error ? (
        <div className="rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-4 text-sm text-status-red">
          <p>Erro ao simular: {error}</p>
        </div>
      ) : null}

      <GlassCard hover={false} className="rounded-[28px]">
        <div className="flex items-start justify-between gap-6">
          <div className="flex-1 min-w-0">
            <p className="text-[10px] uppercase tracking-[0.22em] text-accent-primary">
              Sala de estrategia
            </p>
            <h3 className="mt-2 text-2xl font-semibold text-text-primary">
              Escolha o proximo passo
            </h3>
            <p className="mt-2 text-sm text-text-secondary">
              {isLoadingBriefing
                ? "Fechando ultimos detalhes do briefing antes da largada."
                : briefing.actionHint}
            </p>
            {exportNotice ? <p className="mt-3 text-sm text-accent-primary">{exportNotice}</p> : null}

            <div className="mt-4 grid gap-3 sm:grid-cols-3">
              {briefing.goals.map((goal) => (
                <GoalCard key={goal.label} label={goal.label} value={goal.value} />
              ))}
            </div>
          </div>

          <div className="flex flex-shrink-0 flex-col gap-3 pt-9">
            <GlassButton
              variant="primary"
              disabled={isSimulating || !nextRace}
              className="min-w-48"
              onClick={handleSimulate}
            >
              {isSimulating ? "Simulando..." : "Simular corrida"}
            </GlassButton>
            <GlassButton variant="secondary" className="min-w-48" onClick={handleExport}>
              Exportar
            </GlassButton>
          </div>
        </div>
      </GlassCard>
    </div>
  );
}

function EventSummaryBox({ label, value, meta, featured = false }) {
  return (
    <div className="rounded-[18px] bg-white/[0.08] px-4 py-4 text-center">
      <p className="flex items-center justify-center gap-2 text-[11px] uppercase tracking-[0.14em] text-text-muted">
        {label}
      </p>
      <p
        className={[
          "mt-2 font-semibold tracking-[-0.04em] text-text-primary",
          featured ? "text-[30px]" : "text-[24px]",
        ].join(" ")}
      >
        {value}
      </p>
      {meta ? <p className="mt-2 text-[13px] leading-5 text-text-secondary">{meta}</p> : null}
    </div>
  );
}

function ConditionReportRow({ badge, label, value, meta }) {
  return (
    <div className="grid gap-3 rounded-[18px] border border-white/8 bg-white/[0.05] p-4 sm:grid-cols-[72px_1fr] sm:items-center">
      <div className="flex h-14 w-14 items-center justify-center rounded-[16px] bg-accent-primary/12 text-[28px] leading-none text-accent-primary">
        {badge}
      </div>
      <div>
        <p className="text-[11px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
        <p className="mt-1 text-[21px] font-semibold tracking-[-0.03em] text-text-primary">
          {value}
        </p>
        <p className="mt-1 text-sm leading-6 text-text-secondary">{meta}</p>
      </div>
    </div>
  );
}

function GoalCard({ label, value }) {
  return (
    <div className="rounded-[20px] border border-white/8 bg-white/[0.04] px-4 py-4">
      <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
      <p className="mt-2 text-sm font-semibold leading-6 text-text-primary">{value}</p>
    </div>
  );
}

function BriefingPreviewPanel({ title, accentLabel, accentColor, body, support }) {
  return (
    <div className="rounded-[20px] border border-white/8 bg-white/[0.03] p-4">
      <p className="text-[11px] uppercase tracking-[0.16em] text-text-muted">{title}</p>
      {accentLabel ? (
        <p className="mt-3 text-[12px] font-semibold uppercase tracking-[0.12em]" style={{ color: getReadableTeamColor(accentColor) }}>
          {accentLabel}
        </p>
      ) : null}
      <p className="mt-3 text-[14px] leading-7 text-text-primary">{body}</p>
      {support ? <p className="mt-3 text-[14px] leading-7 text-text-secondary">{support}</p> : null}
    </div>
  );
}

function SectionTitle({ eyebrow, title, meta }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <p className="text-[10px] uppercase tracking-[0.22em] text-accent-primary">{eyebrow}</p>
        <h3 className="mt-2 text-2xl font-semibold text-text-primary">{title}</h3>
      </div>
      {meta ? <p className="text-right text-xs text-text-secondary">{meta}</p> : null}
    </div>
  );
}


function FavoriteRow({ driver, index }) {
  const medalTone = getFavoritePositionTone(index);

  return (
    <div className="grid gap-3 border-t border-white/8 bg-white/[0.03] px-4 py-4 md:grid-cols-[72px_0.95fr_0.85fr_1.35fr] md:items-center">
      <div className={`text-[28px] font-extrabold tracking-[-0.05em] ${medalTone}`}>
        {index + 1}
      </div>

      <div className="min-w-0">
        <p className="truncate text-base font-semibold text-text-primary">{driver.nome}</p>
        <p
          className="mt-1 text-[12px] uppercase tracking-[0.12em]"
          style={{ color: getReadableTeamColor(driver.equipe_cor) }}
        >
          {driver.equipe_nome}
        </p>
      </div>

      <div className="flex flex-wrap gap-1.5">
        {driver.formChips.map((chip, chipIndex) => (
          <span
            key={`${driver.id}-chip-${chipIndex}`}
            className={["rounded-full border px-2.5 py-1 text-[11px] font-semibold", chip.tone].join(" ")}
          >
            {chip.label}
          </span>
        ))}
      </div>

      <div>
        <p className="text-[13px] leading-5 text-text-primary">{driver.expectation}</p>
      </div>
    </div>
  );
}

function ChampionshipTableRow({ driver }) {
  const isPlayer = driver.is_jogador;
  const positionTone =
    driver.posicao_campeonato === 1
      ? "text-accent-gold"
      : driver.posicao_campeonato === 2
        ? "text-accent-primary"
        : driver.posicao_campeonato === 3
          ? "text-[#d7c6ff]"
          : "text-text-secondary";

  return (
    <tr>
      <td
        className={[
          "rounded-l-[16px] px-3 py-2.5 text-[13px] font-semibold",
          isPlayer
            ? "border-y border-l border-accent-primary/20 bg-accent-primary/10"
            : "bg-white/[0.04]",
          positionTone,
        ].join(" ")}
      >
        {driver.posicao_campeonato}
      </td>
      <td
        className={[
          "px-2 py-2.5 text-[13px]",
          isPlayer
            ? "border-y border-accent-primary/20 bg-accent-primary/10 font-semibold text-text-primary"
            : "bg-white/[0.04] text-text-primary",
        ].join(" ")}
      >
        <span className="block truncate">{driver.nome_completo ?? driver.nome}</span>
      </td>
      <td
        className={[
          "rounded-r-[16px] px-3 py-2.5 text-right text-[13px] font-semibold text-text-primary",
          isPlayer
            ? "border-y border-r border-accent-primary/20 bg-accent-primary/10"
            : "bg-white/[0.04]",
        ].join(" ")}
      >
        {driver.pontos}
      </td>
    </tr>
  );
}

function buildBriefingContext({
  player,
  playerTeam,
  season,
  nextRace,
  nextRaceBriefing,
  driverStandings,
  teamStandings,
  briefingPhraseHistory,
}) {
  const orderedDrivers = [...driverStandings].sort(
    (left, right) => (left.posicao_campeonato ?? 999) - (right.posicao_campeonato ?? 999),
  );
  const orderedTeams = [...teamStandings].sort(
    (left, right) => (left.posicao ?? 999) - (right.posicao ?? 999),
  );
  const playerStanding =
    orderedDrivers.find((driver) => driver.is_jogador) ??
    orderedDrivers.find((driver) => driver.id === player?.id) ??
    null;
  const standingsTopFive = orderedDrivers.slice(0, 5);
  const leader = standingsTopFive[0] ?? null;
  const trackHistory = nextRaceBriefing?.track_history ?? null;
  const briefingRival = nextRaceBriefing?.primary_rival ?? null;
  const weekendStories = normalizeWeekendStories(nextRaceBriefing?.weekend_stories);
  const rival = resolvePrimaryRival(orderedDrivers, playerStanding, briefingRival);
  const teammate =
    playerStanding && playerStanding.equipe_id
      ? orderedDrivers.find(
          (driver) => driver.equipe_id === playerStanding.equipe_id && driver.id !== playerStanding.id,
        ) ?? null
      : null;
  const teamStanding =
    orderedTeams.find((team) => team.id === playerTeam?.id) ?? orderedTeams[0] ?? null;
  const gapToLeader = Math.max(0, (leader?.pontos ?? 0) - (playerStanding?.pontos ?? 0));
  const behindDriver =
    playerStanding && playerStanding.posicao_campeonato > 0
      ? orderedDrivers[playerStanding.posicao_campeonato] ?? null
      : null;
  const gapBehind =
    playerStanding && behindDriver
      ? Math.max(0, (playerStanding.pontos ?? 0) - (behindDriver.pontos ?? 0))
      : null;
  const remainingRounds = Math.max(0, (season?.total_rodadas ?? 0) - (nextRace?.rodada ?? 0));
  const ratedDrivers = orderedDrivers
    .map((driver) => ({
      ...driver,
      rating: buildFavoriteRating(driver),
      formLabel: buildFormLabel(driver),
      formChips: buildFormChips(driver),
    }))
    .sort((left, right) => right.rating - left.rating || left.posicao_campeonato - right.posicao_campeonato);
  const favorites = ratedDrivers
    .slice()
    .sort((left, right) => right.rating - left.rating || left.posicao_campeonato - right.posicao_campeonato)
    .slice(0, 5)
    .map((driver, index) => {
      const selection = buildFavoriteExpectationSelection(driver, index, {
        seasonNumber: season?.numero,
        roundNumber: nextRace?.rodada,
        historyEntries: briefingPhraseHistory?.entries ?? [],
      });

      return {
        ...driver,
        expectation: selection.text,
        expectationPhraseId: selection.phraseId,
        expectationBucketKey: selection.bucketKey,
      };
    });
  const audienceEstimate = nextRace?.event_interest?.display_value ?? estimateAudience(nextRace?.event_interest?.tier_label);
  const totalRounds = Math.max(1, season?.total_rodadas ?? 1);
  const currentRound = Math.max(1, nextRace?.rodada ?? 1);
  const playerCompetitive = ratedDrivers.find((driver) => driver.id === playerStanding?.id) ?? null;
  const leaderCompetitive = ratedDrivers.find((driver) => driver.id === leader?.id) ?? null;
  const outlook = buildCompetitiveOutlook({
    playerStanding,
    leader,
    remainingRounds,
    playerRating: playerCompetitive?.rating ?? 0,
    leaderRating: leaderCompetitive?.rating ?? 0,
  });
  const attendanceNarrative =
    audienceEstimate > 0
      ? `A expectativa do paddock aponta para ${formatAudience(audienceEstimate)} de publico estimado ao longo do fim de semana.`
      : "O paddock espera bom movimento de publico nesta etapa.";
  const championshipState = classifyChampionshipState({
    playerStanding,
    leader,
    remainingRounds,
    outlook,
    gapBehind,
  });
  const weekendState = classifyWeekendState({
    trackHistory,
    briefingRival,
    nextRace,
    weekendStories,
  });
  const editorialCopy = buildEditorialCopy({
    championshipState,
    weekendState,
    playerStanding,
    leader,
    rival,
    briefingRival,
    playerTeam,
    nextRace,
    trackHistory,
    weekendStories,
    gapToLeader,
    gapBehind,
    remainingRounds,
    audienceEstimate,
  });

  return {
    audienceEstimate,
    audienceRankLabel: buildAudienceRankLabel(nextRace, season),
    eventDateShort: formatEventSummaryDate(nextRace?.display_date),
    interestLabel: nextRace?.event_interest?.tier_label ?? "Padrao da temporada",
    broadcastLabel: isLiveCoverageEvent(nextRace, season) ? "Cobertura" : "Expectativa",
    broadcastValue: isLiveCoverageEvent(nextRace, season)
      ? "Ao vivo"
      : buildTeamExpectationValue({ playerStanding, teamStanding, gapToLeader, outlook }),
    headline: editorialCopy.headline,
    historyValue: editorialCopy.historyValue,
    historyMeta: editorialCopy.historyMeta,
    paragraphs: editorialCopy.paragraphs,
    goals: buildGoals({ playerStanding, teammate, teamStanding, gapToLeader, remainingRounds, outlook }),
    favorites,
    championshipTable: orderedDrivers,
    standingsTopFive,
    gapToLeaderLabel: gapToLeader === 0 ? "Lideranca" : `${gapToLeader} pts`,
    gapBehindLabel: gapBehind == null ? "Sem perseguidor direto" : `${gapBehind} pts`,
    scenario: editorialCopy.scenario,
    progressPercent: Math.max(5, Math.min(100, Math.round((currentRound / totalRounds) * 100))),
    progressLabel: `${currentRound}/${totalRounds}`,
    quote: editorialCopy.quote,
    teamVoiceLabel: playerTeam?.nome ?? "Equipe do jogador",
    paddockSupport: editorialCopy.paddockSupport ?? attendanceNarrative,
    attendanceNarrative,
    weatherIcon: buildWeatherIcon(nextRace?.clima),
    weatherSummary: buildWeatherSummary(nextRace?.clima),
    weatherNarrative: buildWeatherNarrative(nextRace?.clima),
    trackTemperatureLabel:
      nextRace?.temperatura == null ? "-" : `${Math.round(nextRace.temperatura)}°C`,
    temperatureNarrative: buildTemperatureNarrative(nextRace?.temperatura),
    trackConditionLabel: buildTrackConditionLabel(nextRace?.clima),
    boxNarrative: buildBoxNarrative(nextRace?.clima),
    timePeriodPrefix: buildTimePeriodPrefix(nextRace?.horario),
    timePeriodHighlight: buildTimePeriodHighlight(nextRace?.horario),
    actionHint: editorialCopy.actionHint,
    rivalSummary: editorialCopy.rivalSummary,
    rivalSupport: editorialCopy.rivalSupport,
    weekendStories,
    weekendStoriesMeta: editorialCopy.weekendStoriesMeta,
    weekendStoriesEmpty: editorialCopy.weekendStoriesEmpty,
  };
}

function normalizeWeekendStories(stories) {
  if (!Array.isArray(stories)) {
    return [];
  }

  return stories.map((story) => ({
    id: story.id,
    icon: story.icon,
    title: story.title,
    summary: story.summary,
    importanceLabel: story.importance ?? "Contexto",
  }));
}

function resolvePrimaryRival(orderedDrivers, playerStanding, briefingRival) {
  if (briefingRival?.driver_id) {
    const matchingDriver = orderedDrivers.find((driver) => driver.id === briefingRival.driver_id);
    if (matchingDriver) {
      return matchingDriver;
    }

    return {
      id: briefingRival.driver_id,
      nome: briefingRival.driver_name,
      posicao_campeonato: briefingRival.championship_position,
      pontos:
        briefingRival.is_ahead || !playerStanding
          ? (playerStanding?.pontos ?? 0) + (briefingRival.gap_points ?? 0)
          : Math.max(0, (playerStanding?.pontos ?? 0) - (briefingRival.gap_points ?? 0)),
    };
  }

  return resolveDirectRival(orderedDrivers, playerStanding);
}

function buildCompetitiveOutlook({ playerStanding, leader, remainingRounds, playerRating, leaderRating }) {
  if (!playerStanding || !leader) {
    return {
      titleFight: "neutral",
      targetResult: "clean",
    };
  }

  const recentKnown = recentResults(playerStanding).filter(Boolean);
  const averageFinish = recentKnown.length
    ? recentKnown.reduce((total, result) => total + (result.position ?? 12), 0) / recentKnown.length
    : null;
  const topFiveCount = recentKnown.filter((result) => !result.is_dnf && (result.position ?? 99) <= 5).length;
  const podiumCount = recentKnown.filter((result) => !result.is_dnf && (result.position ?? 99) <= 3).length;
  const winCount = recentKnown.filter((result) => !result.is_dnf && result.position === 1).length;
  const racesLeftIncludingCurrent = Math.max(1, remainingRounds + 1);
  const gapToLeader = Math.max(0, (leader.pontos ?? 0) - (playerStanding.pontos ?? 0));
  const ratingGap = Math.max(0, leaderRating - playerRating);
  const weakRecentForm = averageFinish != null && averageFinish >= 7;
  const strongRecentForm = averageFinish != null && averageFinish <= 4.5;
  const titleLongshot =
    playerStanding.posicao_campeonato >= 6 ||
    gapToLeader > racesLeftIncludingCurrent * 12 ||
    (racesLeftIncludingCurrent <= 2 && (weakRecentForm || topFiveCount === 0 || ratingGap >= 10));
  const titleContender =
    gapToLeader <= racesLeftIncludingCurrent * 6 &&
    (strongRecentForm || topFiveCount >= 2 || podiumCount >= 1 || ratingGap <= 4);

  let titleFight = "outsider";
  if (playerStanding.posicao_campeonato === 1) {
    titleFight = "leader";
  } else if (titleContender) {
    titleFight = "contender";
  } else if (titleLongshot) {
    titleFight = "longshot";
  }

  let targetResult = "top8";
  if (winCount >= 1 || podiumCount >= 2 || playerRating >= 80) {
    targetResult = "podium";
  } else if (topFiveCount >= 1 || (averageFinish != null && averageFinish <= 6)) {
    targetResult = "top5";
  }

  return {
    titleFight,
    targetResult,
    averageFinish,
    topFiveCount,
    podiumCount,
    winCount,
    racesLeftIncludingCurrent,
    gapToLeader,
  };
}

function resolveDirectRival(driverStandings, playerStanding) {
  if (!playerStanding || playerStanding.posicao_campeonato <= 0) {
    return null;
  }

  if (playerStanding.posicao_campeonato === 1) {
    return driverStandings[1] ?? null;
  }

  return driverStandings[playerStanding.posicao_campeonato - 2] ?? null;
}

function buildFavoriteRating(driver) {
  const recentScore = recentResults(driver).reduce((total, result) => {
    if (!result) return total;
    if (result.is_dnf) return total - 10;
    return total + Math.max(0, 14 - (result.position ?? 12));
  }, 0);

  const rawScore =
    (driver.skill ?? 70) * 0.74 +
    (driver.pontos ?? 0) * 0.24 +
    (driver.vitorias ?? 0) * 6 +
    (driver.podios ?? 0) * 1.4 +
    recentScore;

  return Math.max(52, Math.min(98, Math.round(rawScore / 2.1)));
}

function buildFormLabel(driver) {
  const snapshot = recentResults(driver)
    .map((result) => {
      if (!result) return "P--";
      if (result.is_dnf) return "DNF";
      return `P${result.position ?? "--"}`;
    })
    .join(" - ");

  return snapshot ? `Forma recente: ${snapshot}` : "Sem historico recente.";
}

function buildFormChips(driver) {
  const chips = recentResults(driver).map((result) => {
    if (!result) {
      return {
        label: "Sem dado",
        tone: "border-white/10 bg-white/[0.04] text-text-secondary",
      };
    }

    if (result.is_dnf) {
      return {
        label: "DNF",
        tone: "border-status-red/30 bg-status-red/12 text-status-red",
      };
    }

    const position = result.position ?? 99;
    if (position === 1) {
      return {
        label: "P1",
        tone: "border-podium-gold/30 bg-podium-gold/10 text-podium-gold",
      };
    }
    if (position === 2) {
      return {
        label: "P2",
        tone: "border-podium-silver/30 bg-podium-silver/10 text-podium-silver",
      };
    }
    if (position === 3) {
      return {
        label: "P3",
        tone: "border-podium-bronze/30 bg-podium-bronze/10 text-podium-bronze",
      };
    }

    if (position <= 6) {
      return {
        label: `P${position}`,
        tone: "border-accent-primary/25 bg-accent-primary/10 text-accent-primary",
      };
    }

    return {
      label: `P${position}`,
      tone: "border-white/10 bg-white/[0.04] text-text-secondary",
    };
  });

  return chips.length > 0
    ? chips
    : [{ label: "Sem historico", tone: "border-white/10 bg-white/[0.04] text-text-secondary" }];
}

function getFavoritePositionTone(index) {
  if (index === 0) return "text-[#f5c76d]";
  if (index === 1) return "text-[#d8dfef]";
  if (index === 2) return "text-[#cf8d63]";
  return "text-text-primary";
}

function buildGoals({ playerStanding, teammate, teamStanding, gapToLeader, remainingRounds, outlook }) {
  const teamGoal =
    teamStanding?.posicao === 1
      ? "Manter a lideranca do campeonato de equipes."
      : teamStanding
        ? `Levar a equipe ao top ${Math.min(3, teamStanding.posicao)} entre os construtores.`
        : "Sair da etapa com pontos fortes para a equipe.";

  const personalGoal = teammate
    ? `Terminar a frente de ${teammate.nome} na leitura interna do box.`
    : "Executar um fim de semana limpo, sem perdas na largada.";

  let championshipGoal = "Pontuar forte para manter o campeonato vivo.";
  if (playerStanding?.posicao_campeonato === 1) {
    championshipGoal = "Controlar os danos e sair da etapa ainda no topo.";
  } else if (outlook?.titleFight === "longshot") {
    championshipGoal = "Somar o maximo de pontos possivel e manter o campeonato respeitavel ate o fim.";
  } else if (gapToLeader <= 7) {
    championshipGoal = "Atacar a lideranca agora que a distancia e curta.";
  } else if (remainingRounds <= 3) {
    championshipGoal = "Maximizar pontos agora para nao deixar a temporada escapar.";
  }

  return [
    { label: "Meta da equipe", value: teamGoal },
    { label: "Meta pessoal", value: personalGoal },
    { label: "Meta do campeonato", value: championshipGoal },
  ];
}

function buildWeatherSummary(clima) {
  if (clima === "HeavyRain") return "Chuva forte";
  if (clima === "Wet") return "Chuva";
  if (clima === "Damp") return "Umido";
  return "Seco";
}

function buildWeatherIcon(clima) {
  if (clima === "HeavyRain") return "⛈";
  if (clima === "Wet") return "🌧";
  if (clima === "Damp") return "🌦";
  return "☀";
}

function buildWeatherNarrative(clima) {
  if (clima === "HeavyRain") return "Corrida reativa, spray alto e erro caro.";
  if (clima === "Wet") return "Pista pedindo paciencia na entrada e tracao limpa.";
  if (clima === "Damp") return "Linha mudando rapido volta a volta.";
  return "Janela previsivel para empurrar mais cedo.";
}

function buildTemperatureNarrative(temperatura) {
  if (temperatura == null) return "Leitura termica ainda indefinida para o fim de semana.";
  if (temperatura <= 16) return "Ar frio ajudando a segurar desgaste.";
  if (temperatura <= 28) return "Temperatura equilibrada para stints consistentes.";
  return "Calor cobrando mais do conjunto de pneus.";
}

function buildTrackConditionLabel(clima) {
  if (clima === "HeavyRain") return "Visibilidade apertada";
  if (clima === "Wet") return "Trajetoria molhada";
  if (clima === "Damp") return "Janela instavel";
  return "Alta aderencia";
}

function buildBoxNarrative(clima) {
  if (clima === "HeavyRain") return "Linha ideal curta e comunicacao constante.";
  if (clima === "Wet") return "Trajetoria molhada e janela sensivel.";
  if (clima === "Damp") return "Aderencia oscilando fora do trilho seco.";
  return "Alta aderencia para atacar mais cedo.";
}

function formatEventSummaryDate(displayDate) {
  if (!displayDate) return "--/--";

  const [year, month, day] = displayDate.split("-");
  if (!year || !month || !day) return displayDate;
  return `${day}/${month}`;
}

function buildTimePeriodPrefix(horario) {
  const hour = parseHour(horario);
  if (hour == null) return "Horario ";
  if (hour < 6) return "Madrugada de ";
  if (hour < 12) return "Inicio da ";
  if (hour < 18) return "Inicio da ";
  return "Inicio da ";
}

function buildTimePeriodHighlight(horario) {
  const hour = parseHour(horario);
  if (hour == null) return "pista";
  if (hour < 6) return "madrugada";
  if (hour < 12) return "manha";
  if (hour < 18) return "tarde";
  return "noite";
}

function parseHour(horario) {
  if (typeof horario !== "string") return null;
  const [rawHour] = horario.split(":");
  const parsed = Number.parseInt(rawHour, 10);
  return Number.isNaN(parsed) ? null : parsed;
}

function buildAudienceRankLabel(nextRace, season) {
  const totalRounds = Math.max(1, season?.total_rodadas ?? 1);
  const round = nextRace?.rodada ?? 1;
  const interestTier = nextRace?.event_interest?.tier_label?.toLowerCase() ?? "";

  if (round === 1 || round === totalRounds) {
    return "Maior publico da temporada";
  }

  if (interestTier.includes("principal")) {
    return "3º Maior publico da temporada";
  }

  if (interestTier.includes("alto")) {
    return "Entre os maiores publicos da temporada";
  }

  return "Movimento forte dentro da temporada";
}

function isLiveCoverageEvent(nextRace, season) {
  const totalRounds = Math.max(1, season?.total_rodadas ?? 1);
  const round = nextRace?.rodada ?? 1;
  const interestTier = nextRace?.event_interest?.tier_label?.toLowerCase() ?? "";

  return round === 1 || round === totalRounds || interestTier.includes("principal");
}

function buildTeamExpectationValue({ playerStanding, teamStanding, gapToLeader, outlook }) {
  if (playerStanding?.posicao_campeonato === 1) {
    return "Controlar a ponta";
  }

  if (outlook?.titleFight === "longshot") {
    return "Pontuar forte";
  }

  if (gapToLeader <= 10) {
    return "Pressionar a frente";
  }

  if ((teamStanding?.posicao ?? 99) <= 3) {
    return "Top 5 no radar";
  }

  return "Fim de semana limpo";
}

function estimateAudience(tierLabel) {
  if (tierLabel?.toLowerCase().includes("principal")) return 84000;
  if (tierLabel?.toLowerCase().includes("alto")) return 62000;
  if (tierLabel?.toLowerCase().includes("moderado")) return 41000;
  return 28000;
}

function formatAudience(value) {
  return value ? value.toLocaleString("pt-BR") : "-";
}

export default NextRaceTab;



function getReadableTeamColor(color) {
  if (!color || !/^#([0-9a-f]{6})$/i.test(color)) {
    return "#58a6ff";
  }

  const hex = color.slice(1);
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;

  if (luminance < 0.32) {
    const mixWithWhite = 0.58;
    const boost = (channel) => Math.round(channel + (255 - channel) * mixWithWhite);
    return `rgb(${boost(r)}, ${boost(g)}, ${boost(b)})`;
  }

  return color;
}
