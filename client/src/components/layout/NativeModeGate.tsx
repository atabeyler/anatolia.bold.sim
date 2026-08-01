import { useEffect, useState } from 'react';
import axios from 'axios';
import { App } from '@capacitor/app';
import { Cloud, Cpu, X } from 'lucide-react';
import { CLOUD_API_URL } from '../../utils/cloud';
import { LOCAL_SERVER_URL, isNativeAndroidApp, startLocalServerAndActivate } from '../../utils/nativeMode';
import { checkForAndroidUpdate, installAndroidUpdate, type AndroidUpdateInfo } from '../../utils/androidUpdate';

// Android counterpart to desktop's dist-chooser/index.html: asks "Local"
// (this device's own CPU, via a bundled sim-server subprocess) or "Cloud"
// (the live production site, synced across every device). A no-op on the
// web and inside desktop's own Tauri shell -- those never render this,
// isNativeAndroidApp() is only true inside the Capacitor-wrapped app.
// Deliberately English-only and visually matched to LoginPage.tsx's brand
// language (Orbitron wordmark, hud-panel borders, neon glow, DNA-decode
// title reveal) rather than the app's usual 5-language system -- this is the
// very first thing a user sees, before any language/account context exists.
type Phase = 'choosing' | 'starting-local' | 'error';

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
          <linearGradient id="gateSweep" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#4f6ef7" stopOpacity="0" />
            <stop offset="100%" stopColor="#4f6ef7" stopOpacity="0.7" />
          </linearGradient>
        </defs>
        <path d="M48 48 L48 4 A44 44 0 0 1 90 48 Z" fill="url(#gateSweep)" />
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

function UpdateBanner({ update, onDismiss }: { update: AndroidUpdateInfo; onDismiss: () => void }) {
  const [installState, setInstallState] = useState<'idle' | 'downloading' | 'permission-required' | 'error'>('idle');
  const [percent, setPercent] = useState(0);

  const handleInstall = async () => {
    setInstallState('downloading');
    setPercent(0);
    const result = await installAndroidUpdate(update, setPercent);
    setInstallState(result === 'ok' ? 'idle' : result);
  };

  return (
    <div className="w-full px-4 py-2.5 border-l-2 border-sim-accent bg-sim-accent/10 text-left flex flex-col gap-1.5 boot-in">
      <div className="flex items-center justify-between gap-3">
        <span className="font-share-tech text-xs tracking-wide text-sim-text">
          New version available — <span className="text-sim-gold">v{update.version}</span>
        </span>
        <div className="flex items-center gap-3 flex-shrink-0">
          {installState !== 'downloading' && (
            <button
              onClick={handleInstall}
              className="font-share-tech text-xs tracking-widest uppercase text-sim-accent hover:underline"
            >
              Update
            </button>
          )}
          <button onClick={onDismiss} className="text-sim-muted hover:text-sim-text" aria-label="Dismiss">
            <X size={13} />
          </button>
        </div>
      </div>
      {installState === 'downloading' && (
        <span className="font-share-tech text-xs text-sim-muted">Downloading… {percent}%</span>
      )}
      {installState === 'permission-required' && (
        <span className="font-share-tech text-xs text-sim-gold">
          Enable "Install unknown apps" for this app in Settings, then try again.
        </span>
      )}
      {installState === 'error' && (
        <span className="font-share-tech text-xs text-sim-red">Download failed. Try again.</span>
      )}
    </div>
  );
}

