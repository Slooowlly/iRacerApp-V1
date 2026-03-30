import { useEffect, useLayoutEffect, useRef, useState } from "react";

const SPECIAL_SCOPE_LABEL = "Rankings";
const SHARED_SCOPE_IDS = new Set(["production_challenger", "endurance"]);
const DRAWER_GAP = 8;
const FAMILY_CONFIG = [
  {
    id: "mazda",
    label: "Mazda",
    defaultScopeId: "mazda_rookie",
    items: [
      { type: "scope", scopeId: "mazda_rookie", label: "Rookie" },
      { type: "scope", scopeId: "mazda_amador", label: "Cup" },
      { type: "scope", scopeId: "production_challenger", scopeClass: "mazda", label: "Production" },
    ],
  },
  {
    id: "toyota",
    label: "Toyota",
    defaultScopeId: "toyota_rookie",
    items: [
      { type: "scope", scopeId: "toyota_rookie", label: "Rookie" },
      { type: "scope", scopeId: "toyota_amador", label: "Cup" },
      { type: "scope", scopeId: "production_challenger", scopeClass: "toyota", label: "Production" },
    ],
  },
  {
    id: "bmw",
    label: "BMW",
    defaultScopeId: "bmw_m2",
    items: [
      { type: "scope", scopeId: "bmw_m2", label: "Rookie" },
      { type: "scope", scopeId: "production_challenger", scopeClass: "bmw", label: "Production" },
    ],
  },
  {
    id: "gt4",
    label: "GT4",
    defaultScopeId: "gt4",
    items: [
      { type: "scope", scopeId: "gt4", label: "Rookie" },
      { type: "scope", scopeId: "endurance", scopeClass: "gt4", label: "Endurance" },
    ],
  },
  {
    id: "gt3",
    label: "GT3",
    defaultScopeId: "gt3",
    items: [
      { type: "scope", scopeId: "gt3", label: "Rookie" },
      { type: "scope", scopeId: "endurance", scopeClass: "gt3", label: "Endurance" },
    ],
  },
  {
    id: "lmp2",
    label: "LMP2",
    defaultScopeId: "endurance",
    defaultScopeClass: "lmp2",
    items: [
      { type: "label", label: "Rookie" },
      { type: "scope", scopeId: "endurance", scopeClass: "lmp2", label: "Endurance" },
    ],
  },
];

