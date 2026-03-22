import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";

function SplashScreen() {
  const navigate = useNavigate();
  const [message, setMessage] = useState("");

  async function testBackend() {
    const response = await invoke("greet", { name: "Piloto" });
    setMessage(response);
  }

  return (
    <div className="entry-shell px-4">
      <div className="entry-backdrop" />
      <div className="entry-glow left-[12%] top-[12%] h-56 w-56 bg-cyan-400/16" />
      <div className="entry-glow bottom-[14%] right-[10%] h-64 w-64 bg-sky-500/14" />

      <div className="entry-panel text-center">
        <img
          src="/logo-nova.png"
          alt="Logo iRacerApp"
          className="h-28 w-28 object-contain drop-shadow-[0_16px_40px_rgba(88,166,255,0.14)]"
        />

        <div className="space-y-2">
          <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-sky-200/70">
            Launcher
          </p>
          <h1 className="text-3xl font-semibold tracking-[0.08em] text-slate-100">
            iRacerApp
          </h1>
          <p className="text-xs uppercase tracking-[0.28em] text-slate-400">
            v0.1.0
          </p>
        </div>

        <p className="max-w-sm text-[12px] leading-6 text-slate-300/80">
          A entrada do simulador agora compartilha a mesma identidade da logo:
          fundo profundo, halos frios e vidro escuro antes de seguir para o menu.
        </p>

        <div className="flex w-full flex-col gap-3 pt-2">
          <button onClick={testBackend} className="entry-action">
            TESTAR BACKEND
          </button>

          <button onClick={() => navigate("/menu")} className="entry-action">
            ENTRAR
          </button>
        </div>

        {message && (
          <p className="text-center text-[12px] font-medium text-sky-200">
            {message}
          </p>
        )}
      </div>
    </div>
  );
}

export default SplashScreen;
