import { useEffect, useState, useRef, useCallback, lazy, Suspense, type CSSProperties } from 'react';
import FooterBar from '../components/layout/FooterBar';
import { useParams, useNavigate, useLocation } from 'react-router-dom';
import axios from 'axios';
import { Play, Pause, LogOut, ChevronLeft, ChevronRight, Users, Globe, Zap, Shield, Flame, Heart, Trash2, Sparkles, BookOpen, CircleSlash2, FastForward, X, Settings } from 'lucide-react';
import { useSimStore } from '../store/simStore';
import ErrorBoundary from '../components/layout/ErrorBoundary';
import { useSimWebSocket } from '../hooks/useSimWebSocket';
import SimMenuOverlay from '../components/layout/SimMenuOverlay';
import WorldGlobe from '../components/simulation/WorldGlobe';
import StatsPanel from '../components/panels/StatsPanel';
import MilestoneToast from '../components/simulation/MilestoneToast';
import WitnessPanel from '../components/simulation/WitnessPanel';
import { translateEventDescription, translateSeason, text, type LangCode } from '../utils/i18n';
import { isLocalOrigin } from '../utils/cloud';

// Toggleable overlay panels are lazy-loaded: they're all mounted at once but
// only one is ever visible, so splitting them out of the main chunk (and the
// heavy libs a few of them pull in, e.g. recharts/jspdf/html2canvas) shrinks
// the initial bundle without changing when a panel actually activates.
const PopulationPanel = lazy(() => import('../components/panels/PopulationPanel'));
const BiologyPanel = lazy(() => import('../components/panels/BiologyPanel'));
const EnvironmentPanel = lazy(() => import('../components/panels/EnvironmentPanel'));
const AstronomyPanel = lazy(() => import('../components/panels/AstronomyPanel'));
const CulturePanel = lazy(() => import('../components/panels/CulturePanel'));
const LanguagePanel = lazy(() => import('../components/panels/LanguagePanel'));
const TechnologyPanel = lazy(() => import('../components/panels/TechnologyPanel'));
const BeliefPanel = lazy(() => import('../components/panels/BeliefPanel'));
const SocialPanel = lazy(() => import('../components/panels/SocialPanel'));
const EconomyPanel = lazy(() => import('../components/panels/EconomyPanel'));
const ArtPanel = lazy(() => import('../components/panels/ArtPanel'));
const ArchitecturePanel = lazy(() => import('../components/panels/ArchitecturePanel'));
const LawPanel = lazy(() => import('../components/panels/LawPanel'));
const MicrobiomePanel = lazy(() => import('../components/panels/MicrobiomePanel'));
const PsychologyPanel = lazy(() => import('../components/panels/PsychologyPanel'));
const EpigeneticsPanel = lazy(() => import('../components/panels/EpigeneticsPanel'));
const GeneticDiversityPanel = lazy(() => import('../components/panels/GeneticDiversityPanel'));
const GodPanel = lazy(() => import('../components/panels/GodPanel'));
const GenealogyPanel = lazy(() => import('../components/panels/GenealogyPanel'));
const TimeMachinePanel = lazy(() => import('../components/panels/TimeMachinePanel'));
const AnalysisPanel = lazy(() => import('../components/panels/AnalysisPanel'));
const HypothesisPanel = lazy(() => import('../components/panels/HypothesisPanel'));
const MomentsPanel = lazy(() => import('../components/panels/MomentsPanel'));
const PopulationPyramidPanel = lazy(() => import('../components/panels/PopulationPyramidPanel'));
const ReportPanel = lazy(() => import('../components/panels/ReportPanel'));
const EventsPanel = lazy(() => import('../components/panels/EventsPanel'));
const PerformancePanel = lazy(() => import('../components/panels/PerformancePanel'));
const LegendsPanel = lazy(() => import('../components/panels/LegendsPanel'));
const DocumentaryPanel = lazy(() => import('../components/panels/DocumentaryPanel'));

const SPEEDS = [1, 5, 20, 100];

const MODULES = [
  { id: 'population',   labels: { tr: 'NÜFUS',      en: 'POPUL.',   de: 'BEVÖLK.',  fr: 'POPUL.',   ar: 'السكان'     }, icon: '👥' },
  { id: 'olaylar',      labels: { tr: 'OLAYLAR',    en: 'EVENTS',   de: 'EREIGN.',  fr: 'ÉVÉNEM.',  ar: 'الأحداث'    }, icon: '📋', special: true },
  { id: 'language',     labels: { tr: 'DİL',        en: 'LANG.',    de: 'SPRACHE',  fr: 'LANGUE',   ar: 'اللغة'      }, icon: '🔤' },
  { id: 'timemachine',  labels: { tr: 'GEÇMİŞ',     en: 'HISTORY',  de: 'GESCH.',   fr: 'HIST.',    ar: 'التاريخ'    }, icon: '⏳' },
  { id: 'analysis',     labels: { tr: 'ANALİZ',     en: 'ANALYS.',  de: 'ANALYSE',  fr: 'ANALYSE',  ar: 'التحليل'    }, icon: '📊' },
  { id: 'pyramid',      labels: { tr: 'PİRAMİT',    en: 'PYRAMID',  de: 'PYRAMIDE',  fr: 'PYRAMIDE',  ar: 'الهرم'     }, icon: '📐' },
  { id: 'biology',      labels: { tr: 'MUTASYON',   en: 'MUTAT.',   de: 'MUTAT.',   fr: 'MUTAT.',   ar: 'الطفرة'     }, icon: '🧬' },
  { id: 'god',          labels: { tr: 'TANRI',      en: 'GOD',      de: 'GOTT',     fr: 'DIEU',     ar: 'الإله'      }, icon: '✦',  accent: '#f97316' },
  { id: 'psychology',   labels: { tr: 'AKIL',       en: 'MIND',     de: 'GEIST',    fr: 'ESPRIT',   ar: 'العقل'      }, icon: '🧠' },
  { id: 'environment',  labels: { tr: 'ÇEVRE',      en: 'ENV.',     de: 'UMWELT',   fr: 'ENVIRON.', ar: 'البيئة'     }, icon: '🌿' },
  { id: 'technology',   labels: { tr: 'TEKNOLOJİ',  en: 'TECH',     de: 'TECHNOL.', fr: 'TECHNOL.', ar: 'التكنول.'   }, icon: '⚙' },
  { id: 'belief',       labels: { tr: 'İNANÇ',      en: 'BELIEF',   de: 'GLAUBE',   fr: 'CROYANCE', ar: 'المعتقد'    }, icon: '☽' },
  { id: 'social',       labels: { tr: 'SOSYAL',     en: 'SOCIAL',   de: 'SOZIAL',   fr: 'SOCIAL',   ar: 'الاجتماعي'  }, icon: '🤝' },
  { id: 'economy',      labels: { tr: 'EKONOMİ',    en: 'ECON.',    de: 'WIRTSCH.', fr: 'ÉCON.',    ar: 'الاقتصاد'   }, icon: '💰' },
  { id: 'culture',      labels: { tr: 'KÜLTÜR',     en: 'CULT.',    de: 'KULTUR',   fr: 'CULTURE',  ar: 'الثقافة'    }, icon: '🎭' },
  { id: 'art',          labels: { tr: 'SANAT',      en: 'ART',      de: 'KUNST',    fr: 'ART',      ar: 'الفن'       }, icon: '🎨' },
  { id: 'astronomy',    labels: { tr: 'ASTRONOMİ',  en: 'ASTRO.',   de: 'ASTRON.',  fr: 'ASTRON.',  ar: 'الفلك'      }, icon: '🌙' },
  { id: 'moments',      labels: { tr: 'ANLAR',      en: 'MOMENTS',  de: 'MOMENTE',  fr: 'MOMENTS',  ar: 'اللحظات'    }, icon: '✦' },
  { id: 'legends',      labels: { tr: 'EFSANELER',  en: 'LEGENDS',  de: 'LEGENDEN', fr: 'LÉGENDES', ar: 'الأساطير'   }, icon: '🏆' },
  { id: 'documentary',  labels: { tr: 'BELGESEL',   en: 'DOC.',     de: 'DOKU.',    fr: 'DOC.',     ar: 'وثائقي'     }, icon: '🎬' },
  { id: 'hypothesis',   labels: { tr: 'HİPOTEZ',    en: 'HYPOTH.',  de: 'HYPOTH.',  fr: 'HYPOTH.',  ar: 'الفرضية'    }, icon: '💡' },
  { id: 'epigenetics',  labels: { tr: 'EPİGEN.',    en: 'EPIGEN.',  de: 'EPIGEN.',  fr: 'ÉPIGEN.',  ar: 'التخلق'     }, icon: '🔬' },
  { id: 'genetics',     labels: { tr: 'ÇEŞİTLİLİK', en: 'DIVERSITY',de: 'VIELFALT', fr: 'DIVERSITÉ',ar: 'التنوع'     }, icon: '🧫' },
  { id: 'architecture', labels: { tr: 'MİMARİ',     en: 'ARCH.',    de: 'ARCHIT.',  fr: 'ARCHIT.',  ar: 'العمارة'    }, icon: '🏛️' },
  { id: 'law',          labels: { tr: 'HUKUK',      en: 'LAW',      de: 'RECHT',    fr: 'DROIT',    ar: 'القانون'    }, icon: '⚖️' },
  { id: 'microbiome',   labels: { tr: 'MİKROBİYOM', en: 'MICROB.',  de: 'MIKROBIO.',fr: 'MICROB.',  ar: 'الميكروبيوم'}, icon: '🦠' },
];

const TABS = [
  { id: 'harita',  labels: { tr: 'HARİTA',  en: 'MAP',    de: 'KARTE',  fr: 'CARTE',  ar: 'الخريطة' } },
  { id: 'durum',   labels: { tr: 'DURUM',   en: 'STATUS', de: 'STATUS', fr: 'STATUT', ar: 'الحالة'  } },
];

const IMPORTANT_TYPES = ['birth', 'death', 'language', 'belief', 'technology', 'word', 'discovery'];

