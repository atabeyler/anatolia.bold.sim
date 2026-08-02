import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

const MENTAL_STATES: Record<string, { emoji: string; color: string; tr: string; de: string; fr: string; ar: string }> = {
  content:   { emoji: '😌', color: 'text-green-400',  tr: 'Memnun',    de: 'Zufrieden',  fr: 'Content',     ar: 'مرتاح' },
  excited:   { emoji: '😃', color: 'text-yellow-400', tr: 'Heyecanlı', de: 'Aufgeregt',  fr: 'Excité',      ar: 'متحمس' },
  anxious:   { emoji: '😰', color: 'text-orange-400', tr: 'Kaygılı',   de: 'Ängstlich',  fr: 'Anxieux',     ar: 'قلق' },
  depressed: { emoji: '😞', color: 'text-blue-400',   tr: 'Depresif',  de: 'Deprimiert', fr: 'Déprimé',     ar: 'مكتئب' },
  calm:      { emoji: '🧘', color: 'text-teal-400',   tr: 'Sakin',     de: 'Ruhig',      fr: 'Calme',       ar: 'هادئ' },
  grieving:  { emoji: '😢', color: 'text-purple-400', tr: 'Yasında',   de: 'Traurig',    fr: 'En deuil',    ar: 'حزين' },
};

const TOM_LABELS: Record<number, { en: string; tr: string; de: string; fr: string; ar: string; color: string }> = {
  0: { en: 'None',     tr: 'Yok',        de: 'Keine',    fr: 'Aucune',    ar: 'لا يوجد',  color: 'text-sim-muted' },
  1: { en: 'Basic',    tr: 'Temel',      de: 'Grundlegend', fr: 'De base', ar: 'أساسي',   color: 'text-yellow-400' },
  2: { en: 'Advanced', tr: 'Gelişmiş',   de: 'Fortgeschritten', fr: 'Avancé', ar: 'متقدم', color: 'text-blue-400' },
  3: { en: 'Complex',  tr: 'Kompleks',   de: 'Komplex',  fr: 'Complexe',  ar: 'معقد',     color: 'text-purple-400' },
};

function Bar({ value, color, max = 1 }: { value: number; color: string; max?: number }) {
  return (
    <div className="h-2 bg-sim-border rounded-full overflow-hidden">
      <div className={`h-full rounded-full transition-all duration-500 ${color}`} style={{ width: `${Math.min(100, (value / max) * 100)}%` }} />
    </div>
  );
}

