import { useEffect, useState } from 'react';
import { Cloud, Cpu } from 'lucide-react';
import { isNativeAndroidApp } from '../../utils/nativeMode';
import { isTauriDesktop } from '../../utils/desktopUpdate';
import { isLocalOrigin } from '../../utils/cloud';
import { activateWasmLocalMode } from '../../wasmLocal/mode';

// Plain-web counterpart to Android's NativeModeGate and Desktop's own
// pre-React dist-chooser/index.html -- same "Cloud or Local" choice, same
// visual language, but "Local" here has no subprocess to spawn: it's
// activateWasmLocalMode() (see wasmLocal/mode.ts), a synchronous flag flip,
// so there's no async "starting…" phase to show. Local here still goes
// through the normal LoginPage/account flow afterward -- same as
// Android/Desktop's own "Local" (sign in once, simulation data then stays
// on this device instead of the cloud's Postgres) -- see AGENTS.md's
// "WASM-Local Mode" section.
//
// A no-op inside the Android app, inside desktop's own Tauri shell, and on
// desktop's local-sidecar origin (127.0.0.1) -- those all have their own
// equivalent gate already.
export const CHOICE_KEY = 'anatolia_web_mode';

// sessionStorage, not localStorage: the choice should only stick for the
// current app session (avoids re-asking on every in-app navigation/refresh)
// but must be asked again on every fresh app open -- a new tab, the browser
// reopened, or the native app relaunched, per session semantics, all clear
// it, unlike localStorage which would remember it forever and skip straight
// to LoginPage on every subsequent open.
function getSavedChoice(): 'cloud' | 'local' | null {
  try {
    const v = sessionStorage.getItem(CHOICE_KEY);
    return v === 'cloud' || v === 'local' ? v : null;
  } catch {
    return null;
  }
}

/* ── Faint hex grid backdrop (matches LoginPage.tsx's HexGrid) ──────── */
function HexGrid() {
  return (
    <div className="fixed inset-0 pointer-events-none opacity-[0.035]"
      style={{
        backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='56' height='100' viewBox='0 0 56 100'%3E%3Cpath d='M28 66L0 50V18L28 2l28 16v32L28 66zM28 100l-28-16V68l28 16 28-16v16L28 100z' fill='none' stroke='%234f6ef7' stroke-width='0.8'/%3E%3C/svg%3E")`,
        backgroundSize: '56px 100px',
      }}
    />
  );
}

/* ── Slow horizontal scan beam (matches LoginPage.tsx's ScanBar) ────── */
function ScanBar() {
  return (
    <div className="fixed inset-0 overflow-hidden pointer-events-none">
      <div className="absolute left-0 right-0 h-px bg-gradient-to-r from-transparent via-sim-accent/40 to-transparent"
        style={{ animation: 'hud-scan 4s linear infinite' }} />
    </div>
  );
}

/* ── Rings + radar sweep behind the brand badge (matches LoginPage.tsx) */
function LogoRings() {
  return (
    <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
      {[0, 600, 1200].map(delay => (
        <div key={delay} className="absolute rounded-full border border-sim-accent/40"
          style={{ width: 120, height: 120, animation: `ring-expand 3s ease-out ${delay}ms infinite` }} />
      ))}
      <div className="absolute rounded-full ring-rotate"
        style={{ width: 110, height: 110, border: '1px dashed rgba(79,110,247,0.3)' }} />
      <svg className="absolute ring-rotate-rev" width="140" height="140" viewBox="0 0 140 140">
        <circle cx="70" cy="70" r="65" fill="none" stroke="rgba(79,110,247,0.15)" strokeWidth="1" strokeDasharray="3 8" />
        {Array.from({ length: 12 }, (_, i) => {
          const a = (i / 12) * Math.PI * 2 - Math.PI / 2;
          const r1 = 60, r2 = 65;
          return <line key={i}
            x1={70 + r1 * Math.cos(a)} y1={70 + r1 * Math.sin(a)}
            x2={70 + r2 * Math.cos(a)} y2={70 + r2 * Math.sin(a)}
            stroke="rgba(79,110,247,0.5)" strokeWidth="1.5" />;
        })}
      </svg>
      <svg className="absolute radar-sweep opacity-40" width="96" height="96" viewBox="0 0 96 96">
        <defs>
          <linearGradient id="webGateSweep" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#4f6ef7" stopOpacity="0" />
            <stop offset="100%" stopColor="#4f6ef7" stopOpacity="0.7" />
          </linearGradient>
        </defs>
        <path d="M48 48 L48 4 A44 44 0 0 1 90 48 Z" fill="url(#webGateSweep)" />
      </svg>
    </div>
  );
}