export default function NativeModeGate({ children }: { children: React.ReactNode }) {
  const [phase, setPhase] = useState<Phase>('choosing');
  const [error, setError] = useState('');
  const [ready, setReady] = useState(false);
  const [update, setUpdate] = useState<AndroidUpdateInfo | null>(null);
  // Drives the one-shot DNA-decode reveal of the wordmark/tagline on mount --
  // starts false so ScrambleText's very first render is still scrambled
  // (otherwise the decode would have nothing to animate from).
  const [revealed, setRevealed] = useState(false);

  useEffect(() => {
    if (!isNativeAndroidApp()) return;
    checkForAndroidUpdate().then(setUpdate);
    const t = setTimeout(() => setRevealed(true), 200);

    // Android very often resumes an already-running process instead of
    // truly restarting it when the app is reopened from the recents list,
    // so a check that only ever ran on this component's first mount could
    // go the app's entire lifetime -- days -- without ever noticing a
    // release that shipped after that first launch. Re-check every time
    // the app comes back to the foreground.
    let handle: { remove: () => void } | undefined;
    App.addListener('resume', () => { checkForAndroidUpdate().then(setUpdate); }).then(h => { handle = h; });
    return () => { clearTimeout(t); handle?.remove(); };
  }, []);

  if (!isNativeAndroidApp()) return <>{children}</>;
  if (ready) {
    // Past the Cloud/Local chooser (the common case once a session is under
    // way) -- still surface a newer release here instead of only on that
    // one-time initial screen, which a returning user may never see again.
    return (
      <>
        {update && (
          <div className="fixed top-0 left-0 right-0 z-50">
            <UpdateBanner update={update} onDismiss={() => setUpdate(null)} />
          </div>
        )}
        {children}
      </>
    );
  }

  // Mirrors chooseLocal below rather than navigating the WebView away to
  // CLOUD_API_URL: a real window.location.href navigation tears down this
  // whole document (React state included), and the freshly-loaded page --
  // still inside the same Capacitor WebView -- boots straight back into
  // this same chooser with `ready` reset to false, looking exactly like
  // nothing happened when "Cloud" is tapped. Staying on the app's own
  // bundled origin and just pointing axios at the cloud API instead relies
  // on the same credentialed cross-origin path server-side CORS already
  // carves out for this WebView's own origin (main.rs's is_allowed_origin,
  // "https://localhost") -- no navigation, no reset, no allowNavigation
  // entry needed for this specific flow.
  const chooseCloud = () => {
    axios.defaults.baseURL = CLOUD_API_URL;
    setReady(true);
  };

  const chooseLocal = async () => {
    setPhase('starting-local');
    setError('');
    try {
      await startLocalServerAndActivate();
      axios.defaults.baseURL = LOCAL_SERVER_URL;
      setReady(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase('error');
    }
  };

  return (
    <div className="relative w-screen h-screen overflow-hidden flex items-center justify-center bg-[#030310] scanlines px-6">
      <HexGrid />
      <ScanBar />

      {/* Ambient glow blobs */}
      <div className="fixed w-96 h-96 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(79,110,247,0.08) 0%, transparent 70%)', top: '15%', left: '20%', filter: 'blur(40px)' }} />
      <div className="fixed w-64 h-64 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(212,168,56,0.06) 0%, transparent 70%)', bottom: '15%', right: '18%', filter: 'blur(30px)' }} />

      <div className="relative z-10 max-w-md w-full text-center flex flex-col items-center">
        {/* Badge + rings */}
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

        {/* Wordmark — decodes from scrambled DNA/greek glyphs into place */}
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

        {update && (
          <div className="w-full mb-5">
            <UpdateBanner update={update} onDismiss={() => setUpdate(null)} />
          </div>
        )}

        {/* Divider */}
        <div className="flex items-center gap-3 mb-6 w-full max-w-xs boot-in" style={{ animationDelay: '1050ms' }}>
          <div className="flex-1 h-px bg-gradient-to-r from-transparent to-sim-accent/40" />
          <div className="w-1.5 h-1.5 rotate-45 bg-sim-accent/60" />
          <div className="flex-1 h-px bg-gradient-to-l from-transparent to-sim-accent/40" />
        </div>

        {phase === 'starting-local' && (
          <div className="flex items-center gap-2 mb-6 boot-in">
            <div className="w-1.5 h-1.5 rounded-full bg-sim-cyan pulse-live" />
            <span className="font-share-tech tracking-widest uppercase text-sim-muted" style={{ fontSize: 13 }}>
              Starting local server…
            </span>
          </div>
        )}
        {phase === 'error' && (
          <div className="mb-6 px-3 py-2 border-l-2 border-sim-red bg-sim-red/10 font-share-tech text-sim-red tracking-wide text-left w-full" style={{ fontSize: 13 }}>
            ⚠ {error}
          </div>
        )}

        {phase !== 'starting-local' && (
          <div className="flex flex-col gap-3 w-full boot-in" style={{ animationDelay: '1200ms' }}>
            <button
              onClick={chooseCloud}
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
              onClick={chooseLocal}
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
                  Runs on this device. Upload to the cloud any time.
                </span>
              </span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
