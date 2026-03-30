import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassCard from "../../components/ui/GlassCard";
import NewsScopeDrawers from "./NewsScopeDrawers";
import useCareerStore from "../../stores/useCareerStore";
import {
  buildFallbackPrimaryFilters,
  contextChipStyle,
  contextChipToneClass,
  getReadableTeamColor,
  isUpcomingRaceFilter,
  leadBadgeLabel,
  resolveStoryBlocks,
  storyToneBadgeClass,
  toneDotClass,
} from "./newsHelpers";

function NewsTab() {
  const careerId = useCareerStore((state) => state.careerId);
  const season = useCareerStore((state) => state.season);
  const [bootstrap, setBootstrap] = useState(null);
  const [snapshot, setSnapshot] = useState(null);
  const [requestState, setRequestState] = useState({
    scopeType: "category",
    scopeId: "",
    scopeClass: null,
    primaryFilter: null,
    contextType: null,
    contextId: null,
  });
  const [selectedStoryId, setSelectedStoryId] = useState(null);
  const [loadingBootstrap, setLoadingBootstrap] = useState(true);
  const [loadingSnapshot, setLoadingSnapshot] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    let mounted = true;

    async function loadBootstrap() {
      if (!careerId) {
        if (mounted) {
          setBootstrap(null);
          setSnapshot(null);
          setLoadingBootstrap(false);
          setLoadingSnapshot(false);
          setSelectedStoryId(null);
        }
        return;
      }

      setLoadingBootstrap(true);
      setError("");

      try {
        const data = await invoke("get_news_tab_bootstrap", { careerId });
        if (!mounted) return;

        setBootstrap(data);
        setRequestState({
          scopeType: data.default_scope_type,
          scopeId: data.default_scope_id,
          scopeClass: null,
          primaryFilter: data.default_primary_filter ?? null,
          contextType: null,
          contextId: null,
        });
        setSelectedStoryId(null);
      } catch (invokeError) {
        if (!mounted) return;
        setError(
          typeof invokeError === "string"
            ? invokeError
            : invokeError?.toString?.() ?? "Nao foi possivel carregar a central de noticias.",
        );
      } finally {
        if (mounted) {
          setLoadingBootstrap(false);
        }
      }
    }

    void loadBootstrap();

    return () => {
      mounted = false;
    };
  }, [careerId]);

  useEffect(() => {
    let mounted = true;

    async function loadSnapshot() {
      if (!careerId || !requestState.scopeId) {
        if (mounted) {
          setSnapshot(null);
          setLoadingSnapshot(false);
          setSelectedStoryId(null);
        }
        return;
      }

      setLoadingSnapshot(true);
      setError("");

      try {
        const data = await invoke("get_news_tab_snapshot", {
          careerId,
          request: {
            scope_type: requestState.scopeType,
            scope_id: requestState.scopeId,
            scope_class: requestState.scopeClass,
            primary_filter: requestState.primaryFilter,
            context_type: requestState.contextType,
            context_id: requestState.contextId,
          },
        });

        if (!mounted) return;
        setSnapshot(data);
      } catch (invokeError) {
        if (!mounted) return;
        setError(
          typeof invokeError === "string"
            ? invokeError
            : invokeError?.toString?.() ?? "Nao foi possivel atualizar a leitura editorial.",
        );
      } finally {
        if (mounted) {
          setLoadingSnapshot(false);
        }
      }
    }

    void loadSnapshot();

    return () => {
      mounted = false;
    };
  }, [
    careerId,
    requestState.scopeType,
    requestState.scopeId,
    requestState.scopeClass,
    requestState.primaryFilter,
    requestState.contextType,
    requestState.contextId,
    season?.numero,
    season?.rodada_atual,
  ]);

  useEffect(() => {
    setSelectedStoryId(snapshot?.stories?.[0]?.id ?? null);
  }, [snapshot]);

  const scopeTabs = bootstrap?.scopes ?? [];
  const activeScope = scopeTabs.find(
    (scope) => scope.id === requestState.scopeId && scope.scope_type === requestState.scopeType,
  );
  const primaryFilters = snapshot?.primary_filters ?? buildFallbackPrimaryFilters(requestState.scopeType);
  const contextualFilters = snapshot?.contextual_filters ?? [];
  const stories = snapshot?.stories ?? [];
  const selectedStory = stories.find((story) => story.id === selectedStoryId) ?? stories[0] ?? null;
  const hasActivePrimaryFilter = Boolean(requestState.primaryFilter);
  const isLoading = loadingBootstrap || loadingSnapshot;

  function handleScopeChange(nextSelection) {
    const scope = nextSelection?.scope ?? nextSelection;
    setRequestState((current) => {
      const allowedPrimaryFilterIds = buildFallbackPrimaryFilters(scope.scope_type).map((filter) => filter.id);
      const preservedPrimaryFilter =
        current.primaryFilter && allowedPrimaryFilterIds.includes(current.primaryFilter)
          ? current.primaryFilter
          : null;

      return {
        scopeType: scope.scope_type,
        scopeId: scope.id,
        scopeClass: nextSelection?.scopeClass ?? null,
        primaryFilter: preservedPrimaryFilter,
        contextType: null,
        contextId: null,
      };
    });
    setSelectedStoryId(null);
  }

  function handlePrimaryFilterChange(filterId) {
    setRequestState((current) => ({
      ...current,
      primaryFilter: current.primaryFilter === filterId ? null : filterId,
      contextType: null,
      contextId: null,
    }));
    setSelectedStoryId(null);
  }

  function handleContextFilterClick(filter) {
    const nextContextType = filter.kind ?? null;
    const isSameSelection = requestState.contextType === nextContextType && requestState.contextId === filter.id;

    setRequestState((current) => ({
      ...current,
      contextType: isSameSelection ? null : nextContextType,
      contextId: isSameSelection ? null : filter.id,
    }));
    setSelectedStoryId(null);
  }

  return (
    <div className="space-y-4">
      <section data-news-section="hero" className="px-12">
        <GlassCard
          hover={false}
          className="relative overflow-hidden rounded-[26px] border-white/8 bg-[linear-gradient(135deg,rgba(8,17,31,0.98)_0%,rgba(9,22,39,0.93)_48%,rgba(16,25,39,0.91)_100%)] !p-0"
        >
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_80%_0%,rgba(88,166,255,0.12),transparent_38%),radial-gradient(circle_at_5%_100%,rgba(255,212,122,0.05),transparent_30%)]" />
          <div className="pointer-events-none absolute inset-x-0 top-0 h-0.5 bg-gradient-to-r from-accent-primary via-[rgba(142,208,255,0.4)] to-transparent" />

          {/* header: eyebrow | publicada em | etapa */}
          <div className="relative flex items-center justify-between px-5 pt-3">
            <p className="text-[10px] font-semibold uppercase tracking-[0.26em] text-accent-primary">
              {snapshot?.hero?.section_label ?? "Central de Notícias"}
            </p>
            <div className="absolute left-1/2 -translate-x-1/2 flex items-center gap-2">
              <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-muted">Publicada em</span>
              <span className="h-3 w-px bg-white/10" />
              <span className="text-[13px] font-semibold tracking-[0.04em] text-text-secondary">
                {bootstrap?.pub_date_label ?? String(bootstrap?.season_year ?? "")}
              </span>
            </div>
            <p className="text-[13px] font-semibold tracking-[0.06em] text-text-secondary whitespace-nowrap">
              {bootstrap
                ? `Etapa ${bootstrap.current_round} / ${bootstrap.total_rounds}${bootstrap.last_race_name ? ` · Última — ${bootstrap.last_race_name}` : ""}`
                : ""}
            </p>
          </div>

          {/* título */}
          <div
            data-news-hero-body
            className="relative space-y-0 px-5 pb-3 pt-1"
          >
            <div className="relative">
            <div data-news-hero-summary className="min-w-0 flex flex-col gap-2 pr-[220px]">
              <h2 className="text-[2rem] font-bold leading-[1.0] tracking-[-0.06em] text-text-primary">
                {snapshot?.hero?.title ?? "Panorama do Campeonato"}
              </h2>
              <div className="flex flex-wrap items-center gap-2">
                <span className="inline-flex items-center rounded-lg border border-white/8 bg-white/[0.03] px-3 py-1.5">
                  <span className="text-[0.82rem] font-semibold text-accent-primary">
                    {stories.length} {stories.length === 1 ? "matéria nesta edição" : "matérias nesta edição"}
                  </span>
                </span>
              </div>
            </div>
            <div className="absolute right-0 top-0 w-fit rounded-xl border border-white/8 bg-white/[0.035] flex flex-col justify-center gap-0.5 whitespace-nowrap px-3.5 py-2">
              <p className="text-[9px] font-semibold uppercase tracking-[0.22em] text-text-muted">Próxima etapa</p>
              <p className="text-[1.4rem] font-bold leading-[1.1] tracking-[-0.04em] text-text-primary mt-0.5">
                {bootstrap?.next_race_date_label ?? "—"}
              </p>
              <p className="text-sm font-medium text-text-secondary">
                {bootstrap?.next_race_name ?? ""}
              </p>
            </div>
            </div>
            <NewsScopeDrawers
              scopeTabs={scopeTabs}
              requestState={requestState}
              showDrawer={Boolean(snapshot)}
              onScopeChange={handleScopeChange}
            />
          </div>

        </GlassCard>
      </section>

      {snapshot ? (
        <>
          <section data-news-section="context-panel">
            <GlassCard
              hover={false}
              className="rounded-[24px] border-white/8 bg-[linear-gradient(180deg,rgba(255,255,255,0.05)_0%,rgba(255,255,255,0.03)_100%)] px-4 py-4"
            >
              <div className="flex justify-center">
                <div
                  data-news-primary-pill
                  className="mx-auto inline-flex flex-wrap items-center justify-center gap-1.5 rounded-full border border-white/8 bg-white/[0.03] p-1.5"
                >
                  {primaryFilters.map((filter) => {
                    const isActive = requestState.primaryFilter === filter.id;
                    return (
                      <button
                        key={filter.id}
                        type="button"
                        onClick={() => handlePrimaryFilterChange(filter.id)}
                        className={[
                          "rounded-full px-4 py-2 text-sm font-semibold tracking-[0.01em] transition-glass",
                          isActive
                            ? filter.id === "Mercado"
                              ? "bg-status-yellow/12 text-status-yellow shadow-[inset_0_0_0_1px_rgba(240,190,84,0.16)]"
                              : "bg-accent-primary/12 text-text-primary shadow-[inset_0_0_0_1px_rgba(88,166,255,0.16)]"
                            : "text-text-secondary hover:bg-white/[0.04] hover:text-text-primary",
                        ].join(" ")}
                      >
                        {filter.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div data-news-context-results className="mt-4 flex flex-wrap justify-center gap-2">
                {hasActivePrimaryFilter && contextualFilters.length > 0 ? (
                  contextualFilters.map((filter, index) => {
                    const isActive =
                      requestState.contextType === (filter.kind ?? null) && requestState.contextId === filter.id;
                    const isUpcomingRace =
                      requestState.primaryFilter === "Corridas"
                      && isUpcomingRaceFilter(filter, bootstrap?.current_round)
                      && !isActive;
                    const chipToneClass = contextChipToneClass(filter, requestState.primaryFilter, index, isActive);
                    return (
                      <button
                        key={`${filter.kind ?? "context"}-${filter.id}`}
                        type="button"
                        disabled={isUpcomingRace}
                        onClick={() => handleContextFilterClick(filter)}
                        className={[
                          "rounded-[18px] border px-3.5 py-2 text-center transition-glass",
                          chipToneClass
                            ? chipToneClass
                            : isActive
                            ? "border-accent-primary/32 bg-accent-primary/14"
                            : "border-white/10 bg-white/[0.04] hover:border-white/18 hover:bg-white/[0.06]",
                          isUpcomingRace ? "cursor-not-allowed opacity-35" : "",
                        ].join(" ")}
                        style={contextChipStyle(filter, isActive)}
                      >
                        <div className="flex items-center justify-center gap-2.5">
                          {requestState.primaryFilter === "Corridas" ? null : (
                            <span className={["h-2 w-2 rounded-full", toneDotClass(filter.tone)].join(" ")} />
                          )}
                          <span
                            className={isActive ? "text-sm font-semibold text-text-primary" : "text-sm text-text-secondary"}
                            style={filter.kind === "team" && filter.color_primary ? { color: getReadableTeamColor(filter.color_primary) } : undefined}
                          >
                            {filter.label}
                          </span>
                        </div>
                      </button>
                    );
                  })
                ) : (
                  <p className="w-full text-center text-sm text-text-muted">
                    {hasActivePrimaryFilter
                      ? "Nenhum recorte adicional neste filtro."
                      : "Escolha um filtro."}
                  </p>
                )}
              </div>
            </GlassCard>
          </section>

          <section data-news-section="main-reader">
            <div className="grid gap-4 xl:grid-cols-[1.16fr_0.84fr]">
              <GlassCard
                hover={false}
                className="relative overflow-hidden rounded-[28px] border-white/8 bg-[linear-gradient(180deg,rgba(8,16,26,0.96)_0%,rgba(6,12,20,0.96)_100%)] p-0"
              >
                <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-accent-primary via-[#8ed0ff] to-transparent" />
                <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_center,rgba(255,255,255,0.04),transparent_26%),linear-gradient(180deg,transparent,rgba(4,10,18,0.28))]" />
                <div className="relative p-5 sm:p-6">
                  {error ? (
                    <p className="text-sm font-semibold text-status-red">{error}</p>
                  ) : selectedStory ? (
                    <OpenStory story={selectedStory} />
                  ) : (
                    <ReaderEmptyState loading={isLoading} />
                  )}
                </div>
              </GlassCard>

              <GlassCard
                hover={false}
                className="overflow-hidden rounded-[28px] border-white/8 bg-[linear-gradient(180deg,rgba(255,255,255,0.035)_0%,rgba(255,255,255,0.015)_100%)] p-0"
              >
                <div className="px-5 pb-2 pt-5 sm:px-6">
                  <p className="text-[10px] uppercase tracking-[0.22em] text-text-muted">Leituras do recorte</p>
                  <h3 className="mt-2 text-[1.2rem] font-semibold tracking-[-0.04em] text-text-primary">
                    Capítulos paralelos
                  </h3>
                </div>
                <div className="space-y-0 px-4 pb-4 sm:px-5 sm:pb-5">
                  {stories.length > 0 ? (
                    stories.map((story) => (
                      <StoryListItem
                        key={story.id}
                        story={story}
                        active={selectedStory?.id === story.id}
                        onClick={() => setSelectedStoryId(story.id)}
                      />
                    ))
                  ) : (
                    <p className="text-sm text-text-muted">
                      {isLoading ? "Montando o briefing atual..." : "Não há histórias disponíveis neste recorte."}
                    </p>
                  )}
                </div>
              </GlassCard>
            </div>
          </section>
        </>
      ) : null}
    </div>
  );
}

function OpenStory({ story }) {
  const storyContext = story.entity_label ?? story.category_label ?? "Campeonato";
  const storyTiming = story.race_label ?? story.meta_label ?? "Edicao atual";
  const storyHeadline = story.headline ?? story.title ?? "Leitura atual";
  const storyDeck = story.deck ?? story.summary ?? "";
  const storyBlocks = resolveStoryBlocks(story);

  return (
    <div data-news-open-story className="flex h-full flex-col gap-6">
      <div className="flex flex-wrap items-center gap-2">
        <span className={["inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.22em] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]", storyToneBadgeClass(story.accent_tone)].join(" ")}>
          <span className="h-px w-4 rounded-full bg-current opacity-70" />
          {leadBadgeLabel(story.importance)}
        </span>
        <span className="rounded-full border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[10px] uppercase tracking-[0.18em] text-text-muted">
          {story.news_type}
        </span>
      </div>

      <div>
        <h3 className="max-w-4xl text-[2.05rem] font-semibold leading-[0.98] tracking-[-0.06em] text-text-primary sm:text-[2.45rem]">
          {storyHeadline}
        </h3>
        {storyDeck ? (
          <p className="mt-3 max-w-3xl text-[0.97rem] leading-7 text-text-secondary">
            {storyDeck}
          </p>
        ) : null}
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_260px]">
        <div className="space-y-0">
          {storyBlocks.map((block) => (
            <StoryChapter key={`${block.label}-${block.text}`} label={block.label} text={block.text} />
          ))}
        </div>

        <div className="flex flex-col gap-3">
          <StoryInfoCard label="Publicada em" value={story.time_label} />
          <StoryInfoCard label="Leitura" value={storyTiming} />
          <StoryInfoCard label="Contexto" value={storyContext} />
        </div>
      </div>
    </div>
  );
}

function StoryListItem({ story, active, onClick }) {
  const storyHeadline = story.headline ?? story.title ?? "Leitura atual";
  const storyDeck = story.deck ?? story.summary ?? "";

  return (
    <button
      type="button"
      onClick={onClick}
      data-news-story-list-item={story.id}
      className={[
        "w-full border-t border-white/8 px-0 py-4 text-left transition-glass first:border-t-0 first:pt-0 last:pb-0",
        active
          ? "text-text-primary"
          : "text-text-secondary hover:text-text-primary",
      ].join(" ")}
    >
      <div
        className={[
          "rounded-[20px] border px-4 py-4 transition-glass",
          active
            ? "border-accent-primary/30 bg-accent-primary/[0.1]"
            : "border-white/8 bg-white/[0.02] hover:border-white/12 hover:bg-white/[0.04]",
        ].join(" ")}
      >
        <div className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
          {[story.news_type, story.importance_label].filter(Boolean).join(" / ")}
        </div>
        <h4 className="mt-2 text-[1rem] font-semibold leading-[1.28] tracking-[-0.03em] text-inherit">
          {storyHeadline}
        </h4>
        {storyDeck ? <p className="mt-2 text-sm leading-6 text-text-secondary">{storyDeck}</p> : null}
        <p className="mt-3 text-[11px] uppercase tracking-[0.14em] text-text-muted">
          {[story.entity_label ?? story.category_label, story.time_label].filter(Boolean).join(" / ")}
        </p>
      </div>
    </button>
  );
}

function ReaderEmptyState({ loading }) {
  return (
    <div className="flex min-h-[320px] flex-col justify-end">
      <p className="text-[11px] uppercase tracking-[0.24em] text-accent-primary">Leitura principal</p>
      <h3 className="mt-3 text-3xl font-semibold tracking-[-0.05em] text-text-primary">
        {loading ? "Preparando a leitura do recorte atual" : "Sem leitura disponivel"}
      </h3>
      <p className="mt-3 max-w-2xl text-sm leading-7 text-text-secondary">
        {loading
          ? "A materia principal e a lista navegavel vao aparecer aqui assim que o briefing for montado."
          : "Troque o escopo ou ative outro filtro para abrir uma nova edição."}
      </p>
    </div>
  );
}


function StoryChapter({ label, text }) {
  return (
    <div className="border-t border-white/8 py-4 first:border-t-0 first:pt-0 last:pb-0">
      <p className="text-[10px] uppercase tracking-[0.22em] text-text-muted">{label}</p>
      <p className="mt-2 text-[0.95rem] leading-8 text-text-secondary">{text}</p>
    </div>
  );
}

function StoryInfoCard({ label, value }) {
  return (
    <div className="rounded-[22px] border border-white/8 bg-white/[0.025] px-4 py-4">
      <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">{label}</p>
      <p className="mt-2 text-sm font-semibold leading-6 text-text-primary">{value}</p>
    </div>
  );
}

export default NewsTab;