function NewsScopeDrawers({
  scopeTabs,
  requestState,
  showDrawer = true,
  onScopeChange,
}) {
  const families = buildFamilies(scopeTabs);
  const specialScope =
    scopeTabs.find((scope) => scope.scope_type === "famous") ??
    scopeTabs.find((scope) => scope.special);
  const scopeRowRef = useRef(null);
  const drawerPanelRef = useRef(null);
  const familyButtonRefs = useRef(new Map());
  const [openFamilyId, setOpenFamilyId] = useState(null);
  const [drawerLayout, setDrawerLayout] = useState({ left: 0, top: 0, reserve: 0 });

  useEffect(() => {
    if (families.length === 0) {
      setOpenFamilyId(null);
      return;
    }

    setOpenFamilyId((current) => {
      const inferred = inferUniqueFamilyId(
        requestState.scopeType,
        requestState.scopeId,
        requestState.scopeClass,
        families,
      );
      if (inferred) return inferred;
      if (current && families.some((family) => family.id === current)) return current;
      return families[0].id;
    });
  }, [families, requestState.scopeType, requestState.scopeId, requestState.scopeClass]);

  const openFamily = families.find((family) => family.id === openFamilyId) ?? families[0] ?? null;
  const shouldRenderFamilyDrawer =
    showDrawer && requestState.scopeType === "category" && Boolean(openFamily);

  function syncDrawerLayout() {
    function commitLayout(nextLayout) {
      setDrawerLayout((current) =>
        current.left === nextLayout.left
        && current.top === nextLayout.top
        && current.reserve === nextLayout.reserve
          ? current
          : nextLayout,
      );
    }

    if (!shouldRenderFamilyDrawer || !openFamily || !scopeRowRef.current) {
      commitLayout({ left: 0, top: 0, reserve: 0 });
      return;
    }

    const familyButton = familyButtonRefs.current.get(openFamily.id);
    if (!familyButton) {
      commitLayout({ left: 0, top: 0, reserve: 0 });
      return;
    }

    const rowRect = scopeRowRef.current.getBoundingClientRect();
    const buttonRect = familyButton.getBoundingClientRect();
    const panelRect = drawerPanelRef.current?.getBoundingClientRect?.() ?? null;
    const panelWidth = panelRect?.width ?? 0;
    const panelHeight = panelRect?.height ?? 0;
    const centeredLeft =
      buttonRect.left - rowRect.left + buttonRect.width / 2 - panelWidth / 2;
    const maxLeft = Math.max(0, rowRect.width - panelWidth);
    const left = Math.max(0, Math.min(Math.round(centeredLeft), Math.round(maxLeft)));
    const top = Math.round(buttonRect.bottom - rowRect.top + DRAWER_GAP);
    const reserve = Math.max(0, Math.round(top + panelHeight - rowRect.height));

    commitLayout({ left, top, reserve });
  }

  useLayoutEffect(() => {
    syncDrawerLayout();
  }, [openFamily, requestState.scopeClass, requestState.scopeId, requestState.scopeType, shouldRenderFamilyDrawer]);

  useEffect(() => {
    if (!shouldRenderFamilyDrawer) return undefined;

    window.addEventListener("resize", syncDrawerLayout);
    return () => {
      window.removeEventListener("resize", syncDrawerLayout);
    };
  }, [openFamily, shouldRenderFamilyDrawer]);

  function handleFamilyClick(family) {
    setOpenFamilyId(family.id);
    if (family.defaultSelection) {
      onScopeChange(family.defaultSelection);
    }
  }

  function handleFamilyItemClick(family, item) {
    setOpenFamilyId(family.id);
    if (item.type === "scope" && item.scope) {
      onScopeChange({
        scope: item.scope,
        scopeClass: item.scopeClass ?? null,
      });
    }
  }

  function setFamilyButtonRef(familyId, node) {
    if (node) {
      familyButtonRefs.current.set(familyId, node);
      return;
    }

    familyButtonRefs.current.delete(familyId);
  }

  return (
    <section data-news-section="scope-tabs" className="flex justify-center">
      <div
        className="relative w-full max-w-[1040px] space-y-2.5"
        style={shouldRenderFamilyDrawer && drawerLayout.reserve > 0 ? { paddingBottom: `${drawerLayout.reserve}px` } : undefined}
      >
        <div
          data-news-scope-top-pill
          className="mx-auto flex w-fit max-w-full rounded-full border border-white/8 bg-white/[0.03] p-1.5"
        >
          <div ref={scopeRowRef} data-news-scope-row className="flex flex-wrap justify-center gap-2">
            {families.map((family) => {
              const isOpen = requestState.scopeType === "category" && openFamily?.id === family.id;

              return (
                <button
                  key={family.id}
                  ref={(node) => setFamilyButtonRef(family.id, node)}
                  type="button"
                  onClick={() => handleFamilyClick(family)}
                  className={[
                    "min-h-[48px] min-w-[120px] flex-none rounded-full px-4 py-2 transition-glass",
                    "flex items-center justify-center text-center",
                    isOpen
                      ? "bg-[linear-gradient(180deg,rgba(4,10,20,0.92)_0%,rgba(9,15,29,0.88)_100%)] shadow-[inset_0_0_0_1px_rgba(88,166,255,0.2),0_0_18px_rgba(88,166,255,0.06)]"
                      : "hover:bg-white/[0.04]",
                  ].join(" ")}
                  aria-label={family.label}
                >
                  <span
                    className={[
                      "text-[1rem] font-semibold tracking-[-0.03em]",
                      isOpen ? "text-text-primary" : "text-text-secondary",
                    ].join(" ")}
                  >
                    {family.label}
                  </span>
                </button>
              );
            })}

            {specialScope ? (
              <button
                type="button"
                onClick={() => {
                  onScopeChange(specialScope);
                }}
                className={[
                  "min-h-[48px] min-w-[120px] flex-none rounded-full px-4 py-2 transition-glass",
                  "flex items-center justify-center text-center",
                  requestState.scopeType === specialScope.scope_type &&
                  requestState.scopeId === specialScope.id
                    ? "bg-[linear-gradient(180deg,rgba(22,18,12,0.94)_0%,rgba(26,22,14,0.88)_100%)] shadow-[inset_0_0_0_1px_rgba(255,212,122,0.24),0_0_18px_rgba(255,212,122,0.06)]"
                    : "hover:bg-white/[0.04]",
                ].join(" ")}
                aria-label={SPECIAL_SCOPE_LABEL}
              >
                <span
                  className={[
                    "text-[1rem] font-semibold tracking-[-0.03em]",
                    requestState.scopeType === specialScope.scope_type &&
                    requestState.scopeId === specialScope.id
                      ? "text-text-primary"
                      : "text-text-secondary",
                  ].join(" ")}
                >
                  {SPECIAL_SCOPE_LABEL}
                </span>
              </button>
            ) : null}
          </div>
        </div>

        {shouldRenderFamilyDrawer ? (
          <div
            data-news-scope-drawer-track
            className="absolute overflow-visible"
            style={{
              left: `${drawerLayout.left}px`,
              top: `${drawerLayout.top}px`,
            }}
          >
            <div
              key={openFamily.id}
              ref={drawerPanelRef}
              data-news-scope-drawer-panel
              className="animate-news-scope-drawer-in w-fit max-w-full px-0 py-0"
            >
              <div
                data-news-scope-pill
                className="inline-flex flex-wrap items-center gap-1.5 rounded-full border border-white/8 bg-white/[0.03] p-1.5"
              >
                {openFamily.items.map((item, index) => (
                  <div key={`${openFamily.id}-${item.label}-${index}`} className="flex items-center gap-1.5">
                    {item.type === "scope" ? (
                      <button
                        type="button"
                        onClick={() => handleFamilyItemClick(openFamily, item)}
                        className={[
                          "rounded-full px-3.5 py-2 text-sm font-semibold transition-glass",
                          isFamilyItemActive(item, requestState)
                            ? "bg-accent-primary/12 text-text-primary shadow-[inset_0_0_0_1px_rgba(88,166,255,0.16)]"
                            : "text-text-secondary hover:bg-white/[0.04] hover:text-text-primary",
                        ].join(" ")}
                      >
                        {item.label}
                      </button>
                    ) : (
                      <span className="rounded-full px-3.5 py-2 text-sm font-semibold text-text-secondary">
                        {item.label}
                      </span>
                    )}

                    {index < openFamily.items.length - 1 ? (
                      <span className="inline-flex h-[18px] w-[18px] items-center justify-center rounded-full bg-white/[0.03] text-[10px] leading-none text-text-muted">
                        {">"}
                      </span>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}

function buildFamilies(scopeTabs) {
  const scopeMap = new Map(
    scopeTabs
      .filter((scope) => scope.scope_type === "category")
      .map((scope) => [scope.id, scope]),
  );

  return FAMILY_CONFIG.map((family) => {
    const items = family.items
      .map((item) => {
        if (item.type !== "scope") return item;
        const scope = scopeMap.get(item.scopeId);
        if (!scope) return null;
        return { ...item, scope };
      })
      .filter(Boolean);

    const defaultScope = scopeMap.get(family.defaultScopeId) ?? null;
    if (!defaultScope || items.length === 0) return null;

    return {
      ...family,
      defaultScope,
      defaultSelection: {
        scope: defaultScope,
        scopeClass: family.defaultScopeClass ?? null,
      },
      items,
      memberScopeIds: items.filter((item) => item.type === "scope").map((item) => item.scope.id),
    };
  }).filter(Boolean);
}

function inferUniqueFamilyId(scopeType, scopeId, scopeClass, families) {
  if (scopeType !== "category" || !scopeId) return null;
  if (!SHARED_SCOPE_IDS.has(scopeId)) {
    return families.find((family) => family.memberScopeIds.includes(scopeId))?.id ?? null;
  }

  return families.find((family) =>
    family.items.some(
      (item) =>
        item.type === "scope"
        && item.scope.id === scopeId
        && (item.scopeClass ?? null) === (scopeClass ?? null),
    ),
  )?.id ?? null;
}

function isFamilyItemActive(item, requestState) {
  if (item.type !== "scope") return false;
  if (requestState.scopeType !== "category" || requestState.scopeId !== item.scope.id) return false;
  if (!SHARED_SCOPE_IDS.has(item.scope.id)) return true;
  return requestState.scopeClass === (item.scopeClass ?? null);
}

export default NewsScopeDrawers;
