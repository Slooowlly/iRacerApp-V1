import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import NewsTab from "./NewsTab";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args) => mockInvoke(...args),
}));

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) =>
    selector({
      careerId: "career_001",
      season: {
        numero: 1,
        rodada_atual: 2,
      },
    }),
}));

function buildBootstrap() {
  return {
    default_scope_type: "category",
    default_scope_id: "mazda_rookie",
    default_primary_filter: null,
    scopes: [
      {
        id: "mazda_rookie",
        label: "Mazda MX-5 Rookie Cup",
        short_label: "Mazda MX-5",
        scope_type: "category",
        special: false,
      },
      {
        id: "toyota_rookie",
        label: "Toyota GR86 Rookie Cup",
        short_label: "Toyota Rookie",
        scope_type: "category",
        special: false,
      },
      {
        id: "mazda_amador",
        label: "Mazda MX-5 Championship",
        short_label: "Mazda Championship",
        scope_type: "category",
        special: false,
      },
      {
        id: "toyota_amador",
        label: "Toyota GR86 Cup",
        short_label: "Toyota Cup",
        scope_type: "category",
        special: false,
      },
      {
        id: "bmw_m2",
        label: "BMW M2 CS Racing",
        short_label: "BMW M2",
        scope_type: "category",
        special: false,
      },
      {
        id: "production_challenger",
        label: "Production Car Challenger",
        short_label: "Production",
        scope_type: "category",
        special: false,
      },
      {
        id: "gt4",
        label: "GT4 Series",
        short_label: "GT4",
        scope_type: "category",
        special: false,
      },
      {
        id: "gt3",
        label: "GT3 Championship",
        short_label: "GT3",
        scope_type: "category",
        special: false,
      },
      {
        id: "endurance",
        label: "Endurance Championship",
        short_label: "Endurance",
        scope_type: "category",
        special: false,
      },
      {
        id: "mais_famosos",
        label: "Mais famosos",
        short_label: "Mais famosos",
        scope_type: "famous",
        special: true,
      },
    ],
    season_number: 1,
    season_year: 2026,
    current_round: 2,
    total_rounds: 5,
  };
}

function buildStories() {
  return [
    {
      id: "NT001",
      icon: "N",
      title: "LEGADO_OPENING_TITLE",
      headline: "Abertura forte em Okayama",
      summary: "LEGADO_OPENING_SUMMARY",
      deck: "A etapa inicial redefiniu o humor do grid.",
      body_text: "LEGADO_OPENING_BODY_TEXT",
      blocks: [
        {
          label: "Resumo",
          text: "Okayama abriu a temporada com o grid embaralhado logo na primeira volta.",
        },
        {
          label: "Impacto",
          text: "A margem entre os ponteiros caiu e o paddock saiu da etapa com leitura mais agressiva.",
        },
        {
          label: "Leitura",
          text: "A abertura indica um campeonato menos previsivel do que o desenho de pre-temporada sugeria.",
        },
      ],
      news_type: "Corrida",
      importance: "Destaque",
      importance_label: "Destaque",
      category_label: "Mazda MX-5 Rookie Cup",
      meta_label: "Corrida · R1 · T1",
      time_label: "Edicao recente",
      entity_label: "Apex Academy Racing",
      team_label: "Apex Academy Racing",
      driver_label: "Thomas Baker",
      race_label: "Okayama",
      accent_tone: "gold",
      driver_id: "P001",
      team_id: "T001",
      round: 1,
    },
    {
      id: "NT002",
      icon: "M",
      title: "LEGADO_MARKET_TITLE",
      headline: "Mercado observa pilotos em ascensao",
      summary: "LEGADO_MARKET_SUMMARY",
      deck: "O paddock comenta nomes que chegam valorizados.",
      body_text: "LEGADO_MARKET_BODY_TEXT",
      blocks: [
        {
          label: "Movimento",
          text: "O paddock reposicionou Kenji Sato no mapa das conversas de mercado desta semana.",
        },
        {
          label: "Impacto",
          text: "A valorizacao muda a leitura de vagas competitivas para a janela seguinte.",
        },
        {
          label: "Próximo passo",
          text: "O nome segue em observacao enquanto as equipes cruzam desempenho recente e margem salarial.",
        },
      ],
      news_type: "Mercado",
      importance: "Alta",
      importance_label: "Alta",
      category_label: "Mazda MX-5 Rookie Cup",
      meta_label: "Mercado · T1",
      time_label: "Mais cedo",
      entity_label: "Kenji Sato",
      team_label: null,
      driver_label: "Kenji Sato",
      race_label: null,
      accent_tone: "blue",
      driver_id: "P002",
      team_id: null,
      round: null,
    },
  ];
}

