import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import GlassCard from "../../components/ui/GlassCard";
import useCareerStore from "../../stores/useCareerStore";

const CAR_BUILD_PROFILE_META = {
  balanced: {
    label: "Balanceado",
    description: "Projeto premium para manter competitividade em qualquer tipo de pista.",
    costLabel: "Custo alto",
    weights: { acceleration: 34, power: 33, handling: 33 },
  },
  acceleration_intermediate: {
    label: "Aceleracao Intermediaria",
    description: "Leve vies para tracao e retomadas sem abrir mao da consistencia geral.",
    costLabel: "Custo medio",
    weights: { acceleration: 47, power: 27, handling: 27 },
  },
  power_intermediate: {
    label: "Potencia Intermediaria",
    description: "Projeto voltado para retas longas, mas ainda competitivo no restante do calendario.",
    costLabel: "Custo medio",
    weights: { acceleration: 27, power: 47, handling: 27 },
  },
  handling_intermediate: {
    label: "Dirigibilidade Intermediaria",
    description: "Acerto pensado para pistas tecnicas sem perder totalmente o alcance sazonal.",
    costLabel: "Custo medio",
    weights: { acceleration: 27, power: 27, handling: 47 },
  },
  acceleration_extreme: {
    label: "Aceleracao Extrema",
    description: "Aposta agressiva para sobreviver em pistas de tracao e zonas de baixa velocidade.",
    costLabel: "Custo baixo",
    weights: { acceleration: 60, power: 20, handling: 20 },
  },
  power_extreme: {
    label: "Potencia Extrema",
    description: "Carro desenhado para calendarios de reta, mesmo sacrificando versatilidade.",
    costLabel: "Custo baixo",
    weights: { acceleration: 20, power: 60, handling: 20 },
  },
  handling_extreme: {
    label: "Dirigibilidade Extrema",
    description: "Projeto focado em resposta e estabilidade para pistas travadas ou tecnicas.",
    costLabel: "Custo baixo",
    weights: { acceleration: 20, power: 20, handling: 60 },
  },
};

function MyTeamTab() {
  const careerId = useCareerStore((state) => state.careerId);
  const player = useCareerStore((state) => state.player);
  const playerTeam = useCareerStore((state) => state.playerTeam);
  const [drivers, setDrivers] = useState([]);
  const [error, setError] = useState("");

  useEffect(() => {
    let mounted = true;

    async function fetchCategoryDrivers() {
      if (!careerId || !playerTeam?.categoria) return;

      try {
        const loadedDrivers = await invoke("get_drivers_by_category", {
          careerId,
          category: playerTeam.categoria,
        });
        if (mounted) {
          setDrivers(loadedDrivers);
        }
      } catch (invokeError) {
        if (mounted) {
          setError(
            typeof invokeError === "string"
              ? invokeError
              : "Nao foi possivel carregar os pilotos da equipe.",
          );
        }
      }
    }

    fetchCategoryDrivers();
    return () => {
      mounted = false;
    };
  }, [careerId, playerTeam?.categoria]);

  const piloto1 = drivers.find((driver) => driver.id === playerTeam?.piloto_1_id);
  const piloto2 = drivers.find((driver) => driver.id === playerTeam?.piloto_2_id);

  return (
    <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
      <GlassCard hover={false} className="rounded-[28px]">
        <div className="flex items-center gap-3">
          <span
            className="h-4 w-4 rounded-full border border-white/15"
            style={{ backgroundColor: playerTeam?.cor_primaria ?? "#58a6ff" }}
          />
          <div>
            <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">
              Minha equipe
            </p>
            <h2 className="mt-2 text-3xl font-semibold text-text-primary">
              {playerTeam?.nome ?? "Equipe"}
            </h2>
          </div>
        </div>

        <div className="mt-6 grid gap-4 md:grid-cols-2">
          <DriverPanel
            label="N1"
            driver={piloto1}
            highlight={piloto1?.id === player?.id}
            fallbackName={playerTeam?.piloto_1_nome}
          />
          <DriverPanel
            label="N2"
            driver={piloto2}
            highlight={piloto2?.id === player?.id}
            fallbackName={playerTeam?.piloto_2_nome}
          />
        </div>

        {error ? (
          <div className="mt-4 rounded-2xl border border-status-red/30 bg-status-red/10 px-4 py-3 text-sm text-status-red">
            {error}
          </div>
        ) : null}
      </GlassCard>

      <GlassCard hover={false} className="rounded-[28px]">
        <p className="text-[11px] uppercase tracking-[0.22em] text-accent-primary">Infraestrutura</p>
        <h3 className="mt-2 text-2xl font-semibold text-text-primary">Base tecnica da equipe</h3>

        <BuildProfileCard profile={playerTeam?.car_build_profile} />

        <div className="mt-6 space-y-5">
          <MetricBar
            label="Performance do carro"
            value={normalizeCarPerformance(playerTeam?.car_performance ?? 0)}
            rawValue={`${Math.round((playerTeam?.car_performance ?? 0) * 10) / 10}`}
          />
          <MetricBar
            label="Confiabilidade"
            value={playerTeam?.confiabilidade ?? 0}
            rawValue={`${Math.round(playerTeam?.confiabilidade ?? 0)}`}
          />
          <MetricBar
            label="Budget"
            value={playerTeam?.budget ?? 0}
            rawValue={`${Math.round(playerTeam?.budget ?? 0)}`}
          />
        </div>
      </GlassCard>
    </div>
  );
}

