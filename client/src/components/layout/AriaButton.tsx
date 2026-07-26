import { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { Mic, MicOff, Loader2 } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

type AriaState = 'idle' | 'listening' | 'processing';

const ALLOWED_SPEEDS = [1, 5, 20, 100];

const SpeechRec: any = typeof window !== 'undefined'
  ? ((window as any).SpeechRecognition || (window as any).webkitSpeechRecognition || null)
  : null;
const PILL_W = 148;
const PILL_H = 48;

const LANGUAGE_LOCALES: Record<string, string> = {
  tr: 'tr-TR',
  en: 'en-US',
  de: 'de-DE',
  fr: 'fr-FR',
  ar: 'ar',
};

function resolveLocale(lang: string) {
  return LANGUAGE_LOCALES[lang] ?? 'en-US';
}

function buildPrompt(stats: any, events: any[], ctx: any, lang: string): string {
  const respondIn = {
    tr: 'Turkish',
    en: 'English',
    de: 'German',
    fr: 'French',
    ar: 'Arabic',
  }[lang] ?? 'English';
  const page = ctx?.page ?? '/';
  const isWizard = !!ctx?.wizardOpen;
  const isSim = page.startsWith('/simulation/');
  const isDash = page === '/';
  const pageLabel = isWizard ? 'wizard' : isSim ? 'simulation' : isDash ? 'dashboard' : 'login';

  let state = `Page:${pageLabel} Status:${ctx?.simStatus ?? 'none'}`;
  if (isWizard) state += ` WizStep:${ctx.wizardStep} WizType:${ctx.wizardStepType}${ctx.wizardFounder ? ' founder:'+ctx.wizardFounder : ''}${ctx.wizardTraitName ? ' trait:'+ctx.wizardTraitName : ''}`;
  if (stats) state += `\nYear:${stats.year} Pop:${stats.population} Tech:${stats.technologies} Happy:${Math.round((stats.happiness_index??0.5)*100)}% Food:${(stats.food_abundance??0.5).toFixed(2)} Temp:${stats.temperature??20}C`;
  const evtStr = (events??[]).slice(0,6).map((e:any)=>`[${e.event_type}]${e.description}`).join('|');
  if (evtStr) state += `\nEvents:${evtStr}`;

  return `ARIA, ANATOLIA-SIM controller. Reply in ${respondIn}. Output ONLY valid JSON: {"text":"reply","actions":[...]}
STATE: ${state}
${isWizard ? `WIZARD: wizard_next|wizard_back|wizard_submit|wizard_exit|wizard_set{"type":"wizard_set","field":"F","value":"V","founder":1|2}
Fields: sim_name|sim_latitude|sim_longitude|founder_name|founder_age|founder_sex(male/female)|founder_height|founder_weight|founder_eye(brown/hazel/green/blue)|founder_hair(black/dark/brown/light/blond/red)|founder_skin(fair/light/olive/tan/brown/dark)|current_trait(0-100)
TR: zeka=fluid_intelligence merak=curiosity dil=language_capacity öğrenme=learning_rate disiplin=conscientiousness stres=stress_resilience risk=risk_tolerance inovasyon=innovation sanat=artistic_sense empati=empathy işbirliği=cooperation liderlik=dominance güç=physical_strength dayanıklılık=endurance bağışıklık=immune_strength üreme=fertility uzun_ömür=longevity` : ''}
${isSim ? `SIM: navigate_panel{"panel":"ID"}|close_panel|change_speed{"speed":N}|start_simulation|pause_simulation|toggle_simulation|terminate_simulation|toggle_sidebar|god_mode|set_tab{"tab":"harita/durum/kontrol"}|apply_disaster{"disaster":"earthquake/flood/drought/epidemic/volcano/meteor","params":{}}|open_menu|close_menu|open_menu_page{"menuPage":"guide/about"}
Panels: population olaylar language timemachine analysis biology god psychology environment technology belief social economy culture art astronomy hypothesis epigenetics architecture law microbiome` : ''}
${isDash ? `DASH: create_simulation|open_simulation{"index":0}|delete_simulation{"index":0}|toggle_compare|logout` : ''}
GLOBAL: navigate_to{"route":"/"}|toggle_lang|set_lang{"lang":"tr/en"}`;
}

export default function AriaButton() {
  const store = useSimStore();
  const navigate = useNavigate();
  const storeRef = useRef(store);
  storeRef.current = store;

  const [uiState, setUiState] = useState<AriaState>('idle');
  const [transcript, setTranscript] = useState('');
  const [ariaText, setAriaText] = useState('');
  const [lastAction, setLastAction] = useState<string | null>(null);
  const [textInput, setTextInput] = useState('');
  const textInputRef = useRef<HTMLInputElement | null>(null);
  const [pos, setPos] = useState({
    x: 20,
    y: typeof window !== 'undefined' ? window.innerHeight - 100 : 600,
  });
  const posRef = useRef(pos);
  posRef.current = pos;

  const dragging = useRef(false);
  const didDrag = useRef(false);
  const dragStart = useRef({ x: 0, y: 0 });
  const dragOffset = useRef({ dx: 0, dy: 0 });

  const uiRef = useRef<AriaState>('idle');
  const activeRef = useRef(false);
  const processingRef = useRef(false);
  const recRef = useRef<any>(null);
  const watchdogRef = useRef<any>(null);
  const pendingCmdRef = useRef<string | null>(null);
  const retryTimerRef = useRef<any>(null);
  const speakingRef = useRef(false);

  function emitWhenReady(eventName: string, detail: any, readyFlag?: string, tries = 6) {
    if (!readyFlag || (window as any)[readyFlag]) {
      window.dispatchEvent(new CustomEvent(eventName, { detail }));
      return;
    }
    let attempt = 0;
    const tick = () => {
      attempt += 1;
      if ((window as any)[readyFlag]) {
        window.dispatchEvent(new CustomEvent(eventName, { detail }));
        return;
      }
      if (attempt < tries) setTimeout(tick, 120);
    };
    setTimeout(tick, 120);
  }

  function setUI(s: AriaState) { uiRef.current = s; setUiState(s); }
  function armWatchdog() {
    clearTimeout(watchdogRef.current);
    watchdogRef.current = setTimeout(() => {
      if (activeRef.current && processingRef.current) { processingRef.current = false; startListening(); }
    }, 20000);
  }
  function disarmWatchdog() { clearTimeout(watchdogRef.current); }

  useEffect(() => {
    return () => {
      activeRef.current = false;
      processingRef.current = false;
      disarmWatchdog();
      clearTimeout(retryTimerRef.current);
      killRec();
      stopSpeech();
    };
  }, []);

  // Clamp position on screen rotation / resize
  useEffect(() => {
    function onResize() {
      setPos(p => ({
        x: Math.max(0, Math.min(window.innerWidth - 120, p.x)),
        y: Math.max(0, Math.min(window.innerHeight - PILL_H - 8, p.y)),
      }));
    }
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  /* ── Drag mouse ─────────────────────────────────────────────────────────── */
  useEffect(() => {
    function onMove(e: MouseEvent) {
      if (!dragging.current) return;
      if (Math.hypot(e.clientX - dragStart.current.x, e.clientY - dragStart.current.y) > 4) didDrag.current = true;
      setPos({
        x: Math.max(0, Math.min(window.innerWidth - PILL_W, e.clientX - dragOffset.current.dx)),
        y: Math.max(0, Math.min(window.innerHeight - PILL_H, e.clientY - dragOffset.current.dy)),
      });
    }
    function onUp() { dragging.current = false; }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
  }, []);

  function onMouseDown(e: React.MouseEvent) {
    didDrag.current = false;
    dragStart.current = { x: e.clientX, y: e.clientY };
    dragOffset.current = { dx: e.clientX - posRef.current.x, dy: e.clientY - posRef.current.y };
    dragging.current = true;
  }
  function onTouchStart(e: React.TouchEvent) {
    didDrag.current = false; dragging.current = true;
    const t = e.touches[0];
    dragStart.current = { x: t.clientX, y: t.clientY };
    dragOffset.current = { dx: t.clientX - posRef.current.x, dy: t.clientY - posRef.current.y };
  }
  function onTouchMove(e: React.TouchEvent) {
    if (!dragging.current) return;
    e.preventDefault();
    const t = e.touches[0];
    if (Math.hypot(t.clientX - dragStart.current.x, t.clientY - dragStart.current.y) > 4) didDrag.current = true;
    setPos({
      x: Math.max(0, Math.min(window.innerWidth - PILL_W, t.clientX - dragOffset.current.dx)),
      y: Math.max(0, Math.min(window.innerHeight - PILL_H, t.clientY - dragOffset.current.dy)),
    });
  }
  function onTouchEnd() { dragging.current = false; }

  /* ── TTS ────────────────────────────────────────────────────────────────── */
  function speak(text: string, onDone?: () => void) {
    if (typeof window === 'undefined' || !window.speechSynthesis) { onDone?.(); return; }
    window.speechSynthesis.cancel();
    const utt = new SpeechSynthesisUtterance(text);
    utt.lang = resolveLocale(storeRef.current.lang);
    utt.rate = 0.95;
    const voices = window.speechSynthesis.getVoices();
    const locale = resolveLocale(storeRef.current.lang);
    const match = voices.find(v => v.lang.toLowerCase().startsWith(locale.slice(0, 2).toLowerCase()));
    if (match) utt.voice = match;
    speakingRef.current = true;
    utt.onend = () => { speakingRef.current = false; onDone?.(); };
    utt.onerror = () => { speakingRef.current = false; onDone?.(); };
    window.speechSynthesis.speak(utt);
  }
  function stopSpeech() {
    speakingRef.current = false;
    if (typeof window !== 'undefined' && window.speechSynthesis) window.speechSynthesis.cancel();
  }
  function unlockSpeech() {
    if (typeof window === 'undefined' || !window.speechSynthesis) return;
    const utt = new SpeechSynthesisUtterance(' ');
    utt.volume = 0; utt.rate = 10;
    window.speechSynthesis.speak(utt);
  }

  /* ── Toggle ─────────────────────────────────────────────────────────────── */
  function toggle() {
    if (activeRef.current) {
      activeRef.current = false;
      processingRef.current = false;
      disarmWatchdog();
      clearTimeout(retryTimerRef.current);
      killRec(); stopSpeech();
      setUI('idle'); setTranscript(''); setAriaText(''); setTextInput('');
    } else {
      activeRef.current = true;
      processingRef.current = false;
      pendingCmdRef.current = null;
      unlockSpeech();
      startListening();
    }
  }

  /* ── Speech recognition ─────────────────────────────────────────────────── */
  function killRec() {
    const rec = recRef.current;
    if (!rec) return;
    rec.onresult = null; rec.onerror = null; rec.onend = null;
    try { rec.abort(); } catch {} try { rec.stop(); } catch {}
    recRef.current = null;
  }

  function startListening() {
    if (!activeRef.current || speakingRef.current) return;
    killRec();
    if (!SpeechRec) {
      setUI('listening');
      setTimeout(() => textInputRef.current?.focus(), 50);
      return;
    }
    setUI('listening'); setTranscript('');
    const rec = new SpeechRec();
    rec.continuous = true;
    rec.interimResults = true;
    rec.lang = resolveLocale(storeRef.current.lang);
    let idx = 0;
    rec.onresult = (e: any) => {
      if (!activeRef.current) return;
      const results = Array.from(e.results) as any[];
      const live = results.slice(idx).map((r: any) => r[0].transcript).join(' ');
      if (live) setTranscript(live);
      for (let i = idx; i < results.length; i++) {
        if (results[i].isFinal) {
          const cmd = results[i][0].transcript.trim();
          if (!cmd) continue;
          idx = i + 1;
          if (processingRef.current) { pendingCmdRef.current = cmd; }
          else { processingRef.current = true; clearTimeout(retryTimerRef.current); armWatchdog(); killRec(); processCommand(cmd); }
          return;
        }
      }
    };
    rec.onerror = (e: any) => {
      if (!activeRef.current) return;
      if (e.error === 'aborted' || e.error === 'not-allowed') return;
      recRef.current = null;
      if (!processingRef.current) setTimeout(startListening, 800);
    };
    rec.onend = () => {
      recRef.current = null;
      if (!activeRef.current || processingRef.current) return;
      setTimeout(startListening, 200);
    };
    try { rec.start(); recRef.current = rec; } catch { setTimeout(startListening, 1000); }
  }

  /* ── Process command ────────────────────────────────────────────────────── */
  async function processCommand(cmd: string) {
    if (!activeRef.current) { processingRef.current = false; return; }
    setUI('processing');
    const { currentSim, stats, events, lang } = storeRef.current;
    const l = lang as LangCode;
    let responseText = text(l, { tr: 'Tamam.', en: 'Done.', de: 'Erledigt.', fr: 'Terminé.', ar: 'تم.' });

    if (cmd === '__summary__') {
      responseText = !stats
        ? text(l, { tr: 'Simülasyon verisi yok.', en: 'No simulation data.', de: 'Keine Simulationsdaten.', fr: 'Aucune donnée de simulation.', ar: 'لا توجد بيانات محاكاة.' })
        : text(l, {
            tr: `Yıl ${stats.year}, nüfus ${stats.population}. ${stats.technologies} teknoloji. Mutluluk %${Math.round((stats.happiness_index ?? 0.5) * 100)}.`,
            en: `Year ${stats.year}, population ${stats.population}. ${stats.technologies} technologies. Happiness ${Math.round((stats.happiness_index ?? 0.5) * 100)}%.`,
            de: `Jahr ${stats.year}, Bevölkerung ${stats.population}. ${stats.technologies} Technologien. Glück ${Math.round((stats.happiness_index ?? 0.5) * 100)}%.`,
            fr: `Année ${stats.year}, population ${stats.population}. ${stats.technologies} technologies. Bonheur ${Math.round((stats.happiness_index ?? 0.5) * 100)}%.`,
            ar: `السنة ${stats.year}، السكان ${stats.population}. ${stats.technologies} تقنية. السعادة ${Math.round((stats.happiness_index ?? 0.5) * 100)}%.`,
          });
    } else {
      const wizardOpen = !!(window as any).__ariaWizardOpen;
      const ctx = {
        simStatus: currentSim?.status ?? 'none',
        page: wizardOpen ? '/wizard' : window.location.pathname,
        wizardOpen,
        ...(wizardOpen && {
          wizardStep: (window as any).__ariaWizardStep,
          wizardStepType: (window as any).__ariaWizardStepType,
          wizardFounder: (window as any).__ariaWizardFounder,
          wizardTraitName: (window as any).__ariaWizardTraitName,
        }),
      };
      try {
        const { data } = await axios.post('/api/aria/command', {
          message: cmd,
          lang,
          stats,
          events: events.slice(0, 6),
          context: ctx,
        }, {
          headers: { Authorization: `Bearer ${storeRef.current.accessToken}` },
        });

        const actions = Array.isArray(data?.actions) ? data.actions : [];
        actions.forEach((a: any) => executeAction(a));
        if (actions.length === 1) {
          setLastAction(actionLabel(actions[0], l));
          setTimeout(() => setLastAction(null), 3000);
        } else if (actions.length > 1) {
          setLastAction(`✓ ${actions.length}`);
          setTimeout(() => setLastAction(null), 3000);
        }
        responseText = data?.text ?? text(l, { tr: 'Tamam.', en: 'Done.', de: 'Erledigt.', fr: 'Terminé.', ar: 'تم.' });
      } catch (err: any) {
        const status = err?.response?.status;
        if (status === 429) {
          if (!activeRef.current) { processingRef.current = false; return; }
          clearTimeout(retryTimerRef.current);
          const retryAfter = Number(err?.response?.data?.retry_after ?? 60);
          const msg = text(l, {
            tr: `Limit - ${retryAfter}s sonra tekrar deneyin.`,
            en: `Rate limited - try again in ${retryAfter}s.`,
            de: `Limit erreicht - versuchen Sie es in ${retryAfter}s erneut.`,
            fr: `Limite atteinte - réessayez dans ${retryAfter}s.`,
            ar: `تم تجاوز الحد - أعد المحاولة بعد ${retryAfter} ثانية.`,
          });
          setTranscript(''); setAriaText(msg); speak(msg); disarmWatchdog();
          setUI('listening');
          if (SpeechRec) startListening(); else setTimeout(() => textInputRef.current?.focus(), 50);
          setTimeout(() => setAriaText(t => t === msg ? '' : t), 8000);
          return;
        }
        responseText = err?.response?.data?.text ?? text(l, { tr: 'Bağlantı hatası.', en: 'Connection error.', de: 'Verbindungsfehler.', fr: 'Erreur de connexion.', ar: 'خطأ في الاتصال.' });
      }
    }
    setTranscript(''); setAriaText(responseText);
    disarmWatchdog(); processingRef.current = false;

    if (activeRef.current) {
      speak(responseText, () => {
        if (!activeRef.current) return;
        const pending = pendingCmdRef.current; pendingCmdRef.current = null;
        if (pending) {
          processingRef.current = true; armWatchdog(); killRec(); processCommand(pending);
          return;
        }
        if (SpeechRec) startListening();
        else { setUI('listening'); setTimeout(() => textInputRef.current?.focus(), 50); }
      });
      const ms = Math.min(Math.max(2500, responseText.length * 70), 8000);
      setTimeout(() => setAriaText(t => t === responseText ? '' : t), ms);
    } else {
      setAriaText(''); setUI('idle');
    }
  }

  function actionLabel(a: any, lang: LangCode): string {
    switch (a.type) {
      case 'navigate_panel': return `📂 ${a.panel}`;
      case 'change_speed': return `⚡ x${a.speed}`;
      case 'apply_disaster': return `⚠ ${a.disaster}`;
      case 'start_simulation': return `▶ ${text(lang, { tr: 'başlat', en: 'start', de: 'starten', fr: 'démarrer', ar: 'تشغيل' })}`;
      case 'pause_simulation': return `⏸ ${text(lang, { tr: 'duraklat', en: 'pause', de: 'pausieren', fr: 'pause', ar: 'إيقاف مؤقت' })}`;
      case 'toggle_simulation': return `⏯ ${text(lang, { tr: 'sim', en: 'sim', de: 'sim', fr: 'sim', ar: 'محاكاة' })}`;
      case 'terminate_simulation': return `⏹ ${text(lang, { tr: 'sonlandır', en: 'terminate', de: 'beenden', fr: 'terminer', ar: 'إنهاء' })}`;
      case 'navigate_to': return `→ ${a.route}`;
      case 'create_simulation': return `✚ ${text(lang, { tr: 'yeni sim', en: 'new sim', de: 'neue Sim', fr: 'nouvelle sim', ar: 'محاكاة جديدة' })}`;
      case 'open_simulation': return `▶ ${text(lang, { tr: 'sim', en: 'sim', de: 'sim', fr: 'sim', ar: 'محاكاة' })} #${(a.index ?? 0) + 1}`;
      case 'wizard_next': return `→ ${text(lang, { tr: 'ileri', en: 'next', de: 'weiter', fr: 'suivant', ar: 'التالي' })}`;
      case 'wizard_back': return `← ${text(lang, { tr: 'geri', en: 'back', de: 'zurück', fr: 'retour', ar: 'رجوع' })}`;
      case 'wizard_submit': return `🚀 ${text(lang, { tr: 'başlat', en: 'start', de: 'starten', fr: 'démarrer', ar: 'تشغيل' })}`;
      case 'wizard_set': return `✎ ${a.field}=${a.value}`;
      default: return a.type;
    }
  }

  /* ── Execute action ─────────────────────────────────────────────────────── */
  function executeAction(action: any) {
    if (!action?.type) return;
    const { currentSim, accessToken, setActivePanel, setSpeed, toggleLang, setLang, setCurrentSim, logout, stats } = storeRef.current;
    switch (action.type) {
      case 'navigate_panel': case 'open_panel': setActivePanel(action.panel ?? null); break;
      case 'close_panel': setActivePanel(null); break;
      case 'change_speed': case 'set_speed': {
        const requested = Number(action.speed) || 1;
        const spd = ALLOWED_SPEEDS.includes(requested) ? requested : 1;
        setSpeed(spd);
        if (currentSim && accessToken)
          axios.post(`/api/simulations/${currentSim.id}/speed`, { speed_multiplier: spd }, { headers: { Authorization: `Bearer ${accessToken}` } }).catch(() => {});
        break;
      }
      case 'toggle_simulation':
        if (currentSim && accessToken) {
          const next = currentSim.status === 'running' ? 'pause' : 'start';
          axios.post(`/api/simulations/${currentSim.id}/${next}`, {}, { headers: { Authorization: `Bearer ${accessToken}` } })
            .then(() => setCurrentSim({ ...currentSim, status: next === 'start' ? 'running' : 'paused' })).catch(() => {});
        }
        break;
      case 'start_simulation':
        if (currentSim && accessToken && currentSim.status !== 'running')
          axios.post(`/api/simulations/${currentSim.id}/start`, {}, { headers: { Authorization: `Bearer ${accessToken}` } })
            .then(() => setCurrentSim({ ...currentSim, status: 'running' })).catch(() => {});
        break;
      case 'pause_simulation':
        if (currentSim && accessToken && currentSim.status === 'running')
          axios.post(`/api/simulations/${currentSim.id}/pause`, {}, { headers: { Authorization: `Bearer ${accessToken}` } })
            .then(() => setCurrentSim({ ...currentSim, status: 'paused' })).catch(() => {});
        break;
      case 'apply_disaster':
        if (currentSim && accessToken) {
          // Geographically-targeted disasters default to the population's
          // actual centroid (see routes.rs's derive_stats), not "null
          // island" -- the LLM's own params rarely include lat/lon, so this
          // is the fallback that actually matters in practice, same as
          // GodPanel's manual buttons.
          const geoDisasters = ['earthquake', 'flood', 'volcano', 'meteor'];
          const params = geoDisasters.includes(action.disaster)
            ? { lat: stats?.centroid_y ?? 0, lon: stats?.centroid_x ?? 0, ...(action.params ?? {}) }
            : (action.params ?? {});
          axios.post(`/api/god/${currentSim.id}/intervene`, { type: action.disaster, params, user_note: 'ARIA' }, { headers: { Authorization: `Bearer ${accessToken}` } }).catch(() => {});
        }
        break;
      case 'navigate_to': if (action.route) navigate(action.route); break;
      case 'set_tab': window.dispatchEvent(new CustomEvent('aria-set-tab', { detail: action.tab })); break;
      case 'toggle_lang': toggleLang(); break;
      case 'set_lang': if (action.lang) setLang(action.lang); break;
      case 'god_mode': setActivePanel('god'); break;
      case 'toggle_sidebar': window.dispatchEvent(new CustomEvent('aria-simulation', { detail: { action: 'toggle_sidebar' } })); break;
      case 'terminate_simulation': window.dispatchEvent(new CustomEvent('aria-simulation', { detail: { action: 'terminate_simulation' } })); break;
      case 'open_menu':
        window.dispatchEvent(new CustomEvent('aria-simulation', { detail: { action: 'open_menu' } }));
        emitWhenReady('aria-dashboard', { action: 'open_menu' }, '__ariaDashboardReady'); break;
      case 'open_menu_page': {
        const d = { action: 'open_menu_page', menuPage: action.menuPage };
        window.dispatchEvent(new CustomEvent('aria-simulation', { detail: d }));
        emitWhenReady('aria-dashboard', d, '__ariaDashboardReady'); break;
      }
      case 'close_menu':
        window.dispatchEvent(new CustomEvent('aria-simulation', { detail: { action: 'close_menu' } }));
        emitWhenReady('aria-dashboard', { action: 'close_menu' }, '__ariaDashboardReady'); break;
      case 'logout': logout(); navigate('/login'); break;
      case 'create_simulation': emitWhenReady('aria-dashboard', { action: 'create_simulation' }, '__ariaDashboardReady'); break;
      case 'open_simulation': emitWhenReady('aria-dashboard', { action: 'open_simulation', index: action.index ?? 0 }, '__ariaDashboardReady'); break;
      case 'toggle_compare': emitWhenReady('aria-dashboard', { action: 'toggle_compare' }, '__ariaDashboardReady'); break;
      case 'delete_simulation': emitWhenReady('aria-dashboard', { action: 'delete_simulation', index: action.index ?? 0 }, '__ariaDashboardReady'); break;
      case 'wizard_next': emitWhenReady('aria-wizard', { action: 'next' }, '__ariaWizardReady'); break;
      case 'wizard_back': emitWhenReady('aria-wizard', { action: 'back' }, '__ariaWizardReady'); break;
      case 'wizard_submit': emitWhenReady('aria-wizard', { action: 'submit' }, '__ariaWizardReady'); break;
      case 'wizard_exit':
        emitWhenReady('aria-wizard', { action: 'exit' }, '__ariaWizardReady');
        emitWhenReady('aria-dashboard', { action: 'wizard_exit' }, '__ariaDashboardReady'); break;
      case 'wizard_set':
        emitWhenReady('aria-wizard', { action: 'set', field: action.field, value: action.value, founder: action.founder }, '__ariaWizardReady'); break;
    }
  }

  function submitText() {
    const cmd = textInput.trim();
    if (!cmd || processingRef.current) return;
    setTextInput('');
    processingRef.current = true; armWatchdog(); processCommand(cmd);
  }

  /* ── Visual ─────────────────────────────────────────────────────────────── */
  const COLORS: Record<AriaState, string> = { idle: '#00e887', listening: '#f97316', processing: '#a855f7' };
  const color = COLORS[uiState];
  const Icon = uiState === 'processing' ? Loader2 : uiState === 'listening' ? Mic : MicOff;
  const ARIA_LABELS: Record<string, Record<string, string>> = {
    tr: { idle: 'ASİSTAN', listening: 'DİNLİYOR…', processing: 'İŞLENİYOR…' },
    en: { idle: 'ASSISTANT', listening: 'LISTENING…', processing: 'PROCESSING…' },
    de: { idle: 'ASSISTENT', listening: 'HÖRT ZU…', processing: 'VERARBEITET…' },
    fr: { idle: 'ASSISTANT', listening: 'ÉCOUTE…', processing: 'TRAITEMENT…' },
    ar: { idle: 'مساعد', listening: 'يستمع…', processing: 'يعالج…' },
  };
  const label = (ARIA_LABELS[store.lang] ?? ARIA_LABELS.en)[uiState];
  const hasOverlay = uiState !== 'idle' && (transcript || ariaText);

  return (
    <>
      <button
        onClick={() => { if (!didDrag.current) toggle(); didDrag.current = false; }}
        onMouseDown={onMouseDown}
        onTouchStart={onTouchStart}
        onTouchMove={onTouchMove}
        onTouchEnd={onTouchEnd}
        style={{
          position: 'fixed', left: pos.x, top: pos.y, zIndex: 9999,
          height: PILL_H, borderRadius: 24, padding: '0 16px 0 10px',
          display: 'flex', alignItems: 'center', gap: 8,
          background: uiState !== 'idle' ? `${color}22` : 'rgba(4,4,18,0.92)',
          border: `2px solid ${color}`, color,
          fontFamily: 'Share Tech Mono, monospace', fontSize: 12, fontWeight: 700, letterSpacing: '0.1em',
          boxShadow: uiState !== 'idle' ? `0 0 18px ${color}90, 0 0 36px ${color}40` : `0 0 10px ${color}50`,
          touchAction: 'none', cursor: 'grab', userSelect: 'none', WebkitUserSelect: 'none',
          transition: 'box-shadow 0.3s, background 0.3s, border-color 0.3s',
        }}>
        <Icon size={18} className={uiState === 'processing' ? 'animate-spin' : ''}
          style={{ filter: `drop-shadow(0 0 6px ${color})`, flexShrink: 0 }} />
        <span style={{ whiteSpace: 'nowrap' }}>{label}</span>
        {uiState !== 'idle' && (
          <span style={{ width: 7, height: 7, borderRadius: '50%', background: color,
            boxShadow: `0 0 8px ${color}`, animation: 'pulse 1s infinite', flexShrink: 0 }} />
        )}
      </button>

      {uiState !== 'idle' && (
        <div style={{
          position: 'fixed',
          left: Math.max(0, Math.min(window.innerWidth - 280, pos.x)),
          top: Math.max(0, pos.y - (hasOverlay ? 90 : 48)),
          zIndex: 9998, background: '#07071aee', border: `1px solid ${color}55`,
          borderRadius: 8, padding: '7px 11px', maxWidth: 280,
          boxShadow: `0 0 12px ${color}33`,
          pointerEvents: !SpeechRec && uiState === 'listening' ? 'auto' : 'none',
        }}>
          {transcript && (
            <div style={{ color: '#a0b4ff', fontSize: 10, marginBottom: ariaText ? 6 : 0, fontFamily: 'Share Tech Mono, monospace' }}>
              "{transcript.slice(0, 120)}{transcript.length > 120 ? '…' : ''}"
            </div>
          )}
          {ariaText && (
            <div style={{ color: '#00e887', fontSize: 11, fontFamily: 'Share Tech Mono, monospace',
              lineHeight: 1.5, marginBottom: 4, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
              {ariaText}
            </div>
          )}
          {!SpeechRec && uiState === 'listening' && (
            <div style={{ display: 'flex', gap: 4, marginBottom: 4 }}>
              <input ref={textInputRef} value={textInput} onChange={e => setTextInput(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') submitText(); }}
                placeholder={text(store.lang as LangCode, { tr: 'komut yaz…', en: 'type command…', de: 'Befehl eingeben…', fr: 'entrer commande…', ar: 'أدخل أمراً…' })}
                style={{ flex: 1, background: 'transparent', border: '1px solid #00e88766', borderRadius: 4,
                  color: '#00e887', fontFamily: 'Share Tech Mono, monospace', fontSize: 10,
                  padding: '3px 6px', outline: 'none', pointerEvents: 'auto' }} />
              <button onClick={submitText}
                style={{ background: '#00e88722', border: '1px solid #00e88766', borderRadius: 4,
                  color: '#00e887', fontFamily: 'Share Tech Mono, monospace', fontSize: 10,
                  padding: '3px 7px', cursor: 'pointer', pointerEvents: 'auto' }}>↵</button>
            </div>
          )}
          <div style={{ fontSize: 9, color: '#2a6a48', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.04em' }}>
            {text(store.lang as LangCode, { tr: '[ tekrar bas = durdur ]', en: '[ press again = stop ]', de: '[ nochmal drücken = stop ]', fr: '[ appuyer encore = stop ]', ar: '[ اضغط مجدداً = إيقاف ]' })}
          </div>
        </div>
      )}

      {lastAction && (
        <div style={{
          position: 'fixed',
          left: Math.max(0, Math.min(window.innerWidth - 180, pos.x + PILL_W + 8)),
          top: pos.y + (PILL_H - 26) / 2, zIndex: 9998,
          background: 'rgba(0,232,135,0.12)', border: '1px solid #00e88766',
          borderRadius: 6, padding: '4px 10px', fontSize: 10, color: '#00e887',
          fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.08em',
          pointerEvents: 'none', whiteSpace: 'nowrap',
        }}>
          ✓ {lastAction}
        </div>
      )}
    </>
  );
}