function buildSnapshot(overrides = {}) {
  return {
    hero: {
      section_label: "Central de Notícias",
      title: "Panorama do Campeonato",
      subtitle: "O paddock chega aquecido para a próxima etapa da categoria.",
      badge: "Rodada 2/5",
      badge_tone: "blue",
    },
    primary_filters: [
      { id: "Corridas", label: "Corridas", meta: null, tone: "cool", kind: "tag", color_primary: null, color_secondary: null },
      { id: "Pilotos", label: "Pilotos", meta: null, tone: "cool", kind: "tag", color_primary: null, color_secondary: null },
      { id: "Equipes", label: "Equipes", meta: null, tone: "cool", kind: "tag", color_primary: null, color_secondary: null },
      { id: "Mercado", label: "Mercado", meta: null, tone: "warm", kind: "tag", color_primary: null, color_secondary: null },
    ],
    contextual_filters: [],
    stories: buildStories(),
    scope_meta: {
      scope_type: "category",
      scope_id: "mazda_rookie",
      scope_label: "Mazda MX-5 Rookie Cup",
      scope_class: null,
      primary_filter: null,
      context_type: null,
      context_id: null,
      context_label: null,
      is_special: false,
    },
    ...overrides,
  };
}

function scopeLabelFor(scopeId) {
  const labels = {
    mazda_rookie: "Mazda MX-5 Rookie Cup",
    toyota_rookie: "Toyota GR86 Rookie Cup",
    mazda_amador: "Mazda MX-5 Championship",
    toyota_amador: "Toyota GR86 Cup",
    bmw_m2: "BMW M2 CS Racing",
    production_challenger: "Production Car Challenger",
    gt4: "GT4 Series",
    gt3: "GT3 Championship",
    endurance: "Endurance Championship",
    mais_famosos: "Mais famosos",
  };

  return labels[scopeId] ?? scopeId;
}

