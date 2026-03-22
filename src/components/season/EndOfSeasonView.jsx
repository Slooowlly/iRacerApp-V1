import { useMemo, useState } from "react";

import useCareerStore from "../../stores/useCareerStore";
import {
  extractFlag,
  extractNationalityLabel,
  formatAttributeName,
  formatCategoryName,
} from "../../utils/formatters";
import GlassButton from "../ui/GlassButton";
import GlassCard from "../ui/GlassCard";

function CollapsibleSection({ title, icon, count, children, defaultOpen = false }) {
  const [open, setOpen] = useState(defaultOpen);

  if (!count) return null;

  return (
    <GlassCard hover={false} className="rounded-[28px] p-5">
      <button
        type="button"
        onClick={() => setOpen((current) => !current)}
        className="flex w-full items-center justify-between gap-4 text-left"
      >
        <div className="flex items-center gap-3">
          <span className="text-lg">{icon}</span>
          <div>
            <h3 className="text-sm font-semibold uppercase tracking-[0.18em] text-text-primary">
              {title}
            </h3>
          </div>
          <span className="rounded-full border border-white/10 bg-white/[0.04] px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-text-secondary">
            {count}
          </span>
        </div>
        <span className="text-xs uppercase tracking-[0.18em] text-text-muted">
          {open ? "Ocultar" : "Ver"}
        </span>
      </button>

      {open ? <div className="mt-4 border-t border-white/8 pt-4">{children}</div> : null}
    </GlassCard>
  );
}

