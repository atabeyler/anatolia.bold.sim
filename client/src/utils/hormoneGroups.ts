// Shared hormone grouping used by both PsychologyPanel (population averages,
// stats.mean_hormones) and PopulationPanel's IndividualDetail/CompareModal
// (a specific individual's own live values, ind.hormones) -- one source of
// truth for labels/colors/grouping so the two views can't drift apart.
// Grouped by real endocrine axis (matches hormones.rs's own organization,
// see AGENTS.md's Hormones section) rather than a flat alphabetical list.
export type HormoneLangLabel = { tr: string; en: string; de: string; fr: string; ar: string };
export type HormoneDef = { key: string; color: string; label: HormoneLangLabel };
export type HormoneGroup = { title: HormoneLangLabel; items: HormoneDef[] };

export const HORMONE_GROUPS: HormoneGroup[] = [
  {
    title: { tr: 'Stres Ekseni (HPA)', en: 'Stress Axis (HPA)', de: 'Stressachse (HPA)', fr: 'Axe du stress (HPA)', ar: 'محور الإجهاد (HPA)' },
    items: [
      { key: 'crh',            color: 'bg-red-800',    label: { tr: 'CRH',         en: 'CRH',         de: 'CRH',          fr: 'CRH',           ar: 'CRH' } },
      { key: 'acth',           color: 'bg-red-700',    label: { tr: 'ACTH',        en: 'ACTH',        de: 'ACTH',        fr: 'ACTH',         ar: 'ACTH' } },
      { key: 'cortisol',       color: 'bg-red-500',    label: { tr: 'Kortizol',    en: 'Cortisol',    de: 'Cortisol',    fr: 'Cortisol',     ar: 'الكورتيزول' } },
      { key: 'norepinephrine', color: 'bg-red-400',    label: { tr: 'Norepinefrin', en: 'Norepinephrine', de: 'Noradrenalin', fr: 'Noradrénaline', ar: 'النورإبينفرين' } },
      { key: 'adrenaline',     color: 'bg-orange-500', label: { tr: 'Adrenalin',   en: 'Adrenaline',  de: 'Adrenalin',   fr: 'Adrénaline',   ar: 'الأدرينالين' } },
      { key: 'melatonin',      color: 'bg-slate-500',  label: { tr: 'Melatonin',   en: 'Melatonin',   de: 'Melatonin',   fr: 'Mélatonine',   ar: 'الميلاتونين' } },
    ],
  },
  {
    title: { tr: 'POMC / Bağışıklık', en: 'POMC / Immune', de: 'POMC / Immunsystem', fr: 'POMC / Immunitaire', ar: 'POMC / المناعة' },
    items: [
      { key: 'msh',        color: 'bg-fuchsia-600', label: { tr: 'MSH',         en: 'MSH',         de: 'MSH',         fr: 'MSH',           ar: 'MSH' } },
      { key: 'endorphin',  color: 'bg-fuchsia-400', label: { tr: 'Endorfin',    en: 'Endorphin',   de: 'Endorphin',   fr: 'Endorphine',    ar: 'الإندورفين' } },
      { key: 'il6',        color: 'bg-rose-600',    label: { tr: 'IL-6',        en: 'IL-6',        de: 'IL-6',        fr: 'IL-6',          ar: 'IL-6' } },
      { key: 'tnf_alpha',  color: 'bg-rose-500',    label: { tr: 'TNF-alfa',    en: 'TNF-alpha',   de: 'TNF-alpha',   fr: 'TNF-alpha',     ar: 'TNF-alpha' } },
      { key: 'interferon', color: 'bg-rose-400',    label: { tr: 'İnterferon',  en: 'Interferon',  de: 'Interferon',  fr: 'Interféron',    ar: 'الإنترفيرون' } },
    ],
  },
  {
    title: { tr: 'Metabolik Eksen', en: 'Metabolic Axis', de: 'Metabolische Achse', fr: 'Axe métabolique', ar: 'المحور الأيضي' },
    items: [
      { key: 'tsh',        color: 'bg-cyan-700',  label: { tr: 'TSH',       en: 'TSH',       de: 'TSH',       fr: 'TSH',        ar: 'TSH' } },
      { key: 'thyroid',    color: 'bg-cyan-500',  label: { tr: 'Tiroid',    en: 'Thyroid',   de: 'Schilddrüse', fr: 'Thyroïde',  ar: 'الغدة الدرقية' } },
      { key: 'insulin',    color: 'bg-lime-600',  label: { tr: 'İnsülin',   en: 'Insulin',   de: 'Insulin',   fr: 'Insuline',   ar: 'الأنسولين' } },
      { key: 'glucagon',   color: 'bg-amber-600', label: { tr: 'Glukagon',  en: 'Glucagon',  de: 'Glukagon',  fr: 'Glucagon',   ar: 'الغلوكاغون' } },
      { key: 'leptin',     color: 'bg-lime-400',  label: { tr: 'Leptin',    en: 'Leptin',    de: 'Leptin',    fr: 'Leptine',    ar: 'اللبتين' } },
      { key: 'ghrelin',    color: 'bg-amber-400', label: { tr: 'Grelin',    en: 'Ghrelin',   de: 'Ghrelin',   fr: 'Ghréline',   ar: 'الغريلين' } },
      { key: 'adiponectin', color: 'bg-lime-700', label: { tr: 'Adiponektin', en: 'Adiponectin', de: 'Adiponektin', fr: 'Adiponectine', ar: 'الأديبونكتين' } },
      { key: 'npy',        color: 'bg-amber-700', label: { tr: 'NPY',       en: 'NPY',       de: 'NPY',       fr: 'NPY',        ar: 'NPY' } },
    ],
  },
  {
    title: { tr: 'Üreme Ekseni (HPG)', en: 'Reproductive Axis (HPG)', de: 'Reproduktionsachse (HPG)', fr: 'Axe reproducteur (HPG)', ar: 'المحور التناسلي (HPG)' },
    items: [
      { key: 'lh',           color: 'bg-blue-700',  label: { tr: 'LH',          en: 'LH',           de: 'LH',           fr: 'LH',            ar: 'LH' } },
      { key: 'fsh',          color: 'bg-blue-600',  label: { tr: 'FSH',         en: 'FSH',          de: 'FSH',          fr: 'FSH',           ar: 'FSH' } },
      { key: 'testosterone', color: 'bg-blue-500',  label: { tr: 'Testosteron', en: 'Testosterone', de: 'Testosteron',  fr: 'Testostérone',  ar: 'التستوستيرون' } },
      { key: 'estrogen',     color: 'bg-pink-500',  label: { tr: 'Östrojen',    en: 'Estrogen',     de: 'Östrogen',     fr: 'Œstrogène',     ar: 'الإستروجين' } },
      { key: 'progesterone', color: 'bg-pink-700',  label: { tr: 'Progesteron', en: 'Progesterone', de: 'Progesteron',  fr: 'Progestérone',  ar: 'البروجستيرون' } },
      { key: 'dhea',         color: 'bg-indigo-500', label: { tr: 'DHEA',       en: 'DHEA',         de: 'DHEA',         fr: 'DHEA',          ar: 'DHEA' } },
      { key: 'growth_hormone', color: 'bg-teal-500', label: { tr: 'Büyüme Hormonu', en: 'Growth Hormone', de: 'Wachstumshormon', fr: 'Hormone de croissance', ar: 'هرمون النمو' } },
      { key: 'igf1',          color: 'bg-teal-700', label: { tr: 'IGF-1',      en: 'IGF-1',        de: 'IGF-1',        fr: 'IGF-1',         ar: 'IGF-1' } },
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
  {
    title: { tr: 'Sindirim', en: 'Digestive', de: 'Verdauung', fr: 'Digestif', ar: 'الجهاز الهضمي' },
    items: [
      { key: 'gastrin',                color: 'bg-orange-700', label: { tr: 'Gastrin',       en: 'Gastrin',       de: 'Gastrin',       fr: 'Gastrine',        ar: 'الغاسترين' } },
      { key: 'secretin',               color: 'bg-orange-600', label: { tr: 'Sekretin',      en: 'Secretin',      de: 'Sekretin',      fr: 'Sécrétine',       ar: 'السيكريتين' } },
      { key: 'cck',                    color: 'bg-orange-500', label: { tr: 'CCK',           en: 'CCK',           de: 'CCK',           fr: 'CCK',             ar: 'CCK' } },
      { key: 'motilin',                color: 'bg-orange-400', label: { tr: 'Motilin',       en: 'Motilin',       de: 'Motilin',       fr: 'Motiline',        ar: 'الموتيلين' } },
      { key: 'gip',                    color: 'bg-yellow-700', label: { tr: 'GIP',           en: 'GIP',           de: 'GIP',           fr: 'GIP',             ar: 'GIP' } },
      { key: 'somatostatin',           color: 'bg-yellow-600', label: { tr: 'Somatostatin',  en: 'Somatostatin',  de: 'Somatostatin',  fr: 'Somatostatine',   ar: 'السوماتوستاتين' } },
      { key: 'pyy',                    color: 'bg-yellow-500', label: { tr: 'PYY',           en: 'PYY',           de: 'PYY',           fr: 'PYY',             ar: 'PYY' } },
      { key: 'pancreatic_polypeptide', color: 'bg-yellow-400', label: { tr: 'Pankreatik Polipeptid', en: 'Pancreatic Polypeptide', de: 'Pankreatisches Polypeptid', fr: 'Polypeptide pancréatique', ar: 'الببتيد البنكرياسي' } },
    ],
  },
  {
    title: { tr: 'Kardiyovasküler / Böbrek', en: 'Cardiovascular / Renal', de: 'Herz-Kreislauf / Niere', fr: 'Cardiovasculaire / Rénal', ar: 'القلب والأوعية / الكلى' },
    items: [
      { key: 'renin',          color: 'bg-violet-700', label: { tr: 'Renin',           en: 'Renin',           de: 'Renin',           fr: 'Rénine',            ar: 'الرينين' } },
      { key: 'angiotensin_ii', color: 'bg-violet-600', label: { tr: 'Anjiyotensin II', en: 'Angiotensin II',  de: 'Angiotensin II',  fr: 'Angiotensine II',   ar: 'أنجيوتنسين II' } },
      { key: 'aldosterone',    color: 'bg-violet-500', label: { tr: 'Aldosteron',      en: 'Aldosterone',     de: 'Aldosteron',      fr: 'Aldostérone',       ar: 'الألدوستيرون' } },
      { key: 'anp',            color: 'bg-sky-600',    label: { tr: 'ANP',             en: 'ANP',             de: 'ANP',             fr: 'ANP',               ar: 'ANP' } },
      { key: 'bnp',            color: 'bg-sky-500',    label: { tr: 'BNP',             en: 'BNP',             de: 'BNP',             fr: 'BNP',               ar: 'BNP' } },
      { key: 'epo',            color: 'bg-red-600',    label: { tr: 'Eritropoietin',   en: 'Erythropoietin',  de: 'Erythropoietin',  fr: 'Érythropoïétine',   ar: 'الإريثروبويتين' } },
    ],
  },
  {
    title: { tr: 'Kemik / Kalsiyum', en: 'Bone / Calcium', de: 'Knochen / Kalzium', fr: 'Os / Calcium', ar: 'العظام / الكالسيوم' },
    items: [
      { key: 'pth',         color: 'bg-stone-600', label: { tr: 'PTH',        en: 'PTH',        de: 'PTH',        fr: 'PTH',           ar: 'PTH' } },
      { key: 'calcitonin',  color: 'bg-stone-500', label: { tr: 'Kalsitonin', en: 'Calcitonin', de: 'Calcitonin', fr: 'Calcitonine',   ar: 'الكالسيتونين' } },
      { key: 'vitamin_d',   color: 'bg-stone-400', label: { tr: 'D Vitamini', en: 'Vitamin D',  de: 'Vitamin D',  fr: 'Vitamine D',    ar: 'فيتامين د' } },
      { key: 'osteocalcin', color: 'bg-stone-700', label: { tr: 'Osteokalsin', en: 'Osteocalcin', de: 'Osteocalcin', fr: 'Ostéocalcine', ar: 'الأوستيوكالسين' } },
    ],
  },
];
