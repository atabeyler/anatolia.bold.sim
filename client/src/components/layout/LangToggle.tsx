import { useSimStore } from '../../store/simStore';
import type { LangCode } from '../../utils/i18n';

const LANGS: { code: LangCode; label: string }[] = [
  { code: 'tr', label: 'TR' },
  { code: 'en', label: 'EN' },
  { code: 'de', label: 'DE' },
  { code: 'fr', label: 'FR' },
  { code: 'ar', label: 'AR' },
];

export default function LangToggle() {
  const { lang, setLang } = useSimStore();
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 0,
      border: '1px solid rgba(79,110,247,0.4)', background: 'transparent',
      fontFamily: 'Share Tech Mono, monospace', fontSize: 14,
      letterSpacing: '0.08em', overflow: 'hidden',
    }}>
      {LANGS.map((l, i) => (
        <span key={l.code} style={{ display: 'flex', alignItems: 'center', gap: 0 }}>
          {i > 0 && <span style={{ color: 'rgba(79,110,247,0.4)', padding: '3px 0' }}>|</span>}
          <button
            onClick={() => setLang(l.code)}
            style={{
              padding: '5px 8px',
              color: lang === l.code ? '#a0b4ff' : '#4a5578',
              background: lang === l.code ? 'rgba(79,110,247,0.25)' : 'transparent',
              fontWeight: lang === l.code ? 700 : 400,
              transition: 'all 0.2s',
              border: 'none',
              fontFamily: 'Share Tech Mono, monospace',
              fontSize: 14,
              letterSpacing: '0.08em',
              cursor: 'pointer',
            }}
          >{l.label}</button>
        </span>
      ))}
    </div>
  );
}
