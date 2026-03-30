import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import useCareerStore, {
  buildCalendarAdvanceTiming,
} from "./useCareerStore";

describe("buildCalendarAdvanceTiming", () => {
  it("uses a short 1.5s animation for very small day jumps", () => {
    expect(buildCalendarAdvanceTiming(1)).toEqual({
      totalDurationMs: 1500,
      stepMs: 1500,
    });

    expect(buildCalendarAdvanceTiming(3)).toEqual({
      totalDurationMs: 1500,
      stepMs: 500,
    });
  });

  it("caps large day jumps at 3s and speeds the steps up", () => {
    expect(buildCalendarAdvanceTiming(14)).toEqual({
      totalDurationMs: 3000,
      stepMs: 214,
    });

    expect(buildCalendarAdvanceTiming(30)).toEqual({
      totalDurationMs: 3000,
      stepMs: 100,
    });
  });
});

describe("useCareerStore startCalendarAdvance", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useCareerStore.setState({
      isLoaded: false,
      isLoading: false,
      isSimulating: false,
      isAdvancing: false,
      isCalendarAdvancing: false,
      isAdvancingWeek: false,
      isRespondingProposal: false,
      isDirty: false,
      lastSaved: null,
      error: null,
      careerId: "career-1",
      difficulty: null,
      player: null,
      playerTeam: {
        categoria: "mazda_amador",
      },
      season: {
        id: "season-1",
      },
      nextRace: {
        id: "race-1",
        display_date: "2026-03-04",
      },
      temporalSummary: {
        current_display_date: "2026-03-01",
        next_event_display_date: "2026-03-04",
        days_until_next_event: 3,
      },
      calendarDisplayDate: "2026-03-01",
      displayDaysUntilNextEvent: 3,
      totalDrivers: 0,
      totalTeams: 0,
      lastRaceResult: null,
      otherCategoriesResult: null,
      showResult: false,
      showRaceBriefing: false,
      preseasonState: null,
      preseasonWeeks: [],
      playerProposals: [],
      endOfSeasonResult: null,
      showEndOfSeason: false,
      showPreseason: false,
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("opens the briefing after 1.5s when there are only a few days left", async () => {
    const promise = useCareerStore.getState().startCalendarAdvance();

    await vi.advanceTimersByTimeAsync(1499);
    expect(useCareerStore.getState().showRaceBriefing).toBe(false);
    expect(useCareerStore.getState().isCalendarAdvancing).toBe(true);

    await vi.advanceTimersByTimeAsync(1);
    await promise;

    expect(useCareerStore.getState().showRaceBriefing).toBe(true);
    expect(useCareerStore.getState().isCalendarAdvancing).toBe(false);
    expect(useCareerStore.getState().calendarDisplayDate).toBe("2026-03-04");
  });
});

describe("useCareerStore loadCareer", () => {
  beforeEach(() => {
    invoke.mockReset();
    useCareerStore.setState({
      isLoaded: false,
      isLoading: false,
      isSimulating: false,
      isAdvancing: false,
      isCalendarAdvancing: false,
      isAdvancingWeek: false,
      isRespondingProposal: false,
      isDirty: false,
      lastSaved: null,
      error: null,
      careerId: null,
      difficulty: null,
      player: null,
      playerTeam: null,
      season: null,
      nextRace: null,
      nextRaceBriefing: null,
      temporalSummary: null,
      calendarDisplayDate: null,
      displayDaysUntilNextEvent: null,
      totalDrivers: 0,
      totalTeams: 0,
      lastRaceResult: null,
      otherCategoriesResult: null,
      showResult: false,
      showRaceBriefing: false,
      preseasonState: null,
      preseasonWeeks: [],
      playerProposals: [],
      endOfSeasonResult: null,
      showEndOfSeason: false,
      showPreseason: false,
    });
  });

  it("stores the enriched next-race briefing from load_career", async () => {
    invoke.mockImplementation((command) => {
      if (command === "load_career") {
        return Promise.resolve({
          career_id: "career-77",
          difficulty: "medio",
          player: { id: "drv-player", nome: "R. Silva" },
          player_team: { id: "team-1", categoria: "mazda_amador" },
          season: { id: "season-1", numero: 1 },
          next_race: { id: "race-1", display_date: "2026-03-04" },
          next_race_briefing: {
            track_history: {
              has_data: true,
              starts: 2,
              best_finish: 3,
            },
            primary_rival: {
              driver_id: "drv-rival",
              driver_name: "M. Costa",
              championship_position: 1,
              gap_points: 4,
              is_ahead: true,
            },
            weekend_stories: [
              {
                id: "story-1",
                title: "Duelo esquenta a abertura",
              },
            ],
          },
          total_drivers: 16,
          total_teams: 8,
        });
      }

      if (command === "get_temporal_summary") {
        return Promise.resolve({
          current_display_date: "2026-03-01",
          days_until_next_event: 3,
        });
      }

      return Promise.resolve(null);
    });

    await useCareerStore.getState().loadCareer("career-77");

    expect(useCareerStore.getState().nextRaceBriefing).toEqual({
      track_history: {
        has_data: true,
        starts: 2,
        best_finish: 3,
      },
      primary_rival: {
        driver_id: "drv-rival",
        driver_name: "M. Costa",
        championship_position: 1,
        gap_points: 4,
        is_ahead: true,
      },
      weekend_stories: [
        {
          id: "story-1",
          title: "Duelo esquenta a abertura",
        },
      ],
    });
  });
});