const SHOWCASE_BUTTONS = [
  { labels: { tr: 'BAŞLAT',     en: 'START',  de: 'START',    fr: 'DÉMARRER', ar: 'ابدأ'    }, icon: Play,        tone: '#00e887', bg: 'rgba(0,232,135,0.12)',   border: 'rgba(0,232,135,0.55)' },
  { labels: { tr: 'DURDUR',     en: 'PAUSE',  de: 'PAUSE',    fr: 'PAUSE',    ar: 'إيقاف'   }, icon: Pause,       tone: '#e05a5a', bg: 'rgba(224,90,90,0.12)',   border: 'rgba(224,90,90,0.55)' },
  { labels: { tr: 'HIZLANDIR',  en: 'BOOST',  de: 'BOOST',    fr: 'BOOST',    ar: 'تسريع'   }, icon: Zap,         tone: '#d4a838', bg: 'rgba(212,168,56,0.12)',  border: 'rgba(212,168,56,0.55)' },
  { labels: { tr: 'KORUMA',     en: 'SHIELD', de: 'SCHUTZ',   fr: 'BOUCLIER', ar: 'حماية'   }, icon: Shield,      tone: '#4f6ef7', bg: 'rgba(79,110,247,0.12)',  border: 'rgba(79,110,247,0.55)' },
  { labels: { tr: 'YANGIN',     en: 'FIRE',   de: 'FEUER',    fr: 'FEU',      ar: 'نار'     }, icon: Flame,       tone: '#f97316', bg: 'rgba(249,115,22,0.12)',  border: 'rgba(249,115,22,0.55)' },
  { labels: { tr: 'SEVGİ',      en: 'BOND',   de: 'BINDUNG',  fr: 'LIEN',     ar: 'رابطة'   }, icon: Heart,       tone: '#ff8ab0', bg: 'rgba(255,138,176,0.12)', border: 'rgba(255,138,176,0.55)' },
  { labels: { tr: 'YENİLE',     en: 'RESET',  de: 'RESET',    fr: 'RESET',    ar: 'إعادة'   }, icon: Sparkles,    tone: '#7dd3fc', bg: 'rgba(125,211,252,0.12)', border: 'rgba(125,211,252,0.55)' },
  { labels: { tr: 'KILAVUZ',    en: 'GUIDE',  de: 'ANLEITUNG',fr: 'GUIDE',    ar: 'دليل'    }, icon: BookOpen,    tone: '#a0b4ff', bg: 'rgba(160,180,255,0.12)', border: 'rgba(160,180,255,0.55)' },
  { labels: { tr: 'SİL',        en: 'DELETE', de: 'LÖSCHEN',  fr: 'SUPPR.',   ar: 'حذف'     }, icon: Trash2,      tone: '#ff6b6b', bg: 'rgba(255,107,107,0.12)', border: 'rgba(255,107,107,0.55)' },
  { labels: { tr: 'DEVRE DIŞI', en: 'OFF',    de: 'AUS',      fr: 'OFF',      ar: 'إيقاف'   }, icon: CircleSlash2,tone: '#8abda0', bg: 'rgba(138,189,160,0.12)', border: 'rgba(138,189,160,0.4)' },
];

function ButtonShowcase({ lang, onClose }: { lang: string; onClose: () => void }) {
  return (
    <div
      style={{
        position: 'absolute',
        top: 78,
        right: 8,
        zIndex: 60,
        width: 290,
        maxWidth: 'calc(100vw - 16px)',
        background: 'rgba(2,6,16,0.96)',
        border: '1px solid rgba(79,110,247,0.35)',
        boxShadow: '0 18px 60px rgba(0,0,0,0.55)',
        backdropFilter: 'blur(14px)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 10px', borderBottom: '1px solid rgba(79,110,247,0.2)' }}>
        <div>
          <div style={{ fontSize: 11, color: '#a0b4ff', letterSpacing: '0.18em' }}>BUTTON SAMPLE</div>
          <div style={{ fontSize: 14, color: '#e0e0f0', letterSpacing: '0.08em' }}>{text(lang as LangCode, { tr: 'Örnek Butonlar', en: 'Button Showcase', de: 'Schaltflächen', fr: 'Boutons', ar: 'أزرار نموذجية' })}</div>
        </div>
        <button onClick={onClose} style={{ color: '#a0c8b0', border: 'none', background: 'transparent', cursor: 'pointer', fontSize: 16, lineHeight: 1 }}>✕</button>
      </div>

      <div style={{ padding: 10 }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          {SHOWCASE_BUTTONS.map((btn) => {
            const Icon = btn.icon;
            return (
              <button
                key={btn.labels.tr}
                type="button"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 7,
                  minHeight: 40,
                  padding: '8px 10px',
                  borderRadius: 10,
                  border: `1px solid ${btn.border}`,
                  background: btn.bg,
                  color: btn.tone,
                  cursor: 'pointer',
                  textAlign: 'left',
                  boxShadow: `inset 0 0 0 1px rgba(255,255,255,0.02)`,
                }}
              >
                <Icon size={14} />
                <span style={{ display: 'flex', flexDirection: 'column', lineHeight: 1.1 }}>
                  <span style={{ fontSize: 12, letterSpacing: '0.08em', fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>
                    {text(lang as LangCode, btn.labels)}
                  </span>
                  <span style={{ fontSize: 10, color: 'rgba(220,230,255,0.72)', letterSpacing: '0.04em' }}>
                    {btn.border.includes('255,107,107') ? 'critical' : btn.border.includes('0,232,135') ? 'primary' : btn.border.includes('249,115,22') ? 'alert' : 'utility'}
                  </span>
                </span>
              </button>
            );
          })}
        </div>

        <div style={{ marginTop: 10, display: 'flex', gap: 8 }}>
          <button style={{ flex: 1, padding: '8px 10px', borderRadius: 999, border: '1px solid rgba(0,232,135,0.45)', background: 'rgba(0,232,135,0.08)', color: '#00e887', cursor: 'pointer', letterSpacing: '0.08em' }}>
            {text(lang as LangCode, { tr: 'PİL', en: 'PILL', de: 'PILLE', fr: 'PILULE', ar: 'حبة' })}
          </button>
          <button style={{ flex: 1, padding: '8px 10px', borderRadius: 999, border: '1px solid rgba(160,180,255,0.45)', background: 'rgba(160,180,255,0.08)', color: '#a0b4ff', cursor: 'pointer', letterSpacing: '0.08em' }}>
            {text(lang as LangCode, { tr: 'İKİLİ', en: 'SEGMENT', de: 'SEGMENT', fr: 'SEGMENT', ar: 'قطعة' })}
          </button>
          <button style={{ flex: 1, padding: '8px 10px', borderRadius: 999, border: '1px dashed rgba(212,168,56,0.45)', background: 'rgba(212,168,56,0.08)', color: '#d4a838', cursor: 'pointer', letterSpacing: '0.08em' }}>
            {text(lang as LangCode, { tr: 'ÖNEMLİ', en: 'PRIMARY', de: 'PRIMÄR', fr: 'PRINCIPAL', ar: 'رئيسي' })}
          </button>
        </div>
      </div>
    </div>
  );
}

function DraggableLogPanel({ events, lang, fmtEvent, eventColor }: {
  events: any[]; lang: string; fmtEvent: (ev: any) => string; eventColor: (ev: any, i: number) => string;
}) {
  const isMob = typeof window !== 'undefined' && window.innerWidth < 640;
  const [pos, setPos] = useState({ x: 8, y: 8 });
  const [dragging, setDragging] = useState(false);
  const dragStart = useRef({ clientX: 0, clientY: 0, panelX: 0, panelY: 0 });

  useEffect(() => {
    if (!dragging) return;
    function onMove(e: MouseEvent | TouchEvent) {
      const cx = 'touches' in e ? e.touches[0].clientX : (e as MouseEvent).clientX;
      const cy = 'touches' in e ? e.touches[0].clientY : (e as MouseEvent).clientY;
      setPos({ x: dragStart.current.panelX + cx - dragStart.current.clientX, y: dragStart.current.panelY + cy - dragStart.current.clientY });
    }
    function onUp() { setDragging(false); }
    window.addEventListener('mousemove', onMove as any);
    window.addEventListener('mouseup', onUp);
    window.addEventListener('touchmove', onMove as any, { passive: false });
    window.addEventListener('touchend', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove as any);
      window.removeEventListener('mouseup', onUp);
      window.removeEventListener('touchmove', onMove as any);
      window.removeEventListener('touchend', onUp);
    };
  }, [dragging]);

  function startDrag(clientX: number, clientY: number) {
    setDragging(true);
    dragStart.current = { clientX, clientY, panelX: pos.x, panelY: pos.y };
  }

  const filtered = events
    .filter(ev => IMPORTANT_TYPES.some(t => ev.event_type?.toLowerCase().includes(t)))
    .slice(0, 3);

  const panelW = isMob ? Math.min(200, window.innerWidth - 60) : 250;

  return (
    <div style={{
      position: 'absolute', left: pos.x, top: pos.y, zIndex: 30,
      width: panelW, background: 'rgba(0,4,2,0.93)', border: '1px solid #4a1a1a',
      userSelect: 'none', cursor: dragging ? 'grabbing' : 'default',
      boxShadow: '0 4px 20px rgba(0,0,0,0.7)',
    }}>
      {/* Drag handle — mouse + touch */}
      <div
        onMouseDown={e => { startDrag(e.clientX, e.clientY); e.preventDefault(); }}
        onTouchStart={e => { startDrag(e.touches[0].clientX, e.touches[0].clientY); }}
        style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '4px 8px', borderBottom: '1px solid #4a1a1a', cursor: 'grab', background: 'rgba(0,20,10,0.9)', touchAction: 'none' }}>
        <div style={{ width: 3, height: 12, background: '#00e887', boxShadow: '0 0 4px #00e887', flexShrink: 0 }} />
        <span style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.2em', fontFamily: 'Share Tech Mono, monospace', flex: 1 }}>
          {text(lang as LangCode, { tr: 'OLAY KAYDI', en: 'EVENT LOG', de: 'EREIGNISLOG', fr: 'JOURNAL', ar: 'سجل الأحداث' })}
        </span>
        <div style={{ width: 5, height: 5, borderRadius: '50%', background: '#00e887', flexShrink: 0, animation: 'pulse 1.5s infinite' }} />
        <span style={{ fontSize: 14, color: '#4ecb71', letterSpacing: '0.1em', fontFamily: 'Share Tech Mono, monospace' }}>
          {text(lang as LangCode, { tr: 'CANLI', en: 'LIVE', de: 'LIVE', fr: 'EN DIRECT', ar: 'مباشر' })}
        </span>
      </div>
      {/* Events — 3 rows max */}
      <div style={{ padding: '2px 6px' }}>
        {filtered.length === 0 ? (
          <div style={{ fontSize: 14, color: '#a0c8b0', padding: '4px 0', fontFamily: 'Share Tech Mono, monospace', fontStyle: 'italic' }}>
            {text(lang as LangCode, { tr: 'Olay bekleniyor...', en: 'Awaiting events...', de: 'Warte auf Ereignisse...', fr: 'En attente d\'événements...', ar: 'في انتظار الأحداث...' })}
          </div>
        ) : filtered.map((ev, i) => (
          <div key={i} style={{
            fontSize: 14, color: eventColor(ev, i), lineHeight: '1.55',
            fontFamily: 'Share Tech Mono, monospace',
            whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
            borderBottom: i < filtered.length - 1 ? '1px solid rgba(0,50,20,0.3)' : 'none',
            opacity: Math.max(0.5, 1 - i * 0.15),
          }}>
            {fmtEvent(ev)}
          </div>
        ))}
      </div>
    </div>
  );
}

