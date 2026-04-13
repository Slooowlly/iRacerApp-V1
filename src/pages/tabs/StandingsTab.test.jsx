import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import StandingsTab from "./StandingsTab";

let mockState = {};

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function specialTeam({
  id,
  nome,
  classe,
  pontos,
  vitorias = 0,
  piloto1,
  piloto2,
  cor = "#bc8cff",
}) {
  return {
    id,
    nome,
    nome_curto: id,
    cor_primaria: cor,
    classe,
    pontos,
    vitorias,
    posicao: 1,
    piloto_1_nome: piloto1,
    piloto_2_nome: piloto2,
  };
}

describe("StandingsTab", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(async (command) => {
      if (command === "get_previous_champions") {
        return { driver_champion_id: null, constructor_champions: [] };
      }
      return [];
    });

    mockState = {
      careerId: "career-1",
      playerTeam: {
        categoria: "production_challenger",
      },
      season: {
        ano: 2025,
        rodada_atual: 8,
        total_rodadas: 8,
        fase: "BlocoEspecial",
      },
    };
  });

  it("reloads standings when the season phase changes after skipping the special block", async () => {
    const { rerender } = render(<StandingsTab />);

    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(3));

    mockState = {
      ...mockState,
      season: {
        ...mockState.season,
        fase: "PosEspecial",
      },
    };
    rerender(<StandingsTab />);

    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(6));
    expect(invoke).toHaveBeenLastCalledWith("get_previous_champions", {
      careerId: "career-1",
      category: "production_challenger",
    });
  });

  it("forces production standings during the special block for production-ladder drivers", async () => {
    mockState = {
      ...mockState,
      playerTeam: {
        categoria: "bmw_m2",
      },
    };

    render(<StandingsTab />);

    await waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith("get_previous_champions", {
        careerId: "career-1",
        category: "production_challenger",
      }),
    );
  });

  it("returns to endurance automatically when the user tries to change categories during the special block", async () => {
    mockState = {
      ...mockState,
      playerTeam: {
        categoria: "gt4",
      },
    };

    render(<StandingsTab />);

    await waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith("get_previous_champions", {
        careerId: "career-1",
        category: "endurance",
      }),
    );

    fireEvent.click(screen.getByTitle("Categoria inferior"));

    await waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith("get_previous_champions", {
        careerId: "career-1",
        category: "endurance",
      }),
    );
  });

  it("groups special driver and team standings by car class", async () => {
    invoke.mockImplementation(async (command) => {
      if (command === "get_drivers_by_category") {
        return [
          {
            id: "D1",
            nome: "Bianca Rossi",
            nacionalidade: "it",
            idade: 24,
            equipe_id: "TBMW",
            equipe_nome: "BMW Works",
            equipe_nome_curto: "BMW",
            equipe_cor: "#bc8cff",
            classe: "bmw",
            pontos: 88,
            vitorias: 3,
            podios: 4,
            posicao_campeonato: 1,
            results: [{ position: 1, is_dnf: false }],
          },
          {
            id: "D2",
            nome: "Taro Sato",
            nacionalidade: "jp",
            idade: 22,
            equipe_id: "TTOY",
            equipe_nome: "Toyota Spirit",
            equipe_nome_curto: "TOY",
            equipe_cor: "#f2cc60",
            classe: "toyota",
            pontos: 74,
            vitorias: 2,
            podios: 4,
            posicao_campeonato: 2,
            results: [{ position: 1, is_dnf: false }],
          },
          {
            id: "D3",
            nome: "Marta Vega",
            nacionalidade: "es",
            idade: 21,
            equipe_id: "TMAZ",
            equipe_nome: "Mazda Club",
            equipe_nome_curto: "MAZ",
            equipe_cor: "#c8102e",
            classe: "mazda",
            pontos: 66,
            vitorias: 1,
            podios: 3,
            posicao_campeonato: 3,
            results: [{ position: 1, is_dnf: false }],
          },
        ];
      }
      if (command === "get_teams_standings") {
        return [
          specialTeam({
            id: "TBMW",
            nome: "BMW Works",
            classe: "bmw",
            pontos: 120,
            vitorias: 4,
            piloto1: "Bianca Rossi",
            piloto2: "Luca Neri",
          }),
          specialTeam({
            id: "TBM2",
            nome: "BMW Junior",
            classe: "bmw",
            pontos: 110,
            piloto1: "Ana Longname-Silva",
            piloto2: "Carlo Verylongname",
          }),
          specialTeam({
            id: "TBM3",
            nome: "BMW Academy",
            classe: "bmw",
            pontos: 100,
            piloto1: "Nina Park",
            piloto2: "Otto Klein",
          }),
          specialTeam({
            id: "TBM4",
            nome: "BMW North",
            classe: "bmw",
            pontos: 90,
            piloto1: "Iris Blue",
            piloto2: "Theo Gray",
          }),
          specialTeam({
            id: "TBM5",
            nome: "BMW South",
            classe: "bmw",
            pontos: 80,
            piloto1: "Maya Sun",
            piloto2: null,
          }),
          {
            id: "TTOY",
            nome: "Toyota Spirit",
            nome_curto: "TOY",
            cor_primaria: "#f2cc60",
            classe: "toyota",
            pontos: 108,
            vitorias: 3,
            posicao: 2,
            piloto_1_nome: "Taro Sato",
            piloto_2_nome: "Aiko Tanaka",
          },
          {
            id: "TMAZ",
            nome: "Mazda Club",
            nome_curto: "MAZ",
            cor_primaria: "#c8102e",
            classe: "mazda",
            pontos: 96,
            vitorias: 2,
            posicao: 3,
            piloto_1_nome: "Marta Vega",
            piloto_2_nome: "Diego Sol",
          },
        ];
      }
      if (command === "get_previous_champions") {
        return { driver_champion_id: null, constructor_champions: [] };
      }
      return [];
    });

    render(<StandingsTab />);

    await screen.findByText("Bianca Rossi");
    const driverTable = screen.getByRole("table");
    expect(within(driverTable).getByText("BMW M2")).toBeInTheDocument();
    expect(within(driverTable).getByText("Toyota GR86")).toBeInTheDocument();
    expect(within(driverTable).getByText("Mazda MX-5")).toBeInTheDocument();
    expect(within(driverTable).getByText("BMW M2").closest("div")).toHaveClass("sticky", "left-0", "justify-center");
    expect(within(driverTable).getByText("BMW M2").closest("div")).not.toHaveClass("rounded-xl", "border");
    expect(within(driverTable).getByText("BMW M2")).toHaveClass("text-[17px]", "text-center");
    expect(within(driverTable).queryByText(/inscrito/i)).not.toBeInTheDocument();
    expect(within(screen.getByText("Bianca Rossi").closest("tr")).getByText("1")).toBeInTheDocument();
    expect(within(screen.getByText("Taro Sato").closest("tr")).getByText("1")).toBeInTheDocument();
    expect(within(screen.getByText("Marta Vega").closest("tr")).getByText("1")).toBeInTheDocument();

    expect(screen.getAllByText("BMW M2")).toHaveLength(2);
    expect(screen.getAllByText("Toyota GR86")).toHaveLength(2);
    expect(screen.getAllByText("Mazda MX-5")).toHaveLength(2);
    expect(screen.getByText("Bianca Rossi / Luca Neri")).toHaveClass("whitespace-nowrap");
    expect(screen.getByText("Maya Sun / -")).toHaveClass("whitespace-nowrap");
    expect(screen.queryByText(/Ã/)).not.toBeInTheDocument();
    expect(screen.queryByText("REBAIXAMENTO ↓")).not.toBeInTheDocument();
    expect(screen.getByText("BMW Academy").closest("[data-relegation-zone]")).toHaveAttribute(
      "data-relegation-zone",
      "true",
    );
    expect(screen.getByText("BMW Works").closest("[data-relegation-zone]")).toBeNull();
  });

  it("keeps normal team driver names readable when a seat has no assigned driver", async () => {
    mockState = {
      ...mockState,
      playerTeam: {
        categoria: "gt4",
      },
    };

    invoke.mockImplementation(async (command) => {
      if (command === "get_teams_standings") {
        return [
          {
            id: "GT4A",
            nome: "GT4 Atlas",
            nome_curto: "ATL",
            cor_primaria: "#58a6ff",
            pontos: 42,
            vitorias: 1,
            posicao: 1,
            piloto_1_nome: "Alex Stone",
            piloto_2_nome: null,
          },
        ];
      }
      if (command === "get_previous_champions") {
        return { driver_champion_id: null, constructor_champions: [] };
      }
      return [];
    });

    render(<StandingsTab />);

    await screen.findByText("GT4 Atlas");
    const driverLine = screen.getByText("Alex Stone / -");
    expect(driverLine).toHaveClass("whitespace-nowrap");
    expect(driverLine).toHaveAttribute("title", "Alex Stone / -");
    expect(screen.queryByText(/Ã/)).not.toBeInTheDocument();
  });

  it("explains special standings before the special competition has results", async () => {
    mockState = {
      ...mockState,
      season: {
        ...mockState.season,
        fase: "BlocoRegular",
      },
    };

    invoke.mockImplementation(async (command) => {
      if (command === "get_teams_standings") {
        return [
          specialTeam({
            id: "TBMW",
            nome: "BMW Works",
            classe: "bmw",
            pontos: 0,
            piloto1: null,
            piloto2: null,
          }),
        ];
      }
      if (command === "get_previous_champions") {
        return { driver_champion_id: null, constructor_champions: [] };
      }
      return [];
    });

    render(<StandingsTab />);

    await screen.findByText("Competicao especial ainda nao aconteceu");
    expect(screen.getByText(/acontece depois da temporada regular/i)).toBeInTheDocument();
    expect(screen.queryByText("BMW Works")).not.toBeInTheDocument();
  });
});
