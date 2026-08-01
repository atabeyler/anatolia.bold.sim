import { useEffect } from 'react';
import { Dna, TreePine, Telescope, Brain, MessageSquare, Cpu, Flame, Swords, Coins, Music, Building2, Scale, Microscope, HeartPulse, Zap, Clock, Bot, FlaskConical, Users, GitBranch, ChevronLeft, ChevronRight } from 'lucide-react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

const MODULES = [
  { id: 'population',  icon: Users,          labels: { tr: 'Nüfus',          en: 'Population',      de: 'Bevölkerung',       fr: 'Population',        ar: 'السكان' } },
  { id: 'biology',     icon: Dna,            labels: { tr: 'Biyoloji',       en: 'Biology',         de: 'Biologie',          fr: 'Biologie',          ar: 'علم الأحياء' } },
  { id: 'environment', icon: TreePine,       labels: { tr: 'Çevre',          en: 'Environment',     de: 'Umwelt',            fr: 'Environnement',     ar: 'البيئة' } },
  { id: 'astronomy',   icon: Telescope,      labels: { tr: 'Astronomi',      en: 'Astronomy',       de: 'Astronomie',        fr: 'Astronomie',        ar: 'علم الفلك' } },
  { id: 'culture',     icon: Brain,          labels: { tr: 'Kültür',         en: 'Culture',         de: 'Kultur',            fr: 'Culture',           ar: 'الثقافة' } },
  { id: 'language',    icon: MessageSquare,  labels: { tr: 'Dil',            en: 'Language',        de: 'Sprache',           fr: 'Langue',            ar: 'اللغة' } },
  { id: 'technology',  icon: Cpu,            labels: { tr: 'Teknoloji',      en: 'Technology',      de: 'Technologie',       fr: 'Technologie',       ar: 'التكنولوجيا' } },
  { id: 'belief',      icon: Flame,          labels: { tr: 'İnanç',          en: 'Belief',          de: 'Glaube',            fr: 'Croyance',          ar: 'المعتقد' } },
  { id: 'social',      icon: Swords,         labels: { tr: 'Sosyal',         en: 'Social',          de: 'Sozial',            fr: 'Social',            ar: 'اجتماعي' } },
  { id: 'economy',     icon: Coins,          labels: { tr: 'Ekonomi',        en: 'Economy',         de: 'Wirtschaft',        fr: 'Économie',          ar: 'الاقتصاد' } },
  { id: 'art',         icon: Music,          labels: { tr: 'Sanat',          en: 'Art',             de: 'Kunst',             fr: 'Art',               ar: 'الفن' } },
  { id: 'architecture',icon: Building2,      labels: { tr: 'Mimari',         en: 'Architecture',    de: 'Architektur',       fr: 'Architecture',      ar: 'العمارة' } },
  { id: 'law',         icon: Scale,          labels: { tr: 'Hukuk',          en: 'Law',             de: 'Recht',             fr: 'Droit',             ar: 'القانون' } },
  { id: 'microbiome',  icon: Microscope,     labels: { tr: 'Mikrobiyom',     en: 'Microbiome',      de: 'Mikrobiom',         fr: 'Microbiome',        ar: 'الميكروبيوم' } },
  { id: 'psychology',  icon: HeartPulse,     labels: { tr: 'Psikoloji',      en: 'Psychology',      de: 'Psychologie',       fr: 'Psychologie',       ar: 'علم النفس' } },
  { id: 'epigenetics', icon: Zap,            labels: { tr: 'Epigenetik',     en: 'Epigenetics',     de: 'Epigenetik',        fr: 'Épigénétique',      ar: 'فوق الجينات' } },
  { id: 'genealogy',   icon: GitBranch,      labels: { tr: 'Soy Ağacı',      en: 'Genealogy',       de: 'Genealogie',        fr: 'Généalogie',        ar: 'الأنساب' } },
  { id: 'god',         icon: Zap,            labels: { tr: 'Tanrı Modu',     en: 'God Mode',        de: 'Gottmodus',         fr: 'Mode Dieu',         ar: 'وضع الإله' }, divider: true, accent: '#f97316' },
  { id: 'timemachine', icon: Clock,          labels: { tr: 'Zaman Makinesi', en: 'Time Machine',    de: 'Zeitmaschine',      fr: 'Machine temporelle', ar: 'آلة الزمن' }, accent: '#00d4ff' },
  { id: 'analysis',    icon: Bot,            labels: { tr: 'AI Analiz',      en: 'AI Analysis',     de: 'KI-Analyse',        fr: 'Analyse IA',        ar: 'تحليل الذكاء الاصطناعي' }, accent: '#4ecb71' },
  { id: 'hypothesis',  icon: FlaskConical,   labels: { tr: 'Hipotez',        en: 'Hypothesis',      de: 'Hypothese',         fr: 'Hypothèse',         ar: 'الفرضية' }, accent: '#d4a838' },
];

