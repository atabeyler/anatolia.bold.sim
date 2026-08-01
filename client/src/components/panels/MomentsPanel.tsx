import { useSimStore } from '../../store/simStore';
import DetailPanel from './DetailPanel';
import { Sparkles } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

export default function MomentsPanel() {
  const { moments, clearMoments, lang, stats } = useSimStore();
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });

  return (
    <DetailPanel panelId="moments" title="Moments" titleTr="Anlar">

      <div className="flex items-center justify-between mb-3">
        <span className="font-share-tech tracking-widest" style={{ fontSize: 12, color: '#6a8878' }}>
          {moments.length} {t('an kaydedildi', 'moments recorded', 'Momente aufgezeichnet', 'moments enregistrés', 'لحظات مسجلة')}
        </span>
        {moments.length > 0 && (
          <button onClick={clearMoments}
            className="font-share-tech"
            style={{ fontSize: 12, color: '#6a8878', background: 'transparent', border: '1px solid rgba(160,200,176,0.2)', padding: '1px 6px', cursor: 'pointer' }}>
            {t('Temizle', 'Clear', 'Löschen', 'Effacer', 'مسح')}
          </button>
        )}
      </div>

      {moments.length === 0 && (
        <div className="flex flex-col items-center py-8 gap-2">
          <Sparkles size={24} style={{ color: 'rgba(160,200,176,0.2)' }} />
          <span className="font-share-tech tracking-widest" style={{ fontSize: 12, color: 'rgba(160,200,176,0.3)' }}>
            {t('Henüz önemli bir an yok', 'No moments yet', 'Noch keine Momente', 'Aucun moment pour le moment', 'لا توجد لحظات بعد')}
          </span>
          <span className="font-share-tech" style={{ fontSize: 12, color: 'rgba(160,200,176,0.2)', textAlign: 'center', lineHeight: 1.5 }}>
            {t('İlk ölüm, teknoloji keşfi, afetler\nve dil evrimi burada görünür', 'First deaths, discoveries,\ndisasters and language milestones\nwill appear here',
              'Erste Todesfälle, Entdeckungen,\nKatastrophen und Sprachmeilensteine\nerscheinen hier',
              'Les premiers décès, découvertes,\ncatastrophes et jalons linguistiques\napparaîtront ici',
              'ستظهر هنا أول الوفيات والاكتشافات\nوالكوارث ومحطات تطور اللغة')}
          </span>
        </div>
      )}

      <div className="space-y-2">
        {moments.map(m => (
          <div key={m.id} style={{
            background: 'rgba(4,4,18,0.7)',
            border: `1px solid ${m.color}25`,
            borderLeft: `3px solid ${m.color}`,
            padding: '8px 10px',
          }}>
            <div className="flex items-start gap-2">
              <span style={{ fontSize: 16, lineHeight: 1, flexShrink: 0, marginTop: 1 }}>{m.icon}</span>
              <div className="flex-1 min-w-0">
                <div className="font-share-tech" style={{ fontSize: 12, color: m.color, lineHeight: 1.4 }}>{m.title}</div>
                {m.description && (
                  <div className="font-share-tech" style={{ fontSize: 12, color: '#8898c8', marginTop: 2, lineHeight: 1.4 }}>
                    {m.description.length > 80 ? m.description.slice(0, 80) + '…' : m.description}
                  </div>
                )}
              </div>
              <div className="font-share-tech flex-shrink-0 text-right" style={{ fontSize: 12, color: '#6a8878', lineHeight: 1.6 }}>
                <div>Y{m.year}</div>
                <div>G{m.day}</div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {stats && moments.length === 0 && (
        <div className="mt-4 font-share-tech" style={{ fontSize: 12, color: '#6a8878', textAlign: 'center' }}>
          {t(`Simülasyon ${stats.year}. yılında`, `Simulation is in year ${stats.year}`, `Die Simulation ist im Jahr ${stats.year}`, `La simulation est en l'année ${stats.year}`, `المحاكاة في السنة ${stats.year}`)}
        </div>
      )}

    </DetailPanel>
  );
}
