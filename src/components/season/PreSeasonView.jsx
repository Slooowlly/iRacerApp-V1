import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import useCareerStore from "../../stores/useCareerStore";
import { formatSalary } from "../../utils/formatters";

// ─── Category Definitions ────────────────────────────────────────────────────
const CATEGORIES = [
  { id: "all",        label: "Todas",      color: "rgba(255,255,255,0.3)" },
  { id: "mazda",      dbIds: ["production_challenger","mazda_rookie","mazda_amador"],   label: "Mazda",      color: "#C8102E", filterClass: "mazda" },
  { id: "toyota",     dbIds: ["production_challenger","toyota_rookie","toyota_amador"], label: "Toyota",     color: "#EB0A1E", filterClass: "toyota" },
  { id: "bmw",        dbIds: ["production_challenger","bmw_m2"],                        label: "BMW M2",     color: "#6B4FBB", filterClass: "bmw" },
  { id: "production", dbIds: ["production_challenger"],                                 label: "Production", color: "#1DB954" },
  { id: "sep1", isSeparator: true },
  { id: "gt4",       dbIds: ["gt4"],       label: "GT4",      color: "#FF8000" },
  { id: "gt3",       dbIds: ["gt3"],       label: "GT3",      color: "#E73F47" },
  { id: "endurance", dbIds: ["endurance"], label: "Endurance", color: "#3671C6" },
];

const SUBCAT_LABELS = {
  mazda: "Mazda Cup · Principal", toyota: "Toyota Cup · Principal", bmw: "BMW M2 Cup · Principal",
  mazda_rookie: "Mazda Rookie Series", mazda_amador: "Mazda Amador",
  toyota_rookie: "Toyota Rookie Series", toyota_amador: "Toyota Amador",
  bmw_m2: "BMW M2 Cup", production_challenger: "Production Challenger",
  gt3: "GT3 Championship", gt4: "GT4 Championship", endurance: "Endurance Championship",
};
const CLASS_PRIORITY = ["mazda","toyota","bmw","gt3","gt4","endurance"];

// Vibrant accent color for each subcategory separator glow
const SUBCAT_COLORS = {
  mazda:                "#C8102E",
  mazda_rookie:         "#C8102E",
  mazda_amador:         "#C8102E",
  toyota:               "#EB0A1E",
  toyota_rookie:        "#EB0A1E",
  toyota_amador:        "#EB0A1E",
  bmw:                  "#6B82FF",
  bmw_m2:               "#6B82FF",
  production_challenger:"#1DB954",
  gt4:                  "#FF8000",
  gt3:                  "#E73F47",
  endurance:            "#3671C6",
};
function subcatLabel(key) {
  return SUBCAT_LABELS[key] || key.split("_").map(w => w[0].toUpperCase() + w.slice(1)).join(" ");
}
function subcatColor(key) {
  return SUBCAT_COLORS[key] || "#3B82F6";
}

// ─── Event helpers ────────────────────────────────────────────────────────────
const EVENT_ICONS = {
  ContractExpired:"📋", ContractRenewed:"✍️", TransferCompleted:"📝",
  TransferRejected:"✖", RookieSigned:"🎓", PlayerProposalReceived:"💼",
  HierarchyUpdated:"⚡", PreSeasonComplete:"🏁",
};
const EVENT_COLORS = {
  ContractRenewed:"#34D399", TransferCompleted:"#60A5FA", TransferRejected:"#F87171",
  RookieSigned:"#A78BFA", PlayerProposalReceived:"#60A5FA", PreSeasonComplete:"#34D399",
};
function evIcon(t) { return EVENT_ICONS[t] || "📰"; }
function evColor(t) { return EVENT_COLORS[t] || "#94A3B8"; }

// ─── Rank badge ───────────────────────────────────────────────────────────────
function getRank(pos) {
  if (pos === 1) return { text:"1º", grad:"linear-gradient(135deg,#F59E0B,#FCD34D)", color:"#1a1000", glow:"rgba(245,158,11,0.5)", label:"Campeã" };
  if (pos === 2) return { text:"2º", grad:"linear-gradient(135deg,#9CA3AF,#E5E7EB)", color:"#1a1a1a", glow:null,                    label:"Vice-Campeã" };
  if (pos === 3) return { text:"3º", grad:"linear-gradient(135deg,#CD7F32,#E5A85A)", color:"#1a0d00", glow:null,                    label:"3º Lugar" };
  if (pos  > 0) return { text:`${pos}º`, grad:"rgba(255,255,255,0.12)", color:"#fff", glow:null, label:null };
  return null;
}

