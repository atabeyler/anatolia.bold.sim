import { translateEventDescription, text, type LangCode } from '../../utils/i18n';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { AlertTriangle } from 'lucide-react';

type LMap = { tr: string; en: string; de: string; fr: string; ar: string };

const SEVERITY: Record<string, LMap> = {
  Low:      { tr: 'Düşük',  en: 'Low',      de: 'Niedrig',   fr: 'Faible',    ar: 'منخفض'  },
  Medium:   { tr: 'Orta',   en: 'Medium',   de: 'Mittel',    fr: 'Moyen',     ar: 'متوسط'  },
  High:     { tr: 'Yüksek', en: 'High',     de: 'Hoch',      fr: 'Élevé',     ar: 'مرتفع'  },
  Critical: { tr: 'Kritik', en: 'Critical', de: 'Kritisch',  fr: 'Critique',  ar: 'حرج'    },
};

const PATHOGEN_INFO: Record<string, { name: LMap; transmission: LMap; severity: string; color: string }> = {
  intestinal_parasite: {
    name:         { tr: 'Bağırsak Paraziti',  en: 'Intestinal Parasite', de: 'Darmparasit',        fr: 'Parasite intestinal',   ar: 'طفيلي معوي'        },
    transmission: { tr: 'Dışkı-ağız',         en: 'Fecal-oral',          de: 'Fäkal-oral',         fr: 'Fécal-oral',            ar: 'برازي-فموي'         },
    severity: 'Low', color: 'text-yellow-400',
  },
  cholera_like: {
    name:         { tr: 'Kolera Benzeri',      en: 'Cholera Like',        de: 'Cholera-ähnlich',    fr: 'Type choléra',          ar: 'شبيه بالكوليرا'     },
    transmission: { tr: 'Suyla bulaşan',       en: 'Waterborne',          de: 'Wasserübertragen',   fr: 'Hydrique',              ar: 'منقول بالماء'       },
    severity: 'High', color: 'text-red-400',
  },
  respiratory_common: {
    name:         { tr: 'Solunum Yolu',        en: 'Respiratory Common',  de: 'Atemwegskrankheit',  fr: 'Respiratoire commun',   ar: 'التهاب تنفسي شائع'  },
    transmission: { tr: 'Hava yoluyla',        en: 'Airborne',            de: 'Luftübertragen',     fr: 'Aérien',                ar: 'محمول جواً'         },
    severity: 'Low', color: 'text-yellow-400',
  },
  pneumonia_like: {
    name:         { tr: 'Zatürre Benzeri',     en: 'Pneumonia Like',      de: 'Pneumonie-ähnlich',  fr: 'Type pneumonie',        ar: 'شبيه بالالتهاب الرئوي' },
    transmission: { tr: 'Hava yoluyla',        en: 'Airborne',            de: 'Luftübertragen',     fr: 'Aérien',                ar: 'محمول جواً'         },
    severity: 'Medium', color: 'text-orange-400',
  },
  plague_like: {
    name:         { tr: 'Veba Benzeri',        en: 'Plague Like',         de: 'Pest-ähnlich',       fr: 'Type peste',            ar: 'شبيه بالطاعون'      },
    transmission: { tr: 'Hava yoluyla',        en: 'Airborne',            de: 'Luftübertragen',     fr: 'Aérien',                ar: 'محمول جواً'         },
    severity: 'Critical', color: 'text-red-600',
  },
  malaria_like: {
    name:         { tr: 'Sıtma Benzeri',       en: 'Malaria Like',        de: 'Malaria-ähnlich',    fr: 'Type paludisme',        ar: 'شبيه بالملاريا'     },
    transmission: { tr: 'Vektör ile',          en: 'Vector',              de: 'Vektorübertragen',   fr: 'Par vecteur',           ar: 'عبر ناقل'           },
    severity: 'Medium', color: 'text-orange-400',
  },
  wound_infection: {
    name:         { tr: 'Yara Enfeksiyonu',    en: 'Wound Infection',     de: 'Wundinfektion',      fr: 'Infection de plaie',    ar: 'عدوى الجرح'         },
    transmission: { tr: 'Temas ile',           en: 'Contact',             de: 'Kontakt',            fr: 'Contact',               ar: 'عن طريق اللمس'      },
    severity: 'Medium', color: 'text-orange-400',
  },
};

