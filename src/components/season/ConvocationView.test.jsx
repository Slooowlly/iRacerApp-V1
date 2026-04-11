import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";

import ConvocationView from "./ConvocationView";

const mockRespondToSpecialOffer = vi.fn();
const mockConfirmSpecialBlock = vi.fn();
let mockState = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

describe("ConvocationView", () => {
  beforeEach(() => {
    mockRespondToSpecialOffer.mockReset();
    mockConfirmSpecialBlock.mockReset();
    invoke.mockReset();
    invoke.mockImplementation((command, args) => {
      if (command === "get_teams_standings" && args.category === "production_challenger") {
        return Promise.resolve([
          {
            id: "prod-1",
            nome: "Aurora Mazda",
            classe: "mazda",
            piloto_1_nome: "L. Ramos",
            piloto_2_nome: "C. Dias",
          },
        ]);
      }

      if (command === "get_teams_standings" && args.category === "endurance") {
        return Promise.resolve([
          {
            id: "end-1",
            nome: "Solar GT4",
            classe: "gt4",
            piloto_1_nome: "R. Silva",
            piloto_2_nome: "N. Prado",
          },
        ]);
      }

      return Promise.resolve([]);
    });

    mockState = {
      careerId: "career-1",
      season: {
        ano: 2026,
        fase: "JanelaConvocacao",
      },
      playerSpecialOffers: [
        {
          id: "offer-1",
          team_id: "end-1",
          team_name: "Solar GT4",
          special_category: "endurance",
          class_name: "gt4",
          papel: "Numero2",
        },
      ],
      acceptedSpecialOffer: null,
      isConvocating: false,
      error: null,
      respondToSpecialOffer: mockRespondToSpecialOffer,
      confirmSpecialBlock: mockConfirmSpecialBlock,
    };
  });

  it("renders player offers and special grids", async () => {
    render(<ConvocationView />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_teams_standings", {
        careerId: "career-1",
        category: "production_challenger",
      });
    });

    expect(screen.getByText(/janela de convocacao/i)).toBeInTheDocument();
    expect(screen.getAllByText(/solar gt4/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/piloto principal|segundo piloto/i)).toBeInTheDocument();
    expect(screen.getAllByText(/production/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/endurance/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/aurora mazda/i)).toBeInTheDocument();
  });

  it("lets the player accept an offer or continue to the special block", async () => {
    render(<ConvocationView />);

    fireEvent.click(screen.getByRole("button", { name: /aceitar/i }));
    expect(mockRespondToSpecialOffer).toHaveBeenCalledWith("offer-1", true);

    fireEvent.click(screen.getByRole("button", { name: /seguir sem entrar no especial/i }));
    expect(mockConfirmSpecialBlock).toHaveBeenCalledTimes(1);
  });
});
