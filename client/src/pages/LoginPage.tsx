import { useState, useEffect, useRef } from 'react';
import { text, type LangCode } from '../utils/i18n';
import FooterBar from '../components/layout/FooterBar';
import SimMenuOverlay from '../components/layout/SimMenuOverlay';
import { useNavigate } from 'react-router-dom';
import axios from 'axios';
import { useSimStore } from '../store/simStore';
import { authUrl, isLocalOrigin } from '../utils/cloud';
import { isNativeAndroidApp, returnToChooser as returnToAndroidChooser } from '../utils/nativeMode';
import { isTauriDesktop, returnToDesktopChooser } from '../utils/desktopUpdate';
import { CHOICE_KEY as BROWSER_MODE_CHOICE_KEY } from '../components/layout/BrowserModeGate';

/* ── Canvas starfield ─────────────────────────────────────── */
// `dark` flips the star/trail palette: on the light background a
// star drawn with the same bright hsla() values used on dark would be all
// but invisible (near-white on near-white), so light mode uses deeper,
// more saturated hues and a light-colored trail for the shooting star
// instead of white-tinted ones.
function StarField({ dark }: { dark: boolean }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const darkRef = useRef(dark);
  useEffect(() => { darkRef.current = dark; }, [dark]);

  useEffect(() => {
    const c = ref.current!;
    const ctx = c.getContext('2d')!;
    const resize = () => { c.width = window.innerWidth; c.height = window.innerHeight; };
    resize();
    window.addEventListener('resize', resize);

    const stars = Array.from({ length: 280 }, () => ({
      x: Math.random() * c.width,
      y: Math.random() * c.height,
      r: Math.random() * 1.4 + 0.2,
      vx: (Math.random() - 0.5) * 0.12,
      vy: (Math.random() - 0.5) * 0.12,
      phase: Math.random() * Math.PI * 2,
      speed: 0.015 + Math.random() * 0.025,
      hue: 200 + Math.random() * 60,
    }));

    const shooting: any[] = [];
    function spawnShoot() {
      shooting.push({ x: Math.random() * c.width, y: Math.random() * c.height * 0.5, vx: 8 + Math.random() * 12, vy: 3 + Math.random() * 5, len: 80 + Math.random() * 120, alpha: 1 });
      setTimeout(spawnShoot, 3000 + Math.random() * 5000);
    }
    spawnShoot();

    let frame: number;
    function draw() {
      const isDark = darkRef.current;
      ctx.clearRect(0, 0, c.width, c.height);
      for (let i = shooting.length - 1; i >= 0; i--) {
        const s = shooting[i];
        s.x += s.vx; s.y += s.vy; s.alpha -= 0.015;
        if (s.alpha <= 0) { shooting.splice(i, 1); continue; }
        const grad = ctx.createLinearGradient(s.x - s.vx * 4, s.y - s.vy * 4, s.x, s.y);
        grad.addColorStop(0, isDark ? `rgba(120,160,255,0)` : `rgba(50,80,180,0)`);
        grad.addColorStop(1, isDark ? `rgba(180,210,255,${s.alpha})` : `rgba(60,90,190,${s.alpha})`);
        ctx.beginPath(); ctx.strokeStyle = grad; ctx.lineWidth = 1.5;
        ctx.moveTo(s.x - s.vx * 6, s.y - s.vy * 6); ctx.lineTo(s.x, s.y); ctx.stroke();
      }
      stars.forEach(s => {
        s.x = (s.x + s.vx + c.width) % c.width;
        s.y = (s.y + s.vy + c.height) % c.height;
        s.phase += s.speed;
        const opacity = 0.35 + 0.65 * (0.5 + 0.5 * Math.sin(s.phase));
        const lightness = isDark ? 80 : 40;
        ctx.beginPath(); ctx.arc(s.x, s.y, s.r, 0, Math.PI * 2);
        ctx.fillStyle = `hsla(${s.hue}, 80%, ${lightness}%, ${opacity})`; ctx.fill();
        if (s.r > 1) {
          ctx.beginPath(); ctx.arc(s.x, s.y, s.r * 3, 0, Math.PI * 2);
          const g = ctx.createRadialGradient(s.x, s.y, 0, s.x, s.y, s.r * 3);
          g.addColorStop(0, `hsla(${s.hue}, 90%, ${isDark ? 90 : 45}%, ${opacity * 0.3})`);
          g.addColorStop(1, 'transparent');
          ctx.fillStyle = g; ctx.fill();
        }
      });
      frame = requestAnimationFrame(draw);
    }
    draw();
    return () => { cancelAnimationFrame(frame); window.removeEventListener('resize', resize); };
  }, []);
  return <canvas ref={ref} className="fixed inset-0 pointer-events-none" />;
}

/* ── Scanning line ────────────────────────────────────────── */
function ScanBar() {
  return (
    <div className="fixed inset-0 overflow-hidden pointer-events-none">
      <div className="absolute left-0 right-0 h-px bg-gradient-to-r from-transparent via-sim-accent/40 to-transparent"
        style={{ animation: 'hud-scan 4s linear infinite' }} />
    </div>
  );
}

/* ── HEX grid pattern ─────────────────────────────────────── */
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