export default function MicrobiomePanel() {
  const { stats, events, lang } = useSimStore();

  const sickRate = (stats as any)?.sick_rate ?? 0;
  const epidemicEvents = events.filter(e => e.event_type === 'epidemic');

  return (
    <DetailPanel panelId="microbiome" title="Microbiome" titleTr="Mikrobiyom">
      <div className={`bg-sim-surface rounded-lg p-3 mb-3 flex items-center gap-3 ${sickRate > 0.3 ? 'border border-red-500/40' : ''}`}>
        <AlertTriangle size={20} className={sickRate > 0.3 ? 'text-red-400' : 'text-sim-muted'} />
        <div>
          <div className={`font-bold text-lg ${sickRate > 0.3 ? 'text-red-400' : sickRate > 0.1 ? 'text-yellow-400' : 'text-green-400'}`}>
            {(sickRate * 100).toFixed(1)}%
          </div>
          <div className="text-sim-muted text-sm">
            {text(lang as LangCode, { tr: 'Nüfus Hastalık Oranı', en: 'Population Sick Rate', de: 'Krankheitsrate', fr: 'Taux de maladie', ar: 'معدل المرض في السكان' })}
          </div>
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Patojen Türleri', en: 'Pathogen Types', de: 'Erregerarten', fr: 'Types de pathogènes', ar: 'أنواع مسببات الأمراض' })}
        </h4>
        <div className="space-y-1.5">
          {Object.entries(PATHOGEN_INFO).map(([id, info]) => (
            <div key={id} className="flex items-center gap-2 bg-sim-surface/50 rounded p-1.5">
              <div className="flex-1">
                <div className="text-sm text-sim-text">{text(lang as LangCode, info.name)}</div>
                <div className="text-sm text-sim-muted">{text(lang as LangCode, info.transmission)}</div>
              </div>
              <span className={`text-sm font-medium ${info.color}`}>
                {text(lang as LangCode, SEVERITY[info.severity] ?? { tr: info.severity, en: info.severity, de: info.severity, fr: info.severity, ar: info.severity })}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Bağırsak Mikrobiyomu', en: 'Gut Microbiome', de: 'Darmmikrobiom', fr: 'Microbiome intestinal', ar: 'الميكروبيوم المعوي' })}
        </h4>
        <p className="text-sim-muted text-sm italic">
          {text(lang as LangCode, {
            tr: 'Diyet çeşitliliği → mikrobiyom çeşitliliği → bağışıklık gücü. Tek tip diyetler patojen duyarlılığını artırır.',
            en: 'Diet diversity → microbiome diversity → immune strength. Monoculture diets increase pathogen susceptibility.',
            de: 'Ernährungsvielfalt → Mikrobiomvielfalt → Immunstärke. Monokulturen erhöhen die Anfälligkeit.',
            fr: 'Diversité alimentaire → diversité du microbiome → immunité. Les régimes monocultures augmentent la susceptibilité.',
            ar: 'تنوع النظام الغذائي → تنوع الميكروبيوم → قوة المناعة. الأنظمة الغذائية الأحادية تزيد من قابلية الإصابة بالأمراض.',
          })}
        </p>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Salgın Tarihi', en: 'Epidemic History', de: 'Epidemiegeschichte', fr: 'Historique épidémique', ar: 'تاريخ الأوبئة' })}
        </h4>
        {epidemicEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {text(lang as LangCode, { tr: 'Kayıtlı salgın yok.', en: 'No epidemics recorded.', de: 'Keine Epidemien aufgezeichnet.', fr: 'Aucune épidémie enregistrée.', ar: 'لا توجد أوبئة مسجلة.' })}
          </p>
        ) : (
          <div className="space-y-1 max-h-40 overflow-y-auto">
            {epidemicEvents.map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-red-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
