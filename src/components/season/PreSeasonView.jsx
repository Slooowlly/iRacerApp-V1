import { useState } from "react";

import useCareerStore from "../../stores/useCareerStore";
import {
  formatCategoryName,
  formatPreseasonPhase,
  formatSalary,
} from "../../utils/formatters";
import GlassButton from "../ui/GlassButton";
import GlassCard from "../ui/GlassCard";
import LoadingOverlay from "../ui/LoadingOverlay";

function eventIcon(eventType) {
  const iconMap = {
    ContractExpired: "📋",
    ContractRenewed: "✍️",
    TransferCompleted: "📋",
    TransferRejected: "❌",
    RookieSigned: "🎓",
    PlayerProposalReceived: "💼",
    HierarchyUpdated: "⚡",
    PreSeasonComplete: "🏁",
    Mercado: "📋",
    Rookies: "🎓",
    Hierarquia: "⚡",
    PreTemporada: "🏁",
    Promocao: "⬆️",
    Rebaixamento: "⬇️",
    Aposentadoria: "👴",
    Evolucao: "📈",
  };

  return iconMap[eventType] || "📰";
}

function eventTone(eventType) {
  const toneMap = {
    TransferCompleted: "text-text-primary",
    PlayerProposalReceived: "text-accent-primary",
    PreSeasonComplete: "text-status-green",
    TransferRejected: "text-status-red",
    RookieSigned: "text-[#d2a8ff]",
    Mercado: "text-text-primary",
    Rookies: "text-[#d2a8ff]",
    Hierarquia: "text-text-secondary",
    PreTemporada: "text-status-green",
    Promocao: "text-status-green",
    Rebaixamento: "text-status-red",
  };

  return toneMap[eventType] || "text-text-secondary";
}

function PreSeasonProgress({ state }) {
  if (!state) return null;

  const safeCurrent = Math.min(state.current_week, state.total_weeks || 1);
  const progress = Math.round((safeCurrent / Math.max(state.total_weeks || 1, 1)) * 100);

  return (
    <GlassCard hover={false} className="glass-strong rounded-[32px] p-7">
      <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.24em] text-accent-primary">
            Pre-temporada
          </p>
          <h1 className="mt-3 text-3xl font-semibold tracking-[-0.04em] text-text-primary">
            Semana {safeCurrent} de {state.total_weeks}
          </h1>
          <p className="mt-2 text-sm text-text-secondary">
            Fase atual: {formatPreseasonPhase(state.phase)}
          </p>
        </div>

        <div className="min-w-[220px]">
          <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.18em] text-text-secondary">
            <span>Progresso</span>
            <span>{progress}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-white/8">
            <div
              className="h-full rounded-full bg-accent-primary transition-[width] duration-700 ease-out"
              style={{ width: `${Math.min(progress, 100)}%` }}
            />
          </div>
        </div>
      </div>
    </GlassCard>
  );
}

function NewsEventItem({ event }) {
  return (
    <div className="flex items-start gap-3 rounded-2xl border border-white/6 bg-white/[0.025] px-4 py-3">
      <span className="text-lg">{eventIcon(event.event_type)}</span>
      <div className="min-w-0 flex-1">
        <p className={`text-sm font-medium ${eventTone(event.event_type)}`}>{event.headline}</p>
        {event.description ? (
          <p className="mt-1 text-xs leading-5 text-text-secondary">{event.description}</p>
        ) : null}
      </div>
    </div>
  );
}