function DriverPanel({ label, driver, highlight, fallbackName }) {
  const skill = Math.round(driver?.skill ?? 0);
  return (
    <div
      className={[
        "rounded-[24px] border p-5",
        highlight ? "border-accent-primary/35 bg-accent-primary/10" : "border-white/8 bg-white/[0.03]",
      ].join(" ")}
    >
      <p className="text-[11px] uppercase tracking-[0.2em] text-text-muted">{label}</p>
      <h4 className="mt-2 text-xl font-semibold text-text-primary">{driver?.nome ?? fallbackName ?? "-"}</h4>
      <p className="mt-2 text-sm text-text-secondary">{driver?.nacionalidade ?? "Piloto ainda sem dados detalhados"}</p>

      <div className="mt-5">
        <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.16em] text-text-muted">
          <span>Skill</span>
          <span>{skill}</span>
        </div>
        <div className="h-2 rounded-full bg-white/10">
          <div
            className="h-2 rounded-full bg-accent-primary transition-glass"
            style={{ width: `${Math.max(8, Math.min(100, skill))}%` }}
          />
        </div>
      </div>
    </div>
  );
}

function BuildProfileCard({ profile }) {
  const meta = getCarBuildProfileMeta(profile);

  return (
    <div className="mt-6 rounded-[24px] border border-white/8 bg-white/[0.03] p-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-text-muted">Construcao do carro</p>
          <h4 className="mt-2 text-lg font-semibold text-text-primary">{meta.label}</h4>
          <p className="mt-2 text-sm text-text-secondary">{meta.description}</p>
        </div>
        <span className="rounded-full border border-accent-primary/30 bg-accent-primary/10 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-accent-primary">
          {meta.costLabel}
        </span>
      </div>

      <div className="mt-5 space-y-3">
        <AttributeBar label="Aceleracao" value={meta.weights.acceleration} />
        <AttributeBar label="Potencia" value={meta.weights.power} />
        <AttributeBar label="Dirigibilidade" value={meta.weights.handling} />
      </div>
    </div>
  );
}

function MetricBar({ label, value, rawValue }) {
  const clamped = Math.max(0, Math.min(100, Math.round(value)));
  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-sm text-text-secondary">
        <span>{label}</span>
        <span className="font-mono text-text-primary">{rawValue}</span>
      </div>
      <div className="h-3 rounded-full bg-white/10">
        <div
          className="h-3 rounded-full bg-gradient-to-r from-accent-primary to-accent-hover transition-glass"
          style={{ width: `${Math.max(6, clamped)}%` }}
        />
      </div>
    </div>
  );
}

function AttributeBar({ label, value }) {
  const clamped = Math.max(0, Math.min(100, Math.round(value)));
  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-xs uppercase tracking-[0.16em] text-text-muted">
        <span>{label}</span>
        <span className="font-mono text-text-primary">{clamped}%</span>
      </div>
      <div className="h-2 rounded-full bg-white/10">
        <div
          className="h-2 rounded-full bg-white/70 transition-glass"
          style={{ width: `${Math.max(6, clamped)}%` }}
        />
      </div>
    </div>
  );
}

function getCarBuildProfileMeta(profile) {
  return CAR_BUILD_PROFILE_META[profile] ?? CAR_BUILD_PROFILE_META.balanced;
}

function normalizeCarPerformance(value) {
  const normalized = ((value + 5) / 21) * 100;
  return Math.max(0, Math.min(100, normalized));
}

export default MyTeamTab;