export default function PsychologyPanel() {
  const { stats, lang } = useSimStore();
  const s = stats as any;

  const happiness   = s?.happiness_index    ?? 0.5;
  const intelligence = s?.avg_intelligence  ?? 0;
  const consciousness = s?.avg_consciousness ?? 0;
  const tomStage    = s?.max_tom_stage      ?? 0;
  // Real population-wide stress (psychology::compute_population_psych_stats,
  // gini-penalized), not an inverse of happiness -- these are related but
  // distinct quantities in the simulation (see AGENTS.md's Psychology
  // section). Falls back to the old approximation only for a stats snapshot
  // from before this field existed.
  const meanStress  = s?.mean_stress ?? (1 - happiness);
  const stateDistribution: Record<string, number> = s?.mental_state_distribution ?? {};
  const totalWithState = Object.values(stateDistribution).reduce((a, b) => a + b, 0);

  // Population-average dynamic hormone levels (hormones::compute_population_hormone_stats,
  // see AGENTS.md's Hormones section) -- distinct from the static genetic
  // traits above (oxytocin_sensitivity, stress_reactivity, ...): these rise
  // and fall tick by tick with real per-individual state.
  const meanHormones: Record<string, number> = s?.mean_hormones ?? {};
  type HormoneDef = { key: string; color: string; label: { tr: string; en: string; de: string; fr: string; ar: string } };
  type HormoneGroup = { title: { tr: string; en: string; de: string; fr: string; ar: string }; items: HormoneDef[] };
  // Grouped by real endocrine axis (matches hormones.rs's own organization,
  // see AGENTS.md's Hormones section) rather than a flat alphabetical list --
  // e.g. ACTH/Cortisol/Norepinephrine/Adrenaline are the HPA (stress) axis,
  // LH/Testosterone/Estrogen/DHEA/Progesterone the HPG (reproductive) axis.
  const HORMONE_GROUPS: HormoneGroup[] = [
    {
      title: { tr: 'Stres Ekseni (HPA)', en: 'Stress Axis (HPA)', de: 'Stressachse (HPA)', fr: 'Axe du stress (HPA)', ar: 'محور الإجهاد (HPA)' },
      items: [
        { key: 'acth',           color: 'bg-red-700',    label: { tr: 'ACTH',        en: 'ACTH',        de: 'ACTH',        fr: 'ACTH',         ar: 'ACTH' } },
        { key: 'cortisol',       color: 'bg-red-500',    label: { tr: 'Kortizol',    en: 'Cortisol',    de: 'Cortisol',    fr: 'Cortisol',     ar: 'الكورتيزول' } },
        { key: 'norepinephrine', color: 'bg-red-400',    label: { tr: 'Norepinefrin', en: 'Norepinephrine', de: 'Noradrenalin', fr: 'Noradrénaline', ar: 'النورإبينفرين' } },
        { key: 'adrenaline',     color: 'bg-orange-500', label: { tr: 'Adrenalin',   en: 'Adrenaline',  de: 'Adrenalin',   fr: 'Adrénaline',   ar: 'الأدرينالين' } },
      ],
    },
    {
      title: { tr: 'Metabolik Eksen', en: 'Metabolic Axis', de: 'Metabolische Achse', fr: 'Axe métabolique', ar: 'المحور الأيضي' },
      items: [
        { key: 'tsh',     color: 'bg-cyan-700', label: { tr: 'TSH',     en: 'TSH',     de: 'TSH',     fr: 'TSH',      ar: 'TSH' } },
        { key: 'thyroid', color: 'bg-cyan-500', label: { tr: 'Tiroid',  en: 'Thyroid', de: 'Schilddrüse', fr: 'Thyroïde', ar: 'الغدة الدرقية' } },
        { key: 'insulin', color: 'bg-lime-600', label: { tr: 'İnsülin', en: 'Insulin', de: 'Insulin', fr: 'Insuline', ar: 'الأنسولين' } },
        { key: 'glucagon', color: 'bg-amber-600', label: { tr: 'Glukagon', en: 'Glucagon', de: 'Glukagon', fr: 'Glucagon', ar: 'الغلوكاغون' } },
        { key: 'leptin',  color: 'bg-lime-400', label: { tr: 'Leptin',  en: 'Leptin',  de: 'Leptin',  fr: 'Leptine',  ar: 'اللبتين' } },
        { key: 'ghrelin', color: 'bg-amber-400', label: { tr: 'Grelin',  en: 'Ghrelin', de: 'Ghrelin', fr: 'Ghréline', ar: 'الغريلين' } },
      ],
    },
    {
      title: { tr: 'Üreme Ekseni (HPG)', en: 'Reproductive Axis (HPG)', de: 'Reproduktionsachse (HPG)', fr: 'Axe reproducteur (HPG)', ar: 'المحور التناسلي (HPG)' },
      items: [
        { key: 'lh',           color: 'bg-blue-700',  label: { tr: 'LH',          en: 'LH',           de: 'LH',           fr: 'LH',            ar: 'LH' } },
        { key: 'testosterone', color: 'bg-blue-500',  label: { tr: 'Testosteron', en: 'Testosterone', de: 'Testosteron',  fr: 'Testostérone',  ar: 'التستوستيرون' } },
        { key: 'estrogen',     color: 'bg-pink-500',  label: { tr: 'Östrojen',    en: 'Estrogen',     de: 'Östrogen',     fr: 'Œstrogène',     ar: 'الإستروجين' } },
        { key: 'progesterone', color: 'bg-pink-700',  label: { tr: 'Progesteron', en: 'Progesterone', de: 'Progesteron',  fr: 'Progestérone',  ar: 'البروجستيرون' } },
        { key: 'dhea',         color: 'bg-indigo-500', label: { tr: 'DHEA',       en: 'DHEA',         de: 'DHEA',         fr: 'DHEA',          ar: 'DHEA' } },
        { key: 'growth_hormone', color: 'bg-teal-500', label: { tr: 'Büyüme Hormonu', en: 'Growth Hormone', de: 'Wachstumshormon', fr: 'Hormone de croissance', ar: 'هرمون النمو' } },
      ],
    },
    {
      title: { tr: 'Bağlanma / Ödül', en: 'Bonding / Reward', de: 'Bindung / Belohnung', fr: 'Attachement / Récompense', ar: 'الترابط / المكافأة' },
      items: [
        { key: 'dopamine',     color: 'bg-yellow-500', label: { tr: 'Dopamin',     en: 'Dopamine',     de: 'Dopamin',     fr: 'Dopamine',     ar: 'الدوبامين' } },
        { key: 'oxytocin',     color: 'bg-green-500',  label: { tr: 'Oksitosin',   en: 'Oxytocin',     de: 'Oxytocin',    fr: 'Ocytocine',    ar: 'الأوكسيتوسين' } },
        { key: 'vasopressin',  color: 'bg-green-700',  label: { tr: 'Vazopressin', en: 'Vasopressin',  de: 'Vasopressin', fr: 'Vasopressine', ar: 'الفازوبريسين' } },
        { key: 'prolactin',    color: 'bg-purple-500', label: { tr: 'Prolaktin',   en: 'Prolactin',    de: 'Prolaktin',   fr: 'Prolactine',   ar: 'البرولاكتين' } },
      ],
    },
  ];

  const tom = TOM_LABELS[tomStage] ?? TOM_LABELS[0];

  return (
    <DetailPanel panelId="psychology" title="Psychology" titleTr="Psikoloji">

      {/* ── Mutluluk özeti ── */}
      <div className="bg-sim-surface rounded-lg p-3 mb-3 text-center">
        <div className="text-3xl mb-1">
          {happiness > 0.7 ? '😄' : happiness > 0.4 ? '😐' : '😟'}
        </div>
        <div className={`font-bold text-xl ${happiness > 0.6 ? 'text-green-400' : happiness > 0.3 ? 'text-yellow-400' : 'text-red-400'}`}>
          {(happiness * 100).toFixed(0)}%
        </div>
        <div className="text-sim-muted text-sm">{text(lang as LangCode, { tr: 'Nüfus Mutluluğu', en: 'Population Happiness', de: 'Bevölkerungsglück', fr: 'Bonheur de la population', ar: 'سعادة السكان' })}</div>
      </div>

      {/* ── Bilişsel Göstergeler ── */}
      <div className="mb-3">
        <h4 className="text-sim-gold text-xs font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Bilişsel Göstergeler', en: 'Cognitive Metrics', de: 'Kognitive Metriken', fr: 'Métriques cognitives', ar: 'المقاييس المعرفية' })}
        </h4>
        <div className="space-y-2">

          {/* Zeka */}
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: 'Ort. Zekâ', en: 'Avg Intelligence', de: 'Durchschn. Intelligenz', fr: 'Intelligence moy.', ar: 'متوسط الذكاء' })}</span>
              <span className="text-blue-400 text-xs font-mono">{(intelligence * 100).toFixed(1)}%</span>
            </div>
            <Bar value={intelligence} color="bg-blue-500" />
          </div>

          {/* Bilinç */}
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: 'Ort. Bilinç', en: 'Avg Consciousness', de: 'Durchschn. Bewusstsein', fr: 'Conscience moy.', ar: 'متوسط الوعي' })}</span>
              <span className="text-purple-400 text-xs font-mono">{(consciousness * 100).toFixed(1)}%</span>
            </div>
            <Bar value={consciousness} color="bg-purple-500" />
          </div>

          {/* Wellbeing / Stres */}
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: 'Ort. İyi Oluş', en: 'Mean Wellbeing', de: 'Durchschn. Wohlbefinden', fr: 'Bien-être moy.', ar: 'متوسط الرفاهية' })}</span>
              <span className="text-green-400 text-xs font-mono">{(happiness * 100).toFixed(0)}%</span>
            </div>
            <Bar value={happiness} color="bg-green-500" />
          </div>
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: 'Ort. Stres', en: 'Mean Stress', de: 'Durchschn. Stress', fr: 'Stress moy.', ar: 'متوسط التوتر' })}</span>
              <span className="text-red-400 text-xs font-mono">{(meanStress * 100).toFixed(0)}%</span>
            </div>
            <Bar value={meanStress} color="bg-red-500" />
          </div>

          {/* Zihin Teorisi (Theory of Mind) */}
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: 'Zihin Teorisi (maks.)', en: 'Theory of Mind (max)', de: 'Theory of Mind (max)', fr: 'Théorie de l\'esprit (max)', ar: 'نظرية العقل (الحد الأقصى)' })}</span>
              <span className={`text-xs font-mono font-semibold ${tom.color}`}>
                {text(lang as LangCode, tom)} ({tomStage}/3)
              </span>
            </div>
            <div className="flex gap-1">
              {[0, 1, 2, 3].map(i => (
                <div
                  key={i}
                  className={`flex-1 h-2 rounded-full transition-all duration-500 ${
                    i <= tomStage
                      ? i === 0 ? 'bg-sim-muted/50' : i === 1 ? 'bg-yellow-500' : i === 2 ? 'bg-blue-500' : 'bg-purple-500'
                      : 'bg-sim-border'
                  }`}
                />
              ))}
            </div>
            <div className="text-sim-muted text-xs mt-1 italic">
              {tomStage === 0 && text(lang as LangCode, { tr: 'Başkalarının zihinsel durumuna dair farkındalık yok', en: 'No awareness of others\' mental states', de: 'Kein Bewusstsein für mentale Zustände anderer', fr: 'Pas de conscience des états mentaux d\'autrui', ar: 'لا وعي بالحالات الذهنية للآخرين' })}
              {tomStage === 1 && text(lang as LangCode, { tr: 'Başkalarının inançlarını anlayabilir (1. derece)', en: 'Can understand others\' beliefs (1st order)', de: 'Kann Überzeugungen anderer verstehen (1. Ordnung)', fr: 'Peut comprendre les croyances d\'autrui (1er ordre)', ar: 'يمكن فهم معتقدات الآخرين (الدرجة الأولى)' })}
              {tomStage === 2 && text(lang as LangCode, { tr: 'Başkalarının birbirini nasıl algıladığını anlar (2. derece)', en: 'Understands beliefs about beliefs (2nd order)', de: 'Versteht Überzeugungen über Überzeugungen (2. Ordnung)', fr: 'Comprend les croyances sur les croyances (2e ordre)', ar: 'يفهم المعتقدات حول المعتقدات (الدرجة الثانية)' })}
              {tomStage === 3 && text(lang as LangCode, { tr: 'Karmaşık sosyal akıl yürütme kapasitesi (3. derece)', en: 'Complex social reasoning capacity (3rd order)', de: 'Komplexe soziale Denkfähigkeit (3. Ordnung)', fr: 'Capacité de raisonnement social complexe (3e ordre)', ar: 'قدرة التفكير الاجتماعي المعقد (الدرجة الثالثة)' })}
            </div>
          </div>
        </div>
      </div>

      {/* ── Zihinsel Durum Dağılımı ── */}
      <div className="mb-3">
        <h4 className="text-sim-gold text-xs font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Zihinsel Durum Dağılımı', en: 'Mental State Distribution', de: 'Mentale Zustandsverteilung', fr: 'Distribution des états mentaux', ar: 'توزيع الحالات الذهنية' })}
        </h4>
        <div className="grid grid-cols-2 gap-1">
          {Object.entries(MENTAL_STATES).map(([state, info]) => {
            const count = stateDistribution[state] ?? 0;
            const pct = totalWithState > 0 ? (count / totalWithState) * 100 : 0;
            return (
              <div key={state} className="flex items-center justify-between gap-1 bg-sim-surface rounded px-2 py-1">
                <span className="flex items-center gap-1">
                  <span>{info.emoji}</span>
                  <span className={`text-xs capitalize ${info.color}`}>{text(lang as LangCode, { tr: info.tr, en: state, de: info.de, fr: info.fr, ar: info.ar })}</span>
                </span>
                <span className="text-sim-muted text-xs font-mono">{totalWithState > 0 ? `${pct.toFixed(0)}%` : '—'}</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── Hormonal Sistem ── */}
      <div className="mb-3">
        <h4 className="text-sim-gold text-xs font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Hormonal Sistem (Nüfus Ort.)', en: 'Hormonal System (Pop. Avg.)', de: 'Hormonsystem (Bev.-Durchschn.)', fr: 'Système hormonal (moy. pop.)', ar: 'الجهاز الهرموني (متوسط السكان)' })}
        </h4>
        <div className="space-y-3">
          {HORMONE_GROUPS.map(group => (
            <div key={group.title.en}>
              <div className="text-sim-muted text-xs uppercase tracking-wide mb-1 opacity-70">{text(lang as LangCode, group.title)}</div>
              <div className="space-y-1.5">
                {group.items.map(({ key, color, label }) => {
                  const value = meanHormones[key] ?? 0;
                  return (
                    <div key={key}>
                      <div className="flex justify-between mb-0.5">
                        <span className="text-sim-muted text-xs">{text(lang as LangCode, label)}</span>
                        <span className="text-sim-text text-xs font-mono">{(value * 100).toFixed(0)}%</span>
                      </div>
                      <Bar value={value} color={color} />
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* ── Etkenler ── */}
      <div>
        <h4 className="text-sim-gold text-xs font-semibold uppercase tracking-widest mb-1">
          {text(lang as LangCode, { tr: 'Mutluluk Etkenleri (Referans)', en: 'Happiness Drivers (Reference)', de: 'Glücksfaktoren (Referenz)', fr: 'Facteurs de bonheur (référence)', ar: 'محركات السعادة (مرجع)' })}
        </h4>
        <div className="text-sim-muted italic mb-2" style={{ fontSize: 11 }}>
          {text(lang as LangCode, {
            tr: 'Bu simülasyonun etkilerinin genel yönü — belirli bir bireysel ölçüm değildir.',
            en: 'General direction of this simulation\'s effects — not a live per-individual measurement.',
            de: 'Allgemeine Richtung der Effekte dieser Simulation — keine Live-Messung pro Individuum.',
            fr: "Direction générale des effets de cette simulation — pas une mesure individuelle en direct.",
            ar: 'الاتجاه العام لتأثيرات هذه المحاكاة — ليس قياسًا فرديًا حيًا.',
          })}
        </div>
        <div className="space-y-1">
          {[
            { factor: 'Food Security',  tr: 'Gıda Güvenliği', de: 'Nahrungssicherheit', fr: 'Sécurité alimentaire', ar: 'الأمن الغذائي', impact: '+' },
            { factor: 'Social Bonding', tr: 'Sosyal Bağlar',  de: 'Soziale Bindung',    fr: 'Lien social',          ar: 'الترابط الاجتماعي', impact: '+' },
            { factor: 'Disaster',       tr: 'Afet',           de: 'Katastrophe',        fr: 'Catastrophe',          ar: 'كارثة', impact: '−' },
            { factor: 'Oxytocin Gene',  tr: 'Oksitosin Geni', de: 'Oxytocin-Gen',       fr: "Gène de l'ocytocine",  ar: 'جين الأوكسيتوسين', impact: '+' },
            { factor: 'Exile',          tr: 'Sürgün',         de: 'Verbannung',         fr: 'Exil',                 ar: 'النفي', impact: '−−' },
            { factor: 'Art & Music',    tr: 'Sanat & Müzik',  de: 'Kunst & Musik',      fr: 'Art & Musique',        ar: 'الفن والموسيقى', impact: '+' },
          ].map(f => (
            <div key={f.factor} className="flex justify-between py-0.5 border-b border-sim-border/30">
              <span className="text-sim-muted text-xs">{text(lang as LangCode, { tr: f.tr, en: f.factor, de: f.de, fr: f.fr, ar: f.ar })}</span>
              <span className={`text-xs font-bold ${f.impact.startsWith('+') ? 'text-green-400' : 'text-red-400'}`}>{f.impact}</span>
            </div>
          ))}
        </div>
      </div>

    </DetailPanel>
  );
}
