import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { formatDate } from "../../utils/formatters";
import GlassButton from "../../components/ui/GlassButton";
import GlassCard from "../../components/ui/GlassCard";
import LoadingOverlay from "../../components/ui/LoadingOverlay";
import useCareerStore from "../../stores/useCareerStore";

function NextRaceTab() {
  const [error, setError] = useState("");
  const [hasExistingPreseason, setHasExistingPreseason] = useState(false);
  const nextRace = useCareerStore((state) => state.nextRace);
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
  }, [careerId, nextRace]);

  async function handleSimulate() {
    setError("");

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
            <div className="text-6xl">🏁</div>
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
                  ? "⏳ Processando..."
                  : hasExistingPreseason
                    ? "📋 Continuar pre-temporada"
                    : "⏭ Avancar para pre-temporada"}
              </GlassButton>
            </div>
            {error ? <p className="mt-4 text-sm text-status-red">❌ {error}</p> : null}
          </div>
        </GlassCard>
      </div>
    );
  }

  return (
    <div className="relative grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
      <LoadingOverlay
        open={isSimulating}
        title="Simulando corrida"
        message="Classificacao, corrida e atualizacao do campeonato em andamento."
      />

      <GlassCard hover={false} className="glass-strong rounded-[30px] p-8">
        <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">
          Proxima corrida
        </p>
        <h2 className="mt-3 text-4xl font-semibold tracking-[-0.04em] text-text-primary">
          {nextRace.track_name}
        </h2>
        <p className="mt-3 text-sm text-text-secondary">
          Rodada {nextRace.rodada} de {season?.total_rodadas ?? "?"}
        </p>

        <div className="mt-8 grid gap-4 md:grid-cols-3">
          <InfoBlock label="Clima" value={weatherLabel(nextRace.clima)} />
          <InfoBlock label="Duracao" value={`${nextRace.duracao_corrida_min} min`} />
          <InfoBlock label="Status" value={raceStatusLabel(nextRace.status)} />
          <InfoBlock
            label="Data"
            value={nextRace.display_date ? formatDate(nextRace.display_date) : "—"}
          />
        </div>

        {nextRace.event_interest && (
          <div className="mt-4 rounded-2xl border border-accent-primary/20 bg-accent-primary/5 px-4 py-4">
            <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
              Interesse do evento
            </p>
            <div className="mt-2 flex items-baseline gap-3">
              <span className="text-2xl font-semibold text-text-primary">
                {nextRace.event_interest.display_value.toLocaleString("pt-BR")}
              </span>
              <span className="text-sm text-text-secondary">
                {nextRace.event_interest.tier_label}
              </span>
            </div>
          </div>
        )}

        {thematicBadge(nextRace.thematic_slot)}

        {error ? (
          <div className="mt-6 rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-4 text-sm text-status-red">
            <p>❌ Erro ao simular: {error}</p>
            <button
              type="button"
              onClick={() => setError("")}
              className="mt-2 text-xs text-text-secondary transition-glass hover:text-text-primary"
            >
              Fechar
            </button>
          </div>
        ) : null}

        <div className="mt-8 flex flex-col gap-3 sm:flex-row">
          <GlassButton
            variant="primary"
            disabled={isSimulating || !nextRace}
            className="sm:min-w-44"
            onClick={handleSimulate}
          >
            {isSimulating ? "⏳ Simulando..." : "🎮 Simular corrida"}
          </GlassButton>
          <GlassButton variant="secondary" disabled className="sm:min-w-44">
            📤 Exportar em breve
          </GlassButton>
        </div>
      </GlassCard>

      <GlassCard hover={false} className="rounded-[30px]">
        <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">Briefing</p>
        <h3 className="mt-3 text-2xl font-semibold text-text-primary">Resumo do fim de semana</h3>

        <div className="mt-6 space-y-4 text-sm text-text-secondary">
          <p>
            A etapa ja esta pronta no calendario. Agora o botao de simulacao executa o fim de
            semana completo e atualiza o campeonato ao final.
          </p>
          <p>A exportacao para o iRacing continua reservada para o proximo modulo.</p>
        </div>

        <div className="mt-8 rounded-2xl border border-status-yellow/25 bg-status-yellow/10 px-4 py-4 text-sm text-text-secondary">
          A simulacao roda qualifying, corrida e recalculo do campeonato. Depois do resultado, o
          dashboard recarrega a carreira automaticamente.
        </div>
      </GlassCard>
    </div>
  );
}

function InfoBlock({ label, value }) {
  return (
    <div className="glass-light rounded-2xl p-4">
      <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
      <p className="mt-2 text-base font-semibold text-text-primary">{value}</p>
    </div>
  );
}

function weatherLabel(value) {
  if (value === "HeavyRain") return "Chuva forte";
  if (value === "Wet") return "Chuva";
  if (value === "Damp") return "Umido";
  return "Seco";
}

function raceStatusLabel(value) {
  return value === "Concluida" ? "Concluida" : "Pendente";
}

const THEMATIC_BADGE_CONFIG = {
  AberturaDaTemporada:  { label: "Abertura da Temporada", color: "border-status-green/30 bg-status-green/10 text-status-green" },
  FinalDaTemporada:     { label: "Grande Final",           color: "border-accent-gold/40 bg-accent-gold/10 text-accent-gold" },
  TensaoPreFinal:       { label: "Tensao Pre-Final",       color: "border-status-yellow/30 bg-status-yellow/10 text-status-yellow" },
  MidpointPrestigio:    { label: "Etapa de Prestigio",     color: "border-accent-primary/30 bg-accent-primary/10 text-accent-primary" },
  VisitanteRegional:    { label: "Visita Especial",        color: "border-accent-primary/20 bg-accent-primary/5 text-text-secondary" },
  AberturaEspecial:     { label: "Abertura do Bloco Especial", color: "border-status-green/30 bg-status-green/10 text-status-green" },
  FinalEspecial:        { label: "Final do Bloco Especial",    color: "border-accent-gold/40 bg-accent-gold/10 text-accent-gold" },
};

function thematicBadge(slot) {
  const config = THEMATIC_BADGE_CONFIG[slot];
  if (!config) return null;
  return (
    <div className={`mt-4 inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-medium tracking-wide ${config.color}`}>
      {config.label}
    </div>
  );
}

export default NextRaceTab;
