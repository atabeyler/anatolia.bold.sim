import { X } from 'lucide-react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode, type TranslationMap } from '../../utils/i18n';

interface Props {
  panelId: string;
  title: string;
  titleTr: string;
  titleDe?: string;
  titleFr?: string;
  titleAr?: string;
  children: React.ReactNode;
}

const PANEL_TITLE_FALLBACKS: Record<string, TranslationMap> = {
  population: { tr: 'Nüfus', en: 'Population', de: 'Bevölkerung', fr: 'Population', ar: 'السكان' },
  events: { tr: 'Olaylar', en: 'Events', de: 'Ereignisse', fr: 'Événements', ar: 'الأحداث' },
  language: { tr: 'Dil', en: 'Language', de: 'Sprache', fr: 'Langue', ar: 'اللغة' },
  history: { tr: 'Geçmiş', en: 'History', de: 'Geschichte', fr: 'Historique', ar: 'التاريخ' },
  'time machine': { tr: 'Zaman Makinesi', en: 'Time Machine', de: 'Zeitmaschine', fr: 'Machine temporelle', ar: 'آلة الزمن' },
  analysis: { tr: 'Analiz', en: 'Analysis', de: 'Analyse', fr: 'Analyse', ar: 'التحليل' },
  hypothesis: { tr: 'Hipotez', en: 'Hypothesis', de: 'Hypothese', fr: 'Hypothèse', ar: 'الفرضية' },
  pyramid: { tr: 'Piramit', en: 'Pyramid', de: 'Pyramide', fr: 'Pyramide', ar: 'الهرم' },
  biology: { tr: 'Biyoloji', en: 'Biology', de: 'Biologie', fr: 'Biologie', ar: 'البيولوجيا' },
  mutation: { tr: 'Mutasyon', en: 'Mutation', de: 'Mutation', fr: 'Mutation', ar: 'الطفرة' },
  'god mode': { tr: 'Tanrı Modu', en: 'God Mode', de: 'Gottmodus', fr: 'Mode Dieu', ar: 'وضع الإله' },
  psychology: { tr: 'Psikoloji', en: 'Psychology', de: 'Psychologie', fr: 'Psychologie', ar: 'علم النفس' },
  mind: { tr: 'Akıl', en: 'Mind', de: 'Geist', fr: 'Esprit', ar: 'العقل' },
  environment: { tr: 'Çevre', en: 'Environment', de: 'Umwelt', fr: 'Environnement', ar: 'البيئة' },
  technology: { tr: 'Teknoloji', en: 'Technology', de: 'Technologie', fr: 'Technologie', ar: 'التكنولوجيا' },
  belief: { tr: 'İnanç', en: 'Belief', de: 'Glaube', fr: 'Croyance', ar: 'المعتقد' },
  social: { tr: 'Sosyal', en: 'Social', de: 'Sozial', fr: 'Social', ar: 'اجتماعي' },
  economy: { tr: 'Ekonomi', en: 'Economy', de: 'Wirtschaft', fr: 'Économie', ar: 'الاقتصاد' },
  culture: { tr: 'Kültür', en: 'Culture', de: 'Kultur', fr: 'Culture', ar: 'الثقافة' },
  art: { tr: 'Sanat', en: 'Art', de: 'Kunst', fr: 'Art', ar: 'الفن' },
  astronomy: { tr: 'Astronomi', en: 'Astronomy', de: 'Astronomie', fr: 'Astronomie', ar: 'علم الفلك' },
  moments: { tr: 'Anlar', en: 'Moments', de: 'Momente', fr: 'Moments', ar: 'اللحظات' },
  epigenetics: { tr: 'Epigenetik', en: 'Epigenetics', de: 'Epigenetik', fr: 'Épigénétique', ar: 'علم التخلق' },
  architecture: { tr: 'Mimari', en: 'Architecture', de: 'Architektur', fr: 'Architecture', ar: 'العمارة' },
  law: { tr: 'Hukuk', en: 'Law', de: 'Recht', fr: 'Droit', ar: 'القانون' },
  microbiome: { tr: 'Mikrobiyom', en: 'Microbiome', de: 'Mikrobiom', fr: 'Microbiome', ar: 'الميكروبيوم' },
  genealogy: { tr: 'Soy Ağacı', en: 'Genealogy', de: 'Genealogie', fr: 'Généalogie', ar: 'النسب' },
  'genetic diversity': { tr: 'Genetik Çeşitlilik', en: 'Genetic Diversity', de: 'Genetische Vielfalt', fr: 'Diversité génétique', ar: 'التنوع الجيني' },
  report: { tr: 'Rapor', en: 'Report', de: 'Bericht', fr: 'Rapport', ar: 'تقرير' },
  stats: { tr: 'İstatistik', en: 'Stats', de: 'Statistik', fr: 'Statistiques', ar: 'إحصاءات' },
};