/* ── Rings around logo ────────────────────────────────────── */
function LogoRings() {
  return (
    <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
      {[0, 600, 1200].map(delay => (
        <div key={delay} className="absolute rounded-full border border-sim-accent/30"
          style={{ width: 120, height: 120, animation: `ring-expand 3s ease-out ${delay}ms infinite` }} />
      ))}
      <div className="absolute rounded-full ring-rotate"
        style={{ width: 110, height: 110, border: '1px dashed rgba(79,110,247,0.25)' }} />
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
      <svg className="absolute radar-sweep opacity-30" width="96" height="96" viewBox="0 0 96 96">
        <defs>
          <linearGradient id="sweep" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#4f6ef7" stopOpacity="0" />
            <stop offset="100%" stopColor="#4f6ef7" stopOpacity="0.6" />
          </linearGradient>
        </defs>
        <path d="M48 48 L48 4 A44 44 0 0 1 90 48 Z" fill="url(#sweep)" />
      </svg>
    </div>
  );
}

/* ── HUD input field ──────────────────────────────────────── */
function HudInput({ label, type, value, onChange, placeholder, maxLength }: any) {
  const [focused, setFocused] = useState(false);
  return (
    <div className="mb-2">
      <div className="flex items-center gap-2 mb-1">
        <div className={`w-1 h-3 flex-shrink-0 transition-colors ${focused ? 'bg-sim-accent' : 'bg-sim-border'}`} />
        <label className="font-share-tech tracking-wider uppercase" style={{ fontSize: 16, color: '#c0c8e8' }}>{label}</label>
      </div>
      <div className={`relative transition-all duration-200 ${focused ? 'drop-shadow-[0_0_8px_rgba(79,110,247,0.4)]' : ''}`}>
        <input
          type={type} value={value} onChange={onChange}
          onFocus={() => setFocused(true)} onBlur={() => setFocused(false)}
          placeholder={placeholder} maxLength={maxLength}
          className={`w-full bg-sim-bg/80 px-3 py-2 text-sim-text font-share-tech tracking-wide placeholder-sim-muted/50 focus:outline-none transition-all border ${focused ? 'border-sim-accent/70 bg-sim-surface/80' : 'border-sim-border'}`}
          style={{ fontSize: 16, clipPath: 'polygon(0 0, calc(100% - 10px) 0, 100% 10px, 100% 100%, 10px 100%, 0 calc(100% - 10px))' }}
        />
        {focused && (
          <>
            <div className="absolute top-0 right-0 w-2.5 h-px bg-sim-accent" />
            <div className="absolute top-0 right-0 w-px h-2.5 bg-sim-accent" />
            <div className="absolute bottom-0 left-0 w-2.5 h-px bg-sim-accent" />
            <div className="absolute bottom-0 left-0 w-px h-2.5 bg-sim-accent" />
          </>
        )}
      </div>
    </div>
  );
}

/* ── Scramble text — DNA chars resolve into final text ────── */
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
      }, 75);
    }, delay);
    return () => { clearTimeout(t); clearInterval(iv!); };
  }, [active, text, delay]);

  return <>{displayed.join('')}</>;
}

/* ── Matrix DNA Rain v2 — background layer + intro overlay ── */
type Phase = 'full' | 'fade' | 'bg';

