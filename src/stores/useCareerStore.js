import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

const initialState = {
  isLoaded: false,
  isLoading: false,
  isSimulating: false,
  isAdvancing: false,
  isAdvancingWeek: false,
  isRespondingProposal: false,
  // true quando houve ação relevante desde o último flush_save ou load inicial
  isDirty: false,
  lastSaved: null,
  error: null,
  careerId: null,
  difficulty: null,
  player: null,
  playerTeam: null,
  season: null,
  nextRace: null,
  totalDrivers: 0,
  totalTeams: 0,
  lastRaceResult: null,
  otherCategoriesResult: null,
  showResult: false,
  preseasonState: null,
  preseasonWeeks: [],
  playerProposals: [],
  endOfSeasonResult: null,
  showEndOfSeason: false,
  showPreseason: false,
};

function getErrorMessage(error, fallback) {
  return typeof error === "string" ? error : error?.toString?.() ?? fallback;
}

function applyCareerData(data) {
  return {
    isLoaded: true,
    isLoading: false,
    error: null,
    careerId: data.career_id,
    difficulty: data.difficulty,
    player: data.player,
    playerTeam: data.player_team,
    season: data.season,
    nextRace: data.next_race,
    totalDrivers: data.total_drivers,
    totalTeams: data.total_teams,
    isSimulating: false,
    showResult: false,
    lastRaceResult: null,
    otherCategoriesResult: null,
  };
}

function buildWeeksFromNews(newsItems = []) {
  const grouped = new Map();

  for (const item of newsItems) {
    const weekNumber = item.semana_pretemporada;
    if (!weekNumber) continue;

    if (!grouped.has(weekNumber)) {
      grouped.set(weekNumber, {
        week_number: weekNumber,
        events: [],
        remaining_vacancies: 0,
      });
    }

    grouped.get(weekNumber).events.push({
      event_type: item.tipo,
      headline: item.titulo,
      description: item.texto,
    });
  }

  return [...grouped.values()].sort((a, b) => a.week_number - b.week_number);
}

