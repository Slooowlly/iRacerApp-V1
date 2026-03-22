import GlassCard from "../ui/GlassCard";
import useCareerStore from "../../stores/useCareerStore";
import { categoryLabel } from "../../utils/formatters";
import TabNavigation from "./TabNavigation";

function Header({ activeTab, onTabChange }) {
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const season = useCareerStore((state) => state.season);
  const nextRace = useCareerStore((state) => state.nextRace);

  return (
    <header className="relative z-20 border-b border-white/10 bg-black/20 px-3 py-4 backdrop-blur-xl sm:px-4 lg:px-5 xl:px-6">
      <div className="mx-auto w-full max-w-[1680px]">
        <GlassCard hover={false} className="glass-strong rounded-[28px] px-5 py-5 sm:px-6">
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
              <div className="flex min-w-0 flex-1 items-start gap-4 rounded-2xl">
                <div
                  className="mt-1 h-4 w-4 rounded-full border border-white/20"
                  style={{ backgroundColor: playerTeam?.cor_primaria ?? "#58a6ff" }}
                />

                <div className="min-w-0">
                  <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">
                    Dashboard da carreira
                  </p>
                  <h1 className="mt-2 truncate text-2xl font-semibold text-text-primary sm:text-3xl">
                    {playerTeam?.nome ?? "Equipe ativa"}
                  </h1>
                  <p className="mt-2 text-sm text-text-secondary">
                    {categoryLabel(playerTeam?.categoria)}{" "}
                    {nextRace
                      ? `• Próxima pista: ${nextRace.track_name}`
                      : "• Sem corrida pendente"}
                  </p>
                </div>
              </div>

              <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
                <div className="glass-light rounded-2xl px-4 py-3 text-sm text-text-secondary">
                  <span className="font-semibold text-text-primary">
                    Temporada {season?.numero ?? 1}
                  </span>{" "}
                  • Ano {season?.ano ?? 2024} • Corrida {season?.rodada_atual ?? 1}/
                  {season?.total_rodadas ?? 0}
                </div>
              </div>
            </div>

            <div className="border-t border-white/10 pt-4">
              <TabNavigation activeTab={activeTab} onTabChange={onTabChange} />
            </div>
          </div>
        </GlassCard>
      </div>
    </header>
  );
}

export default Header;
