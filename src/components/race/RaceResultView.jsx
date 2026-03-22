import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassButton from "../ui/GlassButton";
import GlassCard from "../ui/GlassCard";
import useCareerStore from "../../stores/useCareerStore";
import { formatGap, formatLapTime } from "../../utils/formatters";

function RaceResultView({ result, onDismiss }) {
  const careerId = useCareerStore((state) => state.careerId);
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const otherCategoriesResult = useCareerStore((state) => state.otherCategoriesResult);
  const [showChampionship, setShowChampionship] = useState(false);
  const [championship, setChampionship] = useState([]);
  const [loadingChampionship, setLoadingChampionship] = useState(false);
  const [championshipError, setChampionshipError] = useState("");

  const playerResult = useMemo(
    () => result?.race_results?.find((entry) => entry.is_jogador) ?? null,
    [result],
  );
  const winner = useMemo(
    () => result?.race_results?.find((entry) => entry.finish_position === 1) ?? null,
    [result],
  );
  const poleSitter = useMemo(
    () => result?.qualifying_results?.find((entry) => entry.is_pole) ?? null,
    [result],
  );
  const fastestLap = useMemo(
    () => result?.race_results?.find((entry) => entry.has_fastest_lap) ?? null,
    [result],
  );
  const biggestGainer = useMemo(() => {
    const activeResults = result?.race_results?.filter((entry) => !entry.is_dnf) ?? [];
    if (activeResults.length === 0) return null;
    return activeResults.reduce((best, entry) =>
      entry.positions_gained > best.positions_gained ? entry : best,
    );
  }, [result]);

  useEffect(() => {
    let mounted = true;

    async function fetchChampionship() {
      if (!showChampionship || !careerId || !playerTeam?.categoria) return;

      setLoadingChampionship(true);
      setChampionshipError("");

      try {
        const data = await invoke("get_drivers_by_category", {
          careerId,
          category: playerTeam.categoria,
        });
        if (mounted) {
          setChampionship(data);
        }
      } catch (error) {
        if (mounted) {
          setChampionshipError(
            typeof error === "string"
              ? error
              : "Nao foi possivel carregar o campeonato atualizado.",
          );
        }
      } finally {
        if (mounted) {
          setLoadingChampionship(false);
        }
      }
    }

    fetchChampionship();
    return () => {
      mounted = false;
    };
  }, [showChampionship, careerId, playerTeam?.categoria]);

  if (!result) return null;

  return (
    <div className="space-y-6">
      <GlassCard hover={false} className="glass-strong rounded-[30px] p-8 lg:p-10">
        <div className="flex flex-col gap-4 border-b border-white/10 pb-6 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <p className="text-[11px] uppercase tracking-[0.24em] text-accent-primary">
              Resultado da corrida
            </p>
            <h1 className="mt-3 text-3xl font-semibold tracking-[-0.04em] text-text-primary lg:text-4xl">
              {result.track_name}
            </h1>
            <p className="mt-2 text-sm text-text-secondary">
              {weatherLabel(result.weather)} • {result.total_laps} voltas
            </p>
          </div>

          <div className="glass-light rounded-2xl px-5 py-4 text-sm text-text-secondary">
            <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">Vencedor</p>
            <p className="mt-2 text-lg font-semibold text-text-primary">
              {winner?.pilot_name ?? "—"}
            </p>
            <p className="mt-1">{winner?.team_name ?? "—"}</p>
          </div>
        </div>

        <div className="mt-6 grid gap-4 lg:grid-cols-4">
          <HighlightCard
            label="Pole position"
            value={poleSitter?.pilot_name ?? "—"}
            detail={poleSitter ? formatLapTime(poleSitter.best_lap_time_ms) : "—"}
          />
          <HighlightCard
            label="Volta mais rapida"
            value={fastestLap?.pilot_name ?? "—"}
            detail={fastestLap ? formatLapTime(fastestLap.best_lap_time_ms) : "—"}
          />
          <HighlightCard
            label="Maior ganho"
            value={biggestGainer?.pilot_name ?? "—"}
            detail={biggestGainer ? deltaLabel(biggestGainer.positions_gained) : "—"}
          />
          <HighlightCard
            label="Seu resultado"
            value={playerResult ? `P${playerResult.finish_position}` : "—"}
            detail={
              playerResult
                ? `Grid P${playerResult.grid_position} • ${deltaLabel(playerResult.positions_gained)}`
                : "Sem piloto"
            }
          />
        </div>

        <div className="mt-8 overflow-x-auto">
          <table className="w-full min-w-[860px]">
            <thead>
              <tr className="border-b border-white/10 text-left text-[11px] uppercase tracking-[0.18em] text-text-muted">
                <th className="py-3 pr-4">Pos</th>
                <th className="py-3 pr-4">Grid</th>
                <th className="py-3 pr-4">Piloto</th>
                <th className="py-3 pr-4">Equipe</th>
                <th className="py-3 pr-4 text-right">Pts</th>
                <th className="py-3 pr-4 text-right">+/-</th>
                <th className="py-3 text-right">Gap</th>
              </tr>
            </thead>
            <tbody>
              {result.race_results.map((entry) => (
                <tr
                  key={entry.pilot_id}
                  className={[
                    "border-b border-white/5 transition-glass hover:bg-white/5",
                    entry.is_jogador ? "bg-accent-primary/10" : "",
                  ].join(" ")}
                >
                  <td className="py-3 pr-4 text-sm font-semibold">
                    <span className={positionClass(entry.finish_position)}>
                      {positionPrefix(entry.finish_position)}
                      {entry.is_dnf ? "DNF" : entry.finish_position}
                    </span>
                  </td>
                  <td className="py-3 pr-4 text-sm text-text-secondary">{entry.grid_position}</td>
                  <td className="py-3 pr-4">
                    <div className="flex items-center gap-2">
                      {entry.has_fastest_lap ? <span title="Volta mais rapida">⚡</span> : null}
                      <span
                        className={[
                          "text-sm",
                          entry.is_jogador ? "font-semibold text-accent-primary" : "text-text-primary",
                        ].join(" ")}
                      >
                        {entry.is_jogador ? `▶ ${entry.pilot_name} ◀` : entry.pilot_name}
                      </span>
                    </div>
                  </td>
                  <td className="py-3 pr-4 text-sm text-text-secondary">{entry.team_name}</td>
                  <td className="py-3 pr-4 text-right font-mono text-sm font-semibold text-text-primary">
                    {entry.points_earned}
                  </td>
                  <td
                    className={[
                      "py-3 pr-4 text-right font-mono text-sm font-semibold",
                      deltaClass(entry.positions_gained),
                    ].join(" ")}
                  >
                    {deltaLabel(entry.positions_gained)}
                  </td>
                  <td className="py-3 text-right font-mono text-sm text-text-secondary">
                    {entry.finish_position === 1 ? formatLapTime(entry.total_race_time_ms) : formatGap(entry.gap_to_winner_ms)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="mt-8 rounded-[26px] border border-white/10 bg-white/[0.03] px-5 py-5">
          <button
            onClick={() => setShowChampionship((current) => !current)}
            className="text-sm font-semibold text-accent-primary transition-glass hover:text-accent-hover"
          >
            {showChampionship ? "▲ Ocultar campeonato" : "▼ Ver campeonato atualizado"}
          </button>

          {showChampionship ? (
            <div className="mt-4">
              {loadingChampionship ? (
                <p className="text-sm text-text-secondary">Atualizando classificacao da categoria...</p>
              ) : championshipError ? (
                <div className="rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-3 text-sm text-status-red">
                  {championshipError}
                </div>
              ) : (
                <div className="space-y-2">
                  {championship.slice(0, 10).map((driver) => (
                    <div
                      key={driver.id}
                      className={[
                        "flex items-center justify-between rounded-2xl border border-white/6 bg-white/[0.03] px-4 py-3",
                        driver.is_jogador ? "border-accent-primary/30 bg-accent-primary/10" : "",
                      ].join(" ")}
                    >
                      <div className="flex items-center gap-3">
                        <span className={["w-7 text-center text-sm font-semibold", podiumClass(driver.posicao_campeonato - 1)].join(" ")}>
                          {driver.posicao_campeonato}
                        </span>
                        <div>
                          <p className={["text-sm", driver.is_jogador ? "font-semibold text-accent-primary" : "text-text-primary"].join(" ")}>
                            {driver.nome}
                          </p>
                          <p className="text-xs text-text-secondary">{driver.equipe_nome ?? "—"}</p>
                        </div>
                      </div>
                      <div className="text-right">
                        <p className="font-mono text-base font-semibold text-text-primary">{driver.pontos}</p>
                        <p className="text-xs text-text-secondary">{driver.vitorias} vitorias</p>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : null}
        </div>

        {otherCategoriesResult?.total_races_simulated > 0 ? (
          <div className="mt-8">
            <OtherCategoriesSection summary={otherCategoriesResult} />
          </div>
        ) : null}

        <div className="mt-8 flex justify-end">
          <GlassButton variant="primary" onClick={onDismiss} className="min-w-48">
            Continuar
          </GlassButton>
        </div>
      </GlassCard>
    </div>
  );
}

function HighlightCard({ label, value, detail }) {
  return (
    <div className="glass-light rounded-2xl p-4">
      <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
      <p className="mt-3 text-base font-semibold text-text-primary">{value}</p>
      <p className="mt-1 text-sm text-text-secondary">{detail}</p>
    </div>
  );
}

function OtherCategoriesSection({ summary }) {
  return (
    <div className="rounded-[26px] border border-white/10 bg-white/[0.03] px-5 py-5">
      <div className="flex items-center justify-between gap-4 border-b border-white/10 pb-4">
        <div>
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
            Outras categorias
          </p>
          <p className="mt-2 text-lg font-semibold text-text-primary">
            {summary.total_races_simulated} corrida
            {summary.total_races_simulated > 1 ? "s" : ""} simulada
            {summary.total_races_simulated > 1 ? "s" : ""}
          </p>
        </div>
        <div className="glass-light rounded-2xl px-4 py-3 text-right">
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">Categorias</p>
          <p className="mt-2 text-base font-semibold text-text-primary">
            {summary.categories_simulated.length}
          </p>
        </div>
      </div>

      <div className="mt-5 space-y-4">
        {summary.categories_simulated.map((category) => (
          <div
            key={category.category_id}
            className="rounded-2xl border border-white/6 bg-white/[0.03] px-4 py-4"
          >
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-semibold text-text-primary">{category.category_name}</p>
                <p className="text-xs text-text-secondary">
                  {category.races_simulated} corrida
                  {category.races_simulated > 1 ? "s" : ""} processada
                  {category.races_simulated > 1 ? "s" : ""}
                </p>
              </div>
            </div>

            <div className="mt-3 space-y-2">
              {category.results.map((race) => (
                <div
                  key={race.race_id}
                  className="flex flex-col gap-1 rounded-2xl border border-white/5 bg-black/10 px-3 py-3 text-sm md:flex-row md:items-center md:justify-between"
                >
                  <span className="text-text-primary">{race.track_name}</span>
                  <span className="text-text-secondary">
                    🏆 {race.winner_name} ({race.winner_team})
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function weatherLabel(value) {
  if (value === "HeavyRain") return "Chuva forte";
  if (value === "Wet") return "Chuva";
  if (value === "Damp") return "Umido";
  return "Seco";
}

function positionPrefix(position) {
  if (position === 1) return "🥇";
  if (position === 2) return "🥈";
  if (position === 3) return "🥉";
  return "";
}

function positionClass(position) {
  if (position === 1) return "text-[#ffd700]";
  if (position === 2) return "text-[#c0c0c0]";
  if (position === 3) return "text-[#cd7f32]";
  return "text-text-primary";
}

function podiumClass(index) {
  if (index === 0) return "text-[#ffd700]";
  if (index === 1) return "text-[#c0c0c0]";
  if (index === 2) return "text-[#cd7f32]";
  return "text-text-secondary";
}

function deltaClass(value) {
  if (value > 0) return "text-status-green";
  if (value < 0) return "text-status-red";
  return "text-text-secondary";
}

function deltaLabel(value) {
  if (value > 0) return `+${value}`;
  return `${value}`;
}

export default RaceResultView;