export default function LeftPanel() {
  const { activePanel, setActivePanel, lang, sidebarExpanded, toggleSidebar } = useSimStore();

  useEffect(() => {
    if (window.innerWidth < 768 && sidebarExpanded) toggleSidebar();
  }, []);

  const W = sidebarExpanded ? 176 : 48;

  return (
    <div
      className="fixed left-0 top-12 bottom-0 z-40 flex flex-col py-2 overflow-hidden"
      style={{
        width: W,
        transition: 'width 0.22s ease',
        background: 'rgba(3,3,16,0.97)',
        borderRight: '1px solid rgba(79,110,247,0.28)',
        backdropFilter: 'blur(20px)',
        boxShadow: '2px 0 30px rgba(79,110,247,0.06)',
      }}>
      <div className="w-full h-px mb-2 flex-shrink-0" style={{ background: 'linear-gradient(90deg, transparent, rgba(79,110,247,0.5), transparent)' }} />

      <div className="flex-1 overflow-y-auto overflow-x-hidden flex flex-col gap-0.5 px-1.5">
        {MODULES.map((mod) => {
          const Icon = mod.icon;
          const isActive = activePanel === mod.id;
          const accent = (mod as any).accent ?? '#4f6ef7';

          return (
            <div key={mod.id} className="flex-shrink-0">
              {(mod as any).divider && (
                <div className="mx-1 my-1.5 h-px" style={{ background: 'linear-gradient(90deg, transparent, rgba(79,110,247,0.35), transparent)' }} />
              )}
              <button
                onClick={() => setActivePanel(isActive ? null : mod.id)}
                className="w-full flex items-center gap-2.5 transition-all duration-150 rounded-sm"
                style={{
                  height: 32,
                  padding: sidebarExpanded ? '0 8px' : '0',
                  justifyContent: sidebarExpanded ? 'flex-start' : 'center',
                  background: isActive ? `${accent}22` : 'transparent',
                  border: isActive ? `1px solid ${accent}55` : '1px solid transparent',
                  boxShadow: isActive ? `0 0 10px ${accent}35` : 'none',
                  color: isActive ? accent : '#9aabcf',
                }}>
                <Icon size={14} style={{ flexShrink: 0, filter: isActive ? `drop-shadow(0 0 4px ${accent})` : 'none' }} />
                {sidebarExpanded && (
                  <span className="font-share-tech tracking-wider whitespace-nowrap overflow-hidden text-ellipsis"
                    style={{ fontSize: 12, color: isActive ? accent : '#9aabcf' }}>
                    {text(lang as LangCode, mod.labels)}
                  </span>
                )}
              </button>
            </div>
          );
        })}
      </div>

      <div className="flex-shrink-0 px-1.5 pb-1 pt-2" style={{ borderTop: '1px solid rgba(79,110,247,0.12)' }}>
        <button
          onClick={toggleSidebar}
          className="w-full flex items-center transition-all duration-150 rounded-sm"
          style={{
            height: 28,
            padding: sidebarExpanded ? '0 8px' : '0',
            justifyContent: sidebarExpanded ? 'flex-start' : 'center',
            color: '#6070a0',
            border: '1px solid transparent',
          }}
          onMouseEnter={e => (e.currentTarget.style.color = '#a0b4ff')}
          onMouseLeave={e => (e.currentTarget.style.color = '#6070a0')}>
          {sidebarExpanded
            ? <><ChevronLeft size={12} /><span className="font-share-tech text-sm ml-2 tracking-wider">{text(lang as LangCode, { tr: 'DARALT', en: 'COLLAPSE', de: 'Einklappen', fr: 'Réduire', ar: 'طيّ' })}</span></>
            : <ChevronRight size={12} />}
        </button>
      </div>
    </div>
  );
}
