import { text, translateEventDescription, type LangCode } from '../../utils/i18n';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Scale, ShieldCheck } from 'lucide-react';

const NORM_STAGES = [
  {
    stage: 1,
    en: 'Spontaneous Norms',
    tr: 'Kendiliğinden Normlar',
    de: 'Spontane Normen',
    fr: 'Normes spontanées',
    ar: 'معايير تلقائية',
    norms: [
      { id: 'reciprocity',        en: 'Reciprocity',   tr: 'Karşılıklılık',    de: 'Gegenseitigkeit', fr: 'Réciprocité',      ar: 'المعاملة بالمثل' },
      { id: 'no_theft',           en: 'No Theft',      tr: 'Hırsızlık Yasağı', de: 'Diebstahlverbot', fr: 'Interdiction du vol', ar: 'منع السرقة' },
      { id: 'incest_taboo',       en: 'Incest Taboo',  tr: 'Ensest Tabusu',    de: 'Inzesttabu',      fr: 'Tabou de l’inceste', ar: 'تحريم زواج الأقارب' },
    ],
  },
  {
    stage: 2,
    en: 'Social Norms',
    tr: 'Sosyal Normlar',
    de: 'Soziale Normen',
    fr: 'Normes sociales',
    ar: 'معايير اجتماعية',
    norms: [
      { id: 'elder_respect',      en: 'Elder Respect',  tr: 'Yaşlılara Saygı', de: 'Respekt vor Ältesten', fr: 'Respect des anciens', ar: 'احترام كبار السن' },
      { id: 'hospitality',        en: 'Hospitality',    tr: 'Misafirperverlik', de: 'Gastfreundschaft', fr: 'Hospitalité', ar: 'الضيافة' },
      { id: 'blood_feud',         en: 'Blood Feud',     tr: 'Kan Davası',       de: 'Blutrache', fr: 'Vendetta', ar: 'الثأر' },
      { id: 'communal_work',      en: 'Communal Work',  tr: 'Ortak Çalışma',    de: 'Gemeinschaftsarbeit', fr: 'Travail communautaire', ar: 'العمل الجماعي' },
    ],
  },
  {
    stage: 3,
    en: 'Proto-Law',
    tr: 'Proto-Hukuk',
    de: 'Proto-Recht',
    fr: 'Proto-droit',
    ar: 'قانون أولي',
    norms: [
      { id: 'leader_arbitration', en: 'Leader Arbitration', tr: 'Lider Tahkimi',  de: 'Schlichtung durch Anführer', fr: 'Arbitrage du chef', ar: 'تحكيم القائد' },
      { id: 'property_rights',    en: 'Property Rights',    tr: 'Mülkiyet Hakkı', de: 'Eigentumsrechte', fr: 'Droits de propriété', ar: 'حقوق الملكية' },
      { id: 'punishment_exile',   en: 'Exile Punishment',   tr: 'Sürgün Cezası',  de: 'Verbannungsstrafe', fr: 'Peine d’exil', ar: 'عقوبة النفي' },
    ],
  },
  {
    stage: 4,
    en: 'Formal Law',
    tr: 'Resmi Hukuk',
    de: 'Formales Recht',
    fr: 'Droit formel',
    ar: 'قانون رسمي',
    norms: [
      { id: 'written_law',        en: 'Written Law',   tr: 'Yazılı Hukuk',    de: 'Geschriebenes Recht', fr: 'Loi écrite', ar: 'قانون مكتوب' },
      { id: 'tax_system',         en: 'Tax System',     tr: 'Vergi Sistemi',   de: 'Steuersystem', fr: 'Système fiscal', ar: 'نظام الضرائب' },
      { id: 'contract_law',       en: 'Contract Law',   tr: 'Sözleşme Hukuku', de: 'Vertragsrecht', fr: 'Droit des contrats', ar: 'قانون العقود' },
    ],
  },
];

export default function LawPanel() {
  const { events, lang } = useSimStore();

  // Backend event_type values are `norm_emerged`/`norm_violation` (see
  // rust/sim-core/src/law.rs) -- there is no `"law"` event_type. Matching on
  // the real event_type/data.norm_id (the same pattern BeliefPanel already
  // uses for data.belief_id) instead of substring-matching English prose
  // fixes every count/progression indicator below, which previously always
  // read zero regardless of actual in-simulation legal activity.
  const lawEvents = events.filter(e => e.event_type === 'norm_emerged' || e.event_type === 'norm_violation');
  const normCount = lawEvents.filter(e => e.event_type === 'norm_emerged').length;
  const violationCount = lawEvents.filter(e => e.event_type === 'norm_violation').length;
  const emergedNormIds = new Set(lawEvents.filter(e => e.event_type === 'norm_emerged').map(e => e.data?.norm_id).filter(Boolean));

  return (
    <DetailPanel panelId="law" title="Law" titleTr="Hukuk">
      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <ShieldCheck size={16} className="text-green-400 mx-auto mb-1" />
          <div className="text-green-400 font-bold text-lg">{normCount}</div>
          <div className="text-sim-muted text-sm">{text(lang as LangCode, { en: 'Active Norms', tr: 'Aktif Normlar', de: 'Aktive Normen', fr: 'Normes actives', ar: 'معايير نشطة' })}</div>
        </div>
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <Scale size={16} className="text-yellow-400 mx-auto mb-1" />
          <div className="text-yellow-400 font-bold text-lg">{violationCount}</div>
          <div className="text-sim-muted text-sm">{text(lang as LangCode, { en: 'Violations', tr: 'İhlaller', de: 'Verstöße', fr: 'Violations', ar: 'انتهاكات' })}</div>
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Norm Progression', tr: 'Norm İlerlemesi', de: 'Normentwicklung', fr: 'Progression des normes', ar: 'تطور المعايير' })}
        </h4>
        <div className="space-y-3">
          {NORM_STAGES.map(stage => (
            <div key={stage.stage}>
              <div className="text-sm text-sim-muted mb-1 font-medium">
                {text(lang as LangCode, { en: `Stage ${stage.stage}: ${stage.en}`, tr: `Aşama ${stage.stage}: ${stage.tr}`, de: `Stufe ${stage.stage}: ${stage.de}`, fr: `Étape ${stage.stage}: ${stage.fr}`, ar: `المرحلة ${stage.stage}: ${stage.ar}` })}
              </div>
              <div className="space-y-0.5">
                {stage.norms.map(norm => {
                  const active = emergedNormIds.has(norm.id);
                  return (
                    <div
                      key={norm.en}
                      className={`flex items-center gap-1.5 text-sm px-2 py-0.5 rounded ${active ? 'text-sim-text' : 'text-sim-muted opacity-50'}`}
                    >
                      <div className={`w-1.5 h-1.5 rounded-full ${active ? 'bg-green-400' : 'bg-sim-border'}`} />
                      {text(lang as LangCode, { en: norm.en, tr: norm.tr, de: norm.de, fr: norm.fr, ar: norm.ar })}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Legal Events', tr: 'Hukuki Olaylar', de: 'Rechtsereignisse', fr: 'Événements juridiques', ar: 'أحداث قانونية' })}
        </h4>
        {lawEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {text(lang as LangCode, { en: 'No legal events yet.', tr: 'Henüz hukuki olay yok.', de: 'Noch keine Rechtsereignisse.', fr: 'Aucun événement juridique pour le moment.', ar: 'لا توجد أحداث قانونية بعد.' })}
          </p>
        ) : (
          <div className="space-y-1 max-h-40 overflow-y-auto">
            {lawEvents.slice(0, 10).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-green-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
