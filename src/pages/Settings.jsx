import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

function Settings() {
  const navigate = useNavigate();
  const [config, setConfig] = useState(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  useEffect(() => {
    loadConfig();
  }, []);

  async function loadConfig() {
    try {
      const cfg = await invoke("get_config");
      setConfig(cfg);
    } catch (err) {
      console.error("Falha ao carregar config:", err);
    } finally {
      setLoading(false);
    }
  }

  async function saveConfig(newCfg) {
    setSaving(true);
    setErrorMessage("");
    try {
      await invoke("update_config", { newConfig: newCfg });
    } catch (err) {
      console.error("Falha ao salvar config:", err);
      setErrorMessage(err.toString());
      // Recarrega do backend para restaurar estado consistente
      loadConfig();
    } finally {
      setSaving(false);
    }
  }

  async function selectDirectory(field) {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Selecionar Pasta do iRacing",
      });
      if (selected) {
        const newCfg = { ...config, [field]: selected };
        setConfig(newCfg);
        saveConfig(newCfg);
      }
    } catch (err) {
      console.error("Erro ao abrir seletor:", err);
    }
  }

  const handleToggle = (field) => {
    const newCfg = { ...config, [field]: !config[field] };
    setConfig(newCfg); // atualiza state imediatamente (evita leitura stale em saves rápidos)
    saveConfig(newCfg);
  };

  const handleChange = (field, value) => {
    const newCfg = { ...config, [field]: value };
    setConfig(newCfg); // atualiza state imediatamente
    saveConfig(newCfg);
  };

  if (loading || !config) {
    return (
      <div className="entry-shell flex items-center justify-center">
        <div className="text-sky-400 animate-pulse font-bold tracking-widest uppercase">
          Carregando Configurações...
        </div>
      </div>
    );
  }

  return (
    <div className="entry-shell px-4 overflow-y-auto py-12">
      <div className="entry-backdrop" />
      <div className="entry-glow left-[5%] top-[10%] h-80 w-80 bg-blue-500/10" />
      <div className="entry-glow bottom-[5%] right-[5%] h-96 w-96 bg-cyan-500/10" />

      <div className="relative z-10 mx-auto max-w-2xl space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between pb-4">
          <button
            onClick={() => navigate("/menu")}
            className="flex items-center gap-2 text-slate-400 hover:text-white transition-colors group"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="group-hover:-translate-x-1 transition-transform">
              <path d="m15 18-6-6 6-6"/>
            </svg>
            <span className="text-[11px] font-bold uppercase tracking-widest">Voltar</span>
          </button>
          
          <h1 className="text-2xl font-bold tracking-tight text-white flex items-center gap-3">
            Configurações
            {saving && <span className="text-[10px] bg-sky-500/20 text-sky-400 px-2 py-0.5 rounded-full animate-pulse border border-sky-500/30 font-bold uppercase tracking-tighter">Salvando...</span>}
          </h1>
        </div>

        {/* Error Alert */}
        {errorMessage && (
          <div className="bg-red-500/10 border border-red-500/30 p-4 rounded-xl flex items-center gap-4 animate-scale-in">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ef4444" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <p className="text-xs text-red-200 font-semibold">{errorMessage}</p>
            <button onClick={() => setErrorMessage("")} className="ml-auto text-red-400 hover:text-red-200 transition-colors">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
            </button>
          </div>
        )}

        {/* Section: Geral */}
        <div className="entry-panel !items-stretch !w-full !p-8 gap-6">
          <div className="space-y-1">
            <h2 className="text-sm font-bold text-sky-400 uppercase tracking-widest flex items-center gap-2">
              <div className="w-1.5 h-4 bg-sky-500 rounded-full" />
              Preferências Gerais
            </h2>
            <p className="text-[11px] text-slate-400">Personalize sua experiência de jogo e idioma.</p>
          </div>

          <div className="grid gap-6 pt-2">
            {/* Idioma */}
            <div className="flex items-center justify-between group">
              <div className="space-y-0.5">
                <label className="text-[13px] font-semibold text-slate-100">Idioma da Interface</label>
                <p className="text-[11px] text-slate-500">Escolha o idioma dos menus e ferramentas.</p>
              </div>
              <select
                value={config.language}
                onChange={(e) => handleChange("language", e.target.value)}
                className="bg-slate-900/60 border border-slate-700/50 rounded-lg px-4 py-2 text-xs font-semibold focus:border-sky-500/50 outline-none transition-all cursor-pointer"
              >
                <option value="pt-BR">Português (BR)</option>
                <option value="en-US">English (US)</option>
              </select>
            </div>

            <hr className="border-slate-800/40" />

            {/* Autosave */}
            <div className="flex items-center justify-between cursor-pointer" onClick={() => handleToggle("autosave_enabled")}>
              <div className="space-y-0.5">
                <label className="text-[13px] font-semibold text-slate-100">Salvamento Automático</label>
                <p className="text-[11px] text-slate-500">Salva o progresso ao final de cada semana/corrida.</p>
              </div>
              <div className={`w-12 h-6 rounded-full p-1 transition-all duration-300 ${config.autosave_enabled ? "bg-sky-600" : "bg-slate-800"}`}>
                <div className={`w-4 h-4 bg-white rounded-full transition-all duration-300 transform ${config.autosave_enabled ? "translate-x-6" : "translate-x-0"}`} />
              </div>
            </div>
          </div>
        </div>

        {/* Section: iRacing Paths */}
        <div className="entry-panel !items-stretch !w-full !p-8 gap-6">
          <div className="space-y-1">
            <h2 className="text-sm font-bold text-sky-400 uppercase tracking-widest flex items-center gap-2">
              <div className="w-1.5 h-4 bg-sky-500 rounded-full" />
              Integração iRacing
            </h2>
            <p className="text-[11px] text-slate-400">Configure os locais das pastas para exportação de dados.</p>
          </div>

          <div className="grid gap-8 pt-2">
            {[
              { label: "Pasta AI Rosters", field: "airosters_path", desc: "Local das pastas de oponentes (.json)" },
              { label: "Pasta AI Seasons", field: "aiseasons_path", desc: "Local dos arquivos de temporadas (.json)" }
            ].map((pathItem) => (
              <div key={pathItem.field} className="space-y-3">
                <div className="flex items-center justify-between">
                  <label className="text-[13px] font-semibold text-slate-100">{pathItem.label}</label>
                  <button
                    onClick={() => selectDirectory(pathItem.field)}
                    className="text-[10px] font-bold text-sky-400 border border-sky-500/20 bg-sky-500/5 px-3 py-1.5 rounded-lg hover:bg-sky-500/15 transition-all uppercase tracking-widest"
                  >
                    Alterar Pasta
                  </button>
                </div>
                <div className="bg-slate-900/40 p-3 rounded-lg border border-slate-800/50 flex items-center gap-3">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#475569" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>
                  </svg>
                  <p className="text-[11px] font-mono text-slate-400 truncate flex-1">
                    {config[pathItem.field] || "(Não configurado)"}
                  </p>
                </div>
                <p className="text-[10px] text-slate-500 italic">{pathItem.desc}</p>
              </div>
            ))}
          </div>
        </div>

        <div className="pt-8 text-center">
          <p className="text-[10px] text-slate-600 uppercase tracking-[0.4em] font-bold">
            iRacing Career Simulator — v{config.version}
          </p>
        </div>
      </div>
    </div>
  );
}

export default Settings;
