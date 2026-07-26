import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

export default function UpdateBanner() {
  const { updatePercent, updateReady, updateInstall, setUpdateReady, lang } = useSimStore();
  const l = lang as LangCode;

  if (updatePercent !== null) {
    return (
      <div className="fixed bottom-0 left-0 right-0 z-[9999]" style={{ background: 'rgba(4,4,18,0.97)', borderTop: '1px solid rgba(79,110,247,0.45)', padding: '10px 20px' }}>
        <div className="flex items-center gap-3 max-w-2xl mx-auto">
          <svg className="animate-spin w-4 h-4 flex-shrink-0" style={{ color: '#4f6ef7' }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
          </svg>
          <div className="flex-1">
            <div className="flex justify-between mb-1">
              <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: '#4f6ef7', letterSpacing: '0.08em' }}>
                {text(l, { tr: 'GÜNCELLEME İNDİRİLİYOR', en: 'DOWNLOADING UPDATE', de: 'UPDATE WIRD HERUNTERGELADEN', fr: 'TÉLÉCHARGEMENT DE LA MISE À JOUR', ar: 'جارٍ تنزيل التحديث' })}
              </span>
              <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: '#00e887' }}>
                {updatePercent}%
              </span>
            </div>
            <div className="w-full rounded-full" style={{ height: 4, background: 'rgba(79,110,247,0.2)' }}>
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{ width: `${updatePercent}%`, background: 'linear-gradient(90deg, #4f6ef7, #00e887)' }}
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (updateReady) {
    return (
      <div className="fixed bottom-0 left-0 right-0 z-[9999]" style={{ background: 'rgba(4,4,18,0.97)', borderTop: '1px solid rgba(0,232,135,0.5)', padding: '10px 20px' }}>
        <div className="flex items-center gap-3 max-w-2xl mx-auto">
          <div className="w-2 h-2 rounded-full flex-shrink-0" style={{ background: '#00e887', boxShadow: '0 0 6px #00e887' }} />
          <span className="flex-1" style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: '#00e887', letterSpacing: '0.06em' }}>
            {updateReady.version
              ? `v${updateReady.version} ${text(l, { tr: 'HAZIR', en: 'READY', de: 'BEREIT', fr: 'PRÊT', ar: 'جاهز' })}`
              : text(l, { tr: 'GÜNCELLEME HAZIR', en: 'UPDATE READY', de: 'UPDATE BEREIT', fr: 'MISE À JOUR PRÊTE', ar: 'التحديث جاهز' })}
            {' — '}
            {text(l, { tr: 'yüklemek için tıkla', en: 'click to install', de: 'zum Installieren klicken', fr: 'cliquer pour installer', ar: 'انقر للتثبيت' })}
          </span>
          <button
            onClick={() => updateInstall?.()}
            style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 13, color: '#fff', background: 'rgba(0,232,135,0.2)', border: '1px solid rgba(0,232,135,0.6)', padding: '4px 16px', cursor: 'pointer', letterSpacing: '0.08em' }}
          >
            {text(l, { tr: 'YÜKLE & YENİDEN BAŞLAT', en: 'INSTALL & RESTART', de: 'INSTALLIEREN & NEU STARTEN', fr: 'INSTALLER ET REDÉMARRER', ar: 'تثبيت وإعادة التشغيل' })}
          </button>
          <button
            onClick={() => setUpdateReady(null)}
            style={{ color: '#6a8a9a', background: 'none', border: 'none', cursor: 'pointer', fontSize: 16, lineHeight: 1 }}
          >
            ✕
          </button>
        </div>
      </div>
    );
  }

  return null;
}