function EvolutionSection({ reports }) {
  const grew = useMemo(
    () => (reports || []).filter((report) => report.overall_delta > 0),
    [reports],
  );
  const declined = useMemo(
    () => (reports || []).filter((report) => report.overall_delta < 0),
    [reports],
  );

  return (
    <CollapsibleSection
      title="Evolucao de Pilotos"
      icon="📈"
      count={reports?.length || 0}
      defaultOpen
    >
      <div className="space-y-5">
        {grew.length ? (
          <div>
            <p className="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-status-green">
              Cresceram
            </p>
            <div className="space-y-2">
              {grew.slice(0, 12).map((report) => (
                <div
                  key={report.driver_id}
                  className="flex flex-col gap-1 rounded-2xl border border-status-green/15 bg-status-green/5 px-4 py-3 md:flex-row md:items-center md:justify-between"
                >
                  <span className="text-sm font-medium text-text-primary">{report.driver_name}</span>
                  <span className="text-xs text-status-green">
                    {report.changes
                      .filter((change) => change.delta > 0)
                      .map((change) => `${formatAttributeName(change.attribute)} +${change.delta}`)
                      .join(" • ")}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : null}

        {declined.length ? (
          <div>
            <p className="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-status-red">
              Declinaram
            </p>
            <div className="space-y-2">
              {declined.slice(0, 12).map((report) => (
                <div
                  key={report.driver_id}
                  className="flex flex-col gap-1 rounded-2xl border border-status-red/15 bg-status-red/5 px-4 py-3 md:flex-row md:items-center md:justify-between"
                >
                  <span className="text-sm font-medium text-text-primary">{report.driver_name}</span>
                  <span className="text-xs text-status-red">
                    {report.changes
                      .filter((change) => change.delta < 0)
                      .map((change) => `${formatAttributeName(change.attribute)} ${change.delta}`)
                      .join(" • ")}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </div>
    </CollapsibleSection>
  );
}

function RetirementSection({ retirements }) {
  return (
    <CollapsibleSection
      title="Aposentadorias"
      icon="👴"
      count={retirements?.length || 0}
      defaultOpen
    >
      <div className="space-y-2">
        {retirements?.map((retirement) => (
          <div
            key={retirement.driver_id}
            className="rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-3"
          >
            <p className="text-sm font-medium text-text-primary">
              {retirement.driver_name}
              <span className="ml-2 text-text-secondary">({retirement.age} anos)</span>
            </p>
            <p className="mt-1 text-xs text-text-secondary">{retirement.reason}</p>
          </div>
        ))}
      </div>
    </CollapsibleSection>
  );
}

function PromotionSection({ promotion }) {
  if (!promotion?.movements?.length) return null;

  return (
    <CollapsibleSection
      title="Promocoes e Rebaixamentos"
      icon="⬆️⬇️"
      count={promotion.movements.length}
      defaultOpen
    >
      <div className="space-y-3">
        {promotion.movements.map((movement, index) => {
          const effects = promotion.pilot_effects?.filter(
            (effect) => effect.team_id === movement.team_id,
          );
          const isPromotion = movement.movement_type === "Promocao";

          return (
            <div
              key={`${movement.team_id}-${index}`}
              className={[
                "rounded-[24px] border px-4 py-4",
                isPromotion
                  ? "border-status-green/20 bg-status-green/5"
                  : "border-status-red/20 bg-status-red/5",
              ].join(" ")}
            >
              <div className="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm">{isPromotion ? "⬆️" : "⬇️"}</span>
                  <p className="text-sm font-semibold text-text-primary">{movement.team_name}</p>
                </div>
                <p className="text-xs text-text-secondary">
                  {formatCategoryName(movement.from_category)} →{" "}
                  {formatCategoryName(movement.to_category)}
                </p>
              </div>
              <p className="mt-2 text-xs text-text-secondary">{movement.reason}</p>

              {effects?.length ? (
                <div className="mt-3 space-y-2 border-t border-white/8 pt-3">
                  {effects.map((effect) => (
                    <div key={effect.driver_id} className="flex items-start gap-2 text-xs">
                      <span className="mt-0.5">
                        {effect.effect === "MovesWithTeam"
                          ? "✅"
                          : effect.effect === "FreedNoLicense"
                            ? "❌"
                            : "🔄"}
                      </span>
                      <div>
                        <p className="text-text-primary">{effect.driver_name}</p>
                        <p className="text-text-secondary">{effect.reason}</p>
                      </div>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
    </CollapsibleSection>
  );
}

function RookiesSection({ rookies }) {
  return (
    <CollapsibleSection title="Novos Talentos" icon="🎓" count={rookies?.length || 0}>
      <div className="space-y-2">
        {rookies?.map((rookie) => (
          <div
            key={rookie.driver_id}
            className="flex items-center justify-between gap-3 rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-3"
          >
            <div className="flex items-center gap-3">
              <span className="text-lg">
                {rookie.tipo === "Genio" ? "🌟" : rookie.tipo === "Talento" ? "⭐" : "👤"}
              </span>
              <div>
                <p className="text-sm font-medium text-text-primary">{rookie.driver_name}</p>
                <p className="text-xs text-text-secondary">
                  {rookie.age} anos • {extractFlag(rookie.nationality)}{" "}
                  {extractNationalityLabel(rookie.nationality) || rookie.nationality}
                </p>
              </div>
            </div>
            <span className="rounded-full border border-white/10 bg-white/[0.04] px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-text-secondary">
              {rookie.tipo}
            </span>
          </div>
        ))}
      </div>
    </CollapsibleSection>
  );
}

function LicensesSection({ licenses }) {
  return (
    <CollapsibleSection title="Licencas" icon="📜" count={licenses?.length || 0}>
      <div className="space-y-2">
        {licenses?.map((license) => (
          <div
            key={`${license.driver_id}-${license.license_level}`}
            className="rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-3"
          >
            <p className="text-sm font-medium text-text-primary">{license.driver_name}</p>
            <p className="mt-1 text-xs text-text-secondary">
              Licenca {license.license_level} em {formatCategoryName(license.category)}
            </p>
          </div>
        ))}
      </div>
    </CollapsibleSection>
  );
}

function EndOfSeasonView() {
  const result = useCareerStore((state) => state.endOfSeasonResult);
  const enterPreseason = useCareerStore((state) => state.enterPreseason);

  if (!result) return null;

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-5 pb-8">
      <GlassCard hover={false} className="glass-strong rounded-[34px] p-8 lg:p-10">
        <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-[11px] uppercase tracking-[0.24em] text-accent-primary">
              Fim da temporada
            </p>
            <h1 className="mt-3 text-4xl font-semibold tracking-[-0.04em] text-text-primary">
              O paddock virou a pagina
            </h1>
            <p className="mt-3 max-w-2xl text-sm text-text-secondary">
              A temporada terminou, as equipes foram reorganizadas e a pre-temporada da nova fase
              esta pronta para comecar.
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="glass-light rounded-2xl px-4 py-4">
              <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
                Novo ano
              </p>
              <p className="mt-2 text-2xl font-semibold text-text-primary">{result.new_year}</p>
            </div>
            <div className="glass-light rounded-2xl px-4 py-4">
              <p className="text-[10px] uppercase tracking-[0.18em] text-text-muted">
                Pre-temporada
              </p>
              <p className="mt-2 text-2xl font-semibold text-text-primary">
                {result.preseason_total_weeks} semanas
              </p>
            </div>
          </div>
        </div>
      </GlassCard>

      <EvolutionSection reports={result.growth_reports} />
      <RetirementSection retirements={result.retirements} />
      <PromotionSection promotion={result.promotion_result} />
      <RookiesSection rookies={result.rookies_generated} />
      <LicensesSection licenses={result.licenses_earned} />

      <div className="flex justify-center pt-2">
        <GlassButton variant="primary" className="min-w-[260px]" onClick={() => void enterPreseason()}>
          📋 Iniciar pre-temporada
        </GlassButton>
      </div>
    </div>
  );
}

export default EndOfSeasonView;
