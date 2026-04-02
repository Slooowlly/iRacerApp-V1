import { fireEvent, render, screen, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import Header from "./Header";

const mockSimulateRace = vi.fn();
const mockStartCalendarAdvance = vi.fn();
const mockCloseRaceBriefing = vi.fn();

let mockState = {};

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Header", () => {
  beforeEach(() => {
    mockSimulateRace.mockReset();
    mockStartCalendarAdvance.mockReset();
    mockCloseRaceBriefing.mockReset();
    invoke.mockReset();
    invoke.mockResolvedValue([]);
    mockState = {
      careerId: "career-1",
      playerTeam: {
        nome: "Equipe Teste",
        cor_primaria: "#58a6ff",
        categoria: "mazda_rookie",
      },
      season: {
        numero: 1,
        ano: 2026,
        total_rodadas: 12,
        rodada_atual: 3,
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
    expect(screen.getByRole("button", { name: /voltar/i })).toBeInTheDocument();
  });

  it("shows a back button inside the temporal card while the briefing is open", () => {
    mockState.showRaceBriefing = true;

    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    const temporalLabel = screen.getByText(/data 18\/03\/2026/i);
    const temporalCard = temporalLabel.closest(".rounded-2xl");
    expect(temporalCard).not.toBeNull();

    const backButton = within(temporalCard).getByRole("button", { name: /voltar/i });
    fireEvent.click(backButton);

    expect(mockCloseRaceBriefing).toHaveBeenCalledTimes(1);
  });

  it("shows a celebratory season-finished banner when the championship ends", async () => {
    mockState.nextRace = null;
    mockState.season = {
      numero: 1,
      ano: 2026,
      total_rodadas: 12,
      rodada_atual: 13,
    };
    invoke.mockResolvedValue([
      { id: "P001", nome: "Thomas Baker", posicao_campeonato: 1 },
      { id: "P002", nome: "R. Silva", posicao_campeonato: 2 },
    ]);

    render(<Header activeTab="standings" onTabChange={vi.fn()} />);

    expect(await screen.findByText("Temporada Encerrada")).toBeInTheDocument();
    expect(screen.getByText("Thomas Baker")).toBeInTheDocument();
    expect(screen.getByText(/ano 2026/i)).toBeInTheDocument();
    expect(screen.queryByText(/temporada 1/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/sem corrida pendente/i)).not.toBeInTheDocument();
  });
});
