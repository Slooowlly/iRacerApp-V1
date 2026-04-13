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
      lastMarketWeekResult: null,
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
    expect(categorySeparator).toHaveClass("text-center", "font-black", "text-[18px]");
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

    const title = await screen.findByText("GT4 Championship");
    const count = await screen.findByText("3 vagas");

    expect(title).toHaveClass("text-center", "font-black", "text-[18px]");
    expect(count).toHaveClass("text-center", "text-[10px]");
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

  it("shows a compact weekly closing panel grouped by destination category", async () => {
    mockState = {
      ...mockState,
      lastMarketWeekResult: {
        week_number: 2,
        events: [
          {
            event_type: "TransferCompleted",
            driver_name: "Marta Bianco",
            categoria: "gt3",
            from_categoria: "gt4",
            movement_kind: "promotion",
            championship_position: 1,
          },
          {
            event_type: "TransferCompleted",
            driver_name: "Colin Smith",
            categoria: "gt3",
            from_categoria: "gt3",
            movement_kind: "lateral",
            championship_position: 4,
          },
          {
            event_type: "RookieSigned",
            driver_name: "Giovanni Conti",
            categoria: "mazda_rookie",
            movement_kind: "rookie",
            championship_position: 3,
            team_name: "Vertex BMW",
          },
          {
            event_type: "RookieSigned",
            driver_name: "Victor Almeida",
            categoria: "gt3",
            championship_position: 12,
          },
          {
            event_type: "ContractExpired",
            driver_name: "Nicolas Meyer",
            categoria: "bmw_m2",
            movement_kind: "departure",
            championship_position: 11,
          },
          {
            event_type: "TransferCompleted",
            driver_name: "Lucas Prado",
            categoria: "bmw_m2",
            from_categoria: "gt4",
            movement_kind: "relegation",
            championship_position: 8,
          },
          {
            event_type: "PlayerProposalReceived",
            driver_name: "Rodrigo Vieira",
            categoria: "gt4",
            championship_position: 6,
            team_name: "Apex GT4",
          },
          {
            event_type: "ContractRenewed",
            driver_name: "Austin Williams",
            categoria: "gt4",
            movement_kind: "renewal",
            championship_position: 2,
          },
        ],
      },
    };

    render(<PreSeasonView />);

    const weeklyClosing = within(await screen.findByTestId("weekly-closing-market"));

    expect(weeklyClosing.getByText(/fechamento da semana/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/gt3 championship/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/mazda rookie/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/bmw m2 cup/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(new RegExp("^1\\u00ba$"))).toBeInTheDocument();
    expect(weeklyClosing.getByText(new RegExp("^4\\u00ba$"))).toBeInTheDocument();
    expect(weeklyClosing.getByText(new RegExp("^3\\u00ba$"))).toBeInTheDocument();
    expect(weeklyClosing.getByText(/marta bianco/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/colin smith/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/giovanni conti/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/victor almeida/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/nicolas meyer/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/lucas prado/i)).toBeInTheDocument();
    expect(weeklyClosing.getByText(/rodrigo vieira/i)).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Promoção")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Troca lateral")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Estreia")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Contratação")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Saiu da equipe")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Rebaixamento")).toBeInTheDocument();
    expect(weeklyClosing.getByTitle("Proposta recebida")).toBeInTheDocument();
    expect(weeklyClosing.getByText(new RegExp("^6\\u00ba$"))).toBeInTheDocument();
    expect(weeklyClosing.getByText(new RegExp("^11\\u00ba$"))).toBeInTheDocument();
    expect(screen.getByTestId("weekly-closing-market").textContent).not.toContain("Ã");
    expect(weeklyClosing.queryByText(/austin williams/i)).not.toBeInTheDocument();
    expect(weeklyClosing.queryByText(/vertex bmw/i)).not.toBeInTheDocument();
    expect(weeklyClosing.queryByText(/apex gt4/i)).not.toBeInTheDocument();
    expect(weeklyClosing.queryByText(/^SP$/i)).not.toBeInTheDocument();
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
          last_championship_position: 12,
          last_championship_total_drivers: 20,
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
          last_championship_position: 7,
          last_championship_total_drivers: 18,
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
          last_championship_position: 14,
          last_championship_total_drivers: 20,
          is_rookie: false,
        },
      ],
    };

    render(<PreSeasonView />);

    fireEvent.click(screen.getByRole("button", { name: /iniciar temporada/i }));

    const modalTitle = await screen.findByText("Pilotos sem vaga");
    const modal = modalTitle.closest("div");

    expect(modalTitle).toBeInTheDocument();
    expect(within(modal).getAllByText("GT3 Championship")).toHaveLength(1);
    expect(within(modal).getAllByText("GT4 Championship")).toHaveLength(1);
    expect(within(modal).getAllByText("Ex-equipe").length).toBeGreaterThan(0);
    const displacedDriver = within(modal).getByText("Luca Bianchi");
    expect(displacedDriver).toHaveClass("text-[17px]");
    const previousTeamLine = within(modal).getByText("Vortex Racing").closest("div");
    expect(within(previousTeamLine).getByText("Vortex Racing")).toHaveStyle({ color: "#ff8000" });
    expect(within(previousTeamLine).getByText("Vortex Racing")).toHaveClass("text-[14px]", "font-semibold");
    expect(within(previousTeamLine).getByText(/12º\/20/)).toBeInTheDocument();
    expect(within(modal).getByText("3 temporadas")).toBeInTheDocument();
    expect(within(modal).getByText(/7º\/18/)).toBeInTheDocument();
    expect(within(modal).getByText("Racing Spirit")).toHaveStyle({ color: "#58a6ff" });
    expect(within(modal).getByText("2 temporadas")).toBeInTheDocument();
    expect(within(modal).getByText(/14º\/20/)).toBeInTheDocument();
    expect(within(modal).getByText("Wolf Racing Team")).toHaveStyle({ color: "#3fb950" });
    expect(within(modal).getByText("1 temporada")).toBeInTheDocument();
    expect(within(modal).getByText("SP")).toHaveClass("min-w-[3.25rem]", "text-[11px]");
    expect(within(modal).queryByText(/Correu pela equipe/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText(/Categoria:/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText(/Carreira:/i)).not.toBeInTheDocument();
    expect(within(modal).queryByText("VRT")).not.toBeInTheDocument();
    expect(modal).toHaveClass("max-w-4xl");
  });
});
