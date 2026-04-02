import { render, screen } from "@testing-library/react";

import Dashboard from "./Dashboard";

let mockState = {};

vi.mock("../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

vi.mock("../components/layout/MainLayout", () => ({
  default: ({ children, activeTab, hideHeader = false }) => (
    <div
      data-testid="main-layout"
      data-active-tab={activeTab}
      data-hide-header={hideHeader ? "true" : "false"}
    >
      {children}
    </div>
  ),
}));

vi.mock("../components/race/RaceResultView", () => ({
  default: () => <div>Classificacao final</div>,
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

  it("starts on the drivers tab when loading a save", () => {
    mockState.showRaceBriefing = false;

    render(<Dashboard />);

    expect(screen.getByTestId("main-layout")).toHaveAttribute("data-active-tab", "standings");
  });

  it("hides the main header while showing the final classification screen", () => {
    mockState.showRaceBriefing = false;
    mockState.showResult = true;
    mockState.lastRaceResult = { track_name: "Interlagos", race_results: [] };

    render(<Dashboard />);

    expect(screen.getByTestId("main-layout")).toHaveAttribute("data-hide-header", "true");
    expect(screen.getByText("Classificacao final")).toBeInTheDocument();
  });
});
