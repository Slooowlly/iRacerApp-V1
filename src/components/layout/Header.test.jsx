import { fireEvent, render, screen, within } from "@testing-library/react";
import Header from "./Header";

const mockSimulateRace = vi.fn();
const mockStartCalendarAdvance = vi.fn();
const mockCloseRaceBriefing = vi.fn();

let mockState = {};

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

describe("Header", () => {
  beforeEach(() => {
    mockSimulateRace.mockReset();
    mockStartCalendarAdvance.mockReset();
    mockCloseRaceBriefing.mockReset();
    mockState = {
      playerTeam: {
        nome: "Equipe Teste",
        cor_primaria: "#58a6ff",
        categoria: "mazda_rookie",
      },
      season: {
        numero: 1,
        ano: 2026,
        total_rodadas: 12,
      },
      nextRace: {
        id: "race-1",
        track_name: "Interlagos",
        rodada: 3,
        display_date: "2026-03-25",
        horario: "14:00",
        clima: "Clear",
        temperatura: 27,
      },
      temporalSummary: {
        current_display_date: "2026-03-18",
        next_event_display_date: "2026-03-25",
        days_until_next_event: 7,
        weeks_until_next_event: 1,
      },
      showRaceBriefing: false,
      isCalendarAdvancing: false,
      isSimulating: false,
      simulateRace: mockSimulateRace,
      startCalendarAdvance: mockStartCalendarAdvance,
      closeRaceBriefing: mockCloseRaceBriefing,
    };
  });

  it("renders the temporal block and advances the calendar instead of simulating immediately", () => {
    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    expect(screen.getByText(/data 18\/03\/2026/i)).toBeInTheDocument();
    expect(screen.getByText(/proxima corrida em 7 dias/i)).toBeInTheDocument();

    const actionButton = screen.getByRole("button", { name: /avancar calendario/i });
    fireEvent.click(actionButton);

    expect(screen.getByText("Clima")).toBeInTheDocument();
    expect(mockStartCalendarAdvance).toHaveBeenCalledTimes(1);
    expect(mockSimulateRace).not.toHaveBeenCalled();
  });

  it("shows month-based countdowns before switching to weeks and days", () => {
    mockState.temporalSummary = {
      current_display_date: "2026-01-10",
      next_event_display_date: "2026-03-10",
      days_until_next_event: 59,
      weeks_until_next_event: 8,
    };

    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    expect(screen.getByText(/proxima corrida em 2 meses/i)).toBeInTheDocument();
  });

  it("hides the standings race banner while the pre-race briefing is open", () => {
    mockState.showRaceBriefing = true;

    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    expect(screen.queryByText("Clima")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /avancar calendario/i })).toBeInTheDocument();
  });

  it("shows a back button to the left of the temporal card only while the briefing is open", () => {
    mockState.showRaceBriefing = true;

    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    const temporalLabel = screen.getByText(/data 18\/03\/2026/i);
    const temporalCard = temporalLabel.closest(".rounded-2xl");
    expect(temporalCard).not.toBeNull();
    expect(within(temporalCard).queryByRole("button", { name: /voltar/i })).not.toBeInTheDocument();

    const backButton = screen.getByRole("button", { name: /voltar/i });
    expect(backButton.parentElement).toBe(temporalCard.parentElement);
    expect(backButton.parentElement?.firstElementChild).toBe(backButton);
    fireEvent.click(backButton);

    expect(mockCloseRaceBriefing).toHaveBeenCalledTimes(1);
  });
});
