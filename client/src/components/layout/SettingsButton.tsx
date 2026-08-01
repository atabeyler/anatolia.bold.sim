import type { CSSProperties } from 'react';
import { Settings } from 'lucide-react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

/** A small inline trigger meant to sit right next to each page's "☰ MENÜ"
 * button; opens the single globally-mounted SettingsOverlay (see App.tsx)
 * via the store's settingsOpen flag rather than owning its own modal state,
 * so every page's button opens the same overlay instance. */
export default function SettingsButton({ style }: { style?: CSSProperties }) {
  const lang = useSimStore(s => s.lang);
  const setSettingsOpen = useSimStore(s => s.setSettingsOpen);

  return (
    <button
      onClick={() => setSettingsOpen(true)}
      style={{
        display: 'flex', alignItems: 'center', gap: 3,
        fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
        ...style,
      }}>
      <Settings size={13} />
      <span>{text(lang as LangCode, { tr: 'AYARLAR', en: 'SETTINGS', de: 'EINSTELLUNGEN', fr: 'PARAMÈTRES', ar: 'الإعدادات' })}</span>
    </button>
  );
}