/* ── Scramble text — resolves from random characters into final text ─ */
const SCRAMBLE_CHARS = 'ATCGATCGatcg0123456789ΔΣΨΩΦΛΞ'.split('');

function ScrambleText({ text, active, delay = 0 }: { text: string; active: boolean; delay?: number }) {
  const [displayed, setDisplayed] = useState<string[]>(() =>
    text.split('').map(c => (c === ' ' || c === '-' || c === '…') ? c : SCRAMBLE_CHARS[Math.floor(Math.random() * SCRAMBLE_CHARS.length)])
  );

  useEffect(() => {
    if (!active) return;
    let iv: ReturnType<typeof setInterval>;
    const t = setTimeout(() => {
      let frame = 0;
      iv = setInterval(() => {
        frame++;
        setDisplayed(text.split('').map((c, i) => {
          if (c === ' ' || c === '-' || c === '…') return c;
          if (i < frame) return c;
          return SCRAMBLE_CHARS[Math.floor(Math.random() * SCRAMBLE_CHARS.length)];
        }));
        if (frame >= text.length) clearInterval(iv);
      }, 60);
    }, delay);
    return () => { clearTimeout(t); clearInterval(iv!); };
  }, [active, text, delay]);

  return <>{displayed.join('')}</>;
}

export default function BrowserModeGate({ children }: { children: React.ReactNode }) {
  const [choice, setChoice] = useState<'cloud' | 'local' | null>(() => getSavedChoice());
  const [revealed, setRevealed] = useState(false);

  const skip = isNativeAndroidApp() || isTauriDesktop() || isLocalOrigin();

  useEffect(() => {
    if (skip || choice) return;
    const t = setTimeout(() => setRevealed(true), 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [skip, choice]);

  if (skip) return <>{children}</>;

  // Calling activateWasmLocalMode() here (render body, not an effect) makes
  // it take effect before any child's own effects run -- SimulationPage's
  // useSimWebSocket in particular needs to see the flag flipped before its
  // first mount, not one render cycle later. Cheap and idempotent either way.
  if (choice) {
    if (choice === 'local') activateWasmLocalMode();
    return <>{children}</>;
  }

  function choose(mode: 'cloud' | 'local') {
    try { sessionStorage.setItem(CHOICE_KEY, mode); } catch { /* ignore */ }
    if (mode === 'local') activateWasmLocalMode();
    setChoice(mode);
  }

  return (
    <div className="relative w-screen h-screen overflow-hidden flex items-center justify-center bg-[#030310] scanlines px-6">
      <HexGrid />
      <ScanBar />

      <div className="fixed w-96 h-96 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(79,110,247,0.08) 0%, transparent 70%)', top: '15%', left: '20%', filter: 'blur(40px)' }} />
      <div className="fixed w-64 h-64 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(212,168,56,0.06) 0%, transparent 70%)', bottom: '15%', right: '18%', filter: 'blur(30px)' }} />

      <div className="relative z-10 max-w-md w-full text-center flex flex-col items-center">
        <div className="relative w-24 h-24 flex items-center justify-center mb-3 warp-in">
          <LogoRings />
          <div className="relative z-10 w-12 h-12 flex items-center justify-center neon-breathe"
            style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '1.5px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
              <circle cx="12" cy="12" r="10" />
              <ellipse cx="12" cy="12" rx="6" ry="14.29" />
              <path d="M2 12h20" />
              <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
            </svg>
          </div>
        </div>

        <h1 className="font-orbitron font-bold tracking-[0.2em] flicker"
          style={{ fontSize: 'clamp(24px, 6vw, 34px)', color: '#00e887', textShadow: '0 0 20px rgba(0,232,135,0.8), 0 0 40px rgba(0,232,135,0.4)' }}>
          <ScrambleText text="ANATOLIA-SIM" active={revealed} />
        </h1>
        <p className="font-share-tech tracking-[0.4em] mt-1 mb-4" style={{ fontSize: 16, color: '#4f6ef7' }}>
          <ScrambleText text="CIVILIZATION ENGINE" active={revealed} delay={350} />
        </p>
        <p className="font-share-tech text-sim-muted tracking-wide mb-6 boot-in" style={{ fontSize: 13, lineHeight: 1.6, maxWidth: 360, animationDelay: '900ms' }}>
          Two founders. One genome. Watch a civilization emerge from nothing
          but inheritance and observation.
        </p>

        <div className="flex items-center gap-3 mb-6 w-full max-w-xs boot-in" style={{ animationDelay: '1050ms' }}>
          <div className="flex-1 h-px bg-gradient-to-r from-transparent to-sim-accent/40" />
          <div className="w-1.5 h-1.5 rotate-45 bg-sim-accent/60" />
          <div className="flex-1 h-px bg-gradient-to-l from-transparent to-sim-accent/40" />
        </div>

        <div className="flex flex-col gap-3 w-full boot-in" style={{ animationDelay: '1200ms' }}>
          <button
            onClick={() => choose('cloud')}
            className="group relative flex items-center gap-4 text-left px-5 py-4 border border-sim-border/60 bg-sim-surface/60 transition-all duration-200 hover:border-sim-cyan/70 hover:bg-sim-surface hover:-translate-y-0.5 hover:shadow-neon-cyan"
            style={{ clipPath: 'polygon(0 0, calc(100% - 12px) 0, 100% 12px, 100% 100%, 12px 100%, 0 calc(100% - 12px))' }}
          >
            <span className="hud-corner-tr" />
            <Cloud size={22} className="flex-shrink-0 text-sim-cyan" />
            <span className="flex flex-col">
              <span className="font-orbitron font-semibold tracking-[0.15em] uppercase text-sim-text" style={{ fontSize: 15 }}>
                Cloud
              </span>
              <span className="font-share-tech text-sim-muted mt-0.5" style={{ fontSize: 12 }}>
                Synced across every device. Requires internet.
              </span>
            </span>
          </button>

          <button
            onClick={() => choose('local')}
            className="group relative flex items-center gap-4 text-left px-5 py-4 border border-sim-border/60 bg-sim-surface/60 transition-all duration-200 hover:border-sim-gold/70 hover:bg-sim-surface hover:-translate-y-0.5 hover:shadow-neon-gold"
            style={{ clipPath: 'polygon(0 0, calc(100% - 12px) 0, 100% 12px, 100% 100%, 12px 100%, 0 calc(100% - 12px))' }}
          >
            <span className="hud-corner-tr" />
            <Cpu size={22} className="flex-shrink-0 text-sim-gold" />
            <span className="flex flex-col">
              <span className="font-orbitron font-semibold tracking-[0.15em] uppercase text-sim-text" style={{ fontSize: 15 }}>
                Local
              </span>
              <span className="font-share-tech text-sim-muted mt-0.5" style={{ fontSize: 12 }}>
                Runs on this device's own CPU. Sign in still required; upload to the cloud any time.
              </span>
            </span>
          </button>
        </div>
      </div>
    </div>
  );
}