describe("NewsTab", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockImplementation((command, payload) => {
      if (command === "get_news_tab_bootstrap") {
        return Promise.resolve(buildBootstrap());
      }

      if (command === "get_news_tab_snapshot") {
        const scopeId = payload?.request?.scope_id ?? "mazda_rookie";
        const scopeType = payload?.request?.scope_type ?? "category";
        const filter = payload?.request?.primary_filter ?? null;
        const contextualFilters =
          filter === "Corridas"
            ? [
                {
                  id: "R001",
                  label: "Okayama",
                  meta: "R1",
                  tone: "cool",
                  kind: "race",
                  color_primary: null,
                  color_secondary: null,
                },
                {
                  id: "R002",
                  label: "Laguna Seca",
                  meta: "R2",
                  tone: "cool",
                  kind: "race",
                  color_primary: null,
                  color_secondary: null,
                },
                {
                  id: "R003",
                  label: "Road Atlanta",
                  meta: "R3",
                  tone: "cool",
                  kind: "race",
                  color_primary: null,
                  color_secondary: null,
                },
              ]
            : filter === "Pilotos"
              ? [
                  {
                    id: "P001",
                    label: "Thomas Baker",
                    meta: "120 pts",
                    tone: "cool",
                    kind: "driver",
                    color_primary: null,
                    color_secondary: null,
                  },
                  {
                    id: "P002",
                    label: "Kenji Sato",
                    meta: "108 pts",
                    tone: "cool",
                    kind: "driver",
                    color_primary: null,
                    color_secondary: null,
                  },
                  {
                    id: "P003",
                    label: "Mia Torres",
                    meta: "96 pts",
                    tone: "cool",
                    kind: "driver",
                    color_primary: null,
                    color_secondary: null,
                  },
                ]
            : [];
        return Promise.resolve(
          buildSnapshot({
            contextual_filters: contextualFilters,
            scope_meta: {
            scope_type: scopeType,
            scope_id: scopeId,
            scope_label: scopeLabelFor(scopeId),
            scope_class: payload?.request?.scope_class ?? null,
            primary_filter: filter,
            context_type: null,
            context_id: null,
              context_label: null,
              is_special: scopeType === "famous",
            },
          }),
        );
      }

      return Promise.reject(new Error(`unexpected command: ${command}`));
    });
  });

  it("loads the snapshot with no primary filter active by default", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    expect(screen.queryByText("Situação do campeonato")).not.toBeInTheDocument();
    expect(screen.queryByText("O paddock chega aquecido para a próxima etapa da categoria.")).not.toBeInTheDocument();
    expect(screen.queryByText("Mazda MX-5 Rookie Cup")).not.toBeInTheDocument();
    expect(screen.getAllByText("Próxima etapa")[0].parentElement).toHaveClass('absolute', 'right-0', 'top-0');
    expect(container.querySelector('[data-news-section="hero"] > div')).toHaveClass('rounded-[26px]');
    expect(container.querySelector('[data-news-section="hero"] > div > div.relative.flex.items-center.justify-between')).toHaveClass('pt-3');
    expect(container.querySelector('.grid-cols-\\[1fr_auto\\]')).toBeNull();
    expect(container.querySelector('[data-news-hero-body]')).toHaveClass('space-y-0', 'px-5', 'pb-3', 'pt-1');
    expect(container.querySelector('[data-news-hero-summary]')).toHaveClass('gap-2', 'pr-[220px]');
    expect(screen.getByRole('heading', { name: 'Panorama do Campeonato' })).toHaveClass('text-[2rem]');
    expect(container.querySelector('[data-news-section="main-reader"]')).not.toBeNull();
    expect(screen.queryByRole("button", { name: "Expectativas" })).not.toBeInTheDocument();

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          primary_filter: null,
        }),
      });
    });
  });

  it("toggles the same primary filter on and off", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });

    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    await screen.findByRole("button", { name: "Corridas" });

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Corridas" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          primary_filter: "Corridas",
        }),
      });
    });

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Corridas" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          primary_filter: null,
        }),
      });
    });
  });

  it("renders the unified context panel with compact pill tabs and dims future races", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Corridas" }));

    await screen.findByRole("button", { name: "Okayama" });

    expect(container.querySelector('[data-news-section="primary-filters"]')).toBeNull();
    expect(screen.queryByText("Filtro contextual")).not.toBeInTheDocument();
    expect(screen.queryByText("Leitura da Temporada")).not.toBeInTheDocument();
    expect(screen.queryByText("Recorte em Foco")).not.toBeInTheDocument();

    const contextPanel = container.querySelector('[data-news-section="context-panel"]');
    const primaryPill = container.querySelector("[data-news-primary-pill]");
    const contextResults = container.querySelector("[data-news-context-results]");
    const futureRaceButton = screen.getByRole("button", { name: "Laguna Seca" });

    expect(contextPanel).not.toBeNull();
    expect(primaryPill).not.toBeNull();
    expect(contextResults).not.toBeNull();
    expect(primaryPill.className).toContain("mx-auto");
    expect(contextResults.className).toContain("justify-center");
    expect(futureRaceButton.className).toContain("text-center");
    expect(futureRaceButton.className).toContain("opacity-35");
    expect(futureRaceButton).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Narrativa" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Consequências" })).not.toBeInTheDocument();
  });

  it("uses medal tones for the top three driver filters", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Pilotos" }));

    const firstPlace = await screen.findByRole("button", { name: "Thomas Baker" });
    const secondPlace = screen.getByRole("button", { name: "Kenji Sato" });
    const thirdPlace = screen.getByRole("button", { name: "Mia Torres" });

    expect(firstPlace.className).toContain("border-podium-gold/30");
    expect(firstPlace.className).toContain("text-podium-gold");
    expect(secondPlace.className).toContain("border-podium-silver/30");
    expect(secondPlace.className).toContain("text-podium-silver");
    expect(thirdPlace.className).toContain("border-podium-bronze/30");
    expect(thirdPlace.className).toContain("text-podium-bronze");
  });

  it("switches the open story locally when a list item is clicked", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    await screen.findByText("Okayama abriu a temporada com o grid embaralhado logo na primeira volta.");
    expect(screen.getByText("Capítulos paralelos")).toBeInTheDocument();
    const openStory = document.querySelector("[data-news-open-story]");
    expect(openStory).not.toBeNull();
    expect(within(openStory).getByText("Abertura forte em Okayama")).toBeInTheDocument();
    expect(within(openStory).getByText("Resumo")).toBeInTheDocument();
    expect(within(openStory).getByText("Impacto")).toBeInTheDocument();
    expect(within(openStory).getAllByText("Leitura").length).toBeGreaterThan(0);
    expect(within(openStory).getByText("Publicada em")).toBeInTheDocument();
    expect(within(openStory).getAllByText("Leitura").length).toBeGreaterThan(1);
    expect(within(openStory).getByText("Contexto")).toBeInTheDocument();
    expect(screen.getAllByText("A etapa inicial redefiniu o humor do grid.").length).toBeGreaterThan(0);
    expect(screen.queryByText("LEGADO_OPENING_TITLE")).not.toBeInTheDocument();
    expect(screen.queryByText("LEGADO_OPENING_SUMMARY")).not.toBeInTheDocument();
    expect(screen.queryByText("LEGADO_OPENING_BODY_TEXT")).not.toBeInTheDocument();

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: /Mercado observa pilotos em ascensao/i }));

    expect(screen.getAllByText("O paddock comenta nomes que chegam valorizados.").length).toBeGreaterThan(0);
    expect(screen.getByText("Movimento")).toBeInTheDocument();
    expect(screen.getByText("Próximo passo")).toBeInTheDocument();
    expect(screen.getByText("O nome segue em observacao enquanto as equipes cruzam desempenho recente e margem salarial.")).toBeInTheDocument();
    expect(screen.queryByText("LEGADO_MARKET_TITLE")).not.toBeInTheDocument();
    expect(screen.queryByText("LEGADO_MARKET_SUMMARY")).not.toBeInTheDocument();
    expect(screen.queryByText("LEGADO_MARKET_BODY_TEXT")).not.toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("expands a family and applies the default scope automatically", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });

    expect(screen.getAllByRole("button", { name: "Mazda" })[0]).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Toyota" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "BMW" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "GT4" })).toBeInTheDocument();
    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Toyota" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          scope_id: "toyota_rookie",
        }),
      });
    });

    expect(screen.getByRole("button", { name: "Cup" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Corridas" })).toBeInTheDocument();
    expect(container.querySelector('[data-news-section="main-reader"]')).not.toBeNull();
  });

  it("anchors the family drawer under the clicked category card", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "GT3" });

    const scopeRow = container.querySelector("[data-news-scope-row]");
    const gt3Button = screen.getAllByRole("button", { name: "GT3" })[0];

    fireEvent.click(gt3Button);

    const drawerPanel = await screen.findByRole("button", { name: "Endurance" });
    const drawerCard = drawerPanel.closest("[data-news-scope-drawer-panel]");

    vi.spyOn(scopeRow, "getBoundingClientRect").mockReturnValue({
      x: 80,
      y: 0,
      left: 80,
      top: 0,
      width: 900,
      height: 64,
      right: 980,
      bottom: 64,
      toJSON: () => ({}),
    });
    vi.spyOn(gt3Button, "getBoundingClientRect").mockReturnValue({
      x: 356,
      y: 0,
      left: 356,
      top: 0,
      width: 138,
      height: 64,
      right: 494,
      bottom: 64,
      toJSON: () => ({}),
    });
    vi.spyOn(drawerCard, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      width: 228,
      height: 46,
      right: 228,
      bottom: 46,
      toJSON: () => ({}),
    });

    fireEvent(window, new Event("resize"));

    await waitFor(() => {
      const drawerTrack = container.querySelector("[data-news-scope-drawer-track]");
      expect(drawerTrack).not.toBeNull();
      expect(drawerTrack.style.left).toBe("231px");
      expect(drawerTrack.style.top).toBe("72px");
    });
  });

  it("drops the drawer under the clicked row when categories wrap", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "LMP2" });

    const scopeRow = container.querySelector("[data-news-scope-row]");
    const lmp2Button = screen.getAllByRole("button", { name: "LMP2" })[0];

    fireEvent.click(lmp2Button);

    const drawerPanel = await screen.findByRole("button", { name: "Endurance" });
    const drawerCard = drawerPanel.closest("[data-news-scope-drawer-panel]");

    vi.spyOn(scopeRow, "getBoundingClientRect").mockReturnValue({
      x: 80,
      y: 100,
      left: 80,
      top: 100,
      width: 900,
      height: 140,
      right: 980,
      bottom: 240,
      toJSON: () => ({}),
    });
    vi.spyOn(lmp2Button, "getBoundingClientRect").mockReturnValue({
      x: 760,
      y: 176,
      left: 760,
      top: 176,
      width: 138,
      height: 64,
      right: 898,
      bottom: 240,
      toJSON: () => ({}),
    });
    vi.spyOn(drawerCard, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      width: 190,
      height: 46,
      right: 190,
      bottom: 46,
      toJSON: () => ({}),
    });

    fireEvent(window, new Event("resize"));

    await waitFor(() => {
      const drawerTrack = container.querySelector("[data-news-scope-drawer-track]");
      expect(drawerTrack).not.toBeNull();
      expect(drawerTrack.style.left).toBe("654px");
      expect(drawerTrack.style.top).toBe("148px");
    });
  });

  it("uses the simplified scope labels and rankings naming in the family drawer", async () => {
    const { container } = render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);

    expect(screen.getByRole("button", { name: "Rankings" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Mais famosos" })).not.toBeInTheDocument();
    expect(screen.queryByText("Categoria")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Mazda Rookie" })).not.toBeInTheDocument();
    expect(screen.queryByText("Mazda Championship")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Rookie" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cup" })).toBeInTheDocument();
    expect(screen.getByText("Production")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("justify-center");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("text-center");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("min-h-[48px]");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("rounded-full");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("min-w-[120px]");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("px-4");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).toContain("py-2");
    expect(screen.getAllByRole("button", { name: "Mazda" })[0].className).not.toContain("rounded-[16px]");
    expect(screen.getByRole("button", { name: "Rankings" }).className).toContain("min-h-[48px]");
    expect(screen.getByRole("button", { name: "Rankings" }).className).toContain("rounded-full");
    const drawerPanel = container.querySelector("[data-news-scope-drawer-panel]");
    const drawerPill = container.querySelector("[data-news-scope-pill]");
    const scopeTabsSection = container.querySelector("[data-news-section='scope-tabs']");
    const heroScopeTabsSection = container.querySelector('[data-news-section="hero"] [data-news-section="scope-tabs"]');
    const summaryScopeTabsSection = container.querySelector('[data-news-hero-summary] [data-news-section="scope-tabs"]');
    const heroScopeTabsSlot = heroScopeTabsSection?.parentElement;
    const scopeTopPill = container.querySelector("[data-news-scope-top-pill]");
    const scopeRow = container.querySelector("[data-news-scope-row]");
    expect(drawerPanel).not.toBeNull();
    expect(drawerPill).not.toBeNull();
    expect(scopeTabsSection).not.toBeNull();
    expect(heroScopeTabsSection).not.toBeNull();
    expect(heroScopeTabsSlot).not.toBeNull();
    expect(summaryScopeTabsSection).toBeNull();
    expect(scopeTopPill).not.toBeNull();
    expect(scopeRow).not.toBeNull();
    expect(scopeTabsSection.className).toContain("flex");
    expect(scopeTabsSection.className).toContain("justify-center");
    expect(scopeTopPill.className).toContain("mx-auto");
    expect(scopeTopPill.className).toContain("w-fit");
    expect(scopeTopPill.className).toContain("flex");
    expect(scopeTopPill.className).not.toContain("inline-flex");
    expect(scopeTopPill.className).toContain("rounded-full");
    expect(scopeTopPill.className).toContain("border");
    expect(scopeRow.className).toContain("justify-center");
    expect(drawerPill.className).toContain("rounded-full");
    expect(drawerPanel.className).toContain("w-fit");
    expect(drawerPanel.className).toContain("max-w-full");
    expect(drawerPanel.className).not.toContain("glass");
  });

  it("uses rookie as the first drawer step for the non-mazda families", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Toyota" });
    fireEvent.click(screen.getByRole("button", { name: "Toyota" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Rookie" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Cup" })).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Toyota" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "BMW" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Rookie" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Production" })).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "BMW M2" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "GT4" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Rookie" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Endurance" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "GT3" }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Rookie" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Endurance" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "LMP2" }));
    await waitFor(() => {
      expect(screen.getByText("Rookie")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Endurance" })).toBeInTheDocument();
    });
  });

  it("refines the scope when a championship inside the family is clicked", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    await screen.findByRole("button", { name: "Cup" });

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Cup" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          scope_id: "mazda_amador",
        }),
      });
    });
  });

  it("keeps the active primary filter when changing the category scope", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });
    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Corridas" }));
    await screen.findByRole("button", { name: "Okayama" });

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Cup" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          scope_id: "mazda_amador",
          primary_filter: "Corridas",
        }),
      });
    });

    expect(screen.getByRole("button", { name: "Corridas" })).toBeInTheDocument();
  });

  it("sends the class filter when a shared championship is selected inside a family", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "Mazda" });

    mockInvoke.mockClear();

    fireEvent.click(screen.getAllByRole("button", { name: "Mazda" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Production" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          scope_id: "production_challenger",
          scope_class: "mazda",
        }),
      });
    });
  });

  it("sends the endurance class filter when the shared endurance scope is refined", async () => {
    render(<NewsTab />);

    await screen.findByRole("button", { name: "GT3" });

    mockInvoke.mockClear();

    fireEvent.click(screen.getByRole("button", { name: "GT3" }));
    fireEvent.click(screen.getByRole("button", { name: "Endurance" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_news_tab_snapshot", {
        careerId: "career_001",
        request: expect.objectContaining({
          scope_id: "endurance",
          scope_class: "gt3",
        }),
      });
    });
  });
});