const useCareerStore = create((set, get) => ({
  ...initialState,

  loadCareer: async (careerId) => {
    set({
      isLoading: true,
      isSimulating: false,
      error: null,
      showResult: false,
      lastRaceResult: null,
      otherCategoriesResult: null,
      showEndOfSeason: false,
      showPreseason: false,
      endOfSeasonResult: null,
      preseasonState: null,
      preseasonWeeks: [],
      playerProposals: [],
    });

    try {
      const data = await invoke("load_career", { careerId });
      set({
        ...applyCareerData(data),
        showEndOfSeason: false,
        showPreseason: false,
        endOfSeasonResult: null,
        preseasonState: null,
        preseasonWeeks: [],
        playerProposals: [],
        isDirty: false,
      });
      return data;
    } catch (error) {
      const message = getErrorMessage(error, "Erro ao carregar carreira.");
      set({ isLoading: false, error: message });
      throw error;
    }
  },

  setCareerFromCreation: (createResult) => {
    set({
      isLoaded: true,
      careerId: createResult?.career_id ?? null,
    });
  },

  simulateRace: async () => {
    const { careerId, nextRace } = get();
    if (!careerId || !nextRace?.id) {
      throw new Error("Nao existe corrida pendente para simular.");
    }

    set({ isSimulating: true, error: null });

    try {
      const result = await invoke("simulate_race_weekend", {
        careerId,
        raceId: nextRace.id,
      });

      set({
        lastRaceResult: result.player_race,
        otherCategoriesResult: result.other_categories,
        isSimulating: false,
        showResult: true,
        isDirty: true,
      });

      return result;
    } catch (error) {
      const message = getErrorMessage(error, "Erro ao simular corrida.");
      set({ isSimulating: false, error: message });
      throw error;
    }
  },

  dismissResult: async () => {
    const { careerId, loadCareer } = get();
    set({ showResult: false });

    if (!careerId) return;

    try {
      await loadCareer(careerId);
    } catch (error) {
      console.error("Erro ao recarregar carreira:", error);
    }
  },

  clearLastResult: () => {
    set({ lastRaceResult: null, otherCategoriesResult: null, showResult: false });
  },

  clearCareer: () => {
    set({ ...initialState });
  },

  advanceSeason: async () => {
    const { careerId } = get();
    if (!careerId) {
      throw new Error("Carreira nao carregada.");
    }

    set({ isAdvancing: true, error: null });

    try {
      const result = await invoke("advance_season", { careerId });
      set({
        isAdvancing: false,
        endOfSeasonResult: result,
        showEndOfSeason: true,
        showPreseason: false,
        preseasonState: null,
        preseasonWeeks: [],
        playerProposals: [],
        nextRace: null,
        lastRaceResult: null,
        otherCategoriesResult: null,
        isDirty: true,
      });
      return result;
    } catch (error) {
      set({
        isAdvancing: false,
        error: getErrorMessage(error, "Erro ao avancar temporada."),
      });
      throw error;
    }
  },

  enterPreseason: async () => {
    const { careerId } = get();
    if (!careerId) {
      throw new Error("Carreira nao carregada.");
    }

    try {
      const [state, proposals] = await Promise.all([
        invoke("get_preseason_state", { careerId }),
        invoke("get_player_proposals", { careerId }).catch(() => []),
      ]);
      const news = await invoke("get_news", {
        careerId,
        season: state.season_number,
        tipo: null,
        limit: 400,
      });

      set({
        showEndOfSeason: false,
        showPreseason: true,
        preseasonState: state,
        preseasonWeeks: buildWeeksFromNews(news),
        playerProposals: proposals,
        error: null,
      });

      return state;
    } catch (error) {
      const message = getErrorMessage(error, "Erro ao entrar na pre-temporada.");
      set({ error: message });
      throw error;
    }
  },

  advanceMarketWeek: async () => {
    const { careerId } = get();
    if (!careerId) {
      throw new Error("Carreira nao carregada.");
    }

    set({ isAdvancingWeek: true, error: null });

    try {
      const weekResult = await invoke("advance_market_week", { careerId });

      let proposals = get().playerProposals;
      if ((weekResult.player_proposals?.length ?? 0) > 0) {
        proposals = await invoke("get_player_proposals", { careerId });
      }

      const state = await invoke("get_preseason_state", { careerId });
      const news = await invoke("get_news", {
        careerId,
        season: state.season_number,
        tipo: null,
        limit: 400,
      });

      set({
        preseasonWeeks: buildWeeksFromNews(news),
        preseasonState: state,
        playerProposals: proposals,
        isAdvancingWeek: false,
        isDirty: true,
      });

      return weekResult;
    } catch (error) {
      set({
        isAdvancingWeek: false,
        error: getErrorMessage(error, "Erro ao avancar semana da pre-temporada."),
      });
      throw error;
    }
  },

  respondToProposal: async (proposalId, accept) => {
    const { careerId } = get();
    if (!careerId) {
      throw new Error("Carreira nao carregada.");
    }

    set({ isRespondingProposal: true, error: null });

    try {
      const response = await invoke("respond_to_proposal", {
        careerId,
        proposalId,
        accept,
      });

      const [state, proposals] = await Promise.all([
        invoke("get_preseason_state", { careerId }).catch(() => get().preseasonState),
        response.remaining_proposals === 0
          ? Promise.resolve([])
          : invoke("get_player_proposals", { careerId }),
      ]);

      set({
        preseasonState: state,
        playerProposals: proposals,
        isRespondingProposal: false,
        isDirty: true,
      });

      return response;
    } catch (error) {
      set({
        isRespondingProposal: false,
        error: getErrorMessage(error, "Erro ao responder proposta."),
      });
      throw error;
    }
  },

  finalizePreseason: async () => {
    const { careerId } = get();
    if (!careerId) {
      throw new Error("Carreira nao carregada.");
    }

    try {
      await invoke("finalize_preseason", { careerId });
      const data = await invoke("load_career", { careerId });

      set({
        ...applyCareerData(data),
        showPreseason: false,
        showEndOfSeason: false,
        preseasonState: null,
        preseasonWeeks: [],
        playerProposals: [],
        endOfSeasonResult: null,
        lastRaceResult: null,
        otherCategoriesResult: null,
        isAdvancing: false,
        isAdvancingWeek: false,
        isRespondingProposal: false,
        isDirty: false,
      });

      return data;
    } catch (error) {
      const message = getErrorMessage(error, "Erro ao iniciar a nova temporada.");
      set({ error: message });
      throw error;
    }
  },

  updateSeason: (seasonData) => {
    set({ season: seasonData });
  },

  updateNextRace: (raceData) => {
    set({ nextRace: raceData });
  },

  // Consolida o save: WAL checkpoint + atualiza last_saved no meta.json
  flushSave: async () => {
    const { careerId } = get();
    if (!careerId) return;

    try {
      const result = await invoke("flush_save", { careerId });
      set({ isDirty: false, lastSaved: result.last_saved });
    } catch (error) {
      console.error("Falha ao consolidar save:", error);
      throw error;
    }
  },
}));

export default useCareerStore;
