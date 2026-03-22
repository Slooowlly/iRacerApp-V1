const tabs = [
  { id: "standings", label: "Pilotos" },
  { id: "next-race", label: "Proxima Corrida" },
  { id: "my-team", label: "Minha Equipe" },
  { id: "calendar", label: "Calendario" },
];

function TabNavigation({ activeTab, onTabChange }) {
  return (
    <nav className="flex flex-wrap gap-2">
      {tabs.map((tab) => {
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onTabChange?.(tab.id)}
            className={[
              "rounded-2xl px-4 py-2 text-sm font-semibold tracking-[0.06em] transition-glass",
              isActive
                ? "glass-light border border-accent-primary/40 text-accent-primary glow-blue"
                : "border border-white/8 text-text-secondary hover:bg-white/6 hover:text-text-primary",
            ].join(" ")}
          >
            {tab.label}
          </button>
        );
      })}
    </nav>
  );
}

export default TabNavigation;
