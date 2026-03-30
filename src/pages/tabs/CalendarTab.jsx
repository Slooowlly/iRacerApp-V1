import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassCard from "../../components/ui/GlassCard";
import useCareerStore from "../../stores/useCareerStore";
import { categoryLabel } from "../../utils/formatters";

function CalendarTab() {
  const careerId = useCareerStore((state) => state.careerId);
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const nextRace = useCareerStore((state) => state.nextRace);
  const season = useCareerStore((state) => state.season);
  const [calendar, setCalendar] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let mounted = true;

    async function fetchCalendar() {
      if (!careerId || !playerTeam?.categoria) {
        setLoading(false);
        return;
      }

      setLoading(true);
      setError("");

      try {
        const entries = await invoke("get_calendar_for_category", {
          careerId,
          category: playerTeam.categoria,
        });
        if (mounted) {
          setCalendar(entries);
        }
      } catch (invokeError) {
        if (mounted) {
          setError(
            typeof invokeError === "string"
              ? invokeError
              : "Nao foi possivel carregar o calendario da categoria.",
          );
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    fetchCalendar();
    return () => {
      mounted = false;
    };
  }, [careerId, playerTeam?.categoria, season?.rodada_atual]);

  return (
    <GlassCard hover={false} className="rounded-[28px]">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">Calendario</p>
          <h2 className="mt-2 text-3xl font-semibold text-text-primary">
            {categoryLabel(playerTeam?.categoria)}
          </h2>
        </div>
        <p className="text-sm text-text-secondary">{calendar.length} etapas previstas</p>
      </div>

      {loading ? (
        <p className="mt-6 text-sm text-text-secondary">Carregando calendario da temporada...</p>
      ) : error ? (
        <div className="mt-6 rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-3 text-sm text-status-red">
          {error}
        </div>
      ) : (
        <div className="mt-6 overflow-x-auto">
          <table className="w-full min-w-[720px]">
            <thead>
              <tr className="border-b border-white/10 text-left text-[11px] uppercase tracking-[0.18em] text-text-muted">
                <th className="py-3 pr-4">Rd</th>
                <th className="py-3 pr-4">Pista</th>
                <th className="py-3 pr-4">Clima</th>
                <th className="py-3 pr-4">Duracao</th>
                <th className="py-3">Status</th>
              </tr>
            </thead>
            <tbody>
              {calendar.map((race) => {
                const isNext = nextRace?.id === race.id;
                return (
                  <tr
                    key={race.id}
                    className={[
                      "border-b border-white/5 transition-glass hover:bg-white/5",
                      isNext ? "bg-accent-primary/10" : "",
                    ].join(" ")}
                  >
                    <td className="py-3 pr-4 text-sm font-semibold text-text-primary">{race.rodada}</td>
                    <td className="py-3 pr-4 text-sm text-text-primary">{race.track_name}</td>
                    <td className="py-3 pr-4 text-sm text-text-secondary">{weatherLabel(race.clima)}</td>
                    <td className="py-3 pr-4 text-sm text-text-secondary">
                      {race.duracao_corrida_min} min
                    </td>
                    <td className="py-3 text-sm">
                      <span
                        className={[
                          "rounded-full px-3 py-1",
                          isNext
                            ? "bg-accent-primary/15 text-accent-primary"
                            : race.status === "Concluida"
                              ? "bg-status-green/15 text-status-green"
                              : "bg-white/8 text-text-secondary",
                        ].join(" ")}
                      >
                        {isNext ? "Proxima" : race.status === "Concluida" ? "Concluida" : "Pendente"}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </GlassCard>
  );
}

function weatherLabel(value) {
  if (value === "HeavyRain") return "Chuva forte";
  if (value === "Wet") return "Chuva";
  if (value === "Damp") return "Umido";
  return "Seco";
}

export default CalendarTab;
