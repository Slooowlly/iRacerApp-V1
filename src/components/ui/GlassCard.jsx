export function GlassCard({
  children,
  className = "",
  hover = true,
  selected = false,
  onClick,
  as: Component = "div",
}) {
  const interactive = Boolean(onClick) || hover;

  return (
    <Component
      onClick={onClick}
      className={[
        "glass rounded-3xl border p-6 shadow-[0_18px_50px_rgba(0,0,0,0.24)]",
        "transition-glass",
        interactive ? "card-hover cursor-pointer" : "",
        selected ? "border-accent-primary glow-blue bg-app-card-hover/80" : "border-white/10",
        className,
      ].join(" ")}
    >
      {children}
    </Component>
  );
}

export default GlassCard;
