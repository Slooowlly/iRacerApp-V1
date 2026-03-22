import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import DriverDetailModal from "../../components/driver/DriverDetailModal";
import ResultBadge from "../../components/standings/ResultBadge";
import TrophyBadge from "../../components/standings/TrophyBadge";
import FlagIcon from "../../components/ui/FlagIcon";
import GlassCard from "../../components/ui/GlassCard";
import useCareerStore from "../../stores/useCareerStore";

const STANDARD_POINTS = {
  1: 25,
  2: 18,
  3: 15,
  4: 12,
  5: 10,
  6: 8,
  7: 6,
  8: 4,
  9: 2,
  10: 1,
};

function StandingsTab() {
  const careerId = useCareerStore((state) => state.careerId);
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const season = useCareerStore((state) => state.season);
  const [driverStandings, setDriverStandings] = useState([]);
  const [teamStandings, setTeamStandings] = useState([]);
  const [previousChampionId, setPreviousChampionId] = useState(null);
  const [selectedDriverId, setSelectedDriverId] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let mounted = true;

    async function fetchStandings() {
      if (!careerId || !playerTeam?.categoria) {
        setLoading(false);
        return;
      }

      setLoading(true);
      setError("");

      try {
        const [drivers, teams, previousChampions] = await Promise.all([
          invoke("get_drivers_by_category", {
            careerId,
            category: playerTeam.categoria,
          }),
          invoke("get_teams_standings", {
            careerId,
            category: playerTeam.categoria,
          }),
          invoke("get_previous_champions", {
            careerId,
            category: playerTeam.categoria,
          }),
        ]);

        if (!mounted) return;

        setDriverStandings(drivers);
        setTeamStandings(teams);
        setPreviousChampionId(previousChampions.driver_champion_id ?? null);
      } catch (invokeError) {
        if (!mounted) return;

        setError(
          typeof invokeError === "string"
            ? invokeError
            : "Nao foi possivel carregar a classificacao.",
        );
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    fetchStandings();

    return () => {
      mounted = false;
    };
  }, [careerId, playerTeam?.categoria, season?.ano, season?.rodada_atual]);

  const totalRodadas = season?.total_rodadas || 0;
  const completedRounds = Math.max(0, (season?.rodada_atual || 1) - 1);
  const positionDeltaMap = useMemo(
    () => buildPositionDeltaMap(driverStandings, completedRounds),
    [driverStandings, completedRounds],
  );

  if (loading) {
    return (
      <GlassCard hover={false} className="rounded-[28px] p-10">
        <p className="text-sm uppercase tracking-[0.22em] text-accent-primary">Dashboard</p>
        <h2 className="mt-3 text-3xl font-semibold text-text-primary">Carregando classificacao</h2>
        <p className="mt-3 text-sm text-text-secondary">
          Buscando pilotos, construtores e resultados da categoria atual.
        </p>
      </GlassCard>
    );
  }

  if (error) {
    return (
      <GlassCard hover={false} className="rounded-[28px] border border-status-red/30 p-10">
        <p className="text-sm font-semibold text-status-red">{error}</p>
      </GlassCard>
    );
  }

  return (
    <>
      <div
        className="grid gap-5 xl:grid-cols-[1.6fr_0.95fr]"
      >
        <GlassCard hover={false} className="overflow-hidden rounded-[28px]">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">
                Campeonato
              </p>
              <h2 className="mt-2 text-2xl font-semibold text-text-primary">
                Classificacao de pilotos
              </h2>
            </div>
            <p className="text-sm text-text-secondary">{driverStandings.length} pilotos</p>
          </div>

          <div className="mt-6 overflow-x-auto">
            <table className="w-full min-w-[980px] table-fixed">
              <colgroup>
                <col style={{ width: "62px" }} />
                <col style={{ width: "34px" }} />
                <col style={{ width: "42px" }} />
                <col style={{ width: "214px" }} />
                <col style={{ width: "88px" }} />
                {Array.from({ length: totalRodadas }, (_, index) => (
                  <col key={`rodada-col-${index + 1}`} style={{ width: "68px" }} />
                ))}
                <col style={{ width: "66px" }} />
              </colgroup>
              <thead>
                <tr className="border-b border-white/10 text-left text-[11px] uppercase tracking-[0.18em] text-text-muted">
                  <th className="py-3 pr-0.5">Pos</th>
                  <th className="py-3 pr-0.5 text-center" />
                  <th className="py-3 pr-1 text-center">Id</th>
                  <th className="py-3 pr-1">Piloto</th>
                  <th className="py-3 pr-1">Equipe</th>
                  {Array.from({ length: totalRodadas }, (_, index) => {
                    const rodada = index + 1;
                    return (
                      <th
                        key={rodada}
                        className={[
                          "px-1.5 py-3 text-center",
                          rodada > completedRounds ? "opacity-30" : "",
                          rodada === season?.rodada_atual ? "text-accent-primary" : "",
                        ].join(" ")}
                      >
                        R{rodada}
                      </th>
                    );
                  })}
                  <th className="py-3 pl-3 text-right">Pts</th>
                </tr>
              </thead>
              <tbody>
                {driverStandings.map((driver, index) => {
                  const isSelected = selectedDriverId === driver.id;

                  return (
                    <tr
                      key={driver.id}
                      role="button"
                      tabIndex={0}
                      onClick={() => setSelectedDriverId(driver.id)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          setSelectedDriverId(driver.id);
                        }
                      }}
                      className={[
                        "cursor-pointer border-b border-white/5 transition-glass",
                        isSelected
                          ? "bg-white/[0.08] shadow-[inset_3px_0_0_0_rgba(88,166,255,1)]"
                          : "",
                        driver.is_jogador
                          ? "bg-accent-primary/8 hover:bg-accent-primary/15"
                          : "hover:bg-white/5 focus-visible:bg-white/5",
                      ].join(" ")}
                    >
                      <td className="py-3 pr-0.5 text-sm font-semibold">
                        <div className="flex items-center gap-1">
                          <span className={podiumClass(index)}>{driver.posicao_campeonato}</span>
                          <PositionDelta delta={positionDeltaMap.get(driver.id)} />
                        </div>
                      </td>
                      <td className="py-3 pr-0.5 text-center">
                        <FlagIcon nacionalidade={driver.nacionalidade} className="mx-auto" />
                      </td>
                      <td className="py-3 pr-1 text-center text-sm font-medium text-text-secondary">
                        {driver.idade ?? "—"}
                      </td>
                      <td className="py-3 pr-1">
                        <div className="flex items-center gap-2">
                          <span
                            className={[
                              "block truncate text-sm whitespace-nowrap",
                              driver.is_jogador
                                ? "font-semibold text-accent-primary"
                                : "text-text-primary",
                            ].join(" ")}
                            title={driver.nome}
                          >
                            {driver.is_jogador ? "▸ " : ""}
                            {driver.nome}
                            {driver.is_jogador ? " ◂" : ""}
                          </span>
                          {driver.id === previousChampionId ? (
                            <span className="shrink-0 text-sm" title="Campeao da temporada anterior">
                              🏆
                            </span>
                          ) : null}
                        </div>
                      </td>
                      <td className="py-3 pr-1 pl-0 text-sm font-semibold uppercase tracking-[0.02em]">
                        <div className="flex items-center gap-1.5">
                          <span
                            className="h-7 w-1.5 shrink-0 rounded-full border border-white/10"
                            style={{ backgroundColor: driver.equipe_cor ?? "#30363d" }}
                          />
                          <span
                            className="block truncate"
                            style={{ color: getReadableTeamColor(driver.equipe_cor) }}
                            title={driver.equipe_nome_curto ?? driver.equipe_nome ?? "—"}
                          >
                            {driver.equipe_nome_curto ?? driver.equipe_nome ?? "—"}
                          </span>
                        </div>
                      </td>
                      {(driver.results ?? []).map((result, rodadaIndex) => (
                        <td key={`${driver.id}-r${rodadaIndex + 1}`} className="px-1 py-3 text-center">
                          <ResultBadge result={result} />
                        </td>
                      ))}
                      <td className="py-3 pl-3 text-right font-mono text-sm font-semibold text-text-primary">
                        {driver.pontos}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </GlassCard>

        <GlassCard hover={false} className="rounded-[28px]">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">
                Construtores
              </p>
              <h2 className="mt-2 text-2xl font-semibold text-text-primary">
                Classificacao de equipes
              </h2>
            </div>
            <p className="text-sm text-text-secondary">{teamStandings.length} equipes</p>
          </div>

          <div className="mt-6 space-y-2">
            {teamStandings.map((team, index) => (
              <div
                key={team.id}
                className="flex items-center justify-between rounded-2xl border border-white/6 bg-white/[0.03] px-4 py-3 transition-glass hover:bg-white/[0.05]"
              >
                <div className="flex min-w-0 items-center gap-3">
                  <span className={["w-7 text-center text-sm font-semibold", podiumClass(index)].join(" ")}>
                    {team.posicao}
                  </span>
                  <span
                    className="h-8 w-2 rounded-full border border-white/10"
                    style={{ backgroundColor: team.cor_primaria }}
                  />
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <p
                        className="truncate text-sm font-semibold"
                        style={{ color: getReadableTeamColor(team.cor_primaria) }}
                      >
                        {team.nome}
                      </p>
                      {(team.trofeus ?? []).map((trofeu, trophyIndex) => (
                        <TrophyBadge key={`${team.id}-t${trophyIndex}`} trofeu={trofeu} />
                      ))}
                    </div>
                    <p className="truncate text-xs text-text-secondary">
                      {team.piloto_1_nome ?? "—"} / {team.piloto_2_nome ?? "—"}
                    </p>
                  </div>
                </div>
                <div className="pl-4 text-right">
                  <p className="font-mono text-base font-semibold text-text-primary">{team.pontos}</p>
                  <p className="text-xs text-text-secondary">{team.vitorias} vit.</p>
                </div>
              </div>
            ))}
          </div>
        </GlassCard>
      </div>

      {selectedDriverId ? (
        <DriverDetailModal
          driverId={selectedDriverId}
          driverIds={driverStandings.map((driver) => driver.id)}
          onSelectDriver={setSelectedDriverId}
          onClose={() => setSelectedDriverId(null)}
        />
      ) : null}
    </>
  );
}

function podiumClass(index) {
  if (index === 0) return "text-[#ffd700]";
  if (index === 1) return "text-[#c0c0c0]";
  if (index === 2) return "text-[#cd7f32]";
  return "text-text-secondary";
}

function getReadableTeamColor(color) {
  if (!color || !/^#([0-9a-f]{6})$/i.test(color)) {
    return "#7d8590";
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

function buildPositionDeltaMap(drivers, completedRounds) {
  const deltaMap = new Map();

  if (!Array.isArray(drivers) || completedRounds <= 1) {
    return deltaMap;
  }

  const previousRoundCount = completedRounds - 1;
  const previousStandings = [...drivers]
    .map((driver) => ({
      id: driver.id,
      nome: driver.nome,
      currentPosition: driver.posicao_campeonato ?? Number.MAX_SAFE_INTEGER,
      previousPoints: calculatePointsThroughRound(driver.results, previousRoundCount),
      previousBestFinish: calculateBestFinish(driver.results, previousRoundCount),
    }))
    .sort((left, right) => {
      if (right.previousPoints !== left.previousPoints) {
        return right.previousPoints - left.previousPoints;
      }
      if (left.previousBestFinish !== right.previousBestFinish) {
        return left.previousBestFinish - right.previousBestFinish;
      }
      if (left.currentPosition !== right.currentPosition) {
        return left.currentPosition - right.currentPosition;
      }
      return left.nome.localeCompare(right.nome, "pt-BR");
    });

  previousStandings.forEach((driver, index) => {
    deltaMap.set(driver.id, index + 1);
  });

  return new Map(
    drivers.map((driver) => {
      const previousPosition = deltaMap.get(driver.id);
      const currentPosition = driver.posicao_campeonato ?? 0;
      return [driver.id, previousPosition ? previousPosition - currentPosition : 0];
    }),
  );
}

function calculatePointsThroughRound(results, roundCount) {
  if (!Array.isArray(results) || roundCount <= 0) {
    return 0;
  }

  return results.slice(0, roundCount).reduce((total, result) => total + pointsForResult(result), 0);
}

function calculateBestFinish(results, roundCount) {
  if (!Array.isArray(results) || roundCount <= 0) {
    return Number.MAX_SAFE_INTEGER;
  }

  return results.slice(0, roundCount).reduce((best, result) => {
    if (!result || result.is_dnf) {
      return best;
    }
    return Math.min(best, result.position ?? Number.MAX_SAFE_INTEGER);
  }, Number.MAX_SAFE_INTEGER);
}

function pointsForResult(result) {
  if (!result || result.is_dnf) {
    return 0;
  }

  return STANDARD_POINTS[result.position] ?? 0;
}

function PositionDelta({ delta }) {
  if (!delta) {
    return <span className="text-[10px] font-semibold text-text-muted">•</span>;
  }

  if (delta > 0) {
    return (
      <span className="inline-flex items-center justify-center gap-0.5 text-[10px] font-semibold leading-none text-status-green">
        ▲{delta}
      </span>
    );
  }

  return (
    <span className="inline-flex items-center justify-center gap-0.5 text-[10px] font-semibold leading-none text-status-red">
      ▼{Math.abs(delta)}
    </span>
  );
}

export default StandingsTab;