export default function SimulationPage() {
  const { simId } = useParams<{ simId: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { user, accessToken, setCurrentSim, currentSim, stats, setStats, events, activePanel, setActivePanel, lang, speedMultiplier, setSpeed, resetLiveState, setEvents, simulationEnded, clearSimulationEnded, isWarping, fastForwardTarget, setSettingsOpen } = useSimStore();
  const [individuals, setIndividuals] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'harita' | 'durum'>('harita');
  const [realTime, setRealTime] = useState('');
  const [sidebarExpanded, setSidebarExpanded] = useState(() => typeof window !== 'undefined' && window.innerWidth >= 640);
  const [rightPanelExpanded, setRightPanelExpanded] = useState(() => typeof window !== 'undefined' && window.innerWidth >= 640);
  const [selectedInd, setSelectedInd] = useState<any>(null);
  const [globeCoord, setGlobeCoord] = useState<{ lat: number; lon: number } | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [customSpeed, setCustomSpeed] = useState('');
  const [menuPage, setMenuPage] = useState<'guide' | 'about' | 'mission' | 'contact' | null>(null);
  const [isMobile, setIsMobile] = useState(() => typeof window !== 'undefined' && window.innerWidth < 640);
  const [actionBusy, setActionBusy] = useState(false);
  const [speedBusy, setSpeedBusy] = useState(false);
  const [endModal, setEndModal] = useState<{ mode: 'natural' | 'manual'; reason?: string } | null>(null);
  const [ffTarget, setFfTarget] = useState('');
  const [ffError, setFfError] = useState('');
  const [simNotFound, setSimNotFound] = useState(false);
  const [introTarget, setIntroTarget] = useState<{ lat: number; lon: number } | null>(() => {
    const navTarget = (location.state as any)?.introTarget;
    if (navTarget && Number.isFinite(navTarget.lat) && Number.isFinite(navTarget.lon)) {
      return { lat: Number(navTarget.lat), lon: Number(navTarget.lon) };
    }
    return null;
  });

  // Show end modal when simulation ends naturally
  useEffect(() => {
    if (simulationEnded) {
      setEndModal({ mode: 'natural', reason: simulationEnded });
      clearSimulationEnded();
    }
  }, [simulationEnded]);

  // Responsive breakpoint
  useEffect(() => {
    const handler = () => setIsMobile(window.innerWidth < 640);
    window.addEventListener('resize', handler);
    return () => window.removeEventListener('resize', handler);
  }, []);

  // ARIA tab-switching command listener
  useEffect(() => {
    function onAriaTab(e: Event) {
      const tab = (e as CustomEvent).detail;
      if (tab === 'harita' || tab === 'durum') setActiveTab(tab);
    }
    window.addEventListener('aria-set-tab', onAriaTab);
    return () => window.removeEventListener('aria-set-tab', onAriaTab);
  }, []);

  // ARIA simulation control listener
  useEffect(() => {
    function onAriaSimulation(e: Event) {
      const { action } = (e as CustomEvent).detail;
      switch (action) {
        case 'toggle_sidebar':
          setSidebarExpanded(v => !v);
          break;
        case 'open_menu':
          setMenuOpen(true);
          break;
        case 'open_menu_page':
          setMenuPage((e as CustomEvent).detail.menuPage ?? null);
          setMenuOpen(true);
          break;
        case 'close_menu':
          setMenuOpen(false);
          setMenuPage(null);
          break;
        case 'terminate_simulation': {
          const { currentSim: sim, accessToken: tok } = useSimStore.getState();
          if (!sim || !tok) return;
          setEndModal({ mode: 'manual' });
          break;
        }
      }
    }
    window.addEventListener('aria-simulation', onAriaSimulation);
    return () => window.removeEventListener('aria-simulation', onAriaSimulation);
  }, [navigate]);
  useSimWebSocket(simId ?? null);

  // Real clock
  useEffect(() => {
    function tick() {
      const now = new Date();
      setRealTime(`${now.getFullYear()} ${String(now.getDate()).padStart(2, '0')}/${String(now.getMonth() + 1).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`);
    }
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  // Auto-reload when a new server version is deployed — only reload when sim
  // is paused *and* no panel is open. A user generating a PDF/JSON report
  // (or doing anything else inside an open DetailPanel) got silently reset
  // to the default screen mid-export whenever a deploy happened to land
  // during that window -- pdfLoading/downloadPDF's own multi-second,
  // repeatedly-yielding html2canvas loop (ReportPanel.tsx) made this
  // reachable in practice, since it gives this independent 60s poll plenty
  // of chances to land squarely inside an in-progress export.
  useEffect(() => {
    let currentVersion: string | null = null;
    const check = () => fetch('/api/health').then(r => r.json()).then(d => {
      if (!currentVersion) { currentVersion = d.version; return; }
      if (d.version !== currentVersion) {
        const { currentSim, activePanel } = useSimStore.getState();
        if (currentSim?.status !== 'running' && !activePanel) window.location.reload();
        else currentVersion = d.version; // skip reload while running or a panel is open, pick up new version
      }
    }).catch(() => {});
    check();
    const id = setInterval(check, 60_000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!simId) return;
    const navTarget = (location.state as any)?.introTarget;
    const nextTarget = (navTarget && Number.isFinite(navTarget.lat) && Number.isFinite(navTarget.lon))
      ? { lat: Number(navTarget.lat), lon: Number(navTarget.lon) }
      : null;
    setIntroTarget(nextTarget);
  }, [simId, location.key]);

  useEffect(() => {
    if (!simId || !accessToken) return;
    // Clear previous sim's live data, then load this sim's persisted history
    resetLiveState();
    setIndividuals([]);
    setSimNotFound(false);
    setLoading(true);
    const headers = { Authorization: `Bearer ${accessToken}` };
    let cancelled = false;
    let pollInterval: ReturnType<typeof setInterval> | null = null;
    let simRetryTimer: ReturnType<typeof setTimeout> | null = null;
    let simRetryCount = 0;

    const stopPolling = () => {
      cancelled = true;
      if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
      setSimNotFound(true);
      setLoading(false);
    };

    const scheduleSimRetry = () => {
      if (cancelled) return;
      if (simRetryCount >= 10) {
        stopPolling();
        return;
      }
      const delay = Math.min(10000, 500 * Math.pow(simRetryCount + 1, 2));
      simRetryCount += 1;
      simRetryTimer = setTimeout(loadSimulation, delay);
    };

    const loadSimulation = () => {
      axios.get(`/api/simulations/${simId}`, { headers })
        .then(r => {
          if (cancelled) return;
          setCurrentSim(r.data);
          setSpeed(r.data.speed_multiplier ?? 1);
          setSimNotFound(false);
          setLoading(false);
        })
        .catch((err: any) => {
          if (cancelled) return;
          if (err?.response?.status === 404) {
            scheduleSimRetry();
            return;
          }
          setLoading(false);
        });
    };

    const loadPopulation = () => {
      axios.get(`/api/simulations/${simId}/population?alive=true&limit=500`, { headers })
        .then(r => {
          if (!cancelled) setIndividuals(r.data);
        })
        .catch((err: any) => {
          if (!cancelled && err?.response?.status !== 404) {
            console.warn('Population load failed:', err);
          }
        });
    };

    // Bridges the gap before the WebSocket delivers its next tick: without this,
    // `stats` stays null/stale (resetLiveState wiped it) until the server-side day
    // rolls over again, which can take a while right after mount or after a tab
    // was backgrounded and just became visible.
    const loadStats = () => {
      axios.get(`/api/simulations/${simId}/stats`, { headers })
        .then(r => {
          if (!cancelled) setStats(r.data);
        })
        .catch((err: any) => {
          if (!cancelled && err?.response?.status !== 404) {
            console.warn('Stats load failed:', err);
          }
        });
    };

    const onVisible = () => {
      if (!document.hidden) loadStats();
    };

    // Load recent historical events from DB so the event log isn't empty
    axios.get(`/api/simulations/${simId}/events?limit=100`, { headers })
      .then(r => setEvents(r.data)) // DB returns newest-first; store keeps same order
      .catch((err: any) => {
        if (err?.response?.status !== 404) {
          console.warn('Events load failed:', err);
        }
      });

    // Fetch immediately so population is visible on first paint, then keep it fresh.
    loadSimulation();
    loadPopulation();
    loadStats();
    pollInterval = setInterval(loadPopulation, 3000);
    document.addEventListener('visibilitychange', onVisible);

    // Backstop for `stats` when the WebSocket never delivers a single tick --
    // e.g. Android "Bulut" mode resolving the wrong WS host (see
    // useSimWebSocket.ts), a proxy/firewall blocking the WS upgrade, or any
    // other silent connection failure. Without this, `stats` (population's
    // own "ÖLÜM"/deaths counter included) was only ever fetched once on
    // mount/visibility-change and then relied entirely on WS pushes,
    // leaving the whole stats HUD frozen at whatever it read on that first
    // fetch for the rest of the session even while the simulation kept
    // running server-side. Only polls while the socket isn't actually
    // open, so a healthy WS connection doesn't pay for redundant REST
    // traffic on top of its own pushes.
    const statsPollInterval = setInterval(() => {
      if (useSimStore.getState().wsStatus !== 'open') loadStats();
    }, 5000);

    // Desktop app's "Yerel" (local) mode only: push a lightweight snapshot
    // to the cloud every 20s so this simulation is watchable from any
    // browser (WatchPage.tsx) and listed on the cloud dashboard's "Canlı
    // Simülasyonlar" -- see live_sync_tick/live_sync in routes.rs. A no-op
    // (never even fires) when this page isn't being served by the local
    // sidecar, i.e. for the overwhelming majority of visitors. Reads
    // currentSim fresh from the store on each tick instead of depending on
    // it here, so a status change doesn't re-run this whole effect (which
    // would reset load/poll state).
    let liveSyncInterval: ReturnType<typeof setInterval> | null = null;
    if (isLocalOrigin()) {
      const pushLiveSync = () => {
        const { currentSim: sim, accessToken: tok } = useSimStore.getState();
        if (sim?.status !== 'running') return;
        axios.post(`/api/simulations/${simId}/live-sync-tick`, {}, { headers: { Authorization: `Bearer ${tok}` } }).catch(() => {});
      };
      liveSyncInterval = setInterval(pushLiveSync, 20000);
    }

    return () => {
      cancelled = true;
      if (pollInterval) clearInterval(pollInterval);
      clearInterval(statsPollInterval);
      if (liveSyncInterval) clearInterval(liveSyncInterval);
      if (simRetryTimer) clearTimeout(simRetryTimer);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [simId, accessToken]);

  const syncSimulation = useCallback(async () => {
    if (!simId || !accessToken) return null;
    const headers = { Authorization: `Bearer ${accessToken}` };
    try {
      const { data } = await axios.get(`/api/simulations/${simId}`, { headers });
      setCurrentSim(data);
      setSpeed(data.speed_multiplier ?? 1);
      setSimNotFound(false);
      setLoading(false);
      return data;
    } catch (err: any) {
      if (err?.response?.status !== 404) {
        setLoading(false);
      }
      return null;
    }
  }, [simId, accessToken, setCurrentSim, setSpeed]);

  async function toggleSim() {
    if (!currentSim) { alert(text(lang as LangCode, { tr: 'Simülasyon yüklenmedi, sayfayı yenileyin.', en: 'Simulation not loaded, please refresh the page.', de: 'Simulation nicht geladen, bitte Seite neu laden.', fr: "Simulation non chargée, veuillez rafraîchir la page.", ar: 'لم يتم تحميل المحاكاة، يرجى تحديث الصفحة.' })); return; }
    if (!accessToken) { alert(text(lang as LangCode, { tr: 'Oturum süresi dolmuş, çıkış yapıp tekrar giriş yapın.', en: 'Session expired, please log out and log in again.', de: 'Sitzung abgelaufen, bitte abmelden und erneut anmelden.', fr: 'Session expirée, veuillez vous déconnecter et vous reconnecter.', ar: 'انتهت الجلسة، يرجى تسجيل الخروج والدخول مرة أخرى.' })); return; }
    if (actionBusy) return;
    setActionBusy(true);
    const action = currentSim.status === 'running' ? 'pause' : 'start';
    const previous = currentSim;
    const headers = { Authorization: `Bearer ${accessToken}` };
    try {
      setCurrentSim({ ...currentSim, status: action === 'start' ? 'running' : 'paused' });
      // A brand-new simulation started right after creation (or any request
      // landing during a Render redeploy/cold start) can hit a spurious 404
      // here even though the row genuinely exists -- loadSimulation() above
      // already retries for exactly this reason. Mirror that instead of
      // surfacing a scary error on the very first blip.
      const retryDelaysMs = [300, 800, 1500];
      let lastErr: any = null;
      for (let attempt = 0; attempt <= retryDelaysMs.length; attempt++) {
        try {
          await axios.post(`/api/simulations/${currentSim.id}/${action}`, {}, { headers });
          lastErr = null;
          break;
        } catch (err: any) {
          lastErr = err;
          if (err?.response?.status !== 404 || attempt === retryDelaysMs.length) break;
          await new Promise(r => setTimeout(r, retryDelaysMs[attempt]));
        }
      }
      if (lastErr) throw lastErr;
      if (action === 'start') {
        await syncSimulation();
      }
    } catch (err: any) {
      setCurrentSim(previous);
      alert(`${text(lang as LangCode, { tr: 'Hata', en: 'Error', de: 'Fehler', fr: 'Erreur', ar: 'خطأ' })}: ${err?.response?.data?.error ?? err?.message ?? text(lang as LangCode, { tr: 'Bilinmeyen hata', en: 'Unknown error', de: 'Unbekannter Fehler', fr: 'Erreur inconnue', ar: 'خطأ غير معروف' })}`);
    } finally {
      setActionBusy(false);
    }
  }

  async function changeSpeed(s: number) {
    if (!Number.isInteger(s) || s < 1 || s > 1000 || !currentSim || !accessToken) return;
    if (speedBusy) return;
    setSpeedBusy(true);
    const previous = speedMultiplier;
    setSpeed(s);
    try {
      await axios.post(`/api/simulations/${simId}/speed`, { speed_multiplier: s }, { headers: { Authorization: `Bearer ${accessToken}` } });
    } catch {
      setSpeed(previous);
    } finally {
      setSpeedBusy(false);
    }
  }

  function applyCustomSpeed() {
    const v = parseInt(customSpeed);
    if (v >= 1 && v <= 1000) { changeSpeed(v); setCustomSpeed(''); }
  }

  async function startFastForward() {
    const yr = parseInt(ffTarget);
    if (!yr || yr < 1 || !currentSim || !accessToken) return;
    setFfError('');
    setFfTarget('');
    try {
      await axios.post(`/api/simulations/${currentSim.id}/fast-forward`, { target_year: yr }, { headers: { Authorization: `Bearer ${accessToken}` } });
    } catch (err: any) {
      setFfError(err?.response?.data?.error ?? text(lang as LangCode, { tr: 'Bilinmeyen hata', en: 'Unknown error', de: 'Unbekannter Fehler', fr: 'Erreur inconnue', ar: 'خطأ غير معروف' }));
    }
  }

  async function cancelFastForward() {
    if (!currentSim || !accessToken) return;
    await axios.post(`/api/simulations/${currentSim.id}/fast-forward/cancel`, {}, { headers: { Authorization: `Bearer ${accessToken}` } }).catch(() => {});
  }

  function terminateSim() {
    if (!currentSim || !accessToken) return;
    setEndModal({ mode: 'manual' });
  }

  async function doTerminate(openReport = false) {
    if (!currentSim || !accessToken) return;
    const previous = currentSim;
    setCurrentSim({ ...currentSim, status: 'completed' });
    setEndModal(null);
    try {
      await axios.post(`/api/simulations/${currentSim.id}/terminate`, {}, { headers: { Authorization: `Bearer ${accessToken}` } });
      if (openReport) {
        setActivePanel('report');
      } else {
        navigate('/');
      }
    } catch {
      setCurrentSim(previous);
    }
  }

  const isRunning = currentSim?.status === 'running';
  const simDay = stats?.day ?? 0;
  // Derived from simDay (updated on every tick via setStatsDay), not read
  // directly off stats.year -- that field only refreshes on a full stats
  // sync (fullSync() in WASM-Local, the WebSocket's less-frequent full
  // "tick" message with `stats` in Cloud mode), while the day-of-year shown
  // right next to it (simDay % 365 below) updates continuously. The result
  // was a visible mismatch: the day counter rolls over into a new year
  // smoothly, but the year number itself stays frozen until the next full
  // sync, then jumps forward by however many years accumulated in the
  // meantime -- looking like a completely separate "year jump" bug even
  // after the day-counter jump itself was fixed. current_year is always
  // current_day / 365 server/engine-side (see tick.rs), so this is exactly
  // the same value fullSync would have reported anyway, just never stale.
  const simYear = Math.floor(simDay / 365);
  const seasonLabel = translateSeason(stats?.season ?? '', lang as LangCode);
  const simHour = stats?.hour !== undefined ? `${String(stats.hour).padStart(2, '0')}:00` : '00:00';
  const births = stats?.births ?? 0;
  const deaths = stats?.deaths ?? 0;
  const wordCount = stats?.word_count ?? 0;

  function fmtEvent(ev: any) {
    const prefix = `Y${String(ev.sim_year).padStart(4, '0')} G${String(ev.sim_day % 365).padStart(3, '0')}`;
    const icon = ev.event_type?.includes('birth') ? '+' : ev.event_type?.includes('death') ? '†' : ev.event_type?.includes('discovery') ? '◆' : ev.event_type?.includes('disaster') ? '⚠' : '·';
    const rawDesc = ev.description ?? ev.event_type;
    const desc = translateEventDescription(rawDesc, lang as LangCode, ev);
    return `${prefix} ${icon} ${desc}`;
  }

  function eventColor(ev: any, idx: number): string {
    if (ev.event_type?.includes('birth')) return idx === 0 ? '#7aff9a' : `rgba(100,220,130,${Math.max(0.3, 1 - idx * 0.1)})`;
    if (ev.event_type?.includes('death')) return idx === 0 ? '#e08080' : `rgba(200,80,80,${Math.max(0.3, 1 - idx * 0.1)})`;
    if (ev.event_type?.includes('discovery')) return idx === 0 ? '#7dd3fc' : `rgba(100,180,240,${Math.max(0.3, 1 - idx * 0.1)})`;
    if (ev.event_type?.includes('disaster')) return idx === 0 ? '#f97316' : `rgba(240,100,30,${Math.max(0.3, 1 - idx * 0.1)})`;
    return idx === 0 ? '#a0e8c0' : `rgba(80,160,110,${Math.max(0.25, 1 - idx * 0.07)})`;
  }

  const uiText = {
    live:        text(lang as LangCode, { tr: 'CANLI',         en: 'LIVE',         de: 'LIVE',       fr: 'EN DIRECT', ar: 'مباشر'     }),
    speed:       text(lang as LangCode, { tr: 'HIZ',           en: 'SPEED',        de: 'TEMPO',      fr: 'VITESSE',   ar: 'السرعة'    }),
    set:         text(lang as LangCode, { tr: 'AYARLA',        en: 'SET',          de: 'SETZEN',     fr: 'DÉFINIR',   ar: 'تعيين'     }),
    creatorTime: text(lang as LangCode, { tr: 'KURUCU ZAMANI', en: 'CREATOR TIME', de: 'ERSTELLZEIT',fr: 'HEURE',     ar: 'وقت المنشئ'}),
    user:        text(lang as LangCode, { tr: 'KULLANICI',     en: 'USER',         de: 'BENUTZER',   fr: 'UTILISATEUR',ar: 'المستخدم' }),
  };

  const cardDir = {
    N: text(lang as LangCode, { tr: 'K', en: 'N', de: 'N', fr: 'N', ar: 'ش' }),
    S: text(lang as LangCode, { tr: 'G', en: 'S', de: 'S', fr: 'S', ar: 'ج' }),
    E: text(lang as LangCode, { tr: 'D', en: 'E', de: 'O', fr: 'E', ar: 'ش' }),
    W: text(lang as LangCode, { tr: 'B', en: 'W', de: 'W', fr: 'O', ar: 'غ' }),
  };

  const sidebarW = sidebarExpanded ? 180 : 44;
  const rightPanelW = rightPanelExpanded ? 120 : 36;

  if (loading && !currentSim && !simNotFound) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '100vw', height: '100vh', background: '#040412', color: '#a0c8b0', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.12em' }}>
        {text(lang as LangCode, { tr: 'SİMÜLASYON YÜKLENİYOR...', en: 'LOADING SIMULATION...', de: 'SIMULATION WIRD GELADEN...', fr: 'CHARGEMENT DE LA SIMULATION...', ar: 'جاري تحميل المحاكاة...' })}
      </div>
    );
  }

  if (simNotFound) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', width: '100vw', height: '100vh', background: '#040412', fontFamily: 'Share Tech Mono, monospace', gap: 16 }}>
        <div style={{ fontSize: 16, color: '#e05a5a', letterSpacing: '0.08em' }}>
          {text(lang as LangCode, { tr: 'Simülasyon bulunamadı veya silinmiş.', en: 'Simulation not found or deleted.', de: 'Simulation nicht gefunden.', fr: 'Simulation introuvable.', ar: 'المحاكاة غير موجودة أو محذوفة.' })}
        </div>
        <button
          onClick={() => navigate('/')}
          style={{ padding: '8px 24px', background: 'rgba(0,232,135,0.12)', border: '1px solid #00e887', color: '#00e887', cursor: 'pointer', fontFamily: 'Share Tech Mono, monospace', fontSize: 14, letterSpacing: '0.1em' }}>
          {text(lang as LangCode, { tr: '← Dashboard\'a Dön', en: '← Back to Dashboard', de: '← Zurück', fr: '← Tableau de bord', ar: '← لوحة التحكم' })}
        </button>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', width: '100vw', height: '100vh', overflow: 'hidden', background: '#000', color: '#fff', fontFamily: 'Share Tech Mono, monospace' }}>

      {/* ═══ HEADER (3 rows) ═══ */}
      <div style={{ flexShrink: 0, background: 'rgba(0,0,0,0.97)', borderBottom: '1px solid #4a1a1a' }}>

        {/* Row 1: Logo | SIM time | [ARIA desktop] | [clock desktop] | BAŞLAT/DURDUR */}
          <div style={{ display: 'flex', alignItems: 'center', gap: isMobile ? 4 : 8, padding: isMobile ? '4px 8px' : '5px 10px', borderBottom: '1px solid #4a1a1a' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
              <div style={{ position: 'relative', width: 18, height: 18, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <div className="neon-breathe" style={{ position: 'absolute', inset: 0, background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '1.2px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }} />
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.8" strokeLinecap="round" style={{ overflow: 'visible' }}>
                  <circle cx="12" cy="12" r="10" />
                  <ellipse cx="12" cy="12" rx="6" ry="14.29" />
                  <path d="M2 12h20" />
                  <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
                </svg>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', lineHeight: 1, gap: 2 }}>
                <span style={{ fontSize: isMobile ? 12 : 14, fontFamily: 'Orbitron, monospace', fontWeight: 900, color: '#e0e0f0', letterSpacing: isMobile ? '0.12em' : '0.2em', textShadow: '0 0 10px rgba(79,158,247,0.35)' }}>
                  {lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'}
                </span>
                <span style={{ fontSize: isMobile ? 10 : 11, fontFamily: 'Share Tech Mono, monospace', fontWeight: 700, color: '#cc2222', letterSpacing: isMobile ? '0.16em' : '0.22em', textAlign: 'center', width: '100%' }}>
                  {text(lang as LangCode, { tr: 'MEDENİYET', en: 'CIVILIZATION', de: 'ZIVILISATION', fr: 'CIVILISATION', ar: 'الحضارة' })}
                </span>
              </div>
            </div>

          <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', alignItems: 'flex-start', marginLeft: isMobile ? 3 : 6 }}>
            <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.1em' }}>
              {text(lang as LangCode, { tr: 'SİM ZAMANI', en: 'SIM TIME', de: 'SIM-ZEIT', fr: 'TEMPS SIM', ar: 'وقت المحاكاة' })}
            </span>
            <span style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.04em', fontFamily: 'Orbitron, monospace' }}>
              Y{String(simYear).padStart(4, '0')} G{String(simDay % 365).padStart(3, '0')}
            </span>
          </div>

          <div style={{ flex: 1 }} />

          {!isMobile && (
            <div style={{ flexShrink: 0, display: 'flex', flexDirection: 'column', alignItems: 'flex-end', marginRight: 6 }}>
              <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.1em' }}>{uiText.creatorTime}</span>
              <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.05em' }}>{realTime}</span>
            </div>
          )}

          <button
            onClick={toggleSim}
            disabled={loading || actionBusy}
            style={{
              display: 'flex', alignItems: 'center', gap: 5, flexShrink: 0,
              padding: isMobile ? '5px 12px' : '5px 16px',
              fontSize: 14,
              fontFamily: 'Orbitron, monospace', fontWeight: 700, letterSpacing: '0.1em',
              background: (loading || actionBusy) ? 'rgba(120,120,120,0.18)' : isRunning ? 'rgba(212,56,56,0.18)' : 'rgba(0,232,135,0.18)',
              border: `1px solid ${(loading || actionBusy) ? '#555' : isRunning ? '#c03030' : '#00e887'}`,
              color: (loading || actionBusy) ? '#666' : isRunning ? '#e05a5a' : '#00e887',
              boxShadow: isRunning ? '0 0 10px rgba(200,50,50,0.3)' : '0 0 10px rgba(0,232,135,0.25)',
              cursor: (loading || actionBusy) ? 'not-allowed' : 'pointer',
              opacity: (loading || actionBusy) ? 0.6 : 1,
            }}>
            {loading ? null : isRunning ? <Pause size={11} /> : <Play size={11} />}
            {loading ? '...' : isRunning ? text(lang as LangCode, { tr: 'DURDUR', en: 'PAUSE', de: 'PAUSE', fr: 'PAUSE', ar: 'إيقاف' }) : text(lang as LangCode, { tr: 'BAŞLAT', en: 'START', de: 'START', fr: 'DÉMARRER', ar: 'ابدأ' })}
          </button>

          {isRunning && (
            <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#00e887', boxShadow: '0 0 8px #00e887', flexShrink: 0, animation: 'pulse 1.5s infinite' }} />
          )}
        </div>

        {/* Row 2: Stats | HIZ | SONLANDIR | ÇIKIŞ | MENÜ */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 0, padding: '0 10px', borderBottom: '1px solid #4a1a1a', overflow: 'hidden' }}>
          <div style={{ display: 'flex', alignItems: 'center', overflowX: isMobile ? 'auto' : 'visible', flex: 1, scrollbarWidth: 'none' }}>
              {[
                { key: 'pop',  label: text(lang as LangCode, { tr: 'NÜFUS',     en: 'POP',    de: 'BEV.',   fr: 'POP.',   ar: 'سكان'   }), value: stats?.population ?? '—',   color: '#00e887' },
                { key: 'bir',  label: text(lang as LangCode, { tr: 'DOĞUM',     en: 'BIRTH',  de: 'GEB.',   fr: 'NAISS.', ar: 'مواليد' }), value: births,                      color: '#4ecb71' },
                { key: 'dth',  label: text(lang as LangCode, { tr: 'ÖLÜM',      en: 'DEATH',  de: 'TOD',    fr: 'DÉCÈS',  ar: 'وفيات'  }), value: deaths,                      color: '#e05a5a' },
                { key: 'yr',   label: text(lang as LangCode, { tr: 'YIL',       en: 'YEAR',   de: 'JAHR',   fr: 'ANNÉE',  ar: 'سنة'    }), value: stats ? simYear : '—',          color: '#7dd3fc' },
                { key: 'tech', label: text(lang as LangCode, { tr: 'TEKNOLOJİ', en: 'TECH',   de: 'TECH.',  fr: 'TECH.',  ar: 'تقنية'  }), value: stats?.technologies ?? '—',  color: '#d4a838' },
                { key: 'sea',  label: text(lang as LangCode, { tr: 'MEVSİM',    en: 'SEASON', de: 'SAISON', fr: 'SAISON', ar: 'موسم'   }), value: seasonLabel,                 color: '#a0b4ff' },
                { key: 'temp', label: text(lang as LangCode, { tr: 'SICAKLIK',  en: 'TEMP',   de: 'TEMP.',  fr: 'TEMP.',  ar: 'حرارة'  }), value: stats?.temperature !== undefined ? `${stats.temperature}°` : '—', color: stats?.temperature !== undefined ? (stats.temperature > 30 ? '#e05a5a' : '#7dd3fc') : '#a0b4ff' },
                { key: 'wthr', label: text(lang as LangCode, { tr: 'HAVA',      en: 'WTHR.',  de: 'WETTER', fr: 'MÉTÉO',  ar: 'طقس'    }), value: (() => { const icons: Record<string,string> = { clear: '☀️', rain: '🌧', heavy_rain: '⛈', snow: '❄️', blizzard: '🌨', storm: '🌩', heat_wave: '🔥', drought: '🏜' }; return icons[(stats as any)?.weather] ?? '☀️'; })(), color: '#e0d080' },
                ...(isWarping ? [{ key: 'warp', label: text(lang as LangCode, { tr: 'WARP', en: 'WARP', de: 'WARP', fr: 'WARP', ar: 'وارب' }), value: `→Y${fastForwardTarget ? Math.floor(fastForwardTarget / 365) : '?'}`, color: '#d4a838' }] : []),
              ].map(({ key, label, value, color }) => (
              <div key={key} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', padding: isMobile ? '2px 7px' : '2px 10px', borderRight: '1px solid #4a1a1a', flexShrink: 0, minWidth: isMobile ? 42 : 52 }}>
                <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.08em', whiteSpace: 'nowrap' }}>{label}</span>
                <span style={{ fontSize: 14, color, fontFamily: 'Orbitron, monospace', fontWeight: 700, lineHeight: 1.2 }}>{value}</span>
              </div>
            ))}
          </div>

        </div>
      </div>

      {/* ═══ BODY: SIDEBAR + MAIN ═══ */}
      <div style={{ display: 'flex', flexDirection: 'row', flex: 1, overflow: 'hidden' }}>

        {/* ── LEFT SIDEBAR ── */}
        <div style={{
          width: sidebarW,
          flexShrink: 0,
          display: 'flex',
          flexDirection: 'column',
          background: 'rgba(0,0,0,0.97)',
          borderRight: '1px solid #4a1a1a',
          overflow: 'hidden',
          transition: 'width 0.2s ease',
          position: 'relative',
        }}>

          {/* Module buttons */}
          <div style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden', paddingTop: 4 }}>
            {MODULES.map(mod => {
              const isActive = activePanel === mod.id;
              const accent = (mod as any).accent ?? '#00e887';
              return (
                <button
                  key={mod.id}
                  onClick={() => setActivePanel(isActive ? null : mod.id)}
                  title={text(lang as LangCode, mod.labels)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: sidebarExpanded ? 8 : 0,
                    justifyContent: sidebarExpanded ? 'flex-start' : 'center',
                    width: '100%',
                    padding: sidebarExpanded ? '5px 10px' : '7px 0',
                    fontSize: 14,
                    letterSpacing: '0.06em',
                    fontFamily: 'Share Tech Mono, monospace',
                    background: isActive ? `${accent}18` : 'transparent',
                    borderLeft: `2px solid ${isActive ? accent : 'transparent'}`,
                    borderTop: 'none',
                    borderRight: 'none',
                    borderBottom: '1px solid #4a1a1a',
                    color: isActive ? accent : '#8abda0',
                    cursor: 'pointer',
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    boxSizing: 'border-box',
                  }}>
                  <span style={{ fontSize: 16, flexShrink: 0, lineHeight: 1 }}>{mod.icon}</span>
                  {sidebarExpanded && (
                    <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {text(lang as LangCode, mod.labels)}
                    </span>
                  )}
                </button>
              );
            })}
          </div>


          {/* Collapsed population indicator */}
          {!sidebarExpanded && (
            <div style={{ padding: '6px 0', borderTop: '1px solid #4a1a1a', flexShrink: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
              <Users size={12} color="#00e887" />
              <span style={{ fontSize: 14, color: '#00e887', fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>
                {stats?.population !== undefined ? (stats.population > 999 ? `${Math.floor(stats.population / 1000)}k` : stats.population) : '—'}
              </span>
            </div>
          )}


          {/* Toggle button */}
          <button
            onClick={() => setSidebarExpanded(v => !v)}
            style={{
              flexShrink: 0,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '100%',
              padding: '6px 0',
              background: 'rgba(0,20,10,0.8)',
              border: 'none',
              borderTop: '1px solid #4a1a1a',
              color: '#a0c8b0',
              cursor: 'pointer',
              fontSize: 14,
              gap: 4,
            }}>
            {sidebarExpanded ? <ChevronLeft size={12} /> : <ChevronRight size={12} />}
            {sidebarExpanded && <span style={{ fontSize: 14, letterSpacing: '0.08em' }}>{text(lang as LangCode, { tr: 'DARALT', en: 'COLLAPSE', de: 'EINKLAPPEN', fr: 'RÉDUIRE', ar: 'طي' })}</span>}
          </button>
        </div>

        {/* ── MAIN CONTENT ── */}
        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>

          {/* Tab bar */}
          <div style={{ display: 'flex', flexShrink: 0, borderBottom: '1px solid #4a1a1a' }}>
            {TABS.map(tab => {
              const isActive = activeTab === tab.id;
              return (
                <button key={tab.id}
                  onClick={() => setActiveTab(tab.id as any)}
                  style={{
                    flex: 1,
                    padding: '6px 0',
                    fontSize: 14,
                    letterSpacing: '0.15em',
                    fontFamily: 'Share Tech Mono, monospace',
                    color: isActive ? '#00e887' : '#8abda0',
                    background: isActive ? 'rgba(0,232,135,0.06)' : 'transparent',
                    border: 'none',
                    borderBottom: `2px solid ${isActive ? '#00e887' : 'rgba(255,255,255,0.08)'}`,
                    borderRight: '1px solid #4a1a1a',
                    cursor: 'pointer',
                  }}>
                  {text(lang as LangCode, tab.labels)}
                </button>
              );
            })}
          </div>

          {/* Tab content */}
          <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>

            {/* HARITA tab */}
            {activeTab === 'harita' && (
              <>
                {/* Globe */}
                <div style={{ position: 'absolute', inset: 0 }}>

                  <WorldGlobe
                    individuals={individuals}
                    spawnLat={currentSim?.start_latitude}
                    spawnLon={currentSim?.start_longitude}
                    onSelect={(ind) => { setSelectedInd(ind); setGlobeCoord(null); }}
                    onGlobeClick={(lat, lon) => { setGlobeCoord({ lat, lon }); setSelectedInd(null); }}
                    introPlaying={!!introTarget}
                    onIntroComplete={() => setIntroTarget(null)}
                  />
                </div>

                {/* Sim start coord (bottom-left) */}
                {currentSim && (
                  <div style={{ position: 'absolute', bottom: 8, left: 8, zIndex: 10, background: 'rgba(0,0,0,0.7)', border: '1px solid #4a1a1a', padding: '3px 8px' }}>
                    <span style={{ fontSize: 14, color: '#a0c8b0', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.05em' }}>
                      {text(lang as LangCode, { tr: 'BAŞLANGIÇ', en: 'ORIGIN', de: 'URSPRUNG', fr: 'ORIGINE', ar: 'الأصل' })}: {currentSim.start_latitude?.toFixed(3)}°{(currentSim.start_latitude ?? 0) >= 0 ? cardDir.N : cardDir.S}  {currentSim.start_longitude?.toFixed(3)}°{(currentSim.start_longitude ?? 0) >= 0 ? cardDir.E : cardDir.W}
                    </span>
                  </div>
                )}

                {/* Globe click coordinate overlay (bottom-right) */}
                {globeCoord && (
                  <div style={{ position: 'absolute', bottom: 8, right: 8, zIndex: 20, background: 'rgba(0,5,2,0.92)', border: '1px solid #4a1a1a', padding: '5px 10px', fontFamily: 'Share Tech Mono, monospace' }}>
                    <div style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.1em', marginBottom: 2 }}>
                      {text(lang as LangCode, { tr: '// KONUM', en: '// COORDS', de: '// KOORD.', fr: '// COORDS', ar: '// إحداثيات' })}
                    </div>
                    <div style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.06em' }}>
                      {Math.abs(globeCoord.lat).toFixed(3)}°{globeCoord.lat >= 0 ? cardDir.N : cardDir.S}
                      {'  '}
                      {Math.abs(globeCoord.lon).toFixed(3)}°{globeCoord.lon >= 0 ? cardDir.E : cardDir.W}
                    </div>
                    <button onClick={() => setGlobeCoord(null)} style={{ marginTop: 3, fontSize: 14, color: '#a0c8b0', border: 'none', background: 'transparent', cursor: 'pointer', padding: 0 }}>✕</button>
                  </div>
                )}

                {/* Selected individual overlay */}
                {selectedInd && (
                  <div style={{ position: 'absolute', top: 80, right: 8, zIndex: 50, background: 'rgba(0,5,2,0.97)', border: '1px solid #00e887', padding: 12, minWidth: 160, fontFamily: 'Share Tech Mono, monospace' }}>
                    <div style={{ fontSize: 14, color: '#a0c8b0', marginBottom: 4 }}>// {text(lang as LangCode, { tr: 'BİREY', en: 'INDIVIDUAL', de: 'INDIVIDUUM', fr: 'INDIVIDU', ar: 'فرد' })}</div>
                    <div style={{ fontSize: 14, color: '#00e887', marginBottom: 2 }}>
                      {selectedInd.name ?? `${selectedInd.sex === 'male' ? '♂' : '♀'}-${selectedInd.id?.slice(-4).toUpperCase()}`}
                    </div>
                    {!selectedInd.name && (
                      <div style={{ fontSize: 14, color: '#8abda0', marginBottom: 4, fontStyle: 'italic' }}>
                        {text(lang as LangCode, { tr: '// dil henüz gelişmedi', en: '// pre-linguistic era', de: '// vorsprachliche Ära', fr: '// ère pré-linguistique', ar: '// حقبة ما قبل اللغة' })}
                      </div>
                    )}
                    <div style={{ fontSize: 14, color: '#6090a0' }}>{text(lang as LangCode, { tr: 'Cinsiyet', en: 'Sex', de: 'Geschlecht', fr: 'Sexe', ar: 'الجنس' })}: <span style={{ color: '#fff' }}>{selectedInd.sex === 'male' ? text(lang as LangCode, { tr: 'Erkek', en: 'Male', de: 'Männlich', fr: 'Mâle', ar: 'ذكر' }) : text(lang as LangCode, { tr: 'Kadın', en: 'Female', de: 'Weiblich', fr: 'Femelle', ar: 'أنثى' })}</span></div>
                    <div style={{ fontSize: 14, color: '#6090a0' }}>{text(lang as LangCode, { tr: 'Yaş', en: 'Age', de: 'Alter', fr: 'Âge', ar: 'العمر' })}: <span style={{ color: '#fff' }}>{selectedInd.age_years ?? '—'}</span></div>
                    <div style={{ fontSize: 14, color: '#6090a0' }}>{text(lang as LangCode, { tr: 'Boy', en: 'Height', de: 'Größe', fr: 'Taille', ar: 'الطول' })}: <span style={{ color: '#a0e887' }}>{selectedInd.height_cm ?? selectedInd.phenotype?.height_cm ?? '—'} cm</span></div>
                    <div style={{ fontSize: 14, color: '#6090a0' }}>{text(lang as LangCode, { tr: 'Kilo', en: 'Weight', de: 'Gewicht', fr: 'Poids', ar: 'الوزن' })}: <span style={{ color: '#a0e887' }}>{selectedInd.weight_kg ?? '—'} kg</span></div>
                    <div style={{ fontSize: 14, color: '#6090a0' }}>HP: <span style={{ color: selectedInd.health?.hp > 0.6 ? '#4ecb71' : '#e05a5a' }}>{selectedInd.health?.hp !== undefined ? (selectedInd.health.hp * 100).toFixed(0) + '%' : '—'}</span></div>
                    <button onClick={() => setSelectedInd(null)} style={{ marginTop: 8, fontSize: 14, color: '#a0c8b0', border: '1px solid #4a1a1a', padding: '2px 8px', background: 'transparent', width: '100%', cursor: 'pointer' }}>
                      {text(lang as LangCode, { tr: 'KAPAT', en: 'CLOSE', de: 'SCHLIESSEN', fr: 'FERMER', ar: 'إغلاق' })}
                    </button>
                  </div>
                )}

                {/* Stats telemetry panel */}
                <StatsPanel />

                {/* Draggable event log panel */}
                <DraggableLogPanel
                  events={events}
                  lang={lang}
                  fmtEvent={fmtEvent}
                  eventColor={eventColor}
                />

              </>
            )}

            {/* DURUM tab */}
            {activeTab === 'durum' && (
              <div style={{ height: '100%', overflowY: 'auto', padding: 12, background: '#000' }}>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                  {[
                    { l: text(lang as LangCode, { tr: 'NÜFUS',     en: 'POP.',      de: 'BEV.',     fr: 'POP.',      ar: 'سكان'    }), v: stats?.population ?? '—',                                                                 c: '#00e887' },
                    { l: text(lang as LangCode, { tr: 'YIL',       en: 'YEAR',      de: 'JAHR',     fr: 'ANNÉE',     ar: 'سنة'     }), v: stats ? simYear : '—',                                                                        c: '#7dd3fc' },
                    { l: text(lang as LangCode, { tr: 'GÜN',       en: 'DAY',       de: 'TAG',      fr: 'JOUR',      ar: 'يوم'     }), v: stats?.day ?? '—',                                                                         c: '#a0b4ff' },
                    { l: text(lang as LangCode, { tr: 'DOĞUM',     en: 'BIRTHS',    de: 'GEB.',     fr: 'NAISS.',    ar: 'مواليد'  }), v: births,                                                                                     c: '#4ecb71' },
                    { l: text(lang as LangCode, { tr: 'ÖLÜM',      en: 'DEATHS',    de: 'TODE',     fr: 'DÉCÈS',     ar: 'وفيات'   }), v: deaths,                                                                                     c: '#e05a5a' },
                    { l: text(lang as LangCode, { tr: 'KELİME',    en: 'WORDS',     de: 'WÖRTER',   fr: 'MOTS',      ar: 'كلمات'   }), v: wordCount,                                                                                  c: '#7dd3fc' },
                    { l: text(lang as LangCode, { tr: 'ZEKA ORT.', en: 'AVG IQ',    de: 'Ø IQ',     fr: 'IQ MOY.',   ar: 'متوسط IQ'}), v: stats?.avg_intelligence !== undefined ? (stats.avg_intelligence * 100).toFixed(0) + '%' : '—', c: '#d4a838' },
                    { l: text(lang as LangCode, { tr: 'MUTLULUK',  en: 'HAPPINESS', de: 'GLÜCK',    fr: 'BONHEUR',   ar: 'سعادة'   }), v: stats?.happiness_index !== undefined ? (stats.happiness_index * 100).toFixed(0) + '%' : '—',   c: '#ff8ab0' },
                    { l: text(lang as LangCode, { tr: 'HASTALIK',  en: 'DISEASE',   de: 'KRANKHEIT',fr: 'MALADIE',   ar: 'مرض'     }), v: stats?.sick_rate !== undefined ? (stats.sick_rate * 100).toFixed(0) + '%' : '—',               c: '#f97316' },
                    { l: text(lang as LangCode, { tr: 'TEKNOLOJİ', en: 'TECH',      de: 'TECH.',    fr: 'TECH.',     ar: 'تقنية'   }), v: stats?.technologies ?? '—',                                                                c: '#4ecb71' },
                    { l: text(lang as LangCode, { tr: 'İNANÇ',     en: 'BELIEFS',   de: 'GLAUBEN',  fr: 'CROYANCES', ar: 'معتقدات' }), v: stats?.beliefs ?? '—',                                                                    c: '#a855f7' },
                    { l: text(lang as LangCode, { tr: 'SICAKLIK',  en: 'TEMP',      de: 'TEMP.',    fr: 'TEMP.',     ar: 'حرارة'   }), v: stats?.temperature !== undefined ? `${stats.temperature}°` : '—',                          c: stats?.temperature !== undefined ? (stats.temperature > 30 ? '#e05a5a' : '#7dd3fc') : '#a0b4ff' },
                    { l: text(lang as LangCode, { tr: 'GRUPLAR',   en: 'GROUPS',    de: 'GRUPPEN',  fr: 'GROUPES',   ar: 'مجموعات' }), v: stats?.groups ?? '—',                                                                      c: '#d4a838' },
                  ].map(({ l, v, c }) => (
                    <div key={l} style={{ background: 'rgba(0,20,10,0.6)', border: '1px solid #4a1a1a', padding: '8px 12px' }}>
                      <div style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.1em' }}>{l}</div>
                      <div style={{ fontSize: 18, color: c, fontFamily: 'Orbitron, monospace', fontWeight: 700, marginTop: 2 }}>{v}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

          </div>
        </div>

        {/* ── RIGHT PANEL ── */}
        {(
          <div style={{
            width: rightPanelW,
            flexShrink: 0,
            display: 'flex',
            flexDirection: 'column',
            background: 'rgba(0,0,0,0.97)',
            borderLeft: '1px solid #4a1a1a',
            overflow: 'hidden',
            transition: 'width 0.2s ease',
          }}>
            <div style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden', paddingTop: 4 }}>

              {/* AYARLAR */}
              <button
                onClick={() => setSettingsOpen(true)}
                title={text(lang as LangCode, { tr: 'AYARLAR', en: 'SETTINGS', de: 'EINSTELLUNGEN', fr: 'PARAMÈTRES', ar: 'الإعدادات' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: 'transparent', borderLeft: '2px solid transparent',
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: '#8abda0', cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <Settings size={16} style={{ flexShrink: 0 }} />
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'AYARLAR', en: 'SETTINGS', de: 'EINSTELLUNGEN', fr: 'PARAMÈTRES', ar: 'الإعدادات' })}</span>}
              </button>

              {/* MENÜ */}
              <button
                onClick={() => setMenuOpen(true)}
                title={text(lang as LangCode, { tr: 'MENÜ', en: 'MENU', de: 'MENÜ', fr: 'MENU', ar: 'القائمة' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: 'transparent', borderLeft: '2px solid transparent',
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: '#8abda0', cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <span style={{ fontSize: 16, flexShrink: 0 }}>☰</span>
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'MENÜ', en: 'MENU', de: 'MENÜ', fr: 'MENU', ar: 'القائمة' })}</span>}
              </button>

              {/* User badge */}
              {user && (
                <div style={{
                  padding: rightPanelExpanded ? '5px 10px' : '5px 0',
                  borderBottom: '1px solid #4a1a1a',
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: 1,
                }}>
                  {rightPanelExpanded
                    ? <>
                        <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.18em', fontFamily: 'Share Tech Mono, monospace' }}>{uiText.user}</span>
                        <span style={{ fontSize: 11, color: '#e0e0f0', fontFamily: 'Orbitron, monospace', fontWeight: 700, letterSpacing: '0.06em', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '100%' }}>{user.username.toUpperCase()}</span>
                      </>
                    : <span style={{ fontSize: 10, color: '#e0e0f0', fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{user.username.slice(0, 2).toUpperCase()}</span>
                  }
                </div>
              )}

              {/* RAPOR */}
              <button
                onClick={() => setActivePanel(activePanel === 'report' ? null : 'report')}
                title={text(lang as LangCode, { tr: 'RAPOR', en: 'REPORT', de: 'BERICHT', fr: 'RAPPORT', ar: 'تقرير' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: activePanel === 'report' ? 'rgba(251,191,36,0.12)' : 'transparent',
                  borderLeft: `2px solid ${activePanel === 'report' ? '#fbbf24' : 'transparent'}`,
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: activePanel === 'report' ? '#fbbf24' : '#8abda0',
                  cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <span style={{ fontSize: 16, flexShrink: 0 }}>📄</span>
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'RAPOR', en: 'REPORT', de: 'BERICHT', fr: 'RAPPORT', ar: 'تقرير' })}</span>}
              </button>

              {/* SONLANDIR */}
              <button
                onClick={terminateSim}
                title={text(lang as LangCode, { tr: 'SONLANDIR', en: 'TERMINATE', de: 'BEENDEN', fr: 'TERMINER', ar: 'إنهاء' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: 'transparent', borderLeft: '2px solid transparent',
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: '#c05050', cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <Trash2 size={16} style={{ flexShrink: 0 }} />
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'SONLANDIR', en: 'TERMINATE', de: 'BEENDEN', fr: 'TERMINER', ar: 'إنهاء' })}</span>}
              </button>

              {/* ÇIKIŞ */}
              <button
                onClick={() => navigate('/')}
                title={text(lang as LangCode, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'BEENDEN', fr: 'QUITTER', ar: 'خروج' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: 'transparent', borderLeft: '2px solid transparent',
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: '#8abda0', cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <LogOut size={16} style={{ flexShrink: 0 }} />
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'BEENDEN', fr: 'QUITTER', ar: 'خروج' })}</span>}
              </button>

              {/* Speed label */}
              {rightPanelExpanded && (
                <div style={{ padding: '4px 0', textAlign: 'center', borderBottom: '1px solid #4a1a1a', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 5 }}>
                  <Zap size={12} color="#a0c8b0" />
                  <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.18em', fontFamily: 'Share Tech Mono, monospace' }}>{uiText.speed}</span>
                </div>
              )}

              {/* Speed buttons */}
              {SPEEDS.map(s => {
                const isActive = speedMultiplier === s;
                return (
                  <button
                    key={s}
                    onClick={() => changeSpeed(s)}
                    disabled={speedBusy}
                    title={`${s}×`}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                      width: '100%',
                      padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                      fontSize: 13,
                      fontFamily: 'Orbitron, monospace',
                      background: isActive ? 'rgba(0,232,135,0.18)' : 'transparent',
                      borderLeft: `2px solid ${isActive ? '#00e887' : 'transparent'}`,
                      borderTop: 'none',
                      borderRight: 'none',
                      borderBottom: '1px solid #4a1a1a',
                      color: isActive ? '#00e887' : '#8abda0',
                      cursor: speedBusy ? 'not-allowed' : 'pointer',
                      opacity: speedBusy ? 0.5 : 1,
                      whiteSpace: 'nowrap',
                      boxSizing: 'border-box',
                    }}>
                    {s}×
                  </button>
                );
              })}

              {/* Custom speed */}
              {rightPanelExpanded && (
                <div style={{ padding: '6px 8px', borderBottom: '1px solid #4a1a1a' }}>
                  <input
                    type="number" min={1} max={1000}
                    value={customSpeed}
                    onChange={e => setCustomSpeed(e.target.value)}
                    onKeyDown={e => { if (e.key === 'Enter') applyCustomSpeed(); }}
                    placeholder={`${speedMultiplier}×`}
                    style={{
                      width: '100%', fontSize: 12, padding: '3px 5px',
                      background: 'transparent', border: '1px solid rgba(160,200,176,0.3)',
                      color: '#a0c8b0', fontFamily: 'Share Tech Mono, monospace',
                      outline: 'none', boxSizing: 'border-box', marginBottom: 4,
                    }}
                  />
                  <button
                    onClick={applyCustomSpeed}
                    style={{
                      width: '100%', padding: '3px 0', fontSize: 12,
                      border: '1px solid rgba(0,232,135,0.4)', color: '#00e887',
                      background: 'rgba(0,232,135,0.08)', fontFamily: 'Share Tech Mono, monospace',
                      cursor: 'pointer', letterSpacing: '0.05em',
                    }}>
                    {uiText.set}
                  </button>
                </div>
              )}

              {/* Fast-forward / warp */}
              {rightPanelExpanded && (
                <div style={{ padding: '6px 8px', borderBottom: '1px solid #4a1a1a' }}>
                  <div style={{ fontSize: 10, color: '#a0c8b0', letterSpacing: '0.12em', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 4 }}>
                    <FastForward size={10} />
                    {text(lang as LangCode, { tr: 'IŞINIL GİT', en: 'WARP TO', de: 'WARP ZU', fr: 'WARP VERS', ar: 'قفز إلى' })}
                  </div>
                  {isWarping ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                      <div style={{ fontSize: 10, color: '#d4a838', letterSpacing: '0.06em', animation: 'pulse 1s infinite' }}>
                        ⚡ {text(lang as LangCode, { tr: 'WARP AKTİF', en: 'WARPING', de: 'WARP AKTIV', fr: 'EN WARP', ar: 'وارب نشط' })}
                        {fastForwardTarget && ` → Y${Math.floor(fastForwardTarget / 365)}`}
                      </div>
                      <button onClick={cancelFastForward} style={{ width: '100%', padding: '3px 0', fontSize: 11, border: '1px solid rgba(224,90,90,0.5)', color: '#e05a5a', background: 'rgba(224,90,90,0.08)', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 4 }}>
                        <X size={10} /> {text(lang as LangCode, { tr: 'DURDUR', en: 'CANCEL', de: 'STOPP', fr: 'ANNULER', ar: 'إلغاء' })}
                      </button>
                    </div>
                  ) : (
                    <div style={{ display: 'flex', gap: 4 }}>
                      <input
                        type="number" min={1}
                        value={ffTarget}
                        onChange={e => { setFfTarget(e.target.value); if (ffError) setFfError(''); }}
                        onKeyDown={e => { if (e.key === 'Enter') startFastForward(); }}
                        placeholder={text(lang as LangCode, { tr: 'Yıl', en: 'Year', de: 'Jahr', fr: 'Année', ar: 'سنة' })}
                        style={{ flex: 1, fontSize: 11, padding: '3px 4px', background: 'transparent', border: `1px solid ${ffError ? 'rgba(224,90,90,0.6)' : 'rgba(160,200,176,0.3)'}`, color: '#a0c8b0', fontFamily: 'Share Tech Mono, monospace', outline: 'none', minWidth: 0 }}
                      />
                      <button onClick={startFastForward} style={{ padding: '3px 6px', fontSize: 11, border: '1px solid rgba(212,168,56,0.5)', color: '#d4a838', background: 'rgba(212,168,56,0.08)', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', flexShrink: 0 }}>
                        ⚡
                      </button>
                    </div>
                  )}
                  {ffError && (
                    <div style={{ fontSize: 10, color: '#e05a5a', letterSpacing: '0.04em', marginTop: 4, lineHeight: 1.3 }}>
                      {ffError}
                    </div>
                  )}
                </div>
              )}

              {/* Performance panel button */}
              <button
                onClick={() => setActivePanel(activePanel === 'performance' ? null : 'performance')}
                title={text(lang as LangCode, { tr: 'PERFORMANS', en: 'PERF.', de: 'LEIST.', fr: 'PERF.', ar: 'الأداء' })}
                style={{
                  display: 'flex', alignItems: 'center', justifyContent: rightPanelExpanded ? 'flex-start' : 'center',
                  gap: rightPanelExpanded ? 7 : 0,
                  width: '100%', padding: rightPanelExpanded ? '6px 12px' : '7px 0',
                  fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
                  background: activePanel === 'performance' ? 'rgba(79,110,247,0.12)' : 'transparent',
                  borderLeft: `2px solid ${activePanel === 'performance' ? '#4f6ef7' : 'transparent'}`,
                  borderTop: 'none', borderRight: 'none', borderBottom: '1px solid #4a1a1a',
                  color: activePanel === 'performance' ? '#4f6ef7' : '#8abda0',
                  cursor: 'pointer', whiteSpace: 'nowrap', boxSizing: 'border-box',
                }}>
                <span style={{ fontSize: 16, flexShrink: 0 }}>📈</span>
                {rightPanelExpanded && <span>{text(lang as LangCode, { tr: 'PERFORMANS', en: 'PERF.', de: 'LEIST.', fr: 'PERF.', ar: 'الأداء' })}</span>}
              </button>

            </div>

            {/* Toggle */}
            <button
              onClick={() => setRightPanelExpanded(v => !v)}
              style={{
                flexShrink: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '100%',
                padding: '6px 0',
                background: 'rgba(0,20,10,0.8)',
                border: 'none',
                borderTop: '1px solid #4a1a1a',
                color: '#a0c8b0',
                cursor: 'pointer',
                fontSize: 11,
                gap: 4,
              }}>
              {rightPanelExpanded ? <ChevronRight size={12} /> : <ChevronLeft size={12} />}
              {rightPanelExpanded && <span style={{ fontSize: 11, letterSpacing: '0.08em' }}>{text(lang as LangCode, { tr: 'DARALT', en: 'COLLAPSE', de: 'EINKLAPPEN', fr: 'RÉDUIRE', ar: 'طي' })}</span>}
            </button>
          </div>
        )}
      </div>

      {/* ── FOOTER ── */}
      <FooterBar mode="flow" />

      {/* ═══ MENU OVERLAY ═══ */}
      <SimMenuOverlay
        isOpen={menuOpen}
        onClose={() => { setMenuOpen(false); setMenuPage(null); }}
        menuPage={menuPage}
        onMenuPageChange={setMenuPage}
        mobileActions={isMobile ? (
          <div style={{ padding: '8px 14px', borderTop: '1px solid #4a1a1a', display: 'flex', gap: 8 }}>
            <button onClick={() => { setMenuOpen(false); navigate('/'); }}
              style={{ flex: 1, padding: '7px 0', fontSize: 14, border: '1px solid #4a1a1a', color: '#a0c8b0', background: 'transparent', letterSpacing: '0.06em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer' }}>
              ← {text(lang as LangCode, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'BEENDEN', fr: 'QUITTER', ar: 'خروج' })}
            </button>
            <button onClick={() => { setMenuOpen(false); terminateSim(); }}
              style={{ flex: 1, padding: '7px 0', fontSize: 14, border: '1px solid #6a2020', color: '#c05050', background: 'transparent', letterSpacing: '0.06em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer' }}>
              {text(lang as LangCode, { tr: 'SONLANDIR', en: 'TERMINATE', de: 'BEENDEN', fr: 'TERMINER', ar: 'إنهاء' })}
            </button>
          </div>
        ) : undefined}
      />

      {/* ═══ OVERLAY PANELS ═══ */}
      {/* Each lazy panel gets its own boundary + Suspense so a crash or chunk-load
          failure in one can't blank-screen the rest. */}
      <ErrorBoundary name="Events"><Suspense fallback={null}><EventsPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Population"><Suspense fallback={null}><PopulationPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Biology"><Suspense fallback={null}><BiologyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Environment"><Suspense fallback={null}><EnvironmentPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Astronomy"><Suspense fallback={null}><AstronomyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Culture"><Suspense fallback={null}><CulturePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Language"><Suspense fallback={null}><LanguagePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Technology"><Suspense fallback={null}><TechnologyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Belief"><Suspense fallback={null}><BeliefPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Social"><Suspense fallback={null}><SocialPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Economy"><Suspense fallback={null}><EconomyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Art"><Suspense fallback={null}><ArtPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Architecture"><Suspense fallback={null}><ArchitecturePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Law"><Suspense fallback={null}><LawPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Microbiome"><Suspense fallback={null}><MicrobiomePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Psychology"><Suspense fallback={null}><PsychologyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Epigenetics"><Suspense fallback={null}><EpigeneticsPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="GeneticDiversity"><Suspense fallback={null}><GeneticDiversityPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Genealogy"><Suspense fallback={null}><GenealogyPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="God"><Suspense fallback={null}><GodPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Legends"><Suspense fallback={null}><LegendsPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Documentary"><Suspense fallback={null}><DocumentaryPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="TimeMachine"><Suspense fallback={null}><TimeMachinePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Analysis"><Suspense fallback={null}><AnalysisPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Hypothesis"><Suspense fallback={null}><HypothesisPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="PopulationPyramid"><Suspense fallback={null}><PopulationPyramidPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Report"><Suspense fallback={null}><ReportPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Moments"><Suspense fallback={null}><MomentsPanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="Witness"><WitnessPanel /></ErrorBoundary>
      <ErrorBoundary name="Performance"><Suspense fallback={null}><PerformancePanel /></Suspense></ErrorBoundary>
      <ErrorBoundary name="MilestoneToast"><MilestoneToast /></ErrorBoundary>

      {/* ═══ END MODAL ═══ */}
      {endModal && (() => {
        const isManual = endModal.mode === 'manual';
        const reasonMsg = (() => {
          if (!endModal.reason) return '';
          if (endModal.reason === 'population_zero') return text(lang as LangCode, { tr: 'Toplulukta hiç birey kalmadı.', en: 'No individuals remain in the population.', de: 'Keine Individuen mehr in der Population.', fr: 'Plus d\'individus dans la population.', ar: 'لم يتبق أفراد في السكان.' });
          if (endModal.reason === 'no_males') return text(lang as LangCode, { tr: 'Toplulukta erkek birey kalmadı — nesil devamı imkânsız.', en: 'No males remain — reproduction is impossible.', de: 'Keine Männchen mehr — Fortpflanzung unmöglich.', fr: 'Plus de mâles — reproduction impossible.', ar: 'لم يتبق ذكور — التكاثر مستحيل.' });
          if (endModal.reason === 'no_females') return text(lang as LangCode, { tr: 'Toplulukta dişi birey kalmadı — nesil devamı imkânsız.', en: 'No females remain — reproduction is impossible.', de: 'Keine Weibchen mehr — Fortpflanzung unmöglich.', fr: 'Plus de femelles — reproduction impossible.', ar: 'لم تتبق إناث — التكاثر مستحيل.' });
          if (endModal.reason === 'single_individual') return text(lang as LangCode, { tr: 'Toplulukta tek bir birey kaldı — nesil devamı imkânsız.', en: 'Only a single individual remains — reproduction is impossible.', de: 'Nur noch ein Individuum übrig — Fortpflanzung unmöglich.', fr: 'Il ne reste qu\'un seul individu — reproduction impossible.', ar: 'لم يتبق سوى فرد واحد — التكاثر مستحيل.' });
          return '';
        })();
        const title = isManual
          ? text(lang as LangCode, { tr: 'SİMÜLASYONU SONLANDIR?', en: 'TERMINATE SIMULATION?', de: 'SIMULATION BEENDEN?', fr: 'TERMINER LA SIMULATION?', ar: 'إنهاء المحاكاة؟' })
          : text(lang as LangCode, { tr: 'MEDENİYET SONA ERİYOR — SONLANDIRILSIN MI?', en: 'CIVILIZATION IS ENDING — TERMINATE?', de: 'ZIVILISATION ENDET — BEENDEN?', fr: 'LA CIVILISATION S\'ÉTEINT — TERMINER?', ar: 'الحضارة تنتهي — هل تريد الإنهاء؟' });
        const bodyText = isManual
          ? text(lang as LangCode, { tr: 'Bu işlem geri alınamaz.', en: 'This action cannot be undone.', de: 'Diese Aktion kann nicht rückgängig gemacht werden.', fr: 'Cette action est irréversible.', ar: 'لا يمكن التراجع عن هذا الإجراء.' })
          : reasonMsg;
        const btnBase: CSSProperties = {
          padding: '8px 16px', fontSize: 13, fontFamily: 'Share Tech Mono, monospace',
          letterSpacing: '0.08em', cursor: 'pointer', border: '1px solid',
          background: 'transparent',
        };
        return (
          <div style={{
            position: 'fixed', inset: 0, zIndex: 9999,
            background: 'rgba(0,0,0,0.75)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
          }}>
            <div style={{
              background: '#0a0a0a', border: '1px solid #4a1a1a',
              boxShadow: '0 8px 40px rgba(0,0,0,0.9)',
              padding: '28px 32px', minWidth: 320, maxWidth: 400,
              fontFamily: 'Share Tech Mono, monospace',
            }}>
              <div style={{ fontSize: 15, color: isManual ? '#c05050' : '#fbbf24', letterSpacing: '0.12em', marginBottom: 12 }}>
                {title}
              </div>
              {bodyText && (
                <div style={{ fontSize: 13, color: '#a0c8b0', marginBottom: 20, lineHeight: 1.6 }}>
                  {bodyText}
                </div>
              )}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {isManual ? (
                  <>
                    <button
                      onClick={() => doTerminate(true)}
                      style={{ ...btnBase, borderColor: '#fbbf24', color: '#fbbf24' }}>
                      📄 {text(lang as LangCode, { tr: 'RAPOR AL VE SONLANDIR', en: 'GET REPORT & TERMINATE', de: 'BERICHT & BEENDEN', fr: 'RAPPORT & TERMINER', ar: 'تقرير وإنهاء' })}
                    </button>
                    <button
                      onClick={() => doTerminate(false)}
                      style={{ ...btnBase, borderColor: '#c05050', color: '#c05050' }}>
                      {text(lang as LangCode, { tr: 'SONLANDIR', en: 'TERMINATE', de: 'BEENDEN', fr: 'TERMINER', ar: 'إنهاء' })}
                    </button>
                    <button
                      onClick={() => setEndModal(null)}
                      style={{ ...btnBase, borderColor: '#4a6050', color: '#8abda0' }}>
                      {text(lang as LangCode, { tr: 'İPTAL', en: 'CANCEL', de: 'ABBRECHEN', fr: 'ANNULER', ar: 'إلغاء' })}
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      onClick={() => doTerminate(true)}
                      style={{ ...btnBase, borderColor: '#fbbf24', color: '#fbbf24' }}>
                      📄 {text(lang as LangCode, { tr: 'RAPOR AL VE SONLANDIR', en: 'GET REPORT & TERMINATE', de: 'BERICHT & BEENDEN', fr: 'RAPPORT & TERMINER', ar: 'تقرير وإنهاء' })}
                    </button>
                    <button
                      onClick={() => doTerminate(false)}
                      style={{ ...btnBase, borderColor: '#c05050', color: '#c05050' }}>
                      {text(lang as LangCode, { tr: 'EVET, SONLANDIR', en: 'YES, TERMINATE', de: 'JA, BEENDEN', fr: 'OUI, TERMINER', ar: 'نعم، أنهِ' })}
                    </button>
                    <button
                      onClick={() => setEndModal(null)}
                      style={{ ...btnBase, borderColor: '#4a6050', color: '#8abda0' }}>
                      {text(lang as LangCode, { tr: 'HAYIR, DEVAM ET', en: 'NO, CONTINUE', de: 'NEIN, WEITER', fr: 'NON, CONTINUER', ar: 'لا، تابع' })}
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        );
      })()}
    </div>
  );
}