function normalizeTitleKey(value?: string): string {
  return String(value ?? '').trim().replace(/\s+/g, ' ').toLowerCase();
}

export default function DetailPanel({ panelId, title, titleTr, titleDe, titleFr, titleAr, children }: Props) {
  const { activePanel, setActivePanel, lang } = useSimStore();
  if (activePanel !== panelId) return null;

  const fallbackTitle = PANEL_TITLE_FALLBACKS[normalizeTitleKey(title)] ?? PANEL_TITLE_FALLBACKS[normalizeTitleKey(titleTr)];
  const displayTitle = text(lang as LangCode, fallbackTitle ?? { en: title, tr: titleTr, de: titleDe ?? title, fr: titleFr ?? title, ar: titleAr ?? title });
  const moduleLabel = text(lang as LangCode, { tr: 'MODÜL', en: 'MODULE', de: 'MODUL', fr: 'MODULE', ar: 'وحدة' });

  return (
    <>
      {/* Click-outside backdrop */}
      <div
        className="fixed inset-0 z-30"
        onClick={() => setActivePanel(null)}
      />
      <div
        className="fixed top-0 left-0 bottom-0 z-40 flex flex-col overflow-hidden"
        style={{
          width: typeof window !== 'undefined' && window.innerWidth < 480 ? '100vw' : 368,
          background: 'rgba(2,8,4,0.98)',
          borderRight: '1px solid #cc2222',
          backdropFilter: 'blur(20px)',
          boxShadow: '4px 0 40px rgba(0,80,40,0.2)',
        }}>

      {/* Corner brackets */}
      <span style={{
        position: 'absolute', top: -1, right: -1, width: 14, height: 14,
        borderTop: '2px solid rgba(200,34,34,0.9)', borderRight: '2px solid rgba(200,34,34,0.9)',
        animation: 'corner-blink 4s ease-in-out infinite 1s', pointerEvents: 'none', zIndex: 2,
      }} />
      <span style={{
        position: 'absolute', bottom: -1, left: -1, width: 14, height: 14,
        borderBottom: '2px solid rgba(200,34,34,0.9)', borderLeft: '2px solid rgba(200,34,34,0.9)',
        animation: 'corner-blink 4s ease-in-out infinite 2s', pointerEvents: 'none', zIndex: 2,
      }} />

      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 flex-shrink-0"
        style={{ background: 'rgba(10,0,0,0.95)', borderBottom: '1px solid #cc2222' }}>
        <div className="w-1 h-4 flex-shrink-0" style={{ background: '#00e887', boxShadow: '0 0 6px #00e887' }} />
        <span style={{ fontFamily: 'Orbitron, monospace', fontSize: 12, fontWeight: 700, letterSpacing: '0.2em', color: '#00e887' }}>
          {displayTitle.toUpperCase()}
        </span>
        <div className="flex-1" />
        <button onClick={() => setActivePanel(null)} style={{ color: '#a0c8b0', lineHeight: 0 }}>
          <X size={13} />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4"
        style={{ color: '#a0c8b0', fontSize: 14, lineHeight: 1.6 }}>
        {children}
      </div>

      {/* Footer bar */}
      <div className="px-3 py-1 flex-shrink-0"
        style={{ borderTop: '1px solid #cc2222', background: 'rgba(0,8,4,0.9)' }}>
        <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color: '#6a4040', letterSpacing: '0.15em' }}>
          {moduleLabel}/{displayTitle.toUpperCase().replace(/ /g, '_')}
        </span>
      </div>
    </div>
    </>
  );
}