function NewsFeed({ weeks }) {
  const reversedWeeks = [...weeks].reverse();

  return (
    <GlassCard hover={false} className="rounded-[28px] p-6">
      <div className="mb-4 flex items-center justify-between gap-3">
        <h3 className="text-sm font-semibold uppercase tracking-[0.18em] text-text-primary">
          📰 Noticias do mercado
        </h3>
        <span className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
          {weeks.length} semana{weeks.length === 1 ? "" : "s"}
        </span>
      </div>

      {reversedWeeks.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-white/10 bg-white/[0.02] px-5 py-8 text-center text-sm text-text-secondary">
          Avance a primeira semana para comecar o feed de noticias da pre-temporada.
        </div>
      ) : (
        <div className="space-y-5">
          {reversedWeeks.map((week) => (
            <div key={week.week_number}>
              <div className="mb-3 flex items-center gap-3">
                <div className="h-px flex-1 bg-white/8" />
                <span className="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-muted">
                  Semana {week.week_number}
                </span>
                <div className="h-px flex-1 bg-white/8" />
              </div>
              <div className="space-y-2">
                {week.events.map((event, index) => (
                  <NewsEventItem key={`${week.week_number}-${index}`} event={event} />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </GlassCard>
  );
}

function ProposalCard({ proposal, onRespond, isResponding }) {
  return (
    <div className="rounded-[24px] border border-accent-primary/15 bg-accent-primary/[0.05] p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div
            className="h-3 w-3 rounded-full ring-4 ring-white/5"
            style={{ backgroundColor: proposal.equipe_cor_primaria }}
          />
          <div>
            <h4 className="text-sm font-semibold text-text-primary">{proposal.equipe_nome}</h4>
            <p className="text-xs text-text-secondary">
              {proposal.categoria_nome || formatCategoryName(proposal.categoria)}
            </p>
          </div>
        </div>
        <span
          className={[
            "rounded-full px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.18em]",
            proposal.papel === "N1"
              ? "bg-[#ffd700]/15 text-[#ffd700]"
              : "bg-white/10 text-text-secondary",
          ].join(" ")}
        >
          {proposal.papel}
        </span>
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-2">
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3">
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">Salario</p>
          <p className="mt-2 text-sm font-medium text-text-primary">
            {formatSalary(proposal.salario_oferecido)}/ano
          </p>
        </div>
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3">
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">Duracao</p>
          <p className="mt-2 text-sm font-medium text-text-primary">
            {proposal.duracao_anos} ano{proposal.duracao_anos > 1 ? "s" : ""}
          </p>
        </div>
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3">
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
            Carro da equipe
          </p>
          <div className="mt-2 flex items-center gap-2">
            <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-white/8">
              <div
                className="h-full rounded-full bg-accent-primary"
                style={{ width: `${proposal.car_performance_rating}%` }}
              />
            </div>
            <span className="text-xs text-text-secondary">{proposal.car_performance_rating}</span>
          </div>
        </div>
        <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3">
          <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">Companheiro</p>
          <p className="mt-2 text-sm font-medium text-text-primary">
            {proposal.companheiro_nome || "Vaga em aberto"}
          </p>
        </div>
      </div>

      <div className="mt-4 flex flex-col gap-2 sm:flex-row">
        <GlassButton
          variant="primary"
          className="flex-1"
          disabled={isResponding}
          onClick={() => onRespond(proposal.proposal_id, true)}
        >
          ✅ Aceitar
        </GlassButton>
        <GlassButton
          variant="secondary"
          className="flex-1"
          disabled={isResponding}
          onClick={() => onRespond(proposal.proposal_id, false)}
        >
          ❌ Recusar
        </GlassButton>
      </div>
    </div>
  );
}

function PlayerProposalsSection({ proposals, onRespond, isResponding }) {
  if (!proposals?.length) return null;

  return (
    <GlassCard hover={false} className="rounded-[28px] border border-accent-primary/15 p-6">
      <div className="mb-4 flex items-center gap-3">
        <span className="text-lg">💼</span>
        <h3 className="text-sm font-semibold uppercase tracking-[0.18em] text-accent-primary">
          Propostas para voce
        </h3>
        <span className="rounded-full bg-accent-primary/15 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-accent-primary">
          {proposals.length}
        </span>
      </div>

      <div className="space-y-3">
        {proposals.map((proposal) => (
          <ProposalCard
            key={proposal.proposal_id}
            proposal={proposal}
            onRespond={onRespond}
            isResponding={isResponding}
          />
        ))}
      </div>
    </GlassCard>
  );
}

function ActionFooter({
  canAdvanceWeek,
  canFinalize,
  hasProposals,
  isAdvancingWeek,
  onAdvanceWeek,
  onFinalize,
  preseasonState,
}) {
  return (
    <div className="flex flex-col items-center gap-4 pt-2">
      <div className="flex flex-wrap justify-center gap-3">
        {canAdvanceWeek ? (
          <GlassButton
            variant="primary"
            className="min-w-[220px]"
            disabled={isAdvancingWeek}
            onClick={onAdvanceWeek}
          >
            {isAdvancingWeek ? "⏳ Processando..." : "⏭ Avancar semana"}
          </GlassButton>
        ) : null}

        {canFinalize ? (
          <GlassButton variant="success" className="min-w-[220px] glow-green" onClick={onFinalize}>
            🚀 Iniciar temporada
          </GlassButton>
        ) : null}
      </div>

      {preseasonState?.is_complete && hasProposals ? (
        <p className="text-sm text-status-yellow">
          Resolva suas propostas pendentes antes de iniciar a temporada.
        </p>
      ) : null}
    </div>
  );
}

function PreSeasonView() {
  const preseasonState = useCareerStore((state) => state.preseasonState);
  const preseasonWeeks = useCareerStore((state) => state.preseasonWeeks);
  const playerProposals = useCareerStore((state) => state.playerProposals);
  const isAdvancingWeek = useCareerStore((state) => state.isAdvancingWeek);
  const isRespondingProposal = useCareerStore((state) => state.isRespondingProposal);
  const advanceMarketWeek = useCareerStore((state) => state.advanceMarketWeek);
  const respondToProposal = useCareerStore((state) => state.respondToProposal);
  const finalizePreseason = useCareerStore((state) => state.finalizePreseason);

  const [error, setError] = useState("");
  const [responseMessage, setResponseMessage] = useState("");

  const canAdvanceWeek = preseasonState && !preseasonState.is_complete && !isAdvancingWeek;
  const canFinalize = preseasonState?.is_complete && playerProposals.length === 0;
  const hasProposals = playerProposals.length > 0;

  async function handleAdvanceWeek() {
    setError("");

    try {
      await advanceMarketWeek();
    } catch (invokeError) {
      setError(invokeError?.toString?.() ?? "Nao foi possivel avancar a semana.");
    }
  }

  async function handleRespond(proposalId, accept) {
    setError("");

    try {
      const response = await respondToProposal(proposalId, accept);
      setResponseMessage(response.message);
      window.setTimeout(() => setResponseMessage(""), 4000);
    } catch (invokeError) {
      setError(invokeError?.toString?.() ?? "Nao foi possivel responder a proposta.");
    }
  }

  async function handleFinalize() {
    setError("");

    try {
      await finalizePreseason();
    } catch (invokeError) {
      setError(invokeError?.toString?.() ?? "Nao foi possivel iniciar a temporada.");
    }
  }

  return (
    <div className="relative mx-auto flex max-w-5xl flex-col gap-5 pb-8">
      <LoadingOverlay
        open={isAdvancingWeek}
        title="Pre-temporada em andamento"
        message="Negociando contratos, propostas e movimentacoes do paddock."
      />

      <PreSeasonProgress state={preseasonState} />
      <NewsFeed weeks={preseasonWeeks} />
      <PlayerProposalsSection
        proposals={playerProposals}
        onRespond={handleRespond}
        isResponding={isRespondingProposal}
      />

      <GlassCard hover={false} className="rounded-[28px] p-5">
        <div className="grid gap-4 md:grid-cols-3">
          <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-4">
            <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
              Fase seguinte
            </p>
            <p className="mt-2 text-sm font-medium text-text-primary">
              {formatPreseasonPhase(preseasonState?.phase)}
            </p>
          </div>
          <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-4">
            <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
              Vagas restantes
            </p>
            <p className="mt-2 text-sm font-medium text-text-primary">
              {preseasonWeeks.at(-1)?.remaining_vacancies ?? 0}
            </p>
          </div>
          <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-4">
            <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
              Seu status
            </p>
            <p className="mt-2 text-sm font-medium text-text-primary">
              {hasProposals ? `${playerProposals.length} proposta(s)` : "Sem pendencias"}
            </p>
          </div>
        </div>

        {error ? (
          <div className="mt-4 rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-4 text-sm text-status-red">
            {error}
          </div>
        ) : null}

        <ActionFooter
          canAdvanceWeek={canAdvanceWeek}
          canFinalize={canFinalize}
          hasProposals={hasProposals}
          isAdvancingWeek={isAdvancingWeek}
          onAdvanceWeek={handleAdvanceWeek}
          onFinalize={handleFinalize}
          preseasonState={preseasonState}
        />
      </GlassCard>

      {responseMessage ? (
        <div className="animate-scale-in fixed bottom-6 right-6 z-50 max-w-sm rounded-[22px] border border-accent-primary/20 bg-[#07111fcc] px-5 py-4 shadow-[0_20px_50px_rgba(0,0,0,0.35)] backdrop-blur-xl">
          <p className="text-sm text-text-primary">{responseMessage}</p>
        </div>
      ) : null}
    </div>
  );
}

export default PreSeasonView;
