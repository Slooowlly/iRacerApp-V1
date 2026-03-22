const tabs = [
  { id: "standings", label: "Pilotos" },
  { id: "next-race", label: "Proxima Corrida" },
  { id: "my-team", label: "Minha Equipe" },
  { id: "calendar", label: "Calendario" },
];

function TabNavigation({ activeTab, onTabChange }) {
  return (
    <nav className="flex justify-center">
      {tabs.map((tab) => {
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onTabChange?.(tab.id)}
            className={[
              "px-5 py-4 text-sm font-semibold tracking-[0.06em] transition-glass border-b-2",
              isActive
                ? "border-accent-primary text-accent-primary"
                : "border-transparent text-text-secondary hover:text-text-primary hover:border-white/20",
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
