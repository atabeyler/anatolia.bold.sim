import { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';
import {
  GUIDE_BLOCKS,
  guideText,
  menuText,
  pageText,
} from '../../utils/menuI18n';

type Page = 'guide' | 'about' | 'mission' | 'contact' | null;

function H({ children }: { children: React.ReactNode }) {
  return <div style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.18em', margin: '14px 0 5px', paddingBottom: 3, borderBottom: '1px solid #0d2a18' }}>{children}</div>;
}
function Sub({ children }: { children: React.ReactNode }) {
  return <div style={{ fontSize: 14, color: '#00c870', letterSpacing: '0.08em', margin: '7px 0 3px' }}>{children}</div>;
}
function Row({ label, val }: { label: React.ReactNode; val: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', gap: 6, margin: '2px 0' }}>
      <span style={{ fontSize: 14, color: '#4a8a60', minWidth: 110, flexShrink: 0 }}>{label}</span>
      <span style={{ fontSize: 14, color: '#7aaa90', lineHeight: 1.5 }}>{val}</span>
    </div>
  );
}
function Note({ children }: { children: React.ReactNode }) {
  return <div style={{ fontSize: 14, color: '#6a9a78', margin: '3px 0', lineHeight: 1.5, paddingLeft: 8, borderLeft: '2px solid #0d2a18' }}>{children}</div>;
}
function Bullet({ children }: { children: React.ReactNode }) {
  return <div style={{ fontSize: 14, color: '#7aaa90', margin: '2px 0 2px 8px', lineHeight: 1.5 }}>› {children}</div>;
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
  /** Controlled page — set from outside to navigate to a specific sub-page */
  menuPage?: Page;
  onMenuPageChange?: (p: Page) => void;
  /** Extra items shown at the bottom of the main menu list (e.g. mobile EXIT/TERMINATE for SimulationPage) */
  mobileActions?: React.ReactNode;
}

const MENU_ITEMS: Array<{ id: NonNullable<Page>; labelKey: Parameters<typeof menuText>[1] }> = [
  { id: 'guide', labelKey: 'menu_guide' },
  { id: 'about', labelKey: 'menu_about' },
  { id: 'mission', labelKey: 'menu_mission' },
  { id: 'contact', labelKey: 'menu_contact' },
];

const PAGE_TITLE_KEYS: Record<NonNullable<Page>, Parameters<typeof menuText>[1]> = {
  guide: 'page_guide',
  about: 'page_about',
  mission: 'page_mission',
  contact: 'page_contact',
};

function renderGuideBlock(block: (typeof GUIDE_BLOCKS)[number], lang: LangCode, index: number) {
  switch (block.kind) {
    case 'h':
      return <H key={index}>{guideText(lang, block.text)}</H>;
    case 'sub':
      return <Sub key={index}>{guideText(lang, block.text)}</Sub>;
    case 'row':
      return <Row key={index} label={guideText(lang, block.label)} val={guideText(lang, block.value)} />;
    case 'note':
      return <Note key={index}>{guideText(lang, block.text)}</Note>;
    case 'bullet':
      return <Bullet key={index}>{guideText(lang, block.text)}</Bullet>;
    default:
      return null;
  }
}

export default function SimMenuOverlay({ isOpen, onClose, mobileActions, menuPage, onMenuPageChange }: Props) {
  const { lang, setSettingsOpen } = useSimStore();
  const activeLang = lang as LangCode;
  const [internalPage, setInternalPage] = useState<Page>(null);

  const page = menuPage !== undefined ? menuPage : internalPage;
  function setPage(p: Page) {
    if (onMenuPageChange) onMenuPageChange(p);
    else setInternalPage(p);
  }

  // Reset internal page when menu closes
  useEffect(() => { if (!isOpen) setInternalPage(null); }, [isOpen]);

  if (!isOpen) return null;

  function close() { setPage(null); onClose(); }

  return (
    <div
      style={{ position: 'fixed', inset: 0, zIndex: 200, background: 'rgba(0,0,0,0.85)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
      onClick={close}>
      <div
        dir={activeLang === 'ar' ? 'rtl' : 'ltr'}
        style={{ background: 'rgba(0,4,2,0.98)', border: '1px solid #cc2222', width: 'clamp(300px, 90vw, 560px)', fontFamily: 'Share Tech Mono, monospace', boxShadow: '0 8px 40px rgba(0,0,0,0.8)', overflow: 'hidden' }}
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 14px', borderBottom: '1px solid #cc2222', background: 'rgba(0,20,10,0.9)' }}>
          <div style={{ width: 3, height: 14, background: '#00e887', boxShadow: '0 0 6px #00e887', flexShrink: 0 }} />
          <span style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.2em', flex: 1 }}>
            {page === null ? 'ANATOLIA-SIM' : menuText(activeLang, PAGE_TITLE_KEYS[page])}
          </span>
          <button
            onClick={() => { if (page) setPage(null); else close(); }}
            style={{ background: 'transparent', border: 'none', color: '#6a9a78', cursor: 'pointer', fontSize: 14, letterSpacing: '0.1em', padding: '2px 6px', display: 'flex', alignItems: 'center' }}>
            {page ? menuText(activeLang, 'back') : <X size={12} />}
          </button>
        </div>

        {/* Main menu list */}
        {page === null && (
          <div style={{ padding: '6px 0' }}>
            {/* Settings lives here rather than as its own header button on
                every page -- consolidating the two icon-only header buttons
                (⚙ and ☰, formerly a standalone SettingsButton component) into
                one menu entry point is both a cleaner header and what fixed
                the header overflowing on narrow phones (see DashboardPage's
                own mobile-overflow fix). Opens the globally-mounted
                SettingsOverlay (App.tsx) via the shared store flag. */}
            <button onClick={() => { onClose(); setSettingsOpen(true); }}
              style={{ display: 'block', width: '100%', padding: '9px 14px', background: 'transparent', border: 'none', borderBottom: '1px solid #0a1a10', color: '#a0c8b0', fontSize: 14, textAlign: activeLang === 'ar' ? 'right' : 'left', cursor: 'pointer', letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace' }}>
              › {menuText(activeLang, 'menu_settings')}
            </button>
            {MENU_ITEMS.map(item => (
              <button key={item.id} onClick={() => setPage(item.id)}
                style={{ display: 'block', width: '100%', padding: '9px 14px', background: 'transparent', border: 'none', borderBottom: '1px solid #0a1a10', color: '#a0c8b0', fontSize: 14, textAlign: activeLang === 'ar' ? 'right' : 'left', cursor: 'pointer', letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace' }}>
                › {menuText(activeLang, item.labelKey)}
              </button>
            ))}
            {mobileActions}
            <div style={{ padding: '8px clamp(10px, 2vw, 14px)', borderTop: '1px solid #0a1a10', marginTop: 4, display: 'flex', justifyContent: 'center' }}>
              <div style={{ fontSize: 'clamp(11px, 1.35vw, 13px)', color: '#00e887', letterSpacing: '0.06em', lineHeight: 1.55, textShadow: '0 0 8px rgba(0,232,135,0.35)', textAlign: 'center', maxWidth: '100%', width: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2, wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
                <div>RST Q-Nation 200120401018 © 2026</div>
                <div style={{ whiteSpace: 'normal', lineHeight: 1.5 }}>{text(activeLang, { tr: 'Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.', en: 'Bold Military Technology and Defense Industry Inc.', de: 'Bold Militärtechnologie und Verteidigungsindustrie AG', fr: 'Bold Technologie Militaire et Industrie de Défense S.A.', ar: 'شركة Bold للتكنولوجيا العسكرية وصناعة الدفاع' })}</div>
              </div>
            </div>
          </div>
        )}

        {/* About / Mission / Contact */}
        {page !== null && page !== 'guide' && (
          <div style={{ padding: '12px 14px', maxHeight: 320, overflowY: 'auto' }}>
            {pageText(activeLang, page).split('\n').map((line, i) => (
              <p key={i} style={{ fontSize: 14, color: line === line.toUpperCase() && line.length > 2 ? '#00e887' : '#7aaa90', margin: '0 0 5px 0', letterSpacing: '0.05em', lineHeight: 1.6, textAlign: activeLang === 'ar' ? 'right' : 'left' }}>
                {line || <br />}
              </p>
            ))}
          </div>
        )}

        {/* User Guide */}
        {page === 'guide' && (
          <div style={{ padding: '10px 14px 14px', maxHeight: 480, overflowY: 'auto', fontSize: 14, textAlign: activeLang === 'ar' ? 'right' : 'left' }}>
            {GUIDE_BLOCKS.map((block, index) => renderGuideBlock(block, activeLang, index))}
            <div style={{ marginTop: 16, paddingTop: 8, borderTop: '1px solid #0a1a10', display: 'flex', justifyContent: 'center' }}>
              <div style={{ fontSize: 'clamp(11px, 1.3vw, 13px)', color: '#00e887', letterSpacing: '0.06em', lineHeight: 1.55, textShadow: '0 0 8px rgba(0,232,135,0.25)', maxWidth: '100%', width: '100%', textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2, wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
                <div>ANATOLIA-SIM · RST Q-Nation 200120401018 © 2026</div>
                <div style={{ whiteSpace: 'normal', lineHeight: 1.5 }}>{text(activeLang, { tr: 'Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.', en: 'Bold Military Technology and Defense Industry Inc.', de: 'Bold Militärtechnologie und Verteidigungsindustrie AG', fr: 'Bold Technologie Militaire et Industrie de Défense S.A.', ar: 'شركة Bold للتكنولوجيا العسكرية وصناعة الدفاع' })}</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
