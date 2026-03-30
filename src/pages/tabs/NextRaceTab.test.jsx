import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";

import NextRaceTab from "./NextRaceTab";

const mockSimulateRace = vi.fn();
let mockState = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../stores/useCareerStore", () => ({
  default: (selector) => selector(mockState),
}));

describe("NextRaceTab", () => {
  beforeEach(() => {
    mockSimulateRace.mockReset();
    invoke.mockReset();
    invoke.mockImplementation((command, args) => {
      if (command === "get_drivers_by_category") {
        return Promise.resolve([
          {
            id: "drv-player",
            nome: "R. Silva",
            nacionalidade: "Brasil",
            idade: 24,
            skill: 84,
            equipe_id: "team-1",
            equipe_nome: "Equipe Aurora",
            equipe_nome_curto: "Aurora",
            equipe_cor: "#58a6ff",
            is_jogador: true,
            pontos: 88,
            vitorias: 2,
            podios: 4,
            posicao_campeonato: 2,
            results: [
              { position: 3, is_dnf: false },
              { position: 2, is_dnf: false },
              { position: 1, is_dnf: false },
              { position: 4, is_dnf: false },
              null,
              null,
              null,
            ],
          },
          {
            id: "drv-rival",
            nome: "M. Costa",
            nacionalidade: "Portugal",
            idade: 27,
            skill: 86,
            equipe_id: "team-9",
            equipe_nome: "Scuderia Costa",
            equipe_nome_curto: "Costa",
            equipe_cor: "#ff7b72",
            is_jogador: false,
            pontos: 94,
            vitorias: 3,
            podios: 5,
            posicao_campeonato: 1,
            results: [
              { position: 1, is_dnf: false },
              { position: 4, is_dnf: false },
              { position: 2, is_dnf: false },
              { position: 2, is_dnf: false },
              null,
              null,
              null,
            ],
          },
          {
            id: "drv-teammate",
            nome: "A. Lima",
            nacionalidade: "Argentina",
            idade: 23,
            skill: 79,
            equipe_id: "team-1",
            equipe_nome: "Equipe Aurora",
            equipe_nome_curto: "Aurora",
            equipe_cor: "#58a6ff",
            is_jogador: false,
            pontos: 72,
            vitorias: 1,
            podios: 3,
            posicao_campeonato: 4,
            results: [
              { position: 6, is_dnf: false },
              { position: 3, is_dnf: false },
              { position: 2, is_dnf: false },
              { position: 6, is_dnf: false },
              null,
              null,
              null,
            ],
          },
          {
            id: "drv-prado",
            nome: "E. Prado",
            nacionalidade: "Brasil",
            idade: 29,
            skill: 76,
            equipe_id: "team-9",
            equipe_nome: "Scuderia Costa",
            equipe_nome_curto: "Costa",
            equipe_cor: "#ff7b72",
            is_jogador: false,
            pontos: 64,
            vitorias: 0,
            podios: 1,
            posicao_campeonato: 5,
            results: [
              { position: 5, is_dnf: false },
              { position: 6, is_dnf: false },
              { position: 4, is_dnf: false },
              { position: 5, is_dnf: false },
              null,
              null,
              null,
            ],
          },
          {
            id: "drv-duarte",
            nome: "C. Duarte",
            nacionalidade: "Chile",
            idade: 26,
            skill: 74,
            equipe_id: "team-7",
            equipe_nome: "Sierra Racing",
            equipe_nome_curto: "Sierra",
            equipe_cor: "#f5c76d",
            is_jogador: false,
            pontos: 58,
            vitorias: 0,
            podios: 1,
            posicao_campeonato: 6,
            results: [
              { position: 9, is_dnf: false },
              { position: 6, is_dnf: false },
              { position: 4, is_dnf: false },
              { position: 3, is_dnf: false },
              null,
              null,
              null,
            ],
          },
        ]);
      }

      if (command === "get_teams_standings") {
        return Promise.resolve([
          {
            posicao: 1,
            id: "team-9",
            nome: "Scuderia Costa",
            nome_curto: "Costa",
            cor_primaria: "#ff7b72",
            pontos: 134,
            vitorias: 4,
            piloto_1_nome: "M. Costa",
            piloto_2_nome: "E. Prado",
            trofeus: [],
          },
          {
            posicao: 2,
            id: "team-1",
            nome: "Equipe Aurora",
            nome_curto: "Aurora",
            cor_primaria: "#58a6ff",
            pontos: 129,
            vitorias: 3,
            piloto_1_nome: "R. Silva",
            piloto_2_nome: "A. Lima",
            trofeus: [],
          },
        ]);
      }

      if (command === "get_briefing_phrase_history") {
        return Promise.resolve({
          season_number: 1,
          entries: [],
        });
      }

      if (command === "save_briefing_phrase_history") {
        return Promise.resolve({
          season_number: args.seasonNumber,
          entries: args.entries.map((entry) => ({
            season_number: args.seasonNumber,
            ...entry,
          })),
        });
      }

      return Promise.resolve(null);
    });

    mockState = {
      careerId: "career-1",
      player: {
        id: "drv-player",
        nome: "R. Silva",
      },
      playerTeam: {
        id: "team-1",
        nome: "Equipe Aurora",
        nome_curto: "Aurora",
        categoria: "mazda_amador",
      },
      season: {
        numero: 1,
        ano: 2026,
        total_rodadas: 20,
        rodada_atual: 5,
      },
      nextRace: {
        id: "race-5",
        rodada: 5,
        track_name: "Interlagos",
        clima: "Wet",
        duracao_corrida_min: 35,
        status: "Pendente",
        temperatura: 28,
        horario: "14:00",
        display_date: "2026-03-25",
        event_interest: {
          display_value: 84200,
          tier_label: "Evento principal",
        },
      },
      nextRaceBriefing: {
        track_history: {
          has_data: true,
          starts: 4,
          best_finish: 1,
          last_finish: 3,
          dnfs: 1,
          last_visit_season: 1,
          last_visit_round: 4,
        },
        primary_rival: {
          driver_id: "drv-rival",
          driver_name: "M. Costa",
          championship_position: 1,
          gap_points: 6,
          is_ahead: true,
          rivalry_label: null,
        },
        weekend_stories: [
          {
            id: "story-1",
            icon: "⚔",
            title: "Duelo esquenta a abertura",
            summary: "O paddock trata a disputa pela ponta como o assunto central desta rodada.",
            importance: "Alta",
          },
          {
            id: "story-2",
            icon: "📈",
            title: "Aurora quer encostar nos lideres",
            summary: "A equipe chega tratando esta etapa como chance real de mexer na tabela.",
            importance: "Media",
          },
        ],
      },
      isSimulating: false,
      isAdvancing: false,
      simulateRace: mockSimulateRace,
      advanceSeason: vi.fn(),
      enterPreseason: vi.fn(),
    };
  });

  it("shows the simplified event summary and handles simulate and export actions", async () => {
    render(<NextRaceTab />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_drivers_by_category", {
        careerId: "career-1",
        category: "mazda_amador",
      });
    });

    expect(screen.getByText(/^resumo do evento$/i)).toBeInTheDocument();
    expect(screen.queryByText(/briefing de equipe/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /interlagos/i, level: 2 })).not.toBeInTheDocument();
    expect(screen.queryByText(/interlagos .* etapa 5 de 20/i)).not.toBeInTheDocument();
    const stageLabels = screen.getAllByText(/^etapa 5 de 20$/i);
    expect(stageLabels.length).toBeGreaterThan(0);
    expect(stageLabels[0].className).toContain("text-[12px]");
    expect(screen.getByText(/^interlagos$/i)).toBeInTheDocument();
    expect(screen.queryByText(/data do evento/i)).not.toBeInTheDocument();
    expect(screen.getByText(/horario local/i)).toBeInTheDocument();
    expect(screen.getByText(/^publico$/i)).toBeInTheDocument();
    expect(screen.getByText(/^cobertura$/i)).toBeInTheDocument();
    expect(screen.getByText(/^historico$/i)).toBeInTheDocument();
    expect(screen.getByText(/^previa da corrida$/i)).toBeInTheDocument();
    expect(screen.getByText(/^o que esta em jogo$/i)).toBeInTheDocument();
    expect(screen.getByText(/^leitura do paddock$/i)).toBeInTheDocument();
    expect(screen.getByText(/^condicoes$/i)).toBeInTheDocument();
    expect(screen.getByText(/^tempo$/i)).toBeInTheDocument();
    expect(screen.getByText(/^temperatura$/i)).toBeInTheDocument();
    expect(screen.getByText("🌧")).toBeInTheDocument();
    expect(screen.getByText("🌡")).toBeInTheDocument();
    expect(screen.getByText("📻")).toBeInTheDocument();
    expect(screen.getByText(/pista pedindo paciencia na entrada e tracao limpa/i)).toBeInTheDocument();
    expect(screen.getByText(/temperatura equilibrada para stints consistentes/i)).toBeInTheDocument();
    expect(screen.getByText(/trajetoria molhada e janela sensivel/i)).toBeInTheDocument();
    expect(screen.getAllByText(/84\.200/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/3º maior publico da temporada/i)).toBeInTheDocument();
    expect(screen.getByText(/^ao vivo$/i)).toBeInTheDocument();
    expect(screen.getByText(/4 largadas/i)).toBeInTheDocument();
    expect(screen.getByText(/ha velocidade para reagir aqui, mas o retrospecto inclui 1 abandono/i)).toBeInTheDocument();
    expect(screen.getAllByText(/equipe aurora/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/^expectativa$/i)).toBeInTheDocument();
    expect(screen.getByText(/^forma recente$/i)).toBeInTheDocument();
    expect(screen.getByText(/^expectativa$/i).parentElement.className).toContain(
      "md:grid-cols-[72px_0.95fr_0.85fr_1.35fr]",
    );
    expect(screen.queryByText(/^voce$/i)).not.toBeInTheDocument();
    const expectationCells = Array.from(document.querySelectorAll("p")).filter((element) =>
      element.className.includes("text-[13px] leading-5 text-text-primary"),
    );
    expect(expectationCells).toHaveLength(5);
    expect(expectationCells.every((element) => (element.textContent ?? "").length > 40)).toBe(true);
    expect(
      expectationCells.some((element) => /batido|referencia|parametro|ponta/i.test(element.textContent ?? "")),
    ).toBe(true);
    expect(
      expectationCells.some((element) => /primeira fila|perseguidor|largada|ataque/i.test(element.textContent ?? "")),
    ).toBe(true);
    expect(
      expectationCells.some((element) => /podio|top 5|outsider|ameaca/i.test(element.textContent ?? "")),
    ).toBe(true);
    expect(screen.queryByText(/^sem dado$/i)).not.toBeInTheDocument();
    expect(screen.getByText(/^sierra racing$/i)).toBeInTheDocument();
    expect(screen.queryByText(/favoritismo da etapa/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/momento da etapa/i)).not.toBeInTheDocument();
    expect(screen.getByText(/^14:00$/i)).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => /inicio da\s+tarde/i.test(element?.textContent ?? ""))
        .length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(/^voz do box$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^publico estimado$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/interesse do evento/i)).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /tabela de pilotos/i })).toBeInTheDocument();
    expect(screen.queryByText(/^contexto da etapa$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/progresso da temporada/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^para o lider$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^para tras$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^cenario$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^mini tabela$/i)).not.toBeInTheDocument();
    const championshipTable = screen.getByRole("table", { name: /tabela do campeonato/i });
    const championshipRows = within(championshipTable).getAllByRole("row");
    expect(championshipRows).toHaveLength(6);
    expect(within(championshipTable).getByText(/^r\. silva$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^m\. costa$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^a\. lima$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^e\. prado$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^c\. duarte$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^94$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^88$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^58$/i)).toBeInTheDocument();
    expect(screen.getByText(/^fim de semana$/i)).toBeInTheDocument();
    expect(screen.getByText(/^rival principal$/i)).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => {
        const content = element?.textContent ?? "";
        return (
          /m\. costa/i.test(content) &&
          /(referencia|comparacao|parametro|espelho|vantagem|margem)/i.test(content)
        );
      }).length,
    ).toBeGreaterThan(0);
    expect(screen.getByText(/duelo esquenta a abertura/i)).toBeInTheDocument();
    expect(screen.getByText(/aurora quer encostar nos lideres/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /simular corrida/i }));
    expect(mockSimulateRace).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /exportar/i }));
    expect(screen.getByText(/exportacao para o iracing chega em breve/i)).toBeInTheDocument();
  });

  it("uses a more realistic briefing when the title fight is already unlikely", async () => {
    invoke.mockImplementation((command, args) => {
      if (command === "get_drivers_by_category") {
        return Promise.resolve([
          {
            id: "drv-player",
            nome: "R. Silva",
            nacionalidade: "Brasil",
            idade: 24,
            skill: 72,
            equipe_id: "team-1",
            equipe_nome: "Equipe Aurora",
            equipe_nome_curto: "Aurora",
            equipe_cor: "#58a6ff",
            is_jogador: true,
            pontos: 9,
            vitorias: 0,
            podios: 0,
            posicao_campeonato: 8,
            results: [
              null,
              null,
              null,
              { position: 8, is_dnf: false },
              { position: 9, is_dnf: false },
              { position: 8, is_dnf: false },
            ],
          },
          {
            id: "drv-rival",
            nome: "M. Costa",
            nacionalidade: "Portugal",
            idade: 27,
            skill: 91,
            equipe_id: "team-9",
            equipe_nome: "Scuderia Costa",
            equipe_nome_curto: "Costa",
            equipe_cor: "#ff7b72",
            is_jogador: false,
            pontos: 50,
            vitorias: 5,
            podios: 6,
            posicao_campeonato: 1,
            results: [
              { position: 1, is_dnf: false },
              { position: 2, is_dnf: false },
              { position: 1, is_dnf: false },
            ],
          },
        ]);
      }

      if (command === "get_teams_standings") {
        return Promise.resolve([
          {
            posicao: 5,
            id: "team-1",
            nome: "Equipe Aurora",
            nome_curto: "Aurora",
            cor_primaria: "#58a6ff",
            pontos: 21,
            vitorias: 0,
            piloto_1_nome: "R. Silva",
            piloto_2_nome: "A. Lima",
            trofeus: [],
          },
        ]);
      }

      if (command === "get_briefing_phrase_history") {
        return Promise.resolve({
          season_number: 1,
          entries: [],
        });
      }

      if (command === "save_briefing_phrase_history") {
        return Promise.resolve({
          season_number: args.seasonNumber,
          entries: args.entries.map((entry) => ({
            season_number: args.seasonNumber,
            ...entry,
          })),
        });
      }

      return Promise.resolve(null);
    });

    mockState.season = {
      numero: 1,
      ano: 2026,
      total_rodadas: 20,
      rodada_atual: 18,
    };
    mockState.nextRace = {
      ...mockState.nextRace,
      rodada: 19,
    };

    render(<NextRaceTab />);

    await waitFor(() => {
      expect(screen.getAllByText(/forma recente/i).length).toBeGreaterThan(0);
    });

    expect(
      screen.getByText(
        /dignidade competitiva|salvar lastro esportivo|reagir com maturidade|pe firme|resposta honesta/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /buscar um top 8 limpo|corrida madura e eficiente|oportunismo e controle de perdas|prova limpa e firme|pontos fortes e poucos danos/i,
      ),
    ).toBeInTheDocument();
    const championshipTable = screen.getByRole("table", { name: /tabela do campeonato/i });
    expect(within(championshipTable).getAllByRole("row")).toHaveLength(3);
    expect(within(championshipTable).getByText(/^r\. silva$/i)).toBeInTheDocument();
    expect(within(championshipTable).getByText(/^m\. costa$/i)).toBeInTheDocument();
    expect(screen.queryByText(/tentando encurtar a distancia para/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/atacar a lideranca agora que a distancia e curta/i)).not.toBeInTheDocument();
  });
});