function MatrixRain({ phase, dark }: { phase: Phase; dark: boolean }) {
  const { lang } = useSimStore();
  const ref = useRef<HTMLCanvasElement>(null);
  const phaseRef = useRef(phase);
  useEffect(() => { phaseRef.current = phase; }, [phase]);
  const darkRef = useRef(dark);
  useEffect(() => { darkRef.current = dark; }, [dark]);

  useEffect(() => {
    const c = ref.current!;
    const ctx = c.getContext('2d')!;
    const resize = () => { c.width = window.innerWidth; c.height = window.innerHeight; };
    resize();
    window.addEventListener('resize', resize);

    const chars = 'ATCGATCGatcg0123456789ACDEFGHIKLMNPQRSTVWYХΔΣΨΩ①②③アイウエオ'.split('');
    const fs = 13;
    const drops = Array.from({ length: Math.floor(window.innerWidth / fs) }, () => Math.floor(Math.random() * -50));
    let speed = 1.0;
    let tick = 0;

    let frame: number;
    function draw() {
      const p = phaseRef.current;
      const isDark = darkRef.current;
      const targetSpeed = p === 'full' ? 1.0 : p === 'fade' ? 0.45 : 0.14;
      speed += (targetSpeed - speed) * 0.035;

      tick++;
      if (tick % Math.max(1, Math.round(1 / speed)) !== 0) {
        frame = requestAnimationFrame(draw);
        return;
      }

      // The whole "digital rain" illusion comes from this translucent
      // clear-rect leaving a fading trail rather than a hard wipe -- on a
      // light background a black trail (however faint) reads as dirty
      // smudging instead of a trail, so light mode clears with a light
      // tint of its own page color instead.
      ctx.fillStyle = isDark ? 'rgba(0,0,0,0.065)' : 'rgba(238,241,247,0.11)';
      ctx.fillRect(0, 0, c.width, c.height);

      const colCount = Math.floor(c.width / fs);
      for (let x = 0; x < colCount; x++) {
        if (drops[x] === undefined) drops[x] = Math.floor(Math.random() * -50);
        const y = drops[x];
        const bright = Math.random() > 0.93 && p !== 'bg';
        const a = p === 'bg' ? 0.28 + Math.random() * 0.14 : 0.65 + Math.random() * 0.35;
        if (isDark) {
          const g = 150 + Math.floor(Math.random() * 80);
          const b = 50 + Math.floor(Math.random() * 60);
          ctx.fillStyle = bright ? `rgba(210,255,210,${a})` : `rgba(0,${g},${b},${a})`;
        } else {
          // Deeper, more saturated green (and a dark navy "bright" accent
          // instead of near-white) so the characters stay legible against
          // the light backdrop instead of washing out.
          const g = 90 + Math.floor(Math.random() * 55);
          const b = 20 + Math.floor(Math.random() * 40);
          ctx.fillStyle = bright ? `rgba(20,40,120,${a})` : `rgba(0,${g},${b},${a})`;
        }
        ctx.font = `${fs}px monospace`;
        ctx.fillText(chars[Math.floor(Math.random() * chars.length)], x * fs, y * fs);
        if (y * fs > c.height && Math.random() > 0.975) drops[x] = 0;
        drops[x]++;
      }

      frame = requestAnimationFrame(draw);
    }
    draw();
    return () => { cancelAnimationFrame(frame); window.removeEventListener('resize', resize); };
  }, []);

  return (
    <>
      {/* Matrix canvas — always at z2, behind all UI */}
      <div style={{
        position: 'fixed', inset: 0, zIndex: 2, pointerEvents: 'none',
        opacity: phase === 'bg' ? 0.42 : 1,
        transition: 'opacity 3s ease',
      }}>
        <canvas ref={ref} style={{ display: 'block', width: '100%', height: '100%' }} />
      </div>

      {/* Intro overlay — z200, fades during 'fade', removed in 'bg' */}
      {phase !== 'bg' && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 200, background: dark ? '#000' : '#eef1f7',
          opacity: phase === 'full' ? 1 : 0,
          transition: phase === 'full' ? 'none' : 'opacity 2.4s ease',
          pointerEvents: 'none',
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10,
        }}>
          <span style={{ fontFamily: 'Orbitron, monospace', fontSize: 'clamp(18px,5vw,32px)', color: dark ? '#00e887' : '#0a8a54', letterSpacing: '0.3em', fontWeight: 900, textShadow: dark ? '0 0 24px #00e887, 0 0 48px rgba(0,232,135,0.4)' : 'none' }}>
            {lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'}
          </span>
          <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: dark ? '#4ecb71' : '#2f8f57', letterSpacing: '0.28em', textShadow: dark ? '0 0 10px #4ecb71' : 'none' }}>
            {text(lang as LangCode, { tr: 'GENOM MATRİSİ YÜKLENİYOR…', en: 'LOADING GENOME MATRIX…', de: 'GENOM-MATRIX WIRD GELADEN…', fr: 'CHARGEMENT DE LA MATRICE GÉNOMIQUE…', ar: 'جارٍ تحميل مصفوفة الجينوم…' })}
          </span>
          <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
            {['A', 'T', 'C', 'G'].map((b, i) => (
              <span key={b} style={{ fontFamily: 'Orbitron, monospace', fontSize: 11, color: ['#4f6ef7', '#e05a5a', '#d4a838', dark ? '#00e887' : '#0a8a54'][i], letterSpacing: '0.1em', animation: `pulse ${0.8 + i * 0.2}s infinite` }}>{b}</span>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

/* ── System status shape ──────────────────────────────────── */
interface SysStatus {
  status: 'online' | 'degraded';
  genome_loci: number;
  epi_loci: number;
  lang_stages: number;
  active_sims: number;
  total_population: number;
}

/* ── Main Login Component ─────────────────────────────────── */
export default function LoginPage() {
  const navigate = useNavigate();
  const { setUser, lang, theme } = useSimStore();
  const dark = theme !== 'light';
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPage, setMenuPage] = useState<'guide' | 'about' | 'mission' | 'contact' | null>(null);
  const [mode, setMode] = useState<'login' | 'register'>('login');
  const [rememberMe, setRememberMe] = useState(() => localStorage.getItem('anatolia_remember') === '1');
  const savedCode = rememberMe ? (localStorage.getItem('anatolia_saved_code') ?? '') : '';
  const [form, setForm] = useState({ user_code: savedCode, reg_user_code: '', first_name: '', last_name: '', tc_no: '', email: '', password: '' });
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [loading, setLoading] = useState(false);
  const [phase, setPhase] = useState<Phase>('full');
  const [scrambling, setScrambling] = useState(false);
  const [sysStatus, setSysStatus] = useState<SysStatus | null>(null);
  const [pendingCode, setPendingCode] = useState('');
  const [coords, setCoords] = useState<{ lat: string; lon: string } | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const f = (k: string) => (e: any) => setForm(p => ({ ...p, [k]: e.target.value }));

  // Fetch real runtime/db stats for the status panel
  useEffect(() => {
    axios.get<SysStatus>('/api/system/status').then(r => setSysStatus(r.data)).catch(() => {});
  }, []);

  // Phase timeline: full (0–1.5s) → fade + scramble (1.5s) → bg (3.9s)
  useEffect(() => {
    const t1 = setTimeout(() => { setPhase('fade'); setScrambling(true); }, 1500);
    const t2 = setTimeout(() => setPhase('bg'), 3900);
    return () => { clearTimeout(t1); clearTimeout(t2); };
  }, []);

  useEffect(() => {
    const fmt = (lat: number, lon: number) => setCoords({
      lat: `${Math.abs(lat).toFixed(4)}°${lat >= 0 ? 'N' : 'S'}`,
      lon: `${Math.abs(lon).toFixed(4)}°${lon >= 0 ? 'E' : 'W'}`,
    });

    const el = (window as any).desktopLocation ?? (window as any).electronLocation;
    if (el) {
      // Desktop shell path: use native coordinates when a shell bridge is available.
      el.getCoords().then((c: { lat: number; lon: number } | null) => {
        if (c) fmt(c.lat, c.lon);
      }).catch(() => {});
    } else if (navigator.geolocation) {
      // Web browser / Android WebView: standard Geolocation API. This is a
      // decorative readout, not a precision feature -- enableHighAccuracy
      // demands a real GPS satellite fix, which routinely takes well past
      // 10s (or never resolves at all indoors), so it left this silently
      // stuck on the hardcoded Ankara fallback below on real devices.
      // Network/cell-based coarse location resolves in a couple of
      // seconds and is all ACCESS_COARSE_LOCATION grants anyway.
      navigator.geolocation.getCurrentPosition(
        pos => fmt(pos.coords.latitude, pos.coords.longitude),
        () => {},
        { timeout: 15000, enableHighAccuracy: false, maximumAge: 300000 }
      );
    }
  }, []);

  useEffect(() => {
    if (!pendingCode) return;
    pollRef.current = setInterval(async () => {
      try {
        const { data } = await axios.get(authUrl(`/api/auth/pending-status/${pendingCode}`));
        if (data.status === 'approved') {
          clearInterval(pollRef.current!);
          setPendingCode('');
          setSuccess(text(lang as LangCode, {
            tr: '✔ Hesabınız onaylandı! Artık giriş yapabilirsiniz.',
            en: '✔ Your account has been approved! You can now sign in.',
            de: '✔ Ihr Konto wurde genehmigt! Sie können sich jetzt anmelden.',
            fr: '✔ Votre compte a été approuvé ! Vous pouvez maintenant vous connecter.',
            ar: '✔ تمت الموافقة على حسابك! يمكنك الآن تسجيل الدخول.',
          }));
          setMode('login');
          setForm(p => ({ ...p, user_code: pendingCode }));
        }
      } catch {}
    }, 10000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [pendingCode, lang]);

  useEffect(() => {
    document.body.classList.add('login-page-active');
    return () => document.body.classList.remove('login-page-active');
  }, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(''); setSuccess(''); setLoading(true);
    try {
      if (mode === 'register') {
        await axios.post(authUrl('/api/auth/register'), {
          first_name: form.first_name, last_name: form.last_name,
          tc_no: form.tc_no, email: form.email, password: form.password,
          user_code: form.reg_user_code.toUpperCase(),
        });
        const code = form.reg_user_code.toUpperCase();
        setSuccess(text(lang as LangCode, {
          tr: `Talebiniz alındı. Kodunuz: ${code} — Yönetim onayı bekleniyor…`,
          en: `Request received. Code: ${code} — waiting for admin approval…`,
          de: `Anfrage erhalten. Code: ${code} — wartet auf Admin-Genehmigung…`,
          fr: `Demande reçue. Code : ${code} — en attente d'approbation de l'administrateur…`,
          ar: `تم استلام الطلب. الرمز: ${code} — بانتظار موافقة الإدارة…`,
        }));
        setMode('login');
        setPendingCode(code);
        setForm(p => ({ ...p, reg_user_code: '', first_name: '', last_name: '', tc_no: '', email: '', password: '' }));
      } else {
        const userCode = form.user_code.trim().toUpperCase();
        const { data } = await axios.post(authUrl('/api/auth/login'), { user_code: userCode, password: form.password }, { withCredentials: true });
        // Set BEFORE setUser: persistAuth (called from setUser) reads
        // 'anatolia_session_active' to decide whether the access token/user
        // itself may land in localStorage -- this must already reflect this
        // login's remember choice by the time that happens, not a stale
        // value from a previous session.
        if (rememberMe) {
          localStorage.setItem('anatolia_session_active', '1');
          localStorage.setItem('anatolia_remember', '1');
          localStorage.setItem('anatolia_saved_code', userCode);
        } else {
          sessionStorage.setItem('anatolia_session_active', '1');
          localStorage.removeItem('anatolia_remember');
          localStorage.removeItem('anatolia_session_active');
          localStorage.removeItem('anatolia_saved_code');
        }
        setUser(data.user, data.access_token);
        navigate(data.user.role === 'admin' ? '/admin' : '/');
      }
    } catch (err: any) {
      setError(err.response?.data?.error ?? text(lang as LangCode, { tr: 'Giriş başarısız', en: 'Authentication failed', de: 'Anmeldung fehlgeschlagen', fr: 'Échec de l\'authentification', ar: 'فشل تسجيل الدخول' }));
    } finally { setLoading(false); }
  }

  return (
    <div className="relative min-h-screen overflow-x-hidden flex flex-col items-center scanlines"
      style={{ background: dark ? '#030310' : '#eef1f7' }}>

      {/* Matrix (canvas at z2 + intro overlay at z200) */}
      <MatrixRain phase={phase} dark={dark} />

      {/* MENÜ button (also the entry point for Settings — see its own "Ayarlar" row) */}
      <div className="fixed z-30" style={{ top: 12, right: 12, display: 'flex', alignItems: 'center', gap: 6 }}>
        <button
          onClick={() => setMenuOpen(true)}
          style={{ display: 'flex', alignItems: 'center', gap: 3, padding: '2px 8px', border: `1px solid ${dark ? 'rgba(160,200,176,0.35)' : 'rgba(60,90,80,0.35)'}`, color: dark ? '#a0c8b0' : '#365044', background: 'transparent', fontSize: 14, letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer' }}
        >
          ☰ {text(lang as LangCode, { tr: 'MENÜ', en: 'MENU', de: 'MENÜ', fr: 'MENU', ar: 'القائمة' })}
        </button>
      </div>

      <SimMenuOverlay
        isOpen={menuOpen}
        onClose={() => setMenuOpen(false)}
        menuPage={menuPage}
        onMenuPageChange={setMenuPage}
      />

      {/* Backgrounds (behind matrix canvas at z2, but matrix canvas has black bg so they're invisible during intro) */}
      <StarField dark={dark} />
      <HexGrid />
      <ScanBar />

      {/* Ambient glow blobs */}
      <div className="fixed w-96 h-96 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(79,110,247,0.08) 0%, transparent 70%)', top: '20%', left: '30%', filter: 'blur(40px)', zIndex: 3 }} />
      <div className="fixed w-64 h-64 rounded-full pointer-events-none"
        style={{ background: 'radial-gradient(circle, rgba(0,212,255,0.06) 0%, transparent 70%)', bottom: '20%', right: '25%', filter: 'blur(30px)', zIndex: 3 }} />

      {/* System status top-left */}
      {(() => {
        const ss = sysStatus;
        const l = lang as LangCode;
        const dot = '…';
        const activeWord = text(l, { tr: 'AKTİF', en: 'ACTIVE', de: 'AKTIV', fr: 'ACTIF', ar: 'نشط' });
        const readyWord = text(l, { tr: 'HAZIR', en: 'READY', de: 'BEREIT', fr: 'PRÊT', ar: 'جاهز' });
        const stageWord = text(l, { tr: 'AŞAMA', en: 'STAGE', de: 'STUFE', fr: 'ÉTAPE', ar: 'مرحلة' });
        const indWord = text(l, { tr: 'BİREY', en: 'IND', de: 'IND', fr: 'IND', ar: 'فرد' });
        const STATUS = [
          { label: text(l, { tr: 'ÇEKİRDEK SİSTEMLER', en: 'CORE SYSTEMS', de: 'KERNSYSTEME', fr: 'SYSTÈMES PRINCIPAUX', ar: 'الأنظمة الأساسية' }), ok: ss?.status === 'online', val: ss ? (ss.status === 'online' ? text(l, { tr: 'ÇEVRİMİÇİ', en: 'ONLINE', de: 'ONLINE', fr: 'EN LIGNE', ar: 'متصل' }) : text(l, { tr: 'BOZULMUŞ', en: 'DEGRADED', de: 'BEEINTRÄCHTIGT', fr: 'DÉGRADÉ', ar: 'متدهور' })) : dot },
          { label: text(l, { tr: 'FİZİK MOTORU', en: 'PHYSICS ENGINE', de: 'PHYSIK-ENGINE', fr: 'MOTEUR PHYSIQUE', ar: 'محرك الفيزياء' }), ok: true, val: 'v1.0' },
          { label: text(l, { tr: 'GENOM MATRİSİ', en: 'GENOME MATRIX', de: 'GENOM-MATRIX', fr: 'MATRICE GÉNOMIQUE', ar: 'مصفوفة الجينوم' }), ok: true, val: ss ? `${ss.genome_loci} LOCI` : dot },
          { label: text(l, { tr: 'EPİGENOM', en: 'EPIGENOME', de: 'EPIGENOM', fr: 'ÉPIGÉNOME', ar: 'التخلق اللاجيني' }), ok: true, val: ss ? `${ss.epi_loci} LOCI` : dot },
          { label: text(l, { tr: 'SİNİR AĞI', en: 'NEURAL NET', de: 'NEURONALES NETZ', fr: 'RÉSEAU NEURONAL', ar: 'الشبكة العصبية' }), ok: !!ss, val: ss ? activeWord : text(l, { tr: 'BAĞLANIYOR…', en: 'CONN…', de: 'VERB…', fr: 'CONN…', ar: 'اتصال…' }) },
          { label: text(l, { tr: 'İKLİM SİMÜL.', en: 'CLIMATE SIM', de: 'KLIMA-SIM', fr: 'SIM CLIMAT', ar: 'محاكاة المناخ' }), ok: true, val: ss ? (ss.active_sims > 0 ? `${ss.active_sims} ${activeWord}` : readyWord) : dot },
          { label: text(l, { tr: 'DİL ÇEKİRDEĞİ', en: 'LANGUAGE CORE', de: 'SPRACHKERN', fr: 'NOYAU LINGUISTIQUE', ar: 'نواة اللغة' }), ok: true, val: ss ? `${ss.lang_stages} ${stageWord}` : dot },
          { label: text(l, { tr: 'SOSYAL MATRİS', en: 'SOCIAL MATRIX', de: 'SOZIALE MATRIX', fr: 'MATRICE SOCIALE', ar: 'المصفوفة الاجتماعية' }), ok: true, val: ss ? (ss.total_population > 0 ? `${ss.total_population.toLocaleString()} ${indWord}` : readyWord) : dot },
        ];
        return (
          <div
            className="fixed top-3 left-3 z-20"
            style={{
              display: phase === 'full' ? 'none' : 'flex',
              flexDirection: 'column',
              gap: 8,
              maxWidth: 'calc(100vw - 24px)',
              opacity: phase === 'fade' ? 0 : 1,
              transition: 'opacity 1.5s ease',
              transitionDelay: '1s',
            }}
          >
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, auto)', columnGap: 12, rowGap: 2 }}>
              {STATUS.map((s, i) => (
                <div key={s.label} className="flex items-center gap-1 boot-in" style={{ animationDelay: `${i * 80}ms` }}>
                  <div className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${s.ok ? 'bg-sim-green pulse-live' : 'bg-sim-red'}`} />
                  <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 'clamp(11px, 1.05vw, 14px)', color: dark ? '#c0c8e8' : '#3a4258', letterSpacing: '0.06em', whiteSpace: 'nowrap' }}>
                    {s.label}
                  </span>
                  <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 'clamp(11px, 1.05vw, 14px)', color: dark ? '#4ecb71' : '#1f8a4c', marginLeft: 2, whiteSpace: 'nowrap' }}>
                    {s.val}
                  </span>
                </div>
              ))}
            </div>
            <div className="font-share-tech tracking-widest" style={{ fontSize: 'clamp(11px, 1.05vw, 14px)', color: dark ? '#c0c8e8' : '#3a4258', maxWidth: 'min(calc(100vw - 24px), 420px)' }}>
              <div style={{ whiteSpace: 'normal', overflowWrap: 'anywhere', lineHeight: 1.25 }}>
                LAT: {coords?.lat ?? '39.9334°N'} · LON: {coords?.lon ?? '32.8597°E'}
              </div>
              <div style={{ marginTop: 1, whiteSpace: 'normal', overflowWrap: 'anywhere', lineHeight: 1.25 }}>
                SYS: {lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'} v{__APP_VERSION__} · BUILD 2026
              </div>
            </div>
          </div>
        );
      })()}

      {/* Main content — always in DOM, fades in as intro overlay dissolves */}
      <div
        className="z-10 flex flex-col items-center w-full px-4 py-8 my-auto"
        style={{
          opacity: phase === 'full' ? 0 : 1,
          transition: 'opacity 2s ease',
          transitionDelay: phase === 'full' ? '0s' : '0.5s',
        }}
      >
        {/* Logo area with rings */}
        <div className="relative w-28 h-28 flex items-center justify-center mb-4">
          <LogoRings />
          <div className="relative z-10 w-14 h-14 flex items-center justify-center neon-breathe"
            style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '1.5px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }}>
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
              <circle cx="12" cy="12" r="10" />
              <ellipse cx="12" cy="12" rx="6" ry="14.29" />
              <path d="M2 12h20" />
              <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
            </svg>
          </div>
        </div>

        {/* Title — crystallizes from DNA chars */}
        <div className="text-center mb-2">
          <h1
            className="font-orbitron text-2xl sm:text-3xl font-bold tracking-[0.2em] flicker"
            style={{
              color: dark ? '#00e887' : '#0a8a54',
              textShadow: dark ? '0 0 20px rgba(0,232,135,0.8), 0 0 40px rgba(0,232,135,0.4)' : 'none',
            }}
          >
            <ScrambleText text={lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'} active={scrambling} />
          </h1>
          <p
            className="font-share-tech tracking-[0.4em] mt-1"
            style={{
              fontSize: 18,
              color: dark ? '#4f6ef7' : '#3652c4',
              opacity: scrambling ? 1 : 0,
              transition: 'opacity 1s ease',
              transitionDelay: '0.8s',
            }}
          >
            <ScrambleText
              text={text(lang as LangCode, { tr: 'MEDENİYET', en: 'CIVILIZATION', de: 'ZIVILISATION', fr: 'CIVILISATION', ar: 'الحضارة' })}
              active={scrambling}
              delay={900}
            />
          </p>
        </div>

        {/* Separator line */}
        <div className="flex items-center gap-3 my-3 w-[460px] max-w-full">
          <div className="flex-1 h-px bg-gradient-to-r from-transparent to-sim-accent/40" />
          <div className="w-1.5 h-1.5 rotate-45 bg-sim-accent/60" />
          <div className="flex-1 h-px bg-gradient-to-l from-transparent to-sim-accent/40" />
        </div>

        {/* Form panel */}
        <div
          className="w-[460px] max-w-full hud-panel relative"
          style={{
            padding: 'clamp(16px, 3vw, 22px) clamp(16px, 3.5vw, 28px)',
            opacity: phase === 'bg' ? 1 : 0,
            transform: phase === 'bg' ? 'translateY(0)' : 'translateY(12px)',
            transition: 'opacity 1.2s ease, transform 1.2s ease',
          }}
        >
          <span className="hud-corner-tr" />
          <span className="hud-corner-bl" />

          <div className="absolute -top-px left-6 right-6 flex items-center justify-center">
            <div className="bg-[#030310] px-3 flex items-center gap-2">
              <div className="w-1 h-1 rounded-full bg-sim-accent pulse-live" />
              <span className="font-share-tech text-sim-accent tracking-[0.1em] sm:tracking-[0.3em]" style={{ fontSize: 'clamp(12px, 3.8vw, 18px)' }}>
                {mode === 'login' ? text(lang as LangCode, { tr: 'KİMLİK DOĞRULAMA', en: 'IDENTITY VERIFICATION', de: 'IDENTITÄTSVERIFIZIERUNG', fr: "VÉRIFICATION D'IDENTITÉ", ar: 'التحقق من الهوية' }) : text(lang as LangCode, { tr: 'HESAP OLUŞTURMA', en: 'ACCOUNT CREATION', de: 'KONTOERSTELLUNG', fr: 'CRÉATION DE COMPTE', ar: 'إنشاء حساب' })}
              </span>
              <div className="w-1 h-1 rounded-full bg-sim-accent pulse-live" />
            </div>
          </div>

          <div className="flex gap-1 mb-3 mt-2">
            {(['login', 'register'] as const).map(m => (
              <button key={m} type="button" onClick={() => setMode(m)}
                style={{ fontSize: 'clamp(14px, 1.4vw, 16px)' }}
                className={`flex-1 py-2.5 font-share-tech tracking-widest uppercase transition-all border ${
                  mode === m
                    ? 'bg-sim-accent/20 border-sim-accent/60 text-sim-accent shadow-neon-sm'
                    : 'border-sim-border/50 text-sim-muted hover:border-sim-accent/30 hover:text-sim-text'
                }`}>
                {m === 'login'
                  ? text(lang as LangCode, { tr: 'GİRİŞ', en: 'SIGN IN', de: 'ANMELDEN', fr: 'CONNEXION', ar: 'تسجيل الدخول' })
                  : text(lang as LangCode, { tr: 'KAYIT', en: 'SIGN UP', de: 'REGISTRIEREN', fr: 'INSCRIPTION', ar: 'تسجيل' })}
              </button>
            ))}
          </div>

          <form onSubmit={handleSubmit}>
            {mode === 'register' ? (<>
              <div className="grid grid-cols-2 gap-1.5">
                <HudInput label={text(lang as LangCode, { tr: 'Ad', en: 'First Name', de: 'Vorname', fr: 'Prénom', ar: 'الاسم الأول' })} type="text"
                  value={form.first_name} onChange={f('first_name')} placeholder={text(lang as LangCode, { tr: 'AD', en: 'FIRST NAME', de: 'VORNAME', fr: 'PRÉNOM', ar: 'الاسم الأول' })} />
                <HudInput label={text(lang as LangCode, { tr: 'Soyad', en: 'Last Name', de: 'Nachname', fr: 'Nom', ar: 'اسم العائلة' })} type="text"
                  value={form.last_name} onChange={f('last_name')} placeholder={text(lang as LangCode, { tr: 'SOYAD', en: 'LAST NAME', de: 'NACHNAME', fr: 'NOM', ar: 'اسم العائلة' })} />
              </div>
              <HudInput label={text(lang as LangCode, { tr: 'KULLANICI KODU', en: 'USER CODE', de: 'BENUTZERCODE', fr: 'CODE UTILISATEUR', ar: 'رمز المستخدم' })} type="text" maxLength={20}
                value={form.reg_user_code}
                onChange={(e: any) => setForm(p => ({ ...p, reg_user_code: e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, '') }))}
                placeholder="ANSYZ0001" />
              <p className="font-share-tech tracking-wide -mt-1 mb-1.5" style={{ fontSize: 12, color: '#6a8a9a' }}>
                {text(lang as LangCode, { tr: '4-20 karakter · harf ve rakam', en: '4-20 chars · letters & numbers only', de: '4-20 Zeichen · nur Buchstaben & Zahlen', fr: '4-20 caractères · lettres et chiffres uniquement', ar: '4-20 حرفاً · أحرف وأرقام فقط' })}
              </p>
              <HudInput label={text(lang as LangCode, { tr: 'TC KİMLİK NO', en: 'NATIONAL ID NO', de: 'AUSWEISNUMMER', fr: "N° D'IDENTITÉ NATIONALE", ar: 'رقم الهوية الوطنية' })} type="text" maxLength={11}
                value={form.tc_no} onChange={f('tc_no')} placeholder="00000000000" />
              <HudInput label={text(lang as LangCode, { tr: 'E-POSTA', en: 'EMAIL', de: 'E-MAIL', fr: 'E-MAIL', ar: 'البريد الإلكتروني' })} type="email"
                value={form.email} onChange={f('email')} placeholder="user@domain.com" />
              <HudInput label={text(lang as LangCode, { tr: 'ŞİFRE', en: 'Password', de: 'Passwort', fr: 'Mot de passe', ar: 'كلمة المرور' })} type="password"
                value={form.password} onChange={f('password')} placeholder="••••••••" />
              <p className="font-share-tech tracking-wide mb-2" style={{ fontSize: 12, color: '#6a8a9a' }}>
                {text(lang as LangCode, {
                  tr: 'Min 8 karakter · büyük · küçük · rakam · sembol',
                  en: 'Min 8 chars · upper · lower · number · symbol',
                  de: 'Min. 8 Zeichen · groß · klein · Zahl · Symbol',
                  fr: 'Min 8 caractères · majuscule · minuscule · chiffre · symbole',
                  ar: '8 أحرف على الأقل · حرف كبير · حرف صغير · رقم · رمز',
                })}
              </p>
            </>) : (<>
              <HudInput label={text(lang as LangCode, { tr: 'KULLANICI KODU', en: 'User Code', de: 'Benutzercode', fr: 'Code utilisateur', ar: 'رمز المستخدم' })} type="text"
                value={form.user_code} onChange={f('user_code')} placeholder="••••••••" />
              <HudInput label={text(lang as LangCode, { tr: 'ŞİFRE', en: 'Password', de: 'Passwort', fr: 'Mot de passe', ar: 'كلمة المرور' })} type="password"
                value={form.password} onChange={f('password')} placeholder="••••••••" />
            </>)}

            {error && (
              <div className="mb-3 px-3 py-2 border-l-2 border-sim-red bg-sim-red/10 font-share-tech text-sim-red tracking-wide" style={{ fontSize: 'clamp(13px, 1.15vw, 14px)' }}>
                ⚠ {error}
              </div>
            )}
            {success && (
              <div className="mb-3 px-3 py-2 border-l-2 border-sim-green bg-sim-green/10 font-share-tech text-sim-green tracking-wide" style={{ fontSize: 'clamp(13px, 1.15vw, 14px)' }}>
                ✓ {success}
              </div>
            )}

            {mode === 'login' && (
              <label className="flex items-center gap-2 mb-3 cursor-pointer select-none" style={{ marginTop: 4 }}>
                <div
                  onClick={() => setRememberMe(v => !v)}
                  className="w-4 h-4 flex-shrink-0 flex items-center justify-center transition-all"
                  style={{ border: `1px solid ${rememberMe ? '#4f6ef7' : '#3a4a6a'}`, background: rememberMe ? 'rgba(79,110,247,0.25)' : 'transparent', cursor: 'pointer' }}
                >
                  {rememberMe && <svg width="10" height="8" viewBox="0 0 10 8" fill="none"><path d="M1 4L3.5 6.5L9 1" stroke="#4f6ef7" strokeWidth="1.5" strokeLinecap="round"/></svg>}
                </div>
                <span
                  onClick={() => setRememberMe(v => !v)}
                  style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: rememberMe ? '#c0c8e8' : '#6a8a9a', letterSpacing: '0.06em' }}
                >
                  {text(lang as LangCode, { tr: 'Beni hatırla', en: 'Remember me', de: 'Angemeldet bleiben', fr: 'Se souvenir de moi', ar: 'تذكرني' })}
                </span>
              </label>
            )}

            <button type="submit" disabled={loading}
              className="w-full py-3.5 font-orbitron font-semibold tracking-[0.2em] text-white transition-all disabled:opacity-40 neon-breathe relative overflow-hidden"
              style={{
                fontSize: 'clamp(15px, 1.5vw, 18px)',
                background: loading ? 'rgba(79,110,247,0.3)' : 'linear-gradient(135deg, rgba(79,110,247,0.35) 0%, rgba(79,110,247,0.2) 100%)',
                border: '1px solid rgba(79,110,247,0.6)',
                clipPath: 'polygon(0 0, calc(100% - 12px) 0, 100% 12px, 100% 100%, 12px 100%, 0 calc(100% - 12px))',
              }}>
              {loading ? (
                <span className="flex items-center justify-center gap-2">
                  <svg className="animate-spin w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                  </svg>
                  {text(lang as LangCode, { tr: 'İŞLENİYOR...', en: 'PROCESSING...', de: 'VERARBEITUNG...', fr: 'TRAITEMENT...', ar: 'جارٍ المعالجة...' })}
                </span>
              ) : (
                mode === 'login'
                  ? text(lang as LangCode, { tr: 'GİRİŞ', en: 'INITIATE', de: 'STARTEN', fr: 'DÉMARRER', ar: 'ابدأ' })
                  : text(lang as LangCode, { tr: 'TALEP GÖNDER', en: 'SUBMIT REQUEST', de: 'ANFRAGE SENDEN', fr: 'ENVOYER LA DEMANDE', ar: 'إرسال الطلب' })
              )}
            </button>
          </form>
        </div>

        {/* A bare browser hitting the local sidecar's own origin directly
            (no Android/Tauri wrapper around it) never went through any
            chooser to begin with, so there's nothing to go back to there --
            every other case (native Android, desktop Tauri shell in either
            of its own Cloud/Local sub-modes, or plain web) does have one. */}
        {(isNativeAndroidApp() || isTauriDesktop() || !isLocalOrigin()) && (
          <button
            type="button"
            onClick={() => {
              if (isNativeAndroidApp()) {
                returnToAndroidChooser();
              } else if (isTauriDesktop()) {
                returnToDesktopChooser();
              } else {
                try { sessionStorage.removeItem(BROWSER_MODE_CHOICE_KEY); } catch { /* ignore */ }
                window.location.href = '/';
              }
            }}
            className={`mt-4 font-share-tech tracking-widest uppercase hover:text-[#a0b4ff] transition-colors ${dark ? 'text-sim-muted' : ''}`}
            style={{ fontSize: 12, color: dark ? undefined : '#4a5a72' }}
          >
            {text(lang as LangCode, {
              tr: '← Bulut / Yerel Seçimine Dön',
              en: '← Back to Cloud / Local Selection',
              de: '← Zurück zur Cloud-/Lokal-Auswahl',
              fr: '← Retour au choix Cloud / Local',
              ar: '← العودة إلى اختيار السحابة / المحلي',
            })}
          </button>
        )}

        <FooterBar mode="inline" className="mt-6 mb-2 px-4" />
      </div>

    </div>
  );
}