// ─── Grid Slot ────────────────────────────────────────────────────────────────
function GridSlot({ driverName, role, color }) {
  if (!driverName) {
    return (
      <div style={{
        display:"flex", alignItems:"center", gap:8, padding:"8px 10px",
        borderRadius:10, background:"rgba(255,255,255,0.015)",
        border:"1px dashed rgba(255,255,255,0.1)",
      }}>
        <div style={{ width:3, height:24, borderRadius:2, flexShrink:0, background:"rgba(255,255,255,0.08)" }} />
        <p style={{ flex:1, fontSize:12, color:"rgba(255,255,255,0.2)", fontStyle:"italic" }}>Vaga Aberta</p>
        <span style={{ fontSize:10, fontWeight:700, color:"#334155" }}>{role}</span>
      </div>
    );
  }
  return (
    <div style={{
      display:"flex", alignItems:"center", gap:8, padding:"8px 10px",
      borderRadius:10, background:"rgba(52,211,153,0.03)",
      borderTop:"1px solid rgba(255,255,255,0.04)",
      borderRight:"1px solid rgba(255,255,255,0.04)",
      borderBottom:"1px solid rgba(255,255,255,0.04)",
      borderLeft:`2px solid ${color || "#3B82F6"}`,
    }}>
      <div style={{ width:3, height:24, borderRadius:2, flexShrink:0, background: color || "#3B82F6", boxShadow:`0 0 6px ${color || "#3B82F6"}80` }} />
      <div style={{ flex:1, minWidth:0 }}>
        <p style={{ fontSize:12, fontWeight:600, color:"#e2e8f0", letterSpacing:"-0.01em", overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }}>{driverName}</p>
        <p style={{ fontSize:10, color:"#475569" }}>Confirmado</p>
      </div>
      <span style={{ fontSize:10, fontWeight:700, color:"#475569" }}>{role}</span>
    </div>
  );
}

