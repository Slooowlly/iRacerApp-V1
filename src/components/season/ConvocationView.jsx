import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassButton from "../ui/GlassButton";
import GlassCard from "../ui/GlassCard";
import LoadingOverlay from "../ui/LoadingOverlay";
import useCareerStore from "../../stores/useCareerStore";

const CATEGORY_LABELS = {
  production_challenger: "Production",
  endurance: "Endurance",
};

function roleLabel(role) {
  if (role === "Numero1") return "Piloto principal";
  if (role === "Numero2") return "Segundo piloto";
  return "Convocado";
}

function normalizeTeamGroups(entries = []) {
  const grouped = new Map();

  for (const team of entries) {
    const categoryKey = team.categoria ?? team._categoria ?? "especial";
    const classKey = team.classe ?? "geral";
    const key = `${categoryKey}:${classKey}`;

    if (!grouped.has(key)) {
      grouped.set(key, {
        category: categoryKey,
        className: classKey,
        teams: [],
      });
    }

    grouped.get(key).teams.push(team);
  }

  return [...grouped.values()].sort((left, right) => {
    if (left.category === right.category) {
      return left.className.localeCompare(right.className);
    }
    return left.category.localeCompare(right.category);
  });
}

export default function ConvocationView() {
  const careerId = useCareerStore((state) => state.careerId);
  const season = useCareerStore((state) => state.season);
  const playerSpecialOffers = useCareerStore((state) => state.playerSpecialOffers);
  const acceptedSpecialOffer = useCareerStore((state) => state.acceptedSpecialOffer);
  const isConvocating = useCareerStore((state) => state.isConvocating);
  const error = useCareerStore((state) => state.error);
  const respondToSpecialOffer = useCareerStore((state) => state.respondToSpecialOffer);
  const confirmSpecialBlock = useCareerStore((state) => state.confirmSpecialBlock);

  const [teamGroups, setTeamGroups] = useState([]);
  const [loadError, setLoadError] = useState("");

  useEffect(() => {
    let active = true;

    async function loadSpecialTeams() {
      if (!careerId) {
        if (active) {
          setTeamGroups([]);
        }
        return;
      }

      try {
        const [production, endurance] = await Promise.all([
          invoke("get_teams_standings", {
            careerId,
            category: "production_challenger",
          }).catch(() => []),
          invoke("get_teams_standings", {
            careerId,
            category: "endurance",
          }).catch(() => []),
        ]);

        if (!active) return;

        const merged = [
          ...(Array.isArray(production)
            ? production.map((team) => ({ ...team, _categoria: "production_challenger" }))
            : []),
          ...(Array.isArray(endurance)
            ? endurance.map((team) => ({ ...team, _categoria: "endurance" }))
            : []),
        ];

        setTeamGroups(normalizeTeamGroups(merged));
        setLoadError("");
      } catch (invokeError) {
        if (!active) return;
        setTeamGroups([]);
        setLoadError(
          typeof invokeError === "string"
            ? invokeError
            : invokeError?.toString?.() ?? "Nao foi possivel carregar as equipes especiais.",
        );
      }
    }

    loadSpecialTeams();

    return () => {
      active = false;
    };
  }, [careerId]);

  const groupedOffers = useMemo(() => {
    const byCategory = new Map();

    for (const offer of playerSpecialOffers) {
      const category = offer.special_category ?? "especial";
      if (!byCategory.has(category)) {
        byCategory.set(category, []);
      }
      byCategory.get(category).push(offer);
    }

    return [...byCategory.entries()];
  }, [playerSpecialOffers]);

  const primaryCtaLabel = acceptedSpecialOffer
    ? "Entrar no bloco especial"
    : "Seguir sem entrar no especial";

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,#142033_0%,#09111d_45%,#06090e_100%)] text-white">
      <LoadingOverlay
        open={isConvocating}
        title="Processando convocacao"
        message="Atualizando ofertas do jogador e preparando o bloco especial."
      />

      <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-6 py-8">
        <header className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-xs font-black uppercase tracking-[0.26em] text-[#58a6ff]">
              Janela de Convocacao
            </p>
            <h1 className="mt-3 text-4xl font-black tracking-tight">
              Production e Endurance
            </h1>
            <p className="mt-3 max-w-3xl text-sm text-white/72">
              Escolha uma equipe se quiser entrar no bloco especial. Se preferir ficar fora, o ano
              segue normalmente e voce podera pular essa fase depois.
            </p>
          </div>

          <GlassCard hover={false} className="rounded-3xl px-5 py-4 lg:min-w-[280px]">
            <p className="text-[11px] uppercase tracking-[0.22em] text-white/45">Temporada</p>
            <p className="mt-2 text-2xl font-black">
              {season?.ano ?? "--"} · {season?.fase ?? "JanelaConvocacao"}
            </p>
          </GlassCard>
        </header>

        {acceptedSpecialOffer ? (
          <GlassCard hover={false} className="rounded-3xl border border-[#58a6ff]/30 bg-[#58a6ff]/10 p-5">
            <p className="text-[11px] font-black uppercase tracking-[0.22em] text-[#8bc2ff]">
              Convocacao aceita
            </p>
            <p className="mt-2 text-xl font-black">
              {acceptedSpecialOffer.team_name} ·{" "}
              {CATEGORY_LABELS[acceptedSpecialOffer.special_category] ??
                acceptedSpecialOffer.special_category}
            </p>
            <p className="mt-2 text-sm text-white/72">
              Sua vaga especial ja esta reservada. Quando quiser, voce pode iniciar o bloco
              especial pelo botao abaixo.
            </p>
          </GlassCard>
        ) : null}

        <div className="grid gap-6 xl:grid-cols-[1.1fr_1.9fr]">
          <GlassCard hover={false} className="rounded-3xl p-6">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-[11px] font-black uppercase tracking-[0.22em] text-[#58a6ff]">
                  Suas convocacoes
                </p>
                <h2 className="mt-2 text-2xl font-black">Decisao do jogador</h2>
              </div>
              <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-bold text-white/70">
                {playerSpecialOffers.length} pendentes
              </span>
            </div>

            {groupedOffers.length === 0 ? (
              <div className="mt-6 rounded-3xl border border-dashed border-white/10 bg-black/20 p-5">
                <p className="text-sm font-semibold text-white">Nenhuma convocacao pendente.</p>
                <p className="mt-2 text-sm text-white/60">
                  Se voce nao entrou em nenhuma equipe especial, pode seguir em frente e deixar o
                  bloco especial rodar sem participacao do jogador.
                </p>
              </div>
            ) : (
              <div className="mt-6 space-y-5">
                {groupedOffers.map(([category, offers]) => (
                  <div key={category} className="space-y-3">
                    <p className="text-xs font-black uppercase tracking-[0.2em] text-white/45">
                      {CATEGORY_LABELS[category] ?? category}
                    </p>
                    {offers.map((offer) => (
                      <div
                        key={offer.id}
                        className="rounded-3xl border border-white/8 bg-black/20 p-4"
                      >
                        <div className="flex items-start justify-between gap-4">
                          <div>
                            <p className="text-lg font-black text-white">{offer.team_name}</p>
                            <p className="mt-1 text-sm text-white/60">
                              Classe {offer.class_name.toUpperCase()} · {roleLabel(offer.papel)}
                            </p>
                          </div>
                          <span className="rounded-full border border-[#58a6ff]/20 bg-[#58a6ff]/10 px-3 py-1 text-[11px] font-black uppercase tracking-[0.18em] text-[#8bc2ff]">
                            {CATEGORY_LABELS[offer.special_category] ?? offer.special_category}
                          </span>
                        </div>

                        <div className="mt-4 flex flex-wrap gap-3">
                          <GlassButton
                            variant="primary"
                            disabled={isConvocating}
                            onClick={() => void respondToSpecialOffer(offer.id, true)}
                          >
                            Aceitar
                          </GlassButton>
                          <GlassButton
                            variant="ghost"
                            disabled={isConvocating}
                            onClick={() => void respondToSpecialOffer(offer.id, false)}
                          >
                            Recusar
                          </GlassButton>
                        </div>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            )}

            <div className="mt-6 flex flex-wrap gap-3">
              <GlassButton
                variant="primary"
                disabled={isConvocating}
                onClick={() => void confirmSpecialBlock()}
              >
                {primaryCtaLabel}
              </GlassButton>
            </div>

            {error ? <p className="mt-4 text-sm text-[#ff9b9b]">{error}</p> : null}
          </GlassCard>

          <GlassCard hover={false} className="rounded-3xl p-6">
            <p className="text-[11px] font-black uppercase tracking-[0.22em] text-[#58a6ff]">
              Grids especiais
            </p>
            <h2 className="mt-2 text-2xl font-black">Equipes convocadas</h2>
            <p className="mt-2 text-sm text-white/60">
              Panorama rapido das equipes que vao disputar o bloco especial.
            </p>

            {teamGroups.length === 0 ? (
              <div className="mt-6 rounded-3xl border border-dashed border-white/10 bg-black/20 p-5">
                <p className="text-sm text-white/70">
                  {loadError || "As equipes especiais aparecem aqui assim que o grid estiver pronto."}
                </p>
              </div>
            ) : (
              <div className="mt-6 space-y-6">
                {teamGroups.map((group) => (
                  <section key={`${group.category}:${group.className}`} className="space-y-3">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <p className="text-xs font-black uppercase tracking-[0.18em] text-white/45">
                          {CATEGORY_LABELS[group.category] ?? group.category}
                        </p>
                        <p className="mt-1 text-lg font-black">{group.className.toUpperCase()}</p>
                      </div>
                      <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-bold text-white/70">
                        {group.teams.length} equipes
                      </span>
                    </div>

                    <div className="grid gap-3 md:grid-cols-2">
                      {group.teams.map((team) => (
                        <div
                          key={team.id}
                          className="rounded-3xl border border-white/8 bg-black/20 p-4"
                        >
                          <p className="text-base font-black text-white">{team.nome}</p>
                          <p className="mt-2 text-sm text-white/60">
                            {team.piloto_1_nome || "Piloto 1 em aberto"}
                          </p>
                          <p className="text-sm text-white/60">
                            {team.piloto_2_nome || "Piloto 2 em aberto"}
                          </p>
                        </div>
                      ))}
                    </div>
                  </section>
                ))}
              </div>
            )}
          </GlassCard>
        </div>
      </div>
    </div>
  );
}
