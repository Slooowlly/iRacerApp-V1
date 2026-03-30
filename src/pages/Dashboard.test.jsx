import { render, screen } from "@testing-library/react";

import Dashboard from "./Dashboard";

let mockState = {};

vi.mock("../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

vi.mock("../components/layout/MainLayout", () => ({
  default: ({ children }) => <div data-testid="main-layout">{children}</div>,
}));

vi.mock("./tabs/NextRaceTab", () => ({
  default: () => <div>Briefing pre-corrida</div>,
}));

describe("Dashboard", () => {
  beforeEach(() => {
    mockState = {
      isLoaded: true,
      showRaceBriefing: true,
      showResult: false,
      lastRaceResult: null,
      dismissResult: vi.fn(),
      showEndOfSeason: false,
      endOfSeasonResult: null,
      showPreseason: false,
    };
  });

  it("renders the pre-race briefing before the regular tabs", () => {
    render(<Dashboard />);

    expect(screen.getByTestId("main-layout")).toBeInTheDocument();
    expect(screen.getByText("Briefing pre-corrida")).toBeInTheDocument();
  });
});