// ─── Main Component ───────────────────────────────────────────────────────────
export default function PreSeasonView() {
  const preseasonState      = useCareerStore(s => s.preseasonState);
  const preseasonWeeks      = useCareerStore(s => s.preseasonWeeks);
  const playerProposals     = useCareerStore(s => s.playerProposals);
  const isAdvancingWeek     = useCareerStore(s => s.isAdvancingWeek);
  const isRespondingProposal= useCareerStore(s => s.isRespondingProposal);
  const advanceMarketWeek   = useCareerStore(s => s.advanceMarketWeek);
  const respondToProposal   = useCareerStore(s => s.respondToProposal);
  const finalizePreseason   = useCareerStore(s => s.finalizePreseason);
  const careerId            = useCareerStore(s => s.careerId);

  const [selectedCat, setSelectedCat] = useState("all");
  const [gridData, setGridData]       = useState([]);
  const [loadingGrid, setLoadingGrid] = useState(false);

  // ── Fetch grid ──────────────────────────────────────────────────────────────
  useEffect(() => {
    if (!careerId) return;
    async function fetchGrid() {
      setLoadingGrid(true);
      try {
        const dbIds = new Set();
        if (selectedCat === "all") {
          CATEGORIES.filter(c => !c.isSeparator && c.id !== "all").forEach(c => c.dbIds.forEach(id => dbIds.add(id)));
        } else {
          const cfg = CATEGORIES.find(c => c.id === selectedCat);
          if (cfg) cfg.dbIds.forEach(id => dbIds.add(id));
        }

        const all = [];
        for (const dbId of dbIds) {
          try { all.push(...await invoke("get_teams_standings", { careerId, category: dbId })); }
          catch { /* category may not exist */ }
        }

        let final = all;
        if (selectedCat !== "all") {
          const cfg = CATEGORIES.find(c => c.id === selectedCat);
          if (cfg?.filterClass) {
            final = all.filter(t => {
              if (t.classe === cfg.filterClass) return true;
              if (t.categoria?.startsWith(cfg.filterClass)) return true;
              if (cfg.filterClass === "bmw" && t.categoria === "bmw_m2") return true;
              return false;
            });
          }
        }
        setGridData(final);
      } finally { setLoadingGrid(false); }
    }
    fetchGrid();
  }, [careerId, selectedCat, preseasonState?.current_week]);

  // ── Actions ─────────────────────────────────────────────────────────────────
  const handleAdvance = async () => {
    if (isAdvancingWeek) return;
    if (preseasonState?.is_complete) {
      if (playerProposals.length > 0) return;
      try { await finalizePreseason(); } catch(e) { console.error(e); }
    } else {
      try { await advanceMarketWeek(); } catch(e) { console.error(e); }
    }
  };
  const handleProposal = async (id, accept) => {
    try { await respondToProposal(id, accept); } catch(e) { console.error(e); }
  };

  // ── Derived ─────────────────────────────────────────────────────────────────
  const isComplete   = preseasonState?.is_complete;
  const currentWeek  = Math.min(preseasonState?.current_week || 1, preseasonState?.total_weeks || 1);
  const totalWeeks   = preseasonState?.total_weeks || 1;
  const allEvents    = [...preseasonWeeks].reverse();
  const hasNews      = allEvents.some(w => w.events?.length > 0);

  const teamsBySubcat = gridData.reduce((acc, t) => {
    const key = t.classe || t.categoria || "Outras";
    (acc[key] = acc[key] || []).push(t);
    return acc;
  }, {});
  const sortedClasses = Object.keys(teamsBySubcat).sort((a, b) => {
    const pa = CLASS_PRIORITY.indexOf(a), pb = CLASS_PRIORITY.indexOf(b);
    if (pa !== -1 && pb !== -1) return pa - pb;
    if (pa !== -1) return -1; if (pb !== -1) return 1;
    return a.localeCompare(b);
  });

  // ── Shared glass styles ──────────────────────────────────────────────────────
  const glass = {
    background: "rgba(28,28,30,0.80)",
    backdropFilter: "blur(20px) saturate(120%)",
    WebkitBackdropFilter: "blur(20px) saturate(120%)",
    border: "1px solid rgba(255,255,255,0.08)",
  };
  const glassGlow = {
    ...glass,
    border: "1px solid rgba(59,130,246,0.30)",
    boxShadow: "0 0 0 1px rgba(59,130,246,0.08), inset 0 1px 0 rgba(255,255,255,0.06)",
  };
  const innerCard = {
    background: "rgba(255,255,255,0.04)",
    border: "1px solid rgba(255,255,255,0.07)",
    borderRadius: 12,
    padding: "12px 14px",
  };
  const bgAtmosphere = {
    background: "#0E0E10",
    backgroundImage: `
      radial-gradient(ellipse 70% 50% at 10% 60%, rgba(30,64,175,0.10) 0%, transparent 60%),
      radial-gradient(ellipse 50% 60% at 90% 20%, rgba(59,130,246,0.07) 0%, transparent 55%)
    `,
  };

  // ── Render ───────────────────────────────────────────────────────────────────
  return (
    <div style={{ ...bgAtmosphere, display:"flex", flexDirection:"column", height:"100vh", overflow:"hidden", color:"#ffffff" }}>

      {/* ══ HEADER ══ */}
      <header style={{ ...glass, display:"flex", alignItems:"center", justifyContent:"space-between", gap:24, padding:"14px 32px", borderTop:"none", borderLeft:"none", borderRight:"none", flexShrink:0, zIndex:10 }}>
        {/* Identity + week */}
        <div style={{ display:"flex", alignItems:"center", gap:20, flexShrink:0 }}>
          <div>
            <p style={{ fontSize:9, textTransform:"uppercase", letterSpacing:"0.22em", color:"#60A5FA", fontWeight:700, marginBottom:2 }}>Pré-Temporada</p>
            <h1 style={{ fontSize:20, fontWeight:700, letterSpacing:"-0.02em", color:"#ffffff", margin:0 }}>Mercado de Transferências</h1>
          </div>
          <div style={{ ...glass, borderRadius:12, padding:"10px 16px", display:"flex", alignItems:"center", gap:16 }}>
            <div>
              <p style={{ fontSize:9, textTransform:"uppercase", letterSpacing:"0.12em", color:"#475569", marginBottom:2 }}>Semana</p>
              <p style={{ fontSize:18, fontWeight:700, letterSpacing:"-0.02em", lineHeight:1, margin:0 }}>
                {currentWeek} <span style={{ fontSize:12, fontWeight:400, color:"#475569" }}>/ {totalWeeks}</span>
              </p>
            </div>
          </div>
        </div>

        {/* Category filter pills */}
        <div style={{ ...glass, borderRadius:9999, padding:"6px 8px", display:"flex", alignItems:"center", gap:4, overflow:"hidden", flexShrink:1, minWidth:0 }}>
          {CATEGORIES.map((cat, i) => {
            if (cat.isSeparator) return <div key={i} style={{ width:1, height:18, background:"rgba(255,255,255,0.07)", margin:"0 4px" }} />;
            const active = selectedCat === cat.id;
            return (
              <button key={cat.id} onClick={() => setSelectedCat(cat.id)} style={{
                display:"flex", alignItems:"center", gap:6, padding:"6px 14px", borderRadius:9999, fontSize:11, fontWeight:600, cursor:"pointer", fontFamily:"inherit",
                background: active ? "rgba(59,130,246,0.2)" : "transparent",
                border: active ? "1px solid rgba(59,130,246,0.4)" : "1px solid transparent",
                color: active ? "#60A5FA" : "#64748B",
                boxShadow: active ? "0 0 12px rgba(59,130,246,0.15)" : "none",
                transition:"all 0.15s",
              }}>
                <span style={{ width:6, height:6, borderRadius:"50%", background: cat.color, flexShrink:0 }} />
                {cat.label}
              </button>
            );
          })}
        </div>

        {/* Advance button */}
        <button onClick={handleAdvance}
          disabled={isAdvancingWeek || (isComplete && playerProposals.length > 0)}
          style={{
            flexShrink:0,
            position:"relative", zIndex:20,
            padding:"10px 24px", borderRadius:9999, fontSize:13, fontWeight:700, fontFamily:"inherit",
            background: isComplete ? "#22C55E" : "#3B82F6",
            color:"#fff",
            border:"none",
            cursor: (isAdvancingWeek || (isComplete && playerProposals.length > 0)) ? "not-allowed" : "pointer",
            boxShadow: isComplete ? "0 0 20px rgba(34,197,94,0.3)" : "0 0 0 1px rgba(59,130,246,0.4), 0 0 20px rgba(59,130,246,0.25)",
            opacity: (isAdvancingWeek || (isComplete && playerProposals.length > 0)) ? 0.5 : 1,
            transition:"all 0.2s",
            pointerEvents:"all",
          }}>
          {isAdvancingWeek ? "⏳ Processando..." : isComplete ? "🚀 Iniciar Temporada" : "⏭ Avançar Semana"}
        </button>
      </header>

      {/* ══ MAIN 3 COLUMNS ══ */}
      <div style={{ display:"flex", flex:1, overflow:"hidden" }}>

        {/* ── ESQUERDA: Feed de Notícias ── */}
        <aside style={{ width:300, flexShrink:0, borderRight:"1px solid rgba(255,255,255,0.06)", overflowY:"auto", padding:20, background:"rgba(14,14,16,0.6)" }}>
          <p style={{ fontSize:10, textTransform:"uppercase", letterSpacing:"0.2em", color:"#3B82F6", fontWeight:700, marginBottom:16 }}>📰 Feed do Paddock</p>
          
          {!hasNews ? (
            <div style={{ padding:"40px 0", textAlign:"center" }}>
              <p style={{ fontSize:12, color:"#334155" }}>Sem movimentações relatadas.</p>
            </div>
          ) : allEvents.map(week => {
            if (!week.events?.length) return null;
            return (
              <div key={week.week_number} style={{ marginBottom:20 }}>
                <p style={{ fontSize:9, fontWeight:700, color:"#334155", textTransform:"uppercase", letterSpacing:"0.15em", paddingLeft:4, marginBottom:10 }}>Semana {week.week_number}</p>
                <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
                  {week.events.map((evt, i) => (
                    <div key={i} style={{ ...innerCard, borderLeft:`2px solid ${evColor(evt.event_type)}` }}>
                      <p style={{ fontSize:10, fontWeight:700, color: evColor(evt.event_type), marginBottom:4 }}>{evIcon(evt.event_type)} {evt.headline}</p>
                      <p style={{ fontSize:11, color:"#94A3B8", lineHeight:1.5 }}>{evt.description}</p>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </aside>

        {/* ── CENTRO: Grid ── */}
        <main style={{ flex:1, overflowY:"auto", padding:24, background:"rgba(14,14,16,0.3)" }}>
          <div style={{ display:"flex", alignItems:"center", justifyContent:"space-between", marginBottom:20 }}>
            <p style={{ fontSize:10, textTransform:"uppercase", letterSpacing:"0.2em", color:"#475569", fontWeight:600 }}>Mapeamento das Equipes · Classificação Anterior</p>
            <div style={{ display:"flex", gap:16, fontSize:10, color:"#475569" }}>
              <span style={{ display:"flex", alignItems:"center", gap:6 }}>
                <span style={{ width:8, height:8, borderRadius:3, background:"rgba(52,211,153,0.25)", border:"1px solid rgba(52,211,153,0.5)", display:"inline-block" }} />
                Renovado
              </span>
              <span style={{ display:"flex", alignItems:"center", gap:6 }}>
                <span style={{ width:8, height:8, borderRadius:3, border:"1px dashed rgba(255,255,255,0.2)", display:"inline-block" }} />
                Vaga Aberta
              </span>
            </div>
          </div>

          {loadingGrid ? (
            <div style={{ padding:"80px 0", textAlign:"center", color:"#334155", fontSize:13 }}>Carregando grid...</div>
          ) : gridData.length === 0 ? (
            <div style={{ padding:"40px 0", textAlign:"center", color:"#334155", fontSize:13 }}>Nenhuma equipe encontrada para esta categoria.</div>
          ) : (
            <div style={{ display:"flex", flexDirection:"column", gap:32 }}>
              {sortedClasses.map(cls => {
                const teams = [...teamsBySubcat[cls]].sort((a, b) => (a.temp_posicao || 999) - (b.temp_posicao || 999));
                const lineCol = teams[0]?.cor_primaria || "#3B82F6";
                const accentCol = subcatColor(cls); // always vivid color for glow
                return (
                  <div key={cls}>
                    {/* Section separator */}
                    <div style={{ display:"flex", alignItems:"center", gap:12, marginBottom:16 }}>
                      <span style={{ fontSize:11, fontWeight:700, textTransform:"uppercase", letterSpacing:"0.12em", color: accentCol, textShadow:`0 0 14px ${accentCol}, 0 0 24px ${accentCol}60`, whiteSpace:"nowrap" }}>
                        {subcatLabel(cls)}
                      </span>
                      <div style={{ flex:1, height:1, background:`linear-gradient(to right, ${accentCol}40, transparent)` }} />
                    </div>

                    {/* Teams grid */}
                    <div style={{ display:"grid", gridTemplateColumns:"repeat(auto-fill, minmax(280px,1fr))", gap:16 }}>
                      {teams.map(team => {
                        const rank = getRank(team.temp_posicao);
                        const abbr = (team.nome_curto || team.nome).substring(0,2).toUpperCase();
                        return (
                          <div key={team.id} style={{
                            ...glass,
                            borderRadius:16, padding:16, position:"relative", overflow:"hidden",
                            transition:"border-color 0.2s",
                          }}
                          onMouseEnter={e => e.currentTarget.style.borderColor = `${team.cor_primaria || lineCol}50`}
                          onMouseLeave={e => e.currentTarget.style.borderColor = "rgba(255,255,255,0.07)"}
                          >
                            {/* Top glow line */}
                            <div style={{ position:"absolute", top:0, left:0, right:0, height:1, background:`linear-gradient(to right, transparent, ${team.cor_primaria || lineCol}60, transparent)` }} />

                            {/* Header */}
                            <div style={{ display:"flex", justifyContent:"space-between", alignItems:"flex-start", marginBottom:14 }}>
                              <div style={{ display:"flex", alignItems:"center", gap:12 }}>
                                <div style={{ position:"relative" }}>
                                  <div style={{
                                    width:34, height:34, borderRadius:10,
                                    background:`${team.cor_primaria || lineCol}18`,
                                    border:`1px solid ${team.cor_primaria || lineCol}35`,
                                    display:"flex", alignItems:"center", justifyContent:"center",
                                    fontWeight:700, fontSize:12, color: team.cor_primaria || lineCol,
                                    boxShadow:`0 0 10px ${team.cor_primaria || lineCol}20`,
                                  }}>{abbr}</div>
                                  {rank && (
                                    <span style={{
                                      position:"absolute", top:-8, right:-8,
                                      background: rank.grad, color: rank.color,
                                      fontWeight:800, fontSize:9, padding:"2px 6px", borderRadius:4,
                                      boxShadow: rank.glow ? `0 0 8px ${rank.glow}` : undefined,
                                    }}>{rank.text}</span>
                                  )}
                                </div>
                                <div>
                                  <p style={{ fontWeight:700, fontSize:13, letterSpacing:"-0.02em", color:"#ffffff", margin:0, maxWidth:160, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }} title={team.nome}>{team.nome}</p>
                                  {rank?.label && <p style={{ fontSize:9, fontWeight:600, textTransform:"uppercase", letterSpacing:"0.1em", color: rank.text === "1º" ? "#F59E0B" : "#64748B", marginTop:2, textShadow: rank.text === "1º" ? "0 0 8px rgba(245,158,11,0.4)" : undefined }}>{rank.label}</p>}
                                </div>
                              </div>
                            </div>

                            {/* Slots */}
                            <div style={{ display:"flex", flexDirection:"column", gap:6 }}>
                              <GridSlot driverName={team.piloto_1_nome} role="N1" color={team.cor_primaria} />
                              <GridSlot driverName={team.piloto_2_nome} role="N2" color={team.cor_primaria} />
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </main>

        {/* ── DIREITA: Decisões Pendentes ── */}
        <aside style={{ width:300, flexShrink:0, borderLeft:"1px solid rgba(255,255,255,0.06)", overflowY:"auto", padding:20, background:"rgba(14,14,16,0.6)" }}>
          {/* Header com ping */}
          <div style={{ display:"flex", alignItems:"center", gap:8, marginBottom:20 }}>
            <span style={{ position:"relative", display:"inline-flex", width:10, height:10 }}>
              {playerProposals.length > 0 && (
                <span style={{ position:"absolute", display:"inline-flex", width:"100%", height:"100%", borderRadius:"50%", background:"#3B82F6", opacity:0.5, animation:"ping 2s ease-in-out infinite" }} />
              )}
              <span style={{ position:"relative", display:"inline-flex", borderRadius:"50%", width:10, height:10, background:"#3B82F6", boxShadow:"0 0 8px rgba(59,130,246,0.8)" }} />
            </span>
            <p style={{ fontSize:10, textTransform:"uppercase", letterSpacing:"0.2em", color:"#60A5FA", fontWeight:700 }}>Decisões Pendentes</p>
          </div>

          <div style={{ display:"flex", flexDirection:"column", gap:12 }}>
            {playerProposals.length === 0 ? (
              <div style={{ padding:"40px 16px", textAlign:"center", border:"1px dashed rgba(255,255,255,0.08)", borderRadius:16 }}>
                <p style={{ fontSize:12, color:"#334155" }}>Nenhuma proposta pendente.</p>
              </div>
            ) : playerProposals.map(prop => (
              <div key={prop.proposal_id} style={{ ...glassGlow, borderRadius:18, padding:16, position:"relative", overflow:"hidden" }}>
                {/* Top glow */}
                <div style={{ position:"absolute", top:0, left:0, right:0, height:1, background:"linear-gradient(to right, transparent, rgba(59,130,246,0.5), transparent)" }} />
                {/* Role badge */}
                <div style={{
                  position:"absolute", top:0, right:0, padding:"4px 12px",
                  borderBottomLeftRadius:12, borderBottom:`1px solid ${prop.equipe_cor_primaria || "#3B82F6"}40`,
                  borderLeft:`1px solid ${prop.equipe_cor_primaria || "#3B82F6"}40`,
                  background:`${prop.equipe_cor_primaria || "#3B82F6"}18`,
                  color: prop.equipe_cor_primaria || "#60A5FA", fontSize:10, fontWeight:700,
                }}>
                  {prop.papel} · {prop.categoria_nome || prop.categoria}
                </div>

                {/* Team */}
                <div style={{ display:"flex", alignItems:"center", gap:12, marginBottom:16, marginTop:8 }}>
                  <div style={{
                    width:42, height:42, borderRadius:12, flexShrink:0,
                    background:`${prop.equipe_cor_primaria || "#3B82F6"}20`,
                    border:`1px solid ${prop.equipe_cor_primaria || "#3B82F6"}40`,
                    display:"flex", alignItems:"center", justifyContent:"center",
                    fontWeight:700, fontSize:14, color: prop.equipe_cor_primaria || "#60A5FA",
                    boxShadow:`0 0 12px ${prop.equipe_cor_primaria || "#3B82F6"}25`,
                  }}>{prop.equipe_nome?.substring(0,2).toUpperCase()}</div>
                  <div>
                    <p style={{ fontSize:14, fontWeight:700, letterSpacing:"-0.02em", color:"#ffffff", margin:0 }}>{prop.equipe_nome}</p>
                    <p style={{ fontSize:10, fontWeight:600, color:"#60A5FA", marginTop:2 }}>{prop.papel === "N1" ? "Piloto Principal" : "Segundo Piloto"}</p>
                  </div>
                </div>

                {/* Details grid */}
                <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:8, marginBottom:16 }}>
                  {[
                    { label:"Salário",     value: formatSalary(prop.salario_oferecido), color:"#34D399" },
                    { label:"Duração",     value: `${prop.duracao_anos} ano${prop.duracao_anos > 1 ? "s" : ""}` },
                    { label:"Companheiro", value: prop.companheiro_nome || "Vaga em aberto" },
                  ].map(d => (
                    <div key={d.label} style={{ background:"rgba(0,0,0,0.3)", border:"1px solid rgba(255,255,255,0.05)", borderRadius:10, padding:"10px 12px" }}>
                      <p style={{ fontSize:9, color:"#334155", textTransform:"uppercase", letterSpacing:"0.1em", marginBottom:3 }}>{d.label}</p>
                      <p style={{ fontSize:12, fontWeight:700, color: d.color || "#ffffff", margin:0 }}>{d.value}</p>
                    </div>
                  ))}
                  <div style={{ background:"rgba(0,0,0,0.3)", border:"1px solid rgba(255,255,255,0.05)", borderRadius:10, padding:"10px 12px" }}>
                    <p style={{ fontSize:9, color:"#334155", textTransform:"uppercase", letterSpacing:"0.1em", marginBottom:6 }}>Carro</p>
                    <div style={{ display:"flex", alignItems:"center", gap:6 }}>
                      <div style={{ flex:1, height:4, background:"rgba(255,255,255,0.08)", borderRadius:99, overflow:"hidden" }}>
                        <div style={{ width:`${prop.car_performance_rating || 0}%`, height:"100%", background: prop.equipe_cor_primaria || "#3B82F6", boxShadow:`0 0 6px ${prop.equipe_cor_primaria || "#3B82F6"}80`, borderRadius:99 }} />
                      </div>
                      <p style={{ fontSize:10, fontWeight:700, color:"#ffffff", margin:0 }}>{prop.car_performance_rating}</p>
                    </div>
                  </div>
                </div>

                {/* Actions */}
                <div style={{ display:"flex", gap:8 }}>
                  <button onClick={() => handleProposal(prop.proposal_id, true)} disabled={isRespondingProposal}
                    style={{ flex:1, padding:"9px 0", fontSize:12, fontWeight:700, fontFamily:"inherit", cursor:"pointer", borderRadius:12, border:"none",
                      background:"#3B82F6", color:"#fff",
                      boxShadow:"0 0 0 1px rgba(59,130,246,0.4), 0 0 16px rgba(59,130,246,0.25)",
                      opacity: isRespondingProposal ? 0.5 : 1,
                    }}>Aceitar</button>
                  <button onClick={() => handleProposal(prop.proposal_id, false)} disabled={isRespondingProposal}
                    style={{ flex:1, padding:"9px 0", fontSize:12, fontWeight:700, fontFamily:"inherit", cursor:"pointer", borderRadius:12,
                      background:"rgba(255,255,255,0.06)", border:"1px solid rgba(255,255,255,0.1)", color:"rgba(255,255,255,0.7)",
                      opacity: isRespondingProposal ? 0.5 : 1,
                    }}>Recusar</button>
                </div>
              </div>
            ))}
          </div>

          {/* Ping keyframes */}
          <style>{`@keyframes ping { 0%,100%{transform:scale(1);opacity:.8} 50%{transform:scale(1.8);opacity:0} }`}</style>
        </aside>

      </div>
    </div>
  );
}
