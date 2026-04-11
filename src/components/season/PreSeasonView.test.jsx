import { fireEvent, render, screen, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import PreSeasonView from "./PreSeasonView";

let mockState = {};

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("PreSeasonView", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue([]);
    mockState = {
      careerId: "career-1",
      preseasonState: {
        current_week: 2,
        total_weeks: 4,
        is_complete: false,
        current_display_date: "2026-03-07",
      },
      preseasonWeeks: [],
      playerProposals: [],
      preseasonFreeAgents: [],
      isAdvancingWeek: false,
      isRespondingProposal: false,
      advanceMarketWeek: vi.fn(),
      respondToProposal: vi.fn(),
      finalizePreseason: vi.fn(),
      playerTeam: {
        categoria: "gt4",
      },
    };
  });

  it("renders the simulated preseason date from state instead of the PC clock", async () => {
    render(<PreSeasonView />);

    expect(await screen.findByText(/7 de março/i)).toBeInTheDocument();
  });

  it("shows compact tenure counters in the team mapping", async () => {
    invoke.mockImplementation(async (command, { category }) => {
      if (command !== "get_teams_standings" || category !== "gt4") {
        return [];
      }

      return [
        {
          id: "team-1",
          nome: "Vortex Racing",
          nome_curto: "VRT",
          cor_primaria: "#FF8000",
          pontos: 0,
          vitorias: 0,
          piloto_1_nome: "Luca Bianchi",
          piloto_1_tenure_seasons: 3,
          piloto_2_nome: "Mateo Silva",
          piloto_2_tenure_seasons: 1,
          trofeus: [],
          classe: "gt4",
          temp_posicao: 1,
          categoria_anterior: "production_challenger",
        },
        {
          id: "team-2",
          nome: "Nova Speed",
          nome_curto: "NSP",
          cor_primaria: "#3671C6",
          pontos: 0,
          vitorias: 0,
          piloto_1_nome: "Rafael Costa",
          piloto_1_tenure_seasons: 2,
          piloto_2_nome: "Bruno Alves",
          piloto_2_tenure_seasons: 4,
          trofeus: [],
          classe: "gt4",
          temp_posicao: 2,
          categoria_anterior: null,
        },
        {
          id: "team-3",
          nome: "Legacy Motorsport",
          nome_curto: "LGM",
          cor_primaria: "#f85149",
          pontos: 0,
          vitorias: 0,
          piloto_1_nome: "Thiago Lima",
          piloto_1_tenure_seasons: 5,
          piloto_2_nome: "Caio Mendes",
          piloto_2_tenure_seasons: 2,
          trofeus: [],
          classe: "gt4",
          temp_posicao: 3,
          categoria_anterior: "gt3",
        },
      ];
    });

    render(<PreSeasonView />);

    const teamName = await screen.findByText("Vortex Racing");
    const primaryDriver = await screen.findByText("Luca Bianchi");
    const secondaryDriver = screen.getByText("Mateo Silva");
    const categorySeparator = screen.getByText("GT4 Championship");
    const newcomerTag = screen.getByText("New");
    const promotedAbbr = screen.getByText("VR");
    const neutralAbbr = screen.getByText("NS");
    const relegatedAbbr = screen.getByText("LG");
    const orderedTeamNames = screen
      .getAllByText(/^(Vortex Racing|Nova Speed|Legacy Motorsport)$/)
      .map((node) => node.textContent);

    expect(teamName).toHaveClass("text-[19px]", "font-bold");
    expect(categorySeparator).toHaveClass("text-[17px]", "font-bold");
    expect(primaryDriver).toHaveClass("text-[15px]", "font-bold");
    expect(primaryDriver).toHaveClass("text-[color:var(--text-primary)]");
    expect(screen.getByText("3 anos")).toBeInTheDocument();
    expect(secondaryDriver).toHaveClass("text-[14px]", "font-semibold");
    expect(secondaryDriver).toHaveClass("text-[color:var(--text-primary)]");
    expect(newcomerTag).toHaveClass("rounded-md");
    expect(promotedAbbr).toHaveStyle({ color: "#3fb950" });
    expect(neutralAbbr).toHaveStyle({ color: "rgb(255, 255, 255)" });
    expect(relegatedAbbr).toHaveStyle({ color: "#f85149" });
    expect(screen.getByText("Promovido")).toBeInTheDocument();
    expect(screen.getByText("Relegado")).toBeInTheDocument();
    expect(orderedTeamNames).toEqual(["Nova Speed", "Vortex Racing", "Legacy Motorsport"]);
    expect(screen.queryByText("Confirmado")).not.toBeInTheDocument();
    expect(screen.queryByText("Novo")).not.toBeInTheDocument();
    expect(screen.queryByText("1T")).not.toBeInTheDocument();
    expect(screen.queryByText("3T")).not.toBeInTheDocument();
  });

  it("shows the total open vacancies on the category header", async () => {
    invoke.mockImplementation(async (command, { category }) => {
      if (command !== "get_teams_standings" || category !== "gt4") {
        return [];
      }

      return [
        {
          id: "team-1",
          nome: "Vortex Racing",
          nome_curto: "VRT",
          cor_primaria: "#FF8000",
          pontos: 0,
          vitorias: 0,
          piloto_1_nome: "Luca Bianchi",
          piloto_1_tenure_seasons: 2,
          piloto_2_nome: null,
          piloto_2_tenure_seasons: 0,
          trofeus: [],
          classe: "gt4",
          temp_posicao: 1,
          categoria_anterior: null,
        },
        {
          id: "team-2",
          nome: "Nova Speed",
          nome_curto: "NSP",
          cor_primaria: "#3671C6",
          pontos: 0,
          vitorias: 0,
          piloto_1_nome: null,
          piloto_1_tenure_seasons: 0,
          piloto_2_nome: null,
          piloto_2_tenure_seasons: 0,
          trofeus: [],
          classe: "gt4",
          temp_posicao: 2,
          categoria_anterior: null,
        },
      ];
    });

    render(<PreSeasonView />);

    expect(await screen.findByText("GT4 Championship")).toBeInTheDocument();
    expect(await screen.findByText("3 vagas")).toBeInTheDocument();
  });

  it("keeps special categories out of the normal preseason market", async () => {
    render(<PreSeasonView />);

    await screen.findByText(/Mercado de Transferências/i);

    expect(screen.queryByRole("button", { name: /Production/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Endurance/i })).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith(
      "get_teams_standings",
      expect.objectContaining({ category: "production_challenger" }),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "get_teams_standings",
      expect.objectContaining({ category: "endurance" }),
    );
  });

  it("groups displaced drivers by category in a larger end-of-preseason modal", async () => {
    mockState = {
      ...mockState,
      preseasonState: {
        current_week: 4,
        total_weeks: 4,
        is_complete: true,
        current_display_date: "2026-03-28",
      },
      preseasonFreeAgents: [
        {
          driver_id: "driver-1",
          driver_name: "Luca Bianchi",
          categoria: "gt3",
          previous_team_name: "Vortex Racing",
          previous_team_color: "#ff8000",
          previous_team_abbr: "VRT",
          seasons_at_last_team: 3,
          total_career_seasons: 8,
          license_sigla: "SP",
          is_rookie: false,
        },
        {
          driver_id: "driver-2",
          driver_name: "Mateo Silva",
          categoria: "gt4",
          previous_team_name: "Racing Spirit",
          previous_team_color: "#58a6ff",
          previous_team_abbr: "RSR",
          seasons_at_last_team: 2,
          total_career_seasons: 5,
          license_sigla: "P",
          is_rookie: false,
        },
        {
          driver_id: "driver-3",
          driver_name: "Rafael Costa",
          categoria: "gt3",
          previous_team_name: "Wolf Racing Team",
          previous_team_color: "#3fb950",
          previous_team_abbr: "WRT",
          seasons_at_last_team: 1,
          total_career_seasons: 4,
          license_sigla: "A",
          is_rookie: false,
        },
      ],
    };

    render(<PreSeasonView />);

    fireEvent.click(screen.getByRole("button", { name: /iniciar temporada/i }));

    const modalTitle = await screen.findByText("Pilotos sem vaga");
    const modal = modalTitle.closest("div");

    expect(modalTitle).toBeInTheDocument();
    expect(within(modal).getAllByText("GT3 Championship").length).toBeGreaterThan(0);
    expect(within(modal).getAllByText("GT4 Championship").length).toBeGreaterThan(0);
    const displacedDriver = within(modal).getByText("Luca Bianchi");
    expect(displacedDriver).toHaveClass("text-[15px]");
    expect(within(modal).getByText("Vortex Racing")).toHaveStyle({ color: "#ff8000" });
    expect(within(modal).getByText("3 temporadas")).toBeInTheDocument();
    expect(within(modal).getByText("Racing Spirit")).toHaveStyle({ color: "#58a6ff" });
    expect(within(modal).getByText("2 temporadas")).toBeInTheDocument();
    expect(within(modal).getByText("Wolf Racing Team")).toHaveStyle({ color: "#3fb950" });
    expect(within(modal).getByText("1 temporada")).toBeInTheDocument();
    expect(within(modal).queryByText(/Correu pela equipe/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText(/Categoria:/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText(/Carreira:/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText("VRT")).not.toBeInTheDocument();
    expect(modal).toHaveClass("max-w-4xl");
  });
});
