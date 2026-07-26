export type LangCode = 'tr' | 'en' | 'de' | 'fr' | 'ar';

export const LANG_CODES = ['tr', 'en', 'de', 'fr', 'ar'] as const;

export function isValidLangCode(code: unknown): code is LangCode {
  return LANG_CODES.includes(code as LangCode);
}

export type TranslationMap = Partial<Record<LangCode, string>> &
  ({ en: string } | { tr: string });

const UI_FALLBACK_LABELS: Record<string, TranslationMap> = {
  total: { tr: 'TOPLAM', en: 'TOTAL', de: 'GESAMT', fr: 'TOTAL', ar: 'المجموع' },
  male: { tr: 'ERKEK', en: 'MALE', de: 'MÄNNLICH', fr: 'HOMME', ar: 'ذكر' },
  female: { tr: 'KADIN', en: 'FEMALE', de: 'WEIBLICH', fr: 'FEMME', ar: 'أنثى' },
  all: { tr: 'TÜMÜ', en: 'ALL', de: 'ALLE', fr: 'TOUS', ar: 'الكل' },
  age: { tr: 'YAŞ', en: 'AGE', de: 'ALTER', fr: 'ÂGE', ar: 'العمر' },
  infant: { tr: 'Bebek', en: 'Infant', de: 'Säugling', fr: 'Nourrisson', ar: 'رضيع' },
  child: { tr: 'Çocuk', en: 'Child', de: 'Kind', fr: 'Enfant', ar: 'طفل' },
  youth: { tr: 'Genç', en: 'Youth', de: 'Jugend', fr: 'Jeune', ar: 'شاب' },
  adult: { tr: 'Yetişkin', en: 'Adult', de: 'Erwachsen', fr: 'Adulte', ar: 'بالغ' },
  elder: { tr: 'Yaşlı', en: 'Elder', de: 'Ältester', fr: 'Aîné', ar: 'مسن' },
  yr: { tr: 'yaş', en: 'yr', de: 'J.', fr: 'an', ar: 'سنة' },
  ' yr': { tr: ' yaş', en: ' yr', de: ' J.', fr: ' an', ar: ' سنة' },
  founder: { tr: 'KURUCU', en: 'FOUNDER', de: 'GRÜNDER', fr: 'FONDATEUR', ar: 'مؤسس' },
  loading_data: { tr: 'VERİ YÜKLENİYOR...', en: 'LOADING DATA...', de: 'DATEN WERDEN GELADEN...', fr: 'CHARGEMENT DES DONNÉES...', ar: 'جارٍ تحميل البيانات...' },
  compare: { tr: 'KARŞILAŞTIR', en: 'COMPARE', de: 'VERGLEICHEN', fr: 'COMPARER', ar: 'قارن' },
  deceased: { tr: 'HAYATINI KAYBETTİLER', en: 'DECEASED', de: 'VERSTORBEN', fr: 'DÉCÉDÉS', ar: 'المتوفون' },
  no_population: { tr: 'NÜFUS YOK', en: 'NO POPULATION', de: 'KEINE BEVÖLKERUNG', fr: 'AUCUNE POPULATION', ar: 'لا يوجد سكان' },
  more_individuals: { tr: 'birey daha', en: 'more individuals', de: 'weitere Individuen', fr: 'individus de plus', ar: 'أفراد آخرون' },
  youngest_first: { tr: 'En genç önce', en: 'Youngest first', de: 'Jüngste zuerst', fr: 'Plus jeunes d’abord', ar: 'الأصغر أولاً' },
  oldest_first: { tr: 'En yaşlı önce', en: 'Oldest first', de: 'Älteste zuerst', fr: 'Plus âgés d’abord', ar: 'الأكبر أولاً' },
  pregnant: { tr: 'Hamile', en: 'Pregnant', de: 'Schwanger', fr: 'Enceinte', ar: 'حامل' },
  population: { tr: 'Nüfus', en: 'Population', de: 'Bevölkerung', fr: 'Population', ar: 'السكان' },
  biology: { tr: 'Biyoloji', en: 'Biology', de: 'Biologie', fr: 'Biologie', ar: 'البيولوجيا' },
  environment: { tr: 'Çevre', en: 'Environment', de: 'Umwelt', fr: 'Environnement', ar: 'البيئة' },
  astronomy: { tr: 'Astronomi', en: 'Astronomy', de: 'Astronomie', fr: 'Astronomie', ar: 'علم الفلك' },
  culture: { tr: 'Kültür', en: 'Culture', de: 'Kultur', fr: 'Culture', ar: 'الثقافة' },
  language: { tr: 'Dil', en: 'Language', de: 'Sprache', fr: 'Langue', ar: 'اللغة' },
  technology: { tr: 'Teknoloji', en: 'Technology', de: 'Technologie', fr: 'Technologie', ar: 'التكنولوجيا' },
  belief: { tr: 'İnanç', en: 'Belief', de: 'Glaube', fr: 'Croyance', ar: 'المعتقد' },
  social: { tr: 'Sosyal', en: 'Social', de: 'Sozial', fr: 'Social', ar: 'اجتماعي' },
  economy: { tr: 'Ekonomi', en: 'Economy', de: 'Wirtschaft', fr: 'Économie', ar: 'الاقتصاد' },
  art: { tr: 'Sanat', en: 'Art', de: 'Kunst', fr: 'Art', ar: 'الفن' },
  architecture: { tr: 'Mimari', en: 'Architecture', de: 'Architektur', fr: 'Architecture', ar: 'العمارة' },
  law: { tr: 'Hukuk', en: 'Law', de: 'Recht', fr: 'Droit', ar: 'القانون' },
  microbiome: { tr: 'Mikrobiyom', en: 'Microbiome', de: 'Mikrobiom', fr: 'Microbiome', ar: 'الميكروبيوم' },
  psychology: { tr: 'Psikoloji', en: 'Psychology', de: 'Psychologie', fr: 'Psychologie', ar: 'علم النفس' },
  epigenetics: { tr: 'Epigenetik', en: 'Epigenetics', de: 'Epigenetik', fr: 'Épigénétique', ar: 'علم التخلق' },
  genealogy: { tr: 'Soy Ağacı', en: 'Genealogy', de: 'Genealogie', fr: 'Généalogie', ar: 'النسب' },
  god_mode: { tr: 'Tanrı Modu', en: 'God Mode', de: 'Gottmodus', fr: 'Mode Dieu', ar: 'وضع الإله' },
  time_machine: { tr: 'Zaman Makinesi', en: 'Time Machine', de: 'Zeitmaschine', fr: 'Machine temporelle', ar: 'آلة الزمن' },
  report: { tr: 'Rapor', en: 'Report', de: 'Bericht', fr: 'Rapport', ar: 'تقرير' },
  terminate: { tr: 'Sonlandır', en: 'Terminate', de: 'Beenden', fr: 'Terminer', ar: 'إنهاء' },
  exit: { tr: 'Çıkış', en: 'Exit', de: 'Ausgang', fr: 'Sortie', ar: 'خروج' },
  speed: { tr: 'Hız', en: 'Speed', de: 'Geschwindigkeit', fr: 'Vitesse', ar: 'السرعة' },
  set: { tr: 'Ayarla', en: 'Set', de: 'Setzen', fr: 'Définir', ar: 'تعيين' },
  menu: { tr: 'Menü', en: 'Menu', de: 'Menü', fr: 'Menu', ar: 'القائمة' },
  user: { tr: 'Kullanıcı', en: 'User', de: 'Benutzer', fr: 'Utilisateur', ar: 'المستخدم' },
  pause: { tr: 'Duraklat', en: 'Pause', de: 'Pause', fr: 'Pause', ar: 'إيقاف مؤقت' },
  season: { tr: 'Mevsim', en: 'Season', de: 'Jahreszeit', fr: 'Saison', ar: 'الموسم' },
  year: { tr: 'Yıl', en: 'Year', de: 'Jahr', fr: 'Année', ar: 'السنة' },
  birth: { tr: 'Doğum', en: 'Birth', de: 'Geburt', fr: 'Naissance', ar: 'ولادة' },
  death: { tr: 'Ölüm', en: 'Death', de: 'Tod', fr: 'Mort', ar: 'وفاة' },
  tech: { tr: 'Tek.', en: 'Tech', de: 'Tech.', fr: 'Tech.', ar: 'تقنية' },
};

function normalizeUiKey(value?: string): string {
  return String(value ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .replace(/\.+$/g, '')
    .toLowerCase();
}

export function text(lang: LangCode, values: TranslationMap): string {
  if (values[lang]) return values[lang] as string;
  const fromEn = UI_FALLBACK_LABELS[normalizeUiKey(values.en)];
  if (fromEn?.[lang]) return fromEn[lang] as string;
  const fromTr = UI_FALLBACK_LABELS[normalizeUiKey(values.tr)];
  if (fromTr?.[lang]) return fromTr[lang] as string;
  return values.en ?? values.tr ?? '';
}

const SEASON_LABELS: Record<string, TranslationMap> = {
  spring: { tr: 'İlkbahar', en: 'Spring', de: 'Frühling', fr: 'Printemps', ar: 'الربيع' },
  summer: { tr: 'Yaz', en: 'Summer', de: 'Sommer', fr: 'Été', ar: 'الصيف' },
  autumn: { tr: 'Sonbahar', en: 'Autumn', de: 'Herbst', fr: 'Automne', ar: 'الخريف' },
  fall:   { tr: 'Sonbahar', en: 'Fall',   de: 'Herbst', fr: 'Automne', ar: 'الخريف' },
  winter: { tr: 'Kış',      en: 'Winter', de: 'Winter', fr: 'Hiver',   ar: 'الشتاء' },
};

export function translateSeason(season: string, lang: LangCode): string {
  const key = (season ?? '').trim().toLowerCase();
  if (!key) return '—';

  const exact = SEASON_LABELS[key];
  if (exact) return text(lang, exact);

  if (key.includes('spring')) return text(lang, SEASON_LABELS.spring);
  if (key.includes('summer')) return text(lang, SEASON_LABELS.summer);
  if (key.includes('autumn') || key.includes('fall')) return text(lang, SEASON_LABELS.autumn);
  if (key.includes('winter')) return text(lang, SEASON_LABELS.winter);

  return season;
}

// The 8 weather types (environment.rs / AGENTS.md). Raw values ("clear",
// "heavy_rain", ...) previously leaked untranslated into the PDF report --
// every other raw engine value (season, death cause) already has its own
// translation helper; this was the one missing.
const WEATHER_LABELS: Record<string, TranslationMap> = {
  clear:      { tr: 'açık',        en: 'clear',      de: 'klar',        fr: 'dégagé',        ar: 'صافٍ' },
  rain:       { tr: 'yağmurlu',    en: 'rain',       de: 'Regen',       fr: 'pluie',         ar: 'مطر' },
  heavy_rain: { tr: 'sağanak',     en: 'heavy rain', de: 'Starkregen',  fr: 'forte pluie',   ar: 'مطر غزير' },
  snow:       { tr: 'karlı',       en: 'snow',       de: 'Schnee',      fr: 'neige',         ar: 'ثلج' },
  blizzard:   { tr: 'tipi',        en: 'blizzard',   de: 'Schneesturm', fr: 'blizzard',      ar: 'عاصفة ثلجية' },
  storm:      { tr: 'fırtınalı',   en: 'storm',      de: 'Sturm',       fr: 'tempête',       ar: 'عاصفة' },
  heat_wave:  { tr: 'sıcak hava dalgası', en: 'heat wave', de: 'Hitzewelle', fr: 'canicule', ar: 'موجة حر' },
  drought:    { tr: 'kuraklık',    en: 'drought',    de: 'Dürre',       fr: 'sécheresse',    ar: 'جفاف' },
};

export function translateWeather(weather: string, lang: LangCode): string {
  const key = (weather ?? '').trim().toLowerCase();
  if (!key) return '—';
  return WEATHER_LABELS[key] ? text(lang, WEATHER_LABELS[key]) : weather;
}

export const CAUSE_LABELS: Record<string, TranslationMap> = {
  starvation:          { tr: 'açlık',                 en: 'starvation',          de: 'Verhungern',           fr: 'famine',              ar: 'جوع' },
  dehydration:         { tr: 'susuzluk',              en: 'dehydration',         de: 'Dehydrierung',         fr: 'déshydratation',      ar: 'جفاف' },
  old_age:             { tr: 'yaşlılık',              en: 'old age',             de: 'Alter',                fr: 'vieillesse',          ar: 'الشيخوخة' },
  predator:            { tr: 'yırtıcı hayvan',        en: 'predator',            de: 'Raubtier',             fr: 'prédateur',           ar: 'مفترس' },
  genetic_disease:     { tr: 'genetik hastalık',      en: 'genetic disease',     de: 'Erbkrankheit',         fr: 'maladie génétique',   ar: 'مرض وراثي' },
  infection:           { tr: 'enfeksiyon',             en: 'infection',           de: 'Infektion',            fr: 'infection',           ar: 'عدوى' },
  trauma:              { tr: 'travma',                 en: 'trauma',              de: 'Trauma',               fr: 'traumatisme',         ar: 'صدمة' },
  birth_complications: { tr: 'doğum komplikasyonu',   en: 'birth complications', de: 'Geburtskomplikationen',fr: 'complications à la naissance', ar: 'مضاعفات الولادة' },
  conflict:            { tr: 'çatışma',               en: 'conflict',            de: 'Konflikt',             fr: 'conflit',             ar: 'نزاع' },
  drowning:            { tr: 'boğulma',               en: 'drowning',            de: 'Ertrinken',            fr: 'noyade',              ar: 'غرق' },
  meteor_tsunami:      { tr: 'meteor çarpması ve tsunami', en: 'meteor impact and tsunami', de: 'Meteoriteneinschlag und Tsunami', fr: 'impact de météore et tsunami', ar: 'اصطدام نيزك وتسونامي' },
  flood:               { tr: 'sel felaketi',           en: 'flood',               de: 'Überschwemmung',       fr: 'inondation',          ar: 'فيضان' },
  earthquake:          { tr: 'deprem',                 en: 'earthquake',          de: 'Erdbeben',             fr: 'séisme',              ar: 'زلزال' },
  drought:             { tr: 'kuraklık',               en: 'drought',             de: 'Dürre',                fr: 'sécheresse',          ar: 'جفاف' },
  fire:                { tr: 'yangın',                 en: 'fire',                de: 'Feuer',                fr: 'incendie',            ar: 'حريق' },
  volcano:             { tr: 'yanardağ patlaması',     en: 'volcanic eruption',   de: 'Vulkanausbruch',       fr: 'éruption volcanique', ar: 'ثوران بركاني' },
  storm:               { tr: 'fırtına',               en: 'storm',               de: 'Sturm',                fr: 'tempête',             ar: 'عاصفة' },
  tsunami:             { tr: 'tsunami',               en: 'tsunami',             de: 'Tsunami',              fr: 'tsunami',             ar: 'تسونامي' },
  landslide:           { tr: 'heyelan',               en: 'landslide',           de: 'Erdrutsch',            fr: 'glissement de terrain', ar: 'انهيار أرضي' },
  unknown:             { tr: 'bilinmeyen neden',       en: 'unknown cause',       de: 'unbekannte Ursache',   fr: 'cause inconnue',      ar: 'سبب مجهول' },
};

// agent.rs's `ACTIONS` -- what population_history/current-snapshot's
// `dominant_drive` field actually carries. Leaked untranslated into the PDF
// report's "Move Reason" column (e.g. "seek_warmth", "craft").
const DRIVE_LABELS: Record<string, TranslationMap> = {
  forage:      { tr: 'yiyecek arama',   en: 'forage',      de: 'Nahrungssuche', fr: 'cueillette',        ar: 'البحث عن الطعام' },
  drink:       { tr: 'su içme',         en: 'drink',       de: 'Trinken',       fr: 'boire',             ar: 'الشرب' },
  flee:        { tr: 'kaçma',           en: 'flee',        de: 'Flucht',        fr: 'fuite',             ar: 'الهروب' },
  seek_warmth: { tr: 'sıcaklık arama',  en: 'seek warmth', de: 'Wärme suchen',  fr: 'recherche de chaleur', ar: 'البحث عن الدفء' },
  rest:        { tr: 'dinlenme',        en: 'rest',        de: 'Ruhe',          fr: 'repos',             ar: 'الراحة' },
  hunt:        { tr: 'avlanma',         en: 'hunt',        de: 'Jagd',          fr: 'chasse',            ar: 'الصيد' },
  craft:       { tr: 'el işi yapma',    en: 'craft',       de: 'Handwerk',      fr: 'artisanat',         ar: 'الحرفة' },
  socialize:   { tr: 'sosyalleşme',     en: 'socialize',   de: 'sozialisieren', fr: 'socialiser',        ar: 'التواصل الاجتماعي' },
  mate:        { tr: 'çiftleşme',       en: 'mate',        de: 'Paarung',       fr: 'accouplement',      ar: 'التزاوج' },
  explore:     { tr: 'keşfetme',        en: 'explore',     de: 'Erkunden',      fr: 'explorer',          ar: 'الاستكشاف' },
};

export function translateDrive(drive: string, lang: LangCode): string {
  const key = (drive ?? '').trim().toLowerCase();
  if (!key) return '—';
  return DRIVE_LABELS[key] ? text(lang, DRIVE_LABELS[key]) : drive;
}

// tick.rs's `track_migration` -- the only values a migration event's
// `reason` field ever carries. "disaster:<type>" reuses CAUSE_LABELS for the
// suffix rather than duplicating all 20 disaster names here.
const MIGRATION_REASON_LABELS: Record<string, TranslationMap> = {
  food_scarcity:  { tr: 'besin kıtlığı', en: 'food scarcity',  de: 'Nahrungsmangel', fr: 'pénurie de nourriture', ar: 'نقص الغذاء' },
  water_scarcity: { tr: 'su kıtlığı',    en: 'water scarcity', de: 'Wassermangel',   fr: 'pénurie d\'eau',        ar: 'نقص الماء' },
  exploration:    { tr: 'keşif',         en: 'exploration',    de: 'Erkundung',      fr: 'exploration',           ar: 'استكشاف' },
};
const DISASTER_PREFIX: TranslationMap = { tr: 'afet', en: 'disaster', de: 'Katastrophe', fr: 'catastrophe', ar: 'كارثة' };

export function translateMigrationReason(reason: string, lang: LangCode): string {
  const key = (reason ?? '').trim().toLowerCase();
  if (!key) return '—';
  if (key.startsWith('disaster:')) {
    const disasterType = key.slice('disaster:'.length);
    const label = CAUSE_LABELS[disasterType] ? text(lang, CAUSE_LABELS[disasterType]) : disasterType.replace(/_/g, ' ');
    return `${text(lang, DISASTER_PREFIX)}: ${label}`;
  }
  return MIGRATION_REASON_LABELS[key] ? text(lang, MIGRATION_REASON_LABELS[key]) : reason;
}

// technology.rs's TECH_TREE ids -- leaked untranslated as both timeline
// badges and the "Technology" table column.
const TECH_LABELS: Record<string, TranslationMap> = {
  fire_making:        { tr: 'ateş yakma',          en: 'fire making',        de: 'Feuermachen',            fr: 'fabrication du feu',       ar: 'صنع النار' },
  stone_tools:        { tr: 'taş aletler',         en: 'stone tools',        de: 'Steinwerkzeuge',         fr: 'outils en pierre',         ar: 'أدوات حجرية' },
  foraging:           { tr: 'yiyecek toplama',     en: 'foraging',           de: 'Nahrungssuche',          fr: 'cueillette',               ar: 'جمع الطعام' },
  hunting_spear:      { tr: 'av mızrağı',          en: 'hunting spear',      de: 'Jagdspeer',              fr: 'lance de chasse',          ar: 'رمح الصيد' },
  shelter_basic:      { tr: 'temel barınak',       en: 'basic shelter',      de: 'einfache Unterkunft',    fr: 'abri de base',             ar: 'مأوى أساسي' },
  water_container:    { tr: 'su kabı',             en: 'water container',    de: 'Wasserbehälter',         fr: 'récipient à eau',          ar: 'وعاء ماء' },
  animal_trap:        { tr: 'hayvan tuzağı',       en: 'animal trap',        de: 'Tierfalle',              fr: 'piège à animaux',          ar: 'فخ حيواني' },
  clothing_basic:     { tr: 'temel giysi',         en: 'basic clothing',     de: 'einfache Kleidung',      fr: 'vêtement de base',         ar: 'ملابس أساسية' },
  swimming:           { tr: 'yüzme',               en: 'swimming',           de: 'Schwimmen',              fr: 'natation',                 ar: 'السباحة' },
  fishing:            { tr: 'balıkçılık',          en: 'fishing',            de: 'Fischerei',              fr: 'pêche',                    ar: 'صيد السمك' },
  plant_cultivation:  { tr: 'bitki yetiştiriciliği', en: 'plant cultivation', de: 'Pflanzenanbau',         fr: 'culture des plantes',      ar: 'زراعة النباتات' },
  animal_herding:     { tr: 'hayvan çobanlığı',    en: 'animal herding',     de: 'Tierhaltung',            fr: 'élevage',                  ar: 'رعي الحيوانات' },
  food_preservation:  { tr: 'gıda saklama',        en: 'food preservation',  de: 'Lebensmittelkonservierung', fr: 'conservation des aliments', ar: 'حفظ الطعام' },
  bow_arrow:          { tr: 'yay ve ok',           en: 'bow and arrow',      de: 'Pfeil und Bogen',        fr: 'arc et flèche',            ar: 'القوس والسهم' },
  pottery:            { tr: 'çömlekçilik',         en: 'pottery',            de: 'Töpferei',               fr: 'poterie',                  ar: 'الفخار' },
  weaving:            { tr: 'dokumacılık',         en: 'weaving',            de: 'Weberei',                fr: 'tissage',                  ar: 'النسيج' },
  metallurgy_copper:  { tr: 'bakır metalurjisi',   en: 'copper metallurgy',  de: 'Kupfermetallurgie',      fr: 'métallurgie du cuivre',    ar: 'معالجة النحاس' },
  writing_system:     { tr: 'yazı sistemi',        en: 'writing system',     de: 'Schriftsystem',          fr: 'système d\'écriture',      ar: 'نظام الكتابة' },
  calendar:           { tr: 'takvim',              en: 'calendar',           de: 'Kalender',               fr: 'calendrier',               ar: 'التقويم' },
  mathematics_basic:  { tr: 'temel matematik',     en: 'basic mathematics',  de: 'Grundmathematik',        fr: 'mathématiques de base',    ar: 'الرياضيات الأساسية' },
  architecture_stone: { tr: 'taş mimari',          en: 'stone architecture', de: 'Steinarchitektur',       fr: 'architecture en pierre',   ar: 'العمارة الحجرية' },
  wheel:              { tr: 'tekerlek',            en: 'wheel',              de: 'Rad',                    fr: 'roue',                     ar: 'العجلة' },
  irrigation:         { tr: 'sulama',              en: 'irrigation',         de: 'Bewässerung',            fr: 'irrigation',               ar: 'الري' },
  sailing:            { tr: 'yelkencilik',         en: 'sailing',            de: 'Segeln',                 fr: 'navigation à voile',       ar: 'الإبحار' },
  metallurgy_iron:    { tr: 'demir metalurjisi',   en: 'iron metallurgy',    de: 'Eisenmetallurgie',       fr: 'métallurgie du fer',       ar: 'معالجة الحديد' },
};

export function translateTech(techId: string, lang: LangCode): string {
  const key = (techId ?? '').trim().toLowerCase();
  if (!key) return '—';
  return TECH_LABELS[key] ? text(lang, TECH_LABELS[key]) : techId;
}

// art.rs's ART_FORMS ids -- leaked untranslated as both timeline badges and
// the "Name" table column (distinct from ART_DESC_TR, which translates the
// full sentence description, not the short form name).
const ART_FORM_LABELS: Record<string, TranslationMap> = {
  cave_painting:      { tr: 'mağara resmi',    en: 'cave painting',      de: 'Höhlenmalerei',        fr: 'peinture rupestre',   ar: 'رسم الكهوف' },
  sculpture:          { tr: 'heykel',          en: 'sculpture',          de: 'Skulptur',             fr: 'sculpture',           ar: 'نحت' },
  pottery_decoration: { tr: 'çömlek süslemesi', en: 'pottery decoration', de: 'Töpferverzierung',   fr: 'décoration de poterie', ar: 'زخرفة الفخار' },
  textile_pattern:    { tr: 'tekstil deseni',  en: 'textile pattern',    de: 'Textilmuster',         fr: 'motif textile',       ar: 'نمط النسيج' },
  architecture_art:   { tr: 'mimari sanat',    en: 'architectural art',  de: 'Architekturkunst',     fr: 'art architectural',   ar: 'فن معماري' },
  rhythmic_percussion:{ tr: 'ritmik perküsyon', en: 'rhythmic percussion', de: 'rhythmische Perkussion', fr: 'percussion rythmique', ar: 'إيقاع نغمي' },
  vocal_melody:       { tr: 'vokal melodi',    en: 'vocal melody',      de: 'Vokalmelodie',          fr: 'mélodie vocale',      ar: 'لحن صوتي' },
  flute_bone:         { tr: 'kemik flüt',      en: 'bone flute',        de: 'Knochenflöte',          fr: 'flûte en os',         ar: 'ناي عظمي' },
  string_instrument:  { tr: 'telli çalgı',     en: 'string instrument', de: 'Saiteninstrument',      fr: 'instrument à cordes', ar: 'آلة وترية' },
  oral_story:         { tr: 'sözlü anlatı',    en: 'oral story',        de: 'mündliche Erzählung',   fr: 'récit oral',          ar: 'حكاية شفوية' },
  epic_poem:          { tr: 'destan',          en: 'epic poem',         de: 'Epos',                  fr: 'poème épique',        ar: 'قصيدة ملحمية' },
  written_story:      { tr: 'yazılı anlatı',   en: 'written story',     de: 'geschriebene Erzählung', fr: 'récit écrit',        ar: 'قصة مكتوبة' },
};

export function translateArtForm(artId: string, lang: LangCode): string {
  const key = (artId ?? '').trim().toLowerCase();
  if (!key) return '—';
  return ART_FORM_LABELS[key] ? text(lang, ART_FORM_LABELS[key]) : artId;
}

// psychology.rs's 6 possible `mental_state` values -- PopulationPanel.tsx
// used to keep its own local tr/en-only copy of this (missing de/fr/ar
// entirely), so German/French/Arabic users saw raw English ("Calm",
// "Anxious", ...) in the Individual Detail modal's Mood row regardless of
// their selected language.
const MENTAL_STATE_LABELS: Record<string, TranslationMap> = {
  calm:      { tr: 'Sakin',    en: 'Calm',      de: 'Ruhig',       fr: 'Calme',       ar: 'هادئ' },
  content:   { tr: 'Memnun',   en: 'Content',   de: 'Zufrieden',   fr: 'Content',     ar: 'راضٍ' },
  excited:   { tr: 'Heyecanlı', en: 'Excited',  de: 'Aufgeregt',   fr: 'Excité',      ar: 'متحمس' },
  anxious:   { tr: 'Kaygılı',  en: 'Anxious',   de: 'Ängstlich',   fr: 'Anxieux',     ar: 'قلق' },
  depressed: { tr: 'Depresif', en: 'Depressed', de: 'Deprimiert',  fr: 'Déprimé',     ar: 'مكتئب' },
  grieving:  { tr: 'Yasında',  en: 'Grieving',  de: 'Trauernd',    fr: 'En deuil',    ar: 'حزين' },
};

export function translateMentalState(state: string, lang: LangCode): string {
  const key = (state ?? '').trim().toLowerCase();
  if (!key) return '—';
  return MENTAL_STATE_LABELS[key] ? text(lang, MENTAL_STATE_LABELS[key]) : state;
}

// social.rs's GROUP_ROLES -- leaked untranslated into the report's
// Individuals table "Role" column.
const ROLE_LABELS: Record<string, TranslationMap> = {
  leader:   { tr: 'lider',    en: 'leader',   de: 'Anführer', fr: 'chef',      ar: 'زعيم' },
  elder:    { tr: 'yaşlı',    en: 'elder',    de: 'Ältester',  fr: 'aîné',      ar: 'شيخ' },
  warrior:  { tr: 'savaşçı',  en: 'warrior',  de: 'Krieger',   fr: 'guerrier',  ar: 'محارب' },
  gatherer: { tr: 'toplayıcı', en: 'gatherer', de: 'Sammler',  fr: 'cueilleur', ar: 'جامع' },
  healer:   { tr: 'şifacı',   en: 'healer',   de: 'Heiler',    fr: 'guérisseur', ar: 'معالج' },
  member:   { tr: 'üye',      en: 'member',   de: 'Mitglied',  fr: 'membre',    ar: 'عضو' },
};

export function translateRole(role: string, lang: LangCode): string {
  const key = (role ?? '').trim().toLowerCase();
  if (!key) return '—';
  return ROLE_LABELS[key] ? text(lang, ROLE_LABELS[key]) : role;
}

const ART_DESC_TR: Record<string, string> = {
  'Pigments applied to rock surfaces depict animals and figures': 'Kaya yüzeylerine uygulanan pigmentler hayvanları ve figürleri tasvir eder',
  'Three-dimensional forms carved from stone or bone': 'Taş veya kemikten oyulan üç boyutlu formlar',
  'Geometric and figurative patterns adorn ceramic surfaces': 'Geometrik ve figüratif desenler seramik yüzeyleri süsler',
  'Woven cloth bears complex repeating patterns': 'Dokuma kumaş karmaşık tekrarlayan desenler taşır',
  'Buildings are decorated with carved reliefs and motifs': 'Binalar oyma kabartmalar ve motiflerle süslenir',
  'Stones and bones struck together in rhythmic patterns': 'Taşlar ve kemikler ritimli örüntülerle birbirine vurulur',
  'Sustained pitched vocalizations form melodic sequences': 'Sürdürülen tonlu sesler melodik diziler oluşturur',
  'A hollow bone with finger holes produces musical tones': 'Parmak delikli oyuk bir kemik müzikal sesler üretir',
  'A taut cord vibrates to produce musical notes': 'Gergin bir ip titreşerek müzikal notalar üretir',
  'Narrative accounts passed between individuals by spoken word': 'Bireyler arasında sözlü olarak aktarılan anlatılar',
  'Long rhythmic verse recounts heroic deeds and origins': 'Uzun ritmik dizeler kahramanca eylemleri ve kökenleri anlatır',
  'Narrative accounts preserved in written symbols': 'Yazılı sembollerde korunan anlatılar',
};

const CULTURE_DESC_TR: Record<string, string> = {
  'A consistent greeting gesture develops': 'Tutarlı bir selamlama jesti gelişir',
  'Communal mourning practices emerge for the dead': 'Ölüler için toplumsal yas uygulamaları ortaya çıkar',
  'Food is shared equally among group members': 'Yiyecek grup üyeleri arasında eşit paylaşılır',
  'Gifts and favors are expected to be returned': 'Hediyelerin ve iyiliklerin karşılıklı verilmesi beklenir',
  'Different tasks become associated with different sexes': 'Farklı görevler farklı cinsiyetlerle ilişkilendirilir',
  'Elders are accorded special respect': 'Yaşlılara özel saygı gösterilir',
  'Ceremonial gift-giving strengthens social bonds': 'Törensel hediye verme sosyal bağları güçlendirir',
  'Pigments and natural materials used for body adornment': 'Beden süslemesi için pigmentler ve doğal malzemeler kullanılır',
  'Oral narratives preserve group memory and values': 'Sözlü anlatılar grup belleğini ve değerlerini korur',
  'Rhythmic percussion emerges as social bonding activity': 'Ritmik vurma sosyal bağ kurma etkinliği olarak ortaya çıkar',
  'Coordinated movement used in group ceremonies': 'Grup törenlerinde koordineli hareket kullanılır',
  'Birth is marked with naming rites': 'Doğum, isim verme ritüelleriyle kutlanır',
  'Pair-bonding is formalized through ritual': 'Çift bağı ritüel aracılığıyla resmîleştirilir',
  'Cyclical celebrations mark the seasons': 'Döngüsel kutlamalar mevsimleri işaretler',
  'Certain behaviors become culturally forbidden': 'Belirli davranışlar kültürel olarak yasaklanır',
  'Exchange is ritualized to build trust': 'Güven inşa etmek için alışveriş ritüelleştirilir',
  'Origin stories are recorded in written form': 'Köken hikayeleri yazılı biçimde kaydedilir',
  'Rules and punishments are written and formalized': 'Kurallar ve cezalar yazılır ve resmîleştirilir',
};

const LAW_DESC_TR: Record<string, string> = {
  'Members are expected to return favors': 'Üyelerin iyilikleri karşılıklı olarak iade etmesi beklenir',
  "Taking others' possessions is prohibited": "Başkalarının eşyalarını almak yasaktır",
  'Mating between close relatives is forbidden': 'Yakın akrabalar arasındaki çiftleşme yasaktır',
  'Elders are addressed with deference': 'Yaşlılara saygıyla davranılır',
  'Strangers must be offered food and shelter': 'Yabancılara yiyecek ve barınak sunulmalıdır',
  'Violence against a kin member demands revenge': 'Bir akraba üyeye yönelik şiddet intikam gerektirir',
  'All able members must contribute to group tasks': 'Tüm yetenekli üyeler grup görevlerine katkıda bulunmalıdır',
  'The leader resolves disputes': 'Lider anlaşmazlıkları çözer',
  'Individual ownership of goods is recognized': 'Malların bireysel mülkiyeti tanınır',
  'Persistent violators may be driven out': 'Sürekli ihlal edenler topluluktan kovulabilir',
  'Rules are codified in written form': 'Kurallar yazılı biçimde kodlanmıştır',
  'Members contribute a portion of resources to the group': 'Üyeler kaynaklarının bir bölümünü gruba katkıda bulunur',
  'Agreements between parties are legally binding': 'Taraflar arasındaki anlaşmalar hukuken bağlayıcıdır',
};

const LAW_DESC_DE: Record<string, string> = {
  'Members are expected to return favors': 'Von Mitgliedern wird erwartet, Gefälligkeiten zu erwidern',
  "Taking others' possessions is prohibited": 'Das Nehmen fremden Besitzes ist verboten',
  'Mating between close relatives is forbidden': 'Paarung zwischen nahen Verwandten ist verboten',
  'Elders are addressed with deference': 'Ältere werden mit Respekt behandelt',
  'Strangers must be offered food and shelter': 'Fremden müssen Nahrung und Unterkunft angeboten werden',
  'Violence against a kin member demands revenge': 'Gewalt gegen ein Familienmitglied verlangt Rache',
  'All able members must contribute to group tasks': 'Alle fähigen Mitglieder müssen zu Gruppenaufgaben beitragen',
  'The leader resolves disputes': 'Der Anführer löst Streitigkeiten',
  'Individual ownership of goods is recognized': 'Individuelles Eigentum an Gütern wird anerkannt',
  'Persistent violators may be driven out': 'Wiederholte Verstöße können zur Vertreibung führen',
  'Rules are codified in written form': 'Regeln werden schriftlich festgehalten',
  'Members contribute a portion of resources to the group': 'Mitglieder tragen einen Teil der Ressourcen zur Gruppe bei',
  'Agreements between parties are legally binding': 'Vereinbarungen zwischen Parteien sind rechtlich bindend',
};

const LAW_DESC_FR: Record<string, string> = {
  'Members are expected to return favors': 'Les membres doivent rendre les faveurs reçues',
  "Taking others' possessions is prohibited": "Il est interdit de prendre les biens d'autrui",
  'Mating between close relatives is forbidden': 'L\'accouplement entre proches parents est interdit',
  'Elders are addressed with deference': 'Les aînés sont traités avec déférence',
  'Strangers must be offered food and shelter': 'Les étrangers doivent recevoir nourriture et abri',
  'Violence against a kin member demands revenge': 'La violence contre un proche exige vengeance',
  'All able members must contribute to group tasks': 'Tous les membres valides doivent contribuer aux tâches du groupe',
  'The leader resolves disputes': 'Le chef résout les conflits',
  'Individual ownership of goods is recognized': 'La propriété individuelle des biens est reconnue',
  'Persistent violators may be driven out': 'Les récidivistes peuvent être bannis',
  'Rules are codified in written form': 'Les règles sont codifiées par écrit',
  'Members contribute a portion of resources to the group': 'Les membres contribuent une part des ressources au groupe',
  'Agreements between parties are legally binding': 'Les accords entre parties sont juridiquement contraignants',
};

const LAW_DESC_AR: Record<string, string> = {
  'Members are expected to return favors': 'يُتوقع من الأعضاء رد الجميل',
  "Taking others' possessions is prohibited": 'يُمنع أخذ ممتلكات الآخرين',
  'Mating between close relatives is forbidden': 'يُحظر التزاوج بين الأقارب المقربين',
  'Elders are addressed with deference': 'يُخاطَب كبار السن باحترام',
  'Strangers must be offered food and shelter': 'يجب تقديم الطعام والمأوى للغرباء',
  'Violence against a kin member demands revenge': 'العنف ضد أحد الأقارب يستوجب الثأر',
  'All able members must contribute to group tasks': 'يجب على جميع الأعضاء القادرين المساهمة في مهام المجموعة',
  'The leader resolves disputes': 'يحل الزعيم النزاعات',
  'Individual ownership of goods is recognized': 'المِلكية الفردية للممتلكات معترف بها',
  'Persistent violators may be driven out': 'قد يُطرد المخالفون المتكررون',
  'Rules are codified in written form': 'تُدوَّن القواعد كتابةً',
  'Members contribute a portion of resources to the group': 'يساهم الأعضاء بجزء من الموارد للمجموعة',
  'Agreements between parties are legally binding': 'الاتفاقات بين الأطراف مُلزمة قانونياً',
};

const ASTRO_DESC_TR: Record<string, string> = {
  'The moon completes another cycle of phases': 'Ay bir evre döngüsünü daha tamamlar',
  'The sun reaches its extreme position': 'Güneş en uç konumuna ulaşır',
  'Day and night are of equal length': 'Gündüz ve gece eşit uzunluktadır',
  'A prominent star rises at sunset': 'Gün batımında belirgin bir yıldız yükselir',
  'The sun is obscured — a solar eclipse': 'Güneş görünmez olur — güneş tutulması',
  'The moon turns blood red — a lunar eclipse': 'Ay kan kırmızısına döner — ay tutulması',
  'A wandering star moves against the fixed stars': 'Gezgin bir yıldız sabit yıldızlara karşı hareket eder',
  'A bright object with a tail crosses the sky': 'Kuyruklu parlak bir nesne gökyüzünü geçer',
  'The phases of the moon can be predicted': 'Ay evreleri tahmin edilebilir hale geldi',
  'A calendar based on sun and moon positions is developed': 'Güneş ve ay konumlarına dayalı bir takvim geliştirildi',
  'Named star constellations guide navigation': 'Adlandırılmış yıldız takımyıldızları yön bulmaya yardım eder',
  'Solar and lunar eclipses can be predicted': 'Güneş ve ay tutulmaları tahmin edilebilir',
  'A model explains the motion of wandering stars': 'Gezgin yıldızların hareketi bir modelle açıklandı',
};

const TECH_TR: Record<string, string> = {
  fire_making: 'Ateş Yakma', 'fire making': 'Ateş Yakma', stone_tools: 'Taş Aletler', 'stone tools': 'Taş Aletler',
  foraging: 'Toplayıcılık', water_container: 'Su Kabı', 'water container': 'Su Kabı', fishing: 'Balıkçılık',
  hunting_spear: 'Av Mızrağı', 'hunting spear': 'Av Mızrağı', shelter_basic: 'Temel Barınak', 'shelter basic': 'Temel Barınak',
  animal_trap: 'Hayvan Tuzağı', 'animal trap': 'Hayvan Tuzağı', clothing_basic: 'Giysi', 'clothing basic': 'Giysi',
  plant_cultivation: 'Tarım', 'plant cultivation': 'Tarım', animal_herding: 'Hayvancılık', 'animal herding': 'Hayvancılık',
  food_preservation: 'Gıda Saklama', 'food preservation': 'Gıda Saklama', bow_arrow: 'Yay ve Ok', 'bow arrow': 'Yay ve Ok',
  pottery: 'Çömlekçilik', weaving: 'Dokumacılık', metallurgy_copper: 'Bakır İşleme', 'metallurgy copper': 'Bakır İşleme',
  writing_system: 'Yazı Sistemi', 'writing system': 'Yazı Sistemi', calendar: 'Takvim', mathematics_basic: 'Temel Matematik',
  'mathematics basic': 'Temel Matematik', architecture_stone: 'Taş Mimari', 'architecture stone': 'Taş Mimari', wheel: 'Tekerlek',
  irrigation: 'Sulama', sailing: 'Denizcilik', metallurgy_iron: 'Demir İşleme', 'metallurgy iron': 'Demir İşleme',
};

const PATHOGEN_TR: Record<string, string> = {
  'intestinal parasite': 'Bağırsak paraziti', 'cholera like': 'Kolera benzeri hastalık', 'respiratory common': 'Solunum yolu enfeksiyonu',
  'pneumonia like': 'Zatürre benzeri hastalık', 'plague like': 'Veba benzeri hastalık', 'malaria like': 'Sıtma benzeri hastalık',
  'fever tick': 'Kene ateşi', 'wound infection': 'Yara enfeksiyonu', 'fungal skin': 'Mantar derisi hastalığı',
};

const STRUCTURE_TR: Record<string, string> = {
  'lean to': 'sığınak', 'pit house': 'çukur ev', 'post frame hut': 'direkli kulübe', 'storage pit': 'depo çukuru',
  'mud brick house': 'kerpiç ev', granary: 'tahıl ambarı', 'defensive wall': 'savunma duvarı', 'stone temple': 'taş tapınak',
  'stone house': 'taş ev', marketplace: 'pazar yeri', 'city wall': 'şehir surları', 'cave dwelling': 'mağara konutu',
};

// wildfire/blizzard_disaster/drought_event are the exact strings the Rust
// engine's pick_natural_disaster() emits -- aliased to the same translations
// as fire/storm/drought so they don't fall back to raw English.
const DISASTER_TR: Record<string, string> = {
  earthquake: 'deprem', flood: 'sel', drought: 'kuraklık', fire: 'yangın', conflict: 'çatışma', volcano: 'yanardağ patlaması',
  storm: 'fırtına', tsunami: 'tsunami', landslide: 'heyelan',
  wildfire: 'yangın', blizzard_disaster: 'fırtına', drought_event: 'kuraklık',
  meteor_tsunami: 'meteor çarpması ve tsunami',
};

const CAUSE_DE: Record<string, string> = {
  starvation: 'Verhungern', dehydration: 'Austrocknung', old_age: 'Alter',
  predator: 'Raubtier', genetic_disease: 'Erbkrankheit', infection: 'Infektion',
  trauma: 'Trauma', birth_complications: 'Geburtskomplikationen', conflict: 'Konflikt',
  drowning: 'Ertrinken', meteor_tsunami: 'Meteoriteneinschlag und Tsunami', unknown: 'Unbekannte Ursache'
};
const CAUSE_FR: Record<string, string> = {
  starvation: 'Famine', dehydration: 'Déshydratation', old_age: 'Vieillesse',
  predator: 'Prédateur', genetic_disease: 'Maladie génétique', infection: 'Infection',
  trauma: 'Traumatisme', birth_complications: 'Complications à la naissance',
  conflict: 'Conflit', drowning: 'noyade', meteor_tsunami: 'impact de météore et tsunami', unknown: 'Cause inconnue'
};
const CAUSE_AR: Record<string, string> = {
  starvation: 'مجاعة', dehydration: 'جفاف', old_age: 'الشيخوخة',
  predator: 'مفترس', genetic_disease: 'مرض وراثي', infection: 'عدوى',
  trauma: 'صدمة', birth_complications: 'مضاعفات الولادة', conflict: 'نزاع',
  drowning: 'غرق', meteor_tsunami: 'اصطدام نيزك وتسونامي', unknown: 'سبب مجهول'
};
const DISASTER_DE: Record<string, string> = {
  earthquake: 'Erdbeben', flood: 'Flut', drought: 'Dürre', fire: 'Feuer',
  conflict: 'Konflikt', volcano: 'Vulkanausbruch', storm: 'Sturm',
  tsunami: 'Tsunami', landslide: 'Erdrutsch',
  wildfire: 'Feuer', blizzard_disaster: 'Sturm', drought_event: 'Dürre',
  meteor_tsunami: 'Meteoriteneinschlag und Tsunami',
};
const DISASTER_FR: Record<string, string> = {
  earthquake: 'Séisme', flood: 'Inondation', drought: 'Sécheresse', fire: 'Incendie',
  conflict: 'Conflit', volcano: 'Éruption volcanique', storm: 'Tempête',
  tsunami: 'Tsunami', landslide: 'Glissement de terrain',
  wildfire: 'Incendie', blizzard_disaster: 'Tempête', drought_event: 'Sécheresse',
  meteor_tsunami: 'impact de météore et tsunami',
};
const DISASTER_AR: Record<string, string> = {
  earthquake: 'زلزال', flood: 'فيضان', drought: 'جفاف', fire: 'حريق',
  conflict: 'نزاع', volcano: 'ثوران بركاني', storm: 'عاصفة',
  tsunami: 'تسونامي', landslide: 'انهيار أرضي',
  wildfire: 'حريق', blizzard_disaster: 'عاصفة', drought_event: 'جفاف',
  meteor_tsunami: 'اصطدام نيزك وتسونامي',
};

// Belief labels are procedurally generated per simulation (see
// sim-core's belief::try_label_belief) -- never a fixed real-world religion
// name, so there is nothing to translate here. The regexes below pass a
// generated label straight through and only localize the surrounding
// sentence + the neutral "not yet named" fallback text.

// Belief archetype ids ("belief_1".."belief_6", see sim-core's
// BELIEF_ARCHETYPES) are opaque engine bucketing keys -- never a real-world
// religion name, and this file must never invent one either. But an opaque
// code alone ("belief_5") tells a player nothing about what kind of belief
// this is, before the population's own language eventually names it. Each
// description below is built only from the same mechanical thresholds
// BELIEF_ARCHETYPES itself gates on (required language stage, IQ, foxp2
// expression, prerequisite tech) -- never a religion or deity concept -- so
// showing it can never assert that this population invented any specific
// real-world belief system.
const BELIEF_DESCRIPTIONS: Record<string, TranslationMap> = {
  belief_1: {
    tr: 'Jestsel iletişimin var olduğu anda ortaya çıkabilen, ileri dil veya teknoloji gerektirmeyen erken ve basit bir inanç.',
    en: 'An early, simple belief that can take hold as soon as gestural communication exists -- no advanced language or technology required.',
    de: 'Ein früher, einfacher Glaube, der schon bei gestischer Kommunikation entstehen kann -- keine fortgeschrittene Sprache oder Technologie nötig.',
    fr: "Une croyance précoce et simple qui peut apparaître dès que la communication gestuelle existe -- aucun langage ou technologie avancée requis.",
    ar: 'معتقد مبكر وبسيط يمكن أن ينشأ بمجرد وجود التواصل الإيمائي - دون الحاجة إلى لغة أو تقنية متقدمة.',
  },
  belief_2: {
    tr: 'Şekillenmesi için temel sözlü kelimeler ve orta düzey zekâ gerektiren bir inanç.',
    en: 'A belief that requires basic spoken words and moderate intelligence to take shape.',
    de: 'Ein Glaube, der grundlegende gesprochene Wörter und mittlere Intelligenz erfordert, um Gestalt anzunehmen.',
    fr: 'Une croyance qui nécessite des mots parlés de base et une intelligence modérée pour se former.',
    ar: 'معتقد يتطلب كلمات منطوقة أساسية وذكاءً متوسطًا ليتشكل.',
  },
  belief_3: {
    tr: 'Aynı erken kelime aşamasında, biraz daha yüksek zekâ ve dil yeteneği gerektiren daha zorlu bir inanç.',
    en: 'A more demanding belief within the same early-word stage, requiring somewhat higher intelligence and language ability.',
    de: 'Ein anspruchsvollerer Glaube derselben frühen Wortstufe, der etwas höhere Intelligenz und Sprachfähigkeit erfordert.',
    fr: 'Une croyance plus exigeante au sein de la même étape de mots précoces, nécessitant une intelligence et une capacité langagière un peu plus élevées.',
    ar: 'معتقد أكثر تطلبًا ضمن نفس مرحلة الكلمات المبكرة، يتطلب ذكاءً وقدرة لغوية أعلى قليلاً.',
  },
  belief_4: {
    tr: 'Topluluk proto-kelimeler ifade edebildiğinde ve çömlekçiliği geliştirdiğinde ortaya çıkan bir inanç.',
    en: 'A belief that only emerges once the population can express proto-words and has developed pottery.',
    de: 'Ein Glaube, der erst entsteht, wenn die Bevölkerung Proto-Wörter ausdrücken kann und Töpferei entwickelt hat.',
    fr: "Une croyance qui n'émerge que lorsque la population peut exprimer des proto-mots et a développé la poterie.",
    ar: 'معتقد لا ينشأ إلا عندما يستطيع المجتمع التعبير عن كلمات أولية ويكون قد طور صناعة الفخار.',
  },
  belief_5: {
    tr: 'Ortaya çıkması için karmaşık dil, bir yazı sistemi ve temel matematik gerektiren gelişmiş bir inanç.',
    en: 'A sophisticated belief that requires complex language, a written system, and basic mathematics to emerge.',
    de: 'Ein anspruchsvoller Glaube, der komplexe Sprache, ein Schriftsystem und grundlegende Mathematik erfordert.',
    fr: "Une croyance sophistiquée qui nécessite un langage complexe, un système d'écriture et des mathématiques de base.",
    ar: 'معتقد متطور يتطلب لغة معقدة ونظام كتابة ورياضيات أساسية لكي ينشأ.',
  },
  belief_6: {
    tr: 'En zorlu inanç katmanı: karmaşık dil, yazı ve matematiğin yanı sıra en yüksek ölçülen zekâ ve dil-geni ifadesini gerektirir.',
    en: 'The most demanding belief tier: complex language, writing, and mathematics, plus the highest measured intelligence and language-gene expression.',
    de: 'Die anspruchsvollste Glaubensstufe: komplexe Sprache, Schrift und Mathematik sowie die höchste gemessene Intelligenz und Sprachgen-Expression.',
    fr: "Le palier de croyance le plus exigeant : langage complexe, écriture et mathématiques, ainsi que l'intelligence et l'expression du gène du langage les plus élevées mesurées.",
    ar: 'أكثر مستويات المعتقد تطلبًا: لغة معقدة وكتابة ورياضيات، إضافة إلى أعلى مستوى مقاس من الذكاء والتعبير الجيني اللغوي.',
  },
};

// Extracts the opaque numeric suffix from a belief_id ("belief_5" -> "5"),
// falling back to the raw id itself if it doesn't match the expected shape.
export function beliefCodeNumber(beliefId: string): string {
  return beliefId.match(/^belief_(\d+)$/)?.[1] ?? beliefId;
}

// A short, neutral, mechanically-derived description of a belief archetype,
// for display alongside its opaque code before the population's own
// language has named it (see BELIEF_DESCRIPTIONS above).
export function describeBeliefCode(beliefId: string, lang: LangCode): string {
  const desc = BELIEF_DESCRIPTIONS[beliefId];
  return desc ? text(lang, desc) : '';
}

const EXACT_DESC_DE: Record<string, string> = {
  'Pigments applied to rock surfaces depict animals and figures': 'Auf Felsoberflächen aufgetragene Pigmente zeigen Tiere und Figuren',
  'Three-dimensional forms carved from stone or bone': 'Dreidimensionale Formen aus Stein oder Knochen geschnitzt',
  'Geometric and figurative patterns adorn ceramic surfaces': 'Geometrische und figurative Muster schmücken Keramikoberflächen',
  'Woven cloth bears complex repeating patterns': 'Gewebter Stoff trägt komplexe Wiederholungsmuster',
  'Buildings are decorated with carved reliefs and motifs': 'Gebäude sind mit geschnitzten Reliefs und Motiven verziert',
  'Stones and bones struck together in rhythmic patterns': 'Steine und Knochen werden rhythmisch zusammengeschlagen',
  'Sustained pitched vocalizations form melodic sequences': 'Anhaltende Tonvokalisierungen bilden melodische Sequenzen',
  'A hollow bone with finger holes produces musical tones': 'Ein hohler Knochen mit Fingerlöchern erzeugt musikalische Töne',
  'A taut cord vibrates to produce musical notes': 'Eine gespannte Saite schwingt und erzeugt Töne',
  'Narrative accounts passed between individuals by spoken word': 'Erzählungen werden mündlich weitergegeben',
  'Long rhythmic verse recounts heroic deeds and origins': 'Lange rhythmische Verse erzählen von heroischen Taten',
  'Narrative accounts preserved in written symbols': 'Erzählungen in Schriftsymbolen erhalten',
  'A consistent greeting gesture develops': 'Eine einheitliche Begrüßungsgeste entwickelt sich',
  'Communal mourning practices emerge for the dead': 'Gemeinschaftliche Trauerrituale entstehen',
  'Food is shared equally among group members': 'Nahrung wird gleichmäßig geteilt',
  'Gifts and favors are expected to be returned': 'Geschenke und Gefälligkeiten werden erwidert',
  'Different tasks become associated with different sexes': 'Verschiedene Aufgaben werden verschiedenen Geschlechtern zugeordnet',
  'Elders are accorded special respect': 'Älteste genießen besonderen Respekt',
  'Ceremonial gift-giving strengthens social bonds': 'Zeremonielles Schenken stärkt soziale Bindungen',
  'Pigments and natural materials used for body adornment': 'Pigmente für Körperverzierung verwendet',
  'Oral narratives preserve group memory and values': 'Mündliche Erzählungen bewahren das Gedächtnis der Gruppe',
  'Rhythmic percussion emerges as social bonding activity': 'Rhythmisches Schlagzeug als soziale Bindungsaktivität',
  'Coordinated movement used in group ceremonies': 'Koordinierte Bewegung bei Gruppenzeremonien',
  'Birth is marked with naming rites': 'Geburt wird mit Benennungsriten markiert',
  'Pair-bonding is formalized through ritual': 'Paarbindung wird durch Ritual formalisiert',
  'Cyclical celebrations mark the seasons': 'Zyklische Feiern markieren die Jahreszeiten',
  'Certain behaviors become culturally forbidden': 'Bestimmte Verhaltensweisen werden kulturell verboten',
  'Exchange is ritualized to build trust': 'Austausch wird ritualisiert',
  'Origin stories are recorded in written form': 'Ursprungsgeschichten werden schriftlich festgehalten',
  'Rules and punishments are written and formalized': 'Regeln und Strafen werden formalisiert',
  'Members are expected to return favors': 'Von Mitgliedern wird erwartet Gefälligkeiten zu erwidern',
  "Taking others' possessions is prohibited": 'Das Entnehmen fremder Besitztümer ist verboten',
  'Mating between close relatives is forbidden': 'Paarung zwischen engen Verwandten ist verboten',
  'Elders are addressed with deference': 'Älteste werden mit Ehrerbietung behandelt',
  'Strangers must be offered food and shelter': 'Fremden muss Nahrung und Unterkunft angeboten werden',
  'Violence against a kin member demands revenge': 'Gewalt gegen Verwandte fordert Rache',
  'All able members must contribute to group tasks': 'Alle fähigen Mitglieder müssen beitragen',
  'The leader resolves disputes': 'Der Anführer löst Streitigkeiten',
  'Individual ownership of goods is recognized': 'Individuelles Eigentum wird anerkannt',
  'Persistent violators may be driven out': 'Hartnäckige Verstöße können zur Vertreibung führen',
  'Rules are codified in written form': 'Regeln sind schriftlich kodifiziert',
  'Members contribute a portion of resources to the group': 'Mitglieder tragen Ressourcen zur Gruppe bei',
  'Agreements between parties are legally binding': 'Vereinbarungen sind rechtlich bindend',
  'The moon completes another cycle of phases': 'Der Mond schließt einen weiteren Phasenzyklus ab',
  'The sun reaches its extreme position': 'Die Sonne erreicht ihre Extremposition',
  'Day and night are of equal length': 'Tag und Nacht sind gleich lang',
  'A prominent star rises at sunset': 'Ein markanter Stern geht bei Sonnenuntergang auf',
  'The sun is obscured — a solar eclipse': 'Die Sonne wird verdeckt — Sonnenfinsternis',
  'The moon turns blood red — a lunar eclipse': 'Der Mond wird blutrot — Mondfinsternis',
  'A wandering star moves against the fixed stars': 'Ein Wanderstern bewegt sich gegen die Fixsterne',
  'A bright object with a tail crosses the sky': 'Ein helles Objekt mit Schweif überquert den Himmel',
  'The phases of the moon can be predicted': 'Die Mondphasen können vorhergesagt werden',
  'A calendar based on sun and moon positions is developed': 'Ein Kalender wird entwickelt',
  'Named star constellations guide navigation': 'Benannte Sternkonstellationen leiten die Navigation',
  'Solar and lunar eclipses can be predicted': 'Finsternisse können vorhergesagt werden',
  'A model explains the motion of wandering stars': 'Ein Modell erklärt die Bewegung der Wandersterne',
};

const EXACT_DESC_FR: Record<string, string> = {
  'Pigments applied to rock surfaces depict animals and figures': 'Des pigments sur des surfaces rocheuses représentent des animaux',
  'Three-dimensional forms carved from stone or bone': 'Des formes tridimensionnelles sculptées dans la pierre ou l\'os',
  'Geometric and figurative patterns adorn ceramic surfaces': 'Des motifs ornent les surfaces céramiques',
  'Woven cloth bears complex repeating patterns': 'Le tissu tissé porte des motifs répétitifs',
  'Buildings are decorated with carved reliefs and motifs': 'Les bâtiments sont décorés de reliefs sculptés',
  'Stones and bones struck together in rhythmic patterns': 'Des pierres et des os sont frappés en rythme',
  'Sustained pitched vocalizations form melodic sequences': 'Des vocalisations forment des séquences mélodiques',
  'A hollow bone with finger holes produces musical tones': 'Un os creux avec des trous produit des sons musicaux',
  'A taut cord vibrates to produce musical notes': 'Une corde tendue vibre pour produire des notes musicales',
  'Narrative accounts passed between individuals by spoken word': 'Des récits transmis oralement entre individus',
  'Long rhythmic verse recounts heroic deeds and origins': 'De longs vers relatent des exploits héroïques',
  'Narrative accounts preserved in written symbols': 'Des récits préservés dans des symboles écrits',
  'A consistent greeting gesture develops': 'Un geste de salutation cohérent se développe',
  'Communal mourning practices emerge for the dead': 'Des pratiques de deuil communautaires émergent',
  'Food is shared equally among group members': 'La nourriture est partagée équitablement',
  'Gifts and favors are expected to be returned': 'Les cadeaux et les faveurs sont rendus',
  'Different tasks become associated with different sexes': 'Différentes tâches sont associées à différents sexes',
  'Elders are accorded special respect': 'Les anciens bénéficient d\'un respect particulier',
  'Ceremonial gift-giving strengthens social bonds': 'Les dons cérémoniaux renforcent les liens sociaux',
  'Pigments and natural materials used for body adornment': 'Des pigments utilisés pour la parure corporelle',
  'Oral narratives preserve group memory and values': 'Les récits oraux préservent la mémoire du groupe',
  'Rhythmic percussion emerges as social bonding activity': 'La percussion rythmique comme activité de lien social',
  'Coordinated movement used in group ceremonies': 'Des mouvements coordonnés dans les cérémonies de groupe',
  'Birth is marked with naming rites': 'La naissance est marquée par des rites de dénomination',
  'Pair-bonding is formalized through ritual': 'Le lien de couple est formalisé par un rituel',
  'Cyclical celebrations mark the seasons': 'Des célébrations cycliques marquent les saisons',
  'Certain behaviors become culturally forbidden': 'Certains comportements deviennent interdits',
  'Exchange is ritualized to build trust': 'L\'échange est ritualisé pour établir la confiance',
  'Origin stories are recorded in written form': 'Les récits d\'origine sont consignés par écrit',
  'Rules and punishments are written and formalized': 'Les règles et punitions sont formalisées',
  'Members are expected to return favors': 'Les membres sont censés rendre les faveurs',
  "Taking others' possessions is prohibited": 'S\'emparer des biens d\'autrui est interdit',
  'Mating between close relatives is forbidden': 'L\'accouplement entre proches parents est interdit',
  'Elders are addressed with deference': 'Les anciens sont traités avec déférence',
  'Strangers must be offered food and shelter': 'Les étrangers doivent se voir offrir nourriture et abri',
  'Violence against a kin member demands revenge': 'La violence contre un membre de la famille exige vengeance',
  'All able members must contribute to group tasks': 'Tous les membres capables doivent contribuer',
  'The leader resolves disputes': 'Le chef résout les différends',
  'Individual ownership of goods is recognized': 'La propriété individuelle des biens est reconnue',
  'Persistent violators may be driven out': 'Les contrevenants persistants peuvent être chassés',
  'Rules are codified in written form': 'Les règles sont codifiées par écrit',
  'Members contribute a portion of resources to the group': 'Les membres contribuent des ressources au groupe',
  'Agreements between parties are legally binding': 'Les accords entre parties sont contraignants',
  'The moon completes another cycle of phases': 'La lune complète un autre cycle de phases',
  'The sun reaches its extreme position': 'Le soleil atteint sa position extrême',
  'Day and night are of equal length': 'Le jour et la nuit sont de longueur égale',
  'A prominent star rises at sunset': 'Une étoile proéminente se lève au coucher du soleil',
  'The sun is obscured — a solar eclipse': 'Le soleil est obscurci — éclipse solaire',
  'The moon turns blood red — a lunar eclipse': 'La lune devient rouge sang — éclipse lunaire',
  'A wandering star moves against the fixed stars': 'Une étoile errante se déplace à contre-courant',
  'A bright object with a tail crosses the sky': 'Un objet brillant avec une queue traverse le ciel',
  'The phases of the moon can be predicted': 'Les phases de la lune peuvent être prédites',
  'A calendar based on sun and moon positions is developed': 'Un calendrier basé sur les positions du soleil est développé',
  'Named star constellations guide navigation': 'Des constellations d\'étoiles guident la navigation',
  'Solar and lunar eclipses can be predicted': 'Les éclipses peuvent être prédites',
  'A model explains the motion of wandering stars': 'Un modèle explique le mouvement des étoiles errantes',
};

const EXACT_DESC_AR: Record<string, string> = {
  'Pigments applied to rock surfaces depict animals and figures': 'الأصباغ على أسطح الصخور تصور الحيوانات والأشكال',
  'Three-dimensional forms carved from stone or bone': 'أشكال ثلاثية الأبعاد منحوتة من الحجر أو العظم',
  'Geometric and figurative patterns adorn ceramic surfaces': 'الأنماط الهندسية تزين الأسطح الخزفية',
  'Woven cloth bears complex repeating patterns': 'القماش المنسوج يحمل أنماطاً متكررة معقدة',
  'Buildings are decorated with carved reliefs and motifs': 'المباني مزينة بالنقوش البارزة والزخارف',
  'Stones and bones struck together in rhythmic patterns': 'الحجارة والعظام تُضرب معاً بأنماط إيقاعية',
  'Sustained pitched vocalizations form melodic sequences': 'الأصوات الصوتية المستدامة تشكل تسلسلات لحنية',
  'A hollow bone with finger holes produces musical tones': 'عظمة مجوفة بثقوب تنتج نغمات موسيقية',
  'A taut cord vibrates to produce musical notes': 'وتر مشدود يتذبذب لإنتاج نغمات',
  'Narrative accounts passed between individuals by spoken word': 'روايات تتناقل بين الأفراد شفهياً',
  'Long rhythmic verse recounts heroic deeds and origins': 'أشعار إيقاعية طويلة تروي الأعمال البطولية',
  'Narrative accounts preserved in written symbols': 'روايات محفوظة في رموز مكتوبة',
  'A consistent greeting gesture develops': 'تطورت إيماءة تحية متسقة',
  'Communal mourning practices emerge for the dead': 'ظهرت ممارسات الحداد الجماعي على الموتى',
  'Food is shared equally among group members': 'يتم توزيع الطعام بالتساوي',
  'Gifts and favors are expected to be returned': 'يُتوقع رد الهدايا والمعروف بالمثل',
  'Different tasks become associated with different sexes': 'تصبح المهام مرتبطة بالجنسين المختلفين',
  'Elders are accorded special respect': 'يُحظى المسنون باحترام خاص',
  'Ceremonial gift-giving strengthens social bonds': 'تبادل الهدايا يعزز الروابط الاجتماعية',
  'Pigments and natural materials used for body adornment': 'الأصباغ والمواد الطبيعية لزينة الجسد',
  'Oral narratives preserve group memory and values': 'الروايات الشفهية تحفظ ذاكرة المجموعة وقيمها',
  'Rhythmic percussion emerges as social bonding activity': 'الإيقاع يظهر كنشاط للترابط الاجتماعي',
  'Coordinated movement used in group ceremonies': 'الحركة المنسقة في الاحتفالات الجماعية',
  'Birth is marked with naming rites': 'تُحيَّا الولادة بطقوس التسمية',
  'Pair-bonding is formalized through ritual': 'يتم تنظيم الارتباط رسمياً من خلال الطقوس',
  'Cyclical celebrations mark the seasons': 'الاحتفالات الدورية تعلم الفصول',
  'Certain behaviors become culturally forbidden': 'تصبح سلوكيات معينة محظورة ثقافياً',
  'Exchange is ritualized to build trust': 'يتم تحويل التبادل إلى طقوس لبناء الثقة',
  'Origin stories are recorded in written form': 'قصص الأصل يتم تسجيلها كتابياً',
  'Rules and punishments are written and formalized': 'القواعد والعقوبات تُكتب وتُرسَّم رسمياً',
  'Members are expected to return favors': 'يُتوقع من الأعضاء رد المعروف بالمثل',
  "Taking others' possessions is prohibited": 'أخذ ممتلكات الآخرين محظور',
  'Mating between close relatives is forbidden': 'التزاوج بين الأقارب المقربين محظور',
  'Elders are addressed with deference': 'يُخاطب المسنون بتبجيل',
  'Strangers must be offered food and shelter': 'يجب تقديم الطعام والمأوى للغرباء',
  'Violence against a kin member demands revenge': 'العنف ضد أحد أفراد العائلة يستوجب الانتقام',
  'All able members must contribute to group tasks': 'جميع الأعضاء القادرين يجب أن يساهموا',
  'The leader resolves disputes': 'القائد يحل النزاعات',
  'Individual ownership of goods is recognized': 'تُعترف بالملكية الفردية للبضائع',
  'Persistent violators may be driven out': 'قد يُطرد المخالفون المستمرون',
  'Rules are codified in written form': 'القواعد مُقنَّنة كتابياً',
  'Members contribute a portion of resources to the group': 'الأعضاء يساهمون بجزء من الموارد',
  'Agreements between parties are legally binding': 'الاتفاقيات بين الأطراف ملزمة قانونياً',
  'The moon completes another cycle of phases': 'القمر يكمل دورة أخرى من مراحله',
  'The sun reaches its extreme position': 'تصل الشمس إلى موضعها الأقصى',
  'Day and night are of equal length': 'النهار والليل متساويان في الطول',
  'A prominent star rises at sunset': 'نجم بارز يشرق عند غروب الشمس',
  'The sun is obscured — a solar eclipse': 'الشمس تُحجب — كسوف الشمس',
  'The moon turns blood red — a lunar eclipse': 'القمر يتحول إلى الأحمر — خسوف القمر',
  'A wandering star moves against the fixed stars': 'نجم سيار يتحرك عكس النجوم الثابتة',
  'A bright object with a tail crosses the sky': 'جسم مضيء بذيل يعبر السماء',
  'The phases of the moon can be predicted': 'يمكن التنبؤ بمراحل القمر',
  'A calendar based on sun and moon positions is developed': 'تم تطوير تقويم يستند إلى مواضع الشمس والقمر',
  'Named star constellations guide navigation': 'كوكبات النجوم المسماة تُرشد الملاحة',
  'Solar and lunar eclipses can be predicted': 'يمكن التنبؤ بكسوف الشمس وخسوف القمر',
  'A model explains the motion of wandering stars': 'نموذج يفسر حركة النجوم السيارة',
};

function replaceByMap(source: string, map: Record<string, string>) {
  let out = source;
  for (const [needle, replacement] of Object.entries(map)) {
    out = out.split(needle).join(replacement);
  }
  return out;
}


// Stage name translations
const STAGE_NAME_TR: Record<string, string> = {
  'pre-linguistic':   'dil öncesi',
  'gestural':         'jest aşaması',
  'emotional-sounds': 'duygusal sesler',
  'proto-words':      'proto kelimeler',
  'syntax':           'sözdizimi',
  'abstract':         'soyut dil',
  'writing':          'yazı sistemi',
};
const STAGE_NAME_DE: Record<string, string> = {
  'pre-linguistic':   'Vorsprachlich', 'gestural': 'Gestural', 'emotional-sounds': 'Emotionale Laute',
  'proto-words': 'Protowörter', 'syntax': 'Syntax', 'abstract': 'Abstrakt', 'writing': 'Schrift',
};
const STAGE_NAME_FR: Record<string, string> = {
  'pre-linguistic':   'Prélinguistique', 'gestural': 'Gestuel', 'emotional-sounds': 'Sons émotionnels',
  'proto-words': 'Proto-mots', 'syntax': 'Syntaxe', 'abstract': 'Abstrait', 'writing': 'Écriture',
};
const STAGE_NAME_AR: Record<string, string> = {
  'pre-linguistic': 'ما قبل اللغة', 'gestural': 'إيمائي', 'emotional-sounds': 'أصوات عاطفية',
  'proto-words': 'كلمات أولى', 'syntax': 'نحو', 'abstract': 'مجرد', 'writing': 'كتابة',
};

export function translateStageName(stageName: string | null | undefined, lang: LangCode): string {
  if (!stageName) return text(lang, { tr: 'dil öncesi', en: 'pre-linguistic', de: 'Vorsprachlich', fr: 'Prélinguistique', ar: 'ما قبل اللغة' });
  if (lang === 'tr') return STAGE_NAME_TR[stageName] ?? stageName;
  if (lang === 'de') return STAGE_NAME_DE[stageName] ?? stageName;
  if (lang === 'fr') return STAGE_NAME_FR[stageName] ?? stageName;
  if (lang === 'ar') return STAGE_NAME_AR[stageName] ?? stageName;
  return stageName;
}

// Communication concept translations for event descriptions
const CONCEPT_TR_CLIENT: Record<string, string> = {
  danger: 'tehlike', food: 'yiyecek', water: 'su', fire: 'ateş',
  here: 'burası', there: 'orası', me: 'ben', you: 'sen', us: 'biz', them: 'onlar',
  good: 'iyi', bad: 'kötü', hunt: 'avlan', eat: 'ye', sleep: 'uyu',
  death: 'ölüm', birth: 'doğum', run: 'koş', sun: 'güneş', moon: 'ay',
  rain: 'yağmur', dark: 'karanlık', light: 'ışık', god: 'tanrı', spirit: 'ruh',
  sky: 'gökyüzü', earth: 'dünya', time: 'zaman',
};
const MOOD_TR: Record<string, string> = {
  calm: 'sakin', excited: 'heyecanlı', grieving: 'yasını tutan', content: 'mutlu',
  stressed: 'stresli', alert: 'tetikte', hungry: 'aç', thirsty: 'susuz',
  curious: 'meraklı', anxious: 'endişeli', happy: 'neşeli', sad: 'üzgün', angry: 'öfkeli',
};
const ACTIVITY_TR: Record<string, string> = {
  'energetic and active': 'enerjik ve aktif',
  'searching for food': 'yiyecek arıyor',
  'searching for water': 'su arıyor',
  'preparing for birth': 'doğuma hazırlanıyor',
  'grieving': 'yas tutuyor',
  'seeking a mate': 'eş arıyor',
  'moving eastward': 'doğuya ilerliyor',
  'moving westward': 'batıya ilerliyor',
  'moving around the area': 'alanda geziniyor',
};

const CONCEPT_DE: Record<string, string> = {
  danger: 'Gefahr', food: 'Essen', water: 'Wasser', fire: 'Feuer',
  here: 'hier', there: 'dort', me: 'ich', you: 'du', us: 'wir', them: 'sie',
  good: 'gut', bad: 'schlecht', hunt: 'jage', eat: 'iss', sleep: 'schlaf',
  death: 'Tod', birth: 'Geburt', run: 'lauf', sun: 'Sonne', moon: 'Mond',
  rain: 'Regen', dark: 'Dunkelheit', light: 'Licht', god: 'Gott', spirit: 'Geist',
  sky: 'Himmel', earth: 'Erde', time: 'Zeit',
};
const CONCEPT_FR: Record<string, string> = {
  danger: 'danger', food: 'nourriture', water: 'eau', fire: 'feu',
  here: 'ici', there: 'là-bas', me: 'moi', you: 'toi', us: 'nous', them: 'eux',
  good: 'bon', bad: 'mauvais', hunt: 'chasse', eat: 'mange', sleep: 'dors',
  death: 'mort', birth: 'naissance', run: 'cours', sun: 'soleil', moon: 'lune',
  rain: 'pluie', dark: 'obscurité', light: 'lumière', god: 'dieu', spirit: 'esprit',
  sky: 'ciel', earth: 'terre', time: 'temps',
};
const CONCEPT_AR: Record<string, string> = {
  danger: 'خطر', food: 'طعام', water: 'ماء', fire: 'نار',
  here: 'هنا', there: 'هناك', me: 'أنا', you: 'أنت', us: 'نحن', them: 'هم',
  good: 'جيد', bad: 'سيء', hunt: 'اصطد', eat: 'كل', sleep: 'نم',
  death: 'موت', birth: 'ولادة', run: 'اركض', sun: 'شمس', moon: 'قمر',
  rain: 'مطر', dark: 'ظلام', light: 'ضوء', god: 'إله', spirit: 'روح',
  sky: 'سماء', earth: 'أرض', time: 'وقت',
};
const MOOD_DE: Record<string, string> = {
  calm: 'ruhig', excited: 'aufgeregt', grieving: 'trauernd', content: 'zufrieden',
  stressed: 'gestresst', alert: 'wachsam', hungry: 'hungrig', thirsty: 'durstig',
  curious: 'neugierig', anxious: 'ängstlich', happy: 'fröhlich', sad: 'traurig', angry: 'wütend',
};
const MOOD_FR: Record<string, string> = {
  calm: 'calme', excited: 'excité', grieving: 'en deuil', content: 'content',
  stressed: 'stressé', alert: 'vigilant', hungry: 'affamé', thirsty: 'assoiffé',
  curious: 'curieux', anxious: 'anxieux', happy: 'heureux', sad: 'triste', angry: 'en colère',
};
const MOOD_AR: Record<string, string> = {
  calm: 'هادئ', excited: 'متحمس', grieving: 'حزين', content: 'راضٍ',
  stressed: 'متوتر', alert: 'متيقظ', hungry: 'جائع', thirsty: 'عطشان',
  curious: 'فضولي', anxious: 'قلق', happy: 'سعيد', sad: 'حزين', angry: 'غاضب',
};
const ACTIVITY_DE: Record<string, string> = {
  'energetic and active': 'energiegeladen und aktiv',
  'searching for food': 'sucht Nahrung',
  'searching for water': 'sucht Wasser',
  'preparing for birth': 'bereitet sich auf die Geburt vor',
  'grieving': 'trauert',
  'seeking a mate': 'sucht einen Partner',
  'moving eastward': 'bewegt sich ostwärts',
  'moving westward': 'bewegt sich westwärts',
  'moving around the area': 'streift durch das Gebiet',
};
const ACTIVITY_FR: Record<string, string> = {
  'energetic and active': 'énergique et actif',
  'searching for food': 'cherche de la nourriture',
  'searching for water': "cherche de l'eau",
  'preparing for birth': "se prépare à l'accouchement",
  'grieving': 'fait son deuil',
  'seeking a mate': 'cherche un partenaire',
  'moving eastward': "se déplace vers l'est",
  'moving westward': "se déplace vers l'ouest",
  'moving around the area': 'se déplace dans la zone',
};
const ACTIVITY_AR: Record<string, string> = {
  'energetic and active': 'نشيط وحيوي',
  'searching for food': 'يبحث عن الطعام',
  'searching for water': 'يبحث عن الماء',
  'preparing for birth': 'يستعد للولادة',
  'grieving': 'ينعى',
  'seeking a mate': 'يبحث عن شريك',
  'moving eastward': 'يتحرك شرقاً',
  'moving westward': 'يتحرك غرباً',
  'moving around the area': 'يتجول في المنطقة',
};

// Keyed by milestones.rs's stable `key` (e.g. "pop_10"), not by description
// text -- the milestone event's raw English sentence is carried in `data.key`
// (see to_client_event in routes.rs), which is more robust to reword than an
// exact-string match.
const MILESTONE_TR: Record<string, string> = {
  pop_10: 'Nüfus 10 bireye ulaştı', pop_25: 'Nüfus 25 bireye ulaştı', pop_50: 'Nüfus 50 bireye ulaştı',
  pop_100: 'Nüfus kilometre taşı: 100 birey', pop_250: 'Nüfus kilometre taşı: 250 birey', pop_500: 'Nüfus kilometre taşı: 500 birey',
  tech_5: '5 teknoloji keşfedildi', tech_10: '10 teknoloji keşfedildi', tech_15: '15 teknoloji keşfedildi',
  belief_first: 'İlk inanç sistemi ortaya çıktı', belief_5: '5 inanç sistemi kaydedildi',
  art_first: 'İlk sanat eseri yaratıldı',
  lang_stage2: 'İlk fonemik dil aşamasına ulaşıldı', lang_stage3: 'Toplulukta biçimbilimsel dilbilgisi ortaya çıktı',
  lang_stage4: 'Karmaşık sözdizimi başarıldı — tam dil kapasitesi', lang_stage5: 'Yazı sistemi icat edildi', lang_stage6: 'Edebiyat çağı başlıyor',
  year_10: 'Medeniyet 10 yıl hayatta kaldı', year_100: 'Medeniyet 100 yıl hayatta kaldı',
  year_500: 'Medeniyet 500 yıl hayatta kaldı', year_1000: 'Medeniyet 1000 yıl hayatta kaldı',
};
const MILESTONE_DE: Record<string, string> = {
  pop_10: 'Bevölkerung erreichte 10 Individuen', pop_25: 'Bevölkerung erreichte 25 Individuen', pop_50: 'Bevölkerung erreichte 50 Individuen',
  pop_100: 'Bevölkerungsmeilenstein: 100 Individuen', pop_250: 'Bevölkerungsmeilenstein: 250 Individuen', pop_500: 'Bevölkerungsmeilenstein: 500 Individuen',
  tech_5: '5 Technologien entdeckt', tech_10: '10 Technologien entdeckt', tech_15: '15 Technologien entdeckt',
  belief_first: 'Erstes Glaubenssystem entstanden', belief_5: '5 Glaubenssysteme verzeichnet',
  art_first: 'Erste Kunstform geschaffen',
  lang_stage2: 'Erste phonemische Sprachstufe erreicht', lang_stage3: 'Morphemische Grammatik in der Gemeinschaft entstanden',
  lang_stage4: 'Komplexe Syntax erreicht — volle Sprachfähigkeit', lang_stage5: 'Schriftsystem erfunden', lang_stage6: 'Literaturzeitalter beginnt',
  year_10: 'Zivilisation überlebte 10 Jahre', year_100: 'Zivilisation überlebte 100 Jahre',
  year_500: 'Zivilisation überlebte 500 Jahre', year_1000: 'Zivilisation überlebte 1000 Jahre',
};
const MILESTONE_FR: Record<string, string> = {
  pop_10: 'La population a atteint 10 individus', pop_25: 'La population a atteint 25 individus', pop_50: 'La population a atteint 50 individus',
  pop_100: 'Étape démographique : 100 individus', pop_250: 'Étape démographique : 250 individus', pop_500: 'Étape démographique : 500 individus',
  tech_5: '5 technologies découvertes', tech_10: '10 technologies découvertes', tech_15: '15 technologies découvertes',
  belief_first: 'Premier système de croyance apparu', belief_5: '5 systèmes de croyance enregistrés',
  art_first: 'Première forme d’art créée',
  lang_stage2: 'Premier stade phonémique du langage atteint', lang_stage3: 'Grammaire morphémique apparue dans la communauté',
  lang_stage4: 'Syntaxe complexe atteinte — capacité linguistique complète', lang_stage5: 'Système d’écriture inventé', lang_stage6: 'L’ère de la littérature commence',
  year_10: 'La civilisation a survécu 10 ans', year_100: 'La civilisation a survécu 100 ans',
  year_500: 'La civilisation a survécu 500 ans', year_1000: 'La civilisation a survécu 1000 ans',
};
const MILESTONE_AR: Record<string, string> = {
  pop_10: 'بلغ عدد السكان 10 أفراد', pop_25: 'بلغ عدد السكان 25 فرداً', pop_50: 'بلغ عدد السكان 50 فرداً',
  pop_100: 'إنجاز سكاني: 100 فرد', pop_250: 'إنجاز سكاني: 250 فرداً', pop_500: 'إنجاز سكاني: 500 فرد',
  tech_5: 'اكتُشفت 5 تقنيات', tech_10: 'اكتُشفت 10 تقنيات', tech_15: 'اكتُشفت 15 تقنية',
  belief_first: 'ظهر أول نظام معتقدات', belief_5: 'سُجلت 5 أنظمة معتقدات',
  art_first: 'أُنشئ أول شكل فني',
  lang_stage2: 'تم الوصول إلى أول مرحلة صوتية للغة', lang_stage3: 'ظهرت قواعد صرفية في المجتمع',
  lang_stage4: 'تم بلوغ تركيب نحوي معقد — قدرة لغوية كاملة', lang_stage5: 'تم اختراع نظام الكتابة', lang_stage6: 'بدأ عصر الأدب',
  year_10: 'نجت الحضارة 10 سنوات', year_100: 'نجت الحضارة 100 سنة',
  year_500: 'نجت الحضارة 500 سنة', year_1000: 'نجت الحضارة 1000 سنة',
};
const MILESTONE_DESC_BY_LANG: Record<Exclude<LangCode, 'en'>, Record<string, string>> = {
  tr: MILESTONE_TR, de: MILESTONE_DE, fr: MILESTONE_FR, ar: MILESTONE_AR,
};

// The server (individual_display_name in routes.rs) falls back to the
// literal English word "Unnamed" for any individual without a phenotype
// name, baked directly into event descriptions like "Born: Unnamed (Damla &
// Unnamed)" before this file ever sees them -- the server has no notion of
// the client's selected language, matching every other raw phrasing here
// (cause/tech/belief/norm text) being in English for this function to
// translate. Every other word in a translated sentence gets localized;
// "Unnamed" was the one literal English token that never did. Applied once,
// after language-specific handling below, so every return path is covered
// without threading it through each one individually.
export const UNNAMED_LABEL: Record<LangCode, string> = { en: 'Unnamed', tr: 'İsimsiz', de: 'Unbenannt', fr: 'Sans nom', ar: 'بدون اسم' };

export function translateEventDescription(desc: string, lang: LangCode, event?: any): string {
  return translateEventDescriptionImpl(desc, lang, event).replace(/\bUnnamed\b/g, UNNAMED_LABEL[lang]);
}

function translateEventDescriptionImpl(desc: string, lang: LangCode, event?: any): string {
  if (!desc) return '';

  const milestoneKey = event?.event_type === 'milestone' ? (event?.data?.key ?? event?.key) : undefined;
  if (milestoneKey && lang !== 'en') {
    const translated = MILESTONE_DESC_BY_LANG[lang]?.[milestoneKey];
    if (translated) return translated;
  }
  if (lang === 'en') return desc;

  if (lang === 'tr') {
  const EXACT: Record<string, string> = {
    ...ART_DESC_TR,
    ...CULTURE_DESC_TR,
    ...LAW_DESC_TR,
    ...ASTRO_DESC_TR,
  };
  if (EXACT[desc]) return EXACT[desc];

  return desc
    .replace(/^(.+) died: (.+)$/, (_: string, person: string, cause: string) =>
      `${person} öldü: ${CAUSE_LABELS[cause]?.tr ?? cause.replace(/_/g, ' ')}`)
    .replace(/^Born: (.+) \((.+) & (.+)\)$/, (_: string, bornName: string, p1: string, p2: string) =>
      `Doğdu: ${bornName} (${p1} & ${p2})`)
    .replace(/^Born: (.+)$/, (_: string, bornName: string) => `Doğdu: ${bornName}`)
    .replace('New individual born', 'Yeni birey doğdu')
    .replace('Individual died: starvation',  'Birey açlıktan öldü')
    .replace('Individual died: dehydration', 'Birey susuzluktan öldü')
    .replace('Individual died: old_age',     'Birey yaşlılıktan öldü')
    .replace('Individual died: predator',    'Birey yırtıcı tarafından öldürüldü')
    .replace(/Individual died: (.+)/, (_: string, cause: string) =>
      `Birey öldü: ${CAUSE_LABELS[cause]?.tr ?? cause.replace(/_/g, ' ')}`)
    .replace(/Technology discovered: fire making/i,   'Teknoloji keşfedildi: Ateş Yakma')
    .replace(/Technology discovered: water container/i,'Teknoloji keşfedildi: Su Kabı')
    .replace(/Technology discovered: fishing/i,        'Teknoloji keşfedildi: Balıkçılık')
    .replace(/Technology discovered: foraging/i,       'Teknoloji keşfedildi: Toplayıcılık')
    .replace(/Technology discovered: stone_tools/i,    'Teknoloji keşfedildi: Taş Aletler')
    .replace(/Technology discovered: (.+)/i, (_: string, tech: string) => {
      const key = tech.trim();
      return `Teknoloji keşfedildi: ${TECH_TR[key] ?? TECH_TR[key.replace(/_/g, ' ')] ?? key.replace(/_/g, ' ')}`;
    })
    .replace(/^A (.+) outbreak begins$/, (_: string, pathogen: string) => {
      const key = pathogen.replace(/_/g, ' ').trim();
      return `${PATHOGEN_TR[key] ?? key} salgını başladı`;
    })
    .replace(/^(.+) completes a (.+)$/, (_: string, settlement: string, structure: string) => {
      const sKey = structure.replace(/_/g, ' ').trim();
      const settlementTr = settlement === 'The settlement' ? 'Yerleşim' : settlement;
      return `${settlementTr}, ${STRUCTURE_TR[sKey] ?? sKey} inşaatını tamamladı`;
    })
    .replace(/^(.+) completed construction of (.+)$/, (_: string, settlement: string, structure: string) => {
      const sKey = structure.replace(/-/g, ' ').replace(/_/g, ' ').trim();
      const settlementTr = settlement === 'Settlement' || settlement === 'The settlement' ? 'Yerleşim' : settlement;
      return `${settlementTr}, ${STRUCTURE_TR[sKey] ?? sKey} inşaatını tamamladı`;
    })
    .replace(/^(.+) is overcrowded \((\d+) of (\d+) capacity\)$/, (_: string, settlement: string, cur: string, cap: string) => {
      const settlementTr = settlement === 'The settlement' ? 'Yerleşim' : settlement;
      return `${settlementTr} doldu taştı — ${cur} birey, kapasite: ${cap}`;
    })
    .replace(/^(.+) killed (\d+) individuals?$/, (_: string, type: string, count: string) => {
      const typeKey = type.toLowerCase().trim();
      return `${DISASTER_TR[typeKey] ?? type}, ${count} bireyi öldürdü`;
    })
    .replace(/killed (\d+) individuals?/, (_: string, count: string) => `${count} bireyi öldürdü`)
    .replace(/^A ritual \(belief #(\d+)\) emerges in the group$/, (_: string, code: string) => `Grupta #${code} numaralı inanç ritüeli ortaya çıktı`)
    .replace('A ritual emerges in the group', 'Grupta bir ritüel ortaya çıktı')
    .replace(/^A (.+) ritual emerges in the group$/, (_: string, label: string) => `Grupta ${label} ritüeli ortaya çıktı`)
    .replace(/^(.+) gave rise to belief #(\d+)$/, (_: string, name: string, code: string) => `${name}, #${code} numaralı inanca öncülük etti`)
    .replace(/^(.+) gave rise to a new belief$/, (_: string, name: string) => `${name} yeni bir inanç başlattı`)
    .replace(/^(.+) gave rise to (.+)$/, (_: string, name: string, label: string) => `${name}, ${label} inancını başlattı`)
    .replace(/^A new belief \(#(\d+)\) takes hold$/, (_: string, code: string) => `Yeni bir inanç (#${code}) filizleniyor`)
    .replace(/^A new belief, (.+), takes hold$/, (_: string, label: string) => `Yeni bir inanç, ${label}, filizleniyor`)
    .replace('A new belief takes hold', 'Yeni bir inanç filizleniyor')
    .replace(/^Their belief becomes known as (.+)$/, (_: string, label: string) => `İnançları artık "${label}" olarak biliniyor`)
    .replace(/^The group becomes known as (.+)$/, (_: string, name: string) => `Grup artık "${name}" olarak biliniyor`)
    .replace(/^Their civilization becomes known as (.+)$/, (_: string, name: string) => `Medeniyetleri artık "${name}" olarak biliniyor`)
    .replace(/^Culture event: (.+)$/,      (_: string, v: string) => `Kültür olayı: ${v.replace(/_/g, ' ')}`)
    .replace(/^Art event: (.+)$/,          (_: string, v: string) => `Sanat olayı: ${v.replace(/_/g, ' ')}`)
    .replace(/^Astronomy event: (.+)$/,    (_: string, v: string) => `Astronomi olayı: ${v.replace(/_/g, ' ')}`)
    .replace(/^Architecture event: (.+)$/, (_: string, v: string) => `Mimari olay: ${v.replace(/_/g, ' ')}`)
    .replace(/^Law event: (.+)$/,          (_: string, v: string) => `Hukuk olayı: ${v.replace(/_/g, ' ')}`)
    .replace(/^Microbiome event: (.+)$/,   (_: string, v: string) => `Mikrobiyom olayı: ${v.replace(/_/g, ' ')}`)
    .replace(/^Epigenetics event: (.+)$/,  (_: string, v: string) => `Epigenetik olay: ${v.replace(/_/g, ' ')}`)
    .replace(/^(.+) jest ile (.+)'a iletişim kurdu$/, (_: string, a: string, b: string) => `${a}, ${b}'a jest yaptı`)
    .replace(/^Norm emerged: (.+)$/, (_: string, v: string) => `Norm oluştu: ${LAW_DESC_TR[v] ?? v.replace(/_/g, ' ')}`)
    .replace(/^Norm violated: (.+)$/, (_: string, v: string) => `Norm ihlal edildi: ${LAW_DESC_TR[v] ?? v.replace(/_/g, ' ')}`)
    .replace(/^(.+) embraced belief #(\d+)$/, (_: string, name: string, code: string) => `${name}, #${code} numaralı inancı benimsedi`)
    .replace(/^(.+) embraced a belief$/, (_: string, name: string) => `${name} bir inanca bağlandı`)
    .replace(/^(.+) embraced (.+)$/, (_: string, name: string, label: string) => `${name}, ${label} inancını benimsedi`)
    .replace('A group split into two bands', 'Bir grup ikiye bölündü')
    .replace(/^(.+) became the new leader$/, (_: string, name: string) => `${name} yeni lider oldu`)
    .replace(/^(.+) traded with (.+)$/, (_: string, a: string, b: string) => `${a}, ${b} ile takas yaptı`)
    .replace(/^(.+) is (searching for food|looking for water|resting)$/, (_: string, name: string, action: string) => {
      const map: Record<string, string> = { 'searching for food': 'yiyecek arıyor', 'looking for water': 'su arıyor', 'resting': 'dinleniyor' };
      return `${name} ${map[action] ?? action}`;
    })
    .replace(/^(.+) \((\d+) yrs?, (\w+)\): (.+)$/, (_: string, name: string, age: string, mood: string, activity: string) =>
      `${name} (${age} yaş, ${MOOD_TR[mood] ?? mood}): ${ACTIVITY_TR[activity] ?? activity}`
    )
    .replace(/^(.+) (said|gestured) ["'](.+)["'] to (.+) — (.+)$/, (_: string, name: string, verb: string, word: string, target: string, concept: string) => {
      const verbTr = verb === 'said' ? 'dedi' : 'jest yaptı';
      return `${name}, ${target}'a "${word}" ${verbTr} — ${CONCEPT_TR_CLIENT[concept] ?? concept}`;
    })
    .replace(/^(.+) made a sound at (.+)$/, (_: string, name: string, target: string) => `${name}, ${target}'a ses çıkardı`)
    .replace(/^(.+) pointed at (.+)$/, (_: string, name: string, target: string) => `${name}, ${target}'ı işaret etti`)
    .replace(/^(.+) was exhausted and fell asleep \(energy: (\d+)%\)$/, (_: string, name: string, pct: string) =>
      `${name} tükendi ve uyuyakaldı (enerji: ${pct}%)`
    )
    .replace(/(.+) language stage advanced to (.+)/, (_: string, person: string, stage: string) =>
      `${person} dil aşamasını ${STAGE_NAME_TR[stage] ?? stage} seviyesine ilerletti`
    );
  }

  if (lang === 'de') {
    if (EXACT_DESC_DE[desc]) return EXACT_DESC_DE[desc];
    const deathMatch = desc.match(/^(.+) died: (.+)$/);
    const birthMatch = desc.match(/^Born: (.+) \((.+) & (.+)\)$/);
    const techMatch = desc.match(/^Technology discovered: (.+)$/i);
    const diseaseMatch = desc.match(/^A (.+) outbreak begins$/);
    const disasterMatch = desc.match(/^(.+) killed (\d+) individuals?$/);
    const settlementMatch = desc.match(/^(.+) completes a (.+)$/);
    const ritualMatch = desc.match(/^A (.+) ritual emerges in the group$/);
    const langStageMatch = desc.match(/^(.+) language stage advanced to (.+)$/);
    if (deathMatch) {
      const [, person, cause] = deathMatch;
      return `${person} starb: ${CAUSE_DE[cause] ?? cause.replace(/_/g, ' ')}`;
    }
    if (birthMatch) {
      const [, bornName, p1, p2] = birthMatch;
      return `Geboren: ${bornName} (${p1} & ${p2})`;
    }
    if (techMatch) return `Technologie entdeckt: ${techMatch[1]}`;
    if (diseaseMatch) return `Ein ${diseaseMatch[1]}-Ausbruch beginnt`;
    if (disasterMatch) {
      const [, name, count] = disasterMatch;
      return `${DISASTER_DE[name.toLowerCase().trim()] ?? name} tötete ${count} Personen`;
    }
    if (settlementMatch) {
      const [, settlement, structure] = settlementMatch;
      return `${settlement} baute ein ${structure}`;
    }
    const settlementMatch2 = desc.match(/^(.+) completed construction of (.+)$/);
    if (settlementMatch2) {
      const [, settlement, structure] = settlementMatch2;
      return `${settlement} vollendete den Bau von ${structure}`;
    }
    const ritualCodeMatchDe = desc.match(/^A ritual \(belief #(\d+)\) emerges in the group$/);
    if (ritualCodeMatchDe) return `Ein Ritual (Glaube #${ritualCodeMatchDe[1]}) entstand in der Gruppe`;
    if (desc === 'A ritual emerges in the group') return 'Ein Ritual entstand in der Gruppe';
    if (ritualMatch) return `Ein ${ritualMatch[1]}-Ritual entstand in der Gruppe`;
    const roseCodeMatchDe = desc.match(/^(.+) gave rise to belief #(\d+)$/);
    if (roseCodeMatchDe) return `${roseCodeMatchDe[1]} begründete den Glauben #${roseCodeMatchDe[2]}`;
    if (desc === 'A new belief takes hold') return 'Ein neuer Glaube entsteht';
    const roseNeutralDe = desc.match(/^(.+) gave rise to a new belief$/);
    if (roseNeutralDe) return `${roseNeutralDe[1]} begründete einen neuen Glauben`;
    const roseMatchDe = desc.match(/^(.+) gave rise to (.+)$/);
    if (roseMatchDe) return `${roseMatchDe[1]} begründete den Glauben ${roseMatchDe[2]}`;
    const newBeliefCodeMatchDe = desc.match(/^A new belief \(#(\d+)\) takes hold$/);
    if (newBeliefCodeMatchDe) return `Ein neuer Glaube (#${newBeliefCodeMatchDe[1]}) entsteht`;
    const newBeliefLabelDe = desc.match(/^A new belief, (.+), takes hold$/);
    if (newBeliefLabelDe) return `Ein neuer Glaube, ${newBeliefLabelDe[1]}, entsteht`;
    const namedMatchDe = desc.match(/^Their belief becomes known as (.+)$/);
    if (namedMatchDe) return `Ihr Glaube wird nun "${namedMatchDe[1]}" genannt`;
    const groupNamedDe = desc.match(/^The group becomes known as (.+)$/);
    if (groupNamedDe) return `Die Gruppe wird nun "${groupNamedDe[1]}" genannt`;
    const civNamedDe = desc.match(/^Their civilization becomes known as (.+)$/);
    if (civNamedDe) return `Ihre Zivilisation wird nun "${civNamedDe[1]}" genannt`;
    if (langStageMatch) {
      const [, person, stage] = langStageMatch;
      return `${person} hat die Sprachstufe auf ${STAGE_NAME_DE[stage] ?? stage} vorgerückt`;
    }
    const overcrowdedMatchDe = desc.match(/^(.+) is overcrowded \((\d+) of (\d+) capacity\)$/);
    if (overcrowdedMatchDe) {
      const [, settlement, cur, cap] = overcrowdedMatchDe;
      const settlementDe = settlement === 'The settlement' ? 'Die Siedlung' : settlement;
      return `${settlementDe} ist überfüllt — ${cur} Individuen, Kapazität: ${cap}`;
    }
    const normEmergedDe = desc.match(/^Norm emerged: (.+)$/);
    if (normEmergedDe) return `Norm entstanden: ${LAW_DESC_DE[normEmergedDe[1]] ?? normEmergedDe[1].replace(/_/g, ' ')}`;
    const normViolatedDe = desc.match(/^Norm violated: (.+)$/);
    if (normViolatedDe) return `Norm verletzt: ${LAW_DESC_DE[normViolatedDe[1]] ?? normViolatedDe[1].replace(/_/g, ' ')}`;
    const embracedCodeMatchDe = desc.match(/^(.+) embraced belief #(\d+)$/);
    if (embracedCodeMatchDe) return `${embracedCodeMatchDe[1]} nahm den Glauben #${embracedCodeMatchDe[2]} an`;
    const embracedNeutralDe = desc.match(/^(.+) embraced a belief$/);
    if (embracedNeutralDe) return `${embracedNeutralDe[1]} nahm einen Glauben an`;
    const embracedMatchDe = desc.match(/^(.+) embraced (.+)$/);
    if (embracedMatchDe) {
      const [, name, label] = embracedMatchDe;
      return `${name} nahm den Glauben ${label} an`;
    }
    if (desc === 'A group split into two bands') return 'Eine Gruppe teilte sich in zwei Banden';
    const newLeaderMatchDe = desc.match(/^(.+) became the new leader$/);
    if (newLeaderMatchDe) return `${newLeaderMatchDe[1]} wurde der neue Anführer`;
    const tradedMatchDe = desc.match(/^(.+) traded with (.+)$/);
    if (tradedMatchDe) return `${tradedMatchDe[1]} handelte mit ${tradedMatchDe[2]}`;
    const soundMatchDe = desc.match(/^(.+) made a sound at (.+)$/);
    if (soundMatchDe) return `${soundMatchDe[1]} machte ein Geräusch zu ${soundMatchDe[2]}`;
    const pointedMatchDe = desc.match(/^(.+) pointed at (.+)$/);
    if (pointedMatchDe) return `${pointedMatchDe[1]} zeigte auf ${pointedMatchDe[2]}`;
    const exhaustedMatchDe = desc.match(/^(.+) was exhausted and fell asleep \(energy: (\d+)%\)$/);
    if (exhaustedMatchDe) return `${exhaustedMatchDe[1]} war erschöpft und schlief ein (Energie: ${exhaustedMatchDe[2]}%)`;
    const activityMatchDe = desc.match(/^(.+) is (searching for food|looking for water|resting)$/);
    if (activityMatchDe) {
      const map: Record<string, string> = { 'searching for food': 'sucht Nahrung', 'looking for water': 'sucht Wasser', 'resting': 'ruht sich aus' };
      return `${activityMatchDe[1]} ${map[activityMatchDe[2]] ?? activityMatchDe[2]}`;
    }
    const saidMatchDe = desc.match(/^(.+) (said|gestured) ["'](.+)["'] to (.+) — (.+)$/);
    if (saidMatchDe) {
      const [, name, verb, word, target, concept] = saidMatchDe;
      const verbDe = verb === 'said' ? 'sagte' : 'gestikulierte';
      return `${name} ${verbDe} "${word}" zu ${target} — ${CONCEPT_DE[concept] ?? concept}`;
    }
    const moodMatchDe = desc.match(/^(.+) \((\d+) yrs?, (\w+)\): (.+)$/);
    if (moodMatchDe) {
      const [, name, age, mood, activity] = moodMatchDe;
      return `${name} (${age} J., ${MOOD_DE[mood] ?? mood}): ${ACTIVITY_DE[activity] ?? activity}`;
    }
    return desc
      .replace('Culture event:', 'Kulturereignis:')
      .replace('Art event:', 'Kunstereignis:')
      .replace('Astronomy event:', 'Astronomieereignis:')
      .replace('Architecture event:', 'Architekturerreignis:')
      .replace('Law event:', 'Rechtsereignis:')
      .replace('Microbiome event:', 'Mikrobiomereignis:')
      .replace('Epigenetics event:', 'Epigenetikereignis:');
  }

  if (lang === 'fr') {
    if (EXACT_DESC_FR[desc]) return EXACT_DESC_FR[desc];
    const deathMatch = desc.match(/^(.+) died: (.+)$/);
    const birthMatch = desc.match(/^Born: (.+) \((.+) & (.+)\)$/);
    const techMatch = desc.match(/^Technology discovered: (.+)$/i);
    const diseaseMatch = desc.match(/^A (.+) outbreak begins$/);
    const disasterMatch = desc.match(/^(.+) killed (\d+) individuals?$/);
    const settlementMatch = desc.match(/^(.+) completes a (.+)$/);
    const ritualMatch = desc.match(/^A (.+) ritual emerges in the group$/);
    const langStageMatch = desc.match(/^(.+) language stage advanced to (.+)$/);
    if (deathMatch) {
      const [, person, cause] = deathMatch;
      return `${person} est décédé: ${CAUSE_FR[cause] ?? cause.replace(/_/g, ' ')}`;
    }
    if (birthMatch) {
      const [, bornName, p1, p2] = birthMatch;
      return `Né: ${bornName} (${p1} & ${p2})`;
    }
    if (techMatch) return `Technologie découverte: ${techMatch[1]}`;
    if (diseaseMatch) return `Une épidémie de ${diseaseMatch[1]} commence`;
    if (disasterMatch) {
      const [, name, count] = disasterMatch;
      return `${DISASTER_FR[name.toLowerCase().trim()] ?? name} a tué ${count} personnes`;
    }
    if (settlementMatch) {
      const [, settlement, structure] = settlementMatch;
      return `${settlement} a construit un ${structure}`;
    }
    const settlementMatch2Fr = desc.match(/^(.+) completed construction of (.+)$/);
    if (settlementMatch2Fr) {
      const [, settlement, structure] = settlementMatch2Fr;
      return `${settlement} a achevé la construction de ${structure}`;
    }
    const ritualCodeMatchFr = desc.match(/^A ritual \(belief #(\d+)\) emerges in the group$/);
    if (ritualCodeMatchFr) return `Un rituel (croyance #${ritualCodeMatchFr[1]}) est apparu dans le groupe`;
    if (desc === 'A ritual emerges in the group') return 'Un rituel est apparu dans le groupe';
    if (ritualMatch) return `Un rituel ${ritualMatch[1]} est apparu dans le groupe`;
    const roseCodeMatchFr = desc.match(/^(.+) gave rise to belief #(\d+)$/);
    if (roseCodeMatchFr) return `${roseCodeMatchFr[1]} a donné naissance à la croyance #${roseCodeMatchFr[2]}`;
    if (desc === 'A new belief takes hold') return 'Une nouvelle croyance prend forme';
    const roseNeutralFr = desc.match(/^(.+) gave rise to a new belief$/);
    if (roseNeutralFr) return `${roseNeutralFr[1]} a donné naissance à une nouvelle croyance`;
    const roseMatchFr = desc.match(/^(.+) gave rise to (.+)$/);
    if (roseMatchFr) return `${roseMatchFr[1]} a donné naissance à la croyance ${roseMatchFr[2]}`;
    const newBeliefCodeMatchFr = desc.match(/^A new belief \(#(\d+)\) takes hold$/);
    if (newBeliefCodeMatchFr) return `Une nouvelle croyance (#${newBeliefCodeMatchFr[1]}) prend forme`;
    const newBeliefLabelFr = desc.match(/^A new belief, (.+), takes hold$/);
    if (newBeliefLabelFr) return `Une nouvelle croyance, ${newBeliefLabelFr[1]}, prend forme`;
    const namedMatchFr = desc.match(/^Their belief becomes known as (.+)$/);
    if (namedMatchFr) return `Leur croyance est désormais appelée « ${namedMatchFr[1]} »`;
    const groupNamedFr = desc.match(/^The group becomes known as (.+)$/);
    if (groupNamedFr) return `Le groupe est désormais appelé « ${groupNamedFr[1]} »`;
    const civNamedFr = desc.match(/^Their civilization becomes known as (.+)$/);
    if (civNamedFr) return `Leur civilisation est désormais appelée « ${civNamedFr[1]} »`;
    if (langStageMatch) {
      const [, person, stage] = langStageMatch;
      return `${person} a avancé l'étape linguistique à ${STAGE_NAME_FR[stage] ?? stage}`;
    }
    const overcrowdedMatchFr = desc.match(/^(.+) is overcrowded \((\d+) of (\d+) capacity\)$/);
    if (overcrowdedMatchFr) {
      const [, settlement, cur, cap] = overcrowdedMatchFr;
      const settlementFr = settlement === 'The settlement' ? 'La colonie' : settlement;
      return `${settlementFr} est surpeuplée — ${cur} individus, capacité : ${cap}`;
    }
    const normEmergedFr = desc.match(/^Norm emerged: (.+)$/);
    if (normEmergedFr) return `Norme apparue : ${LAW_DESC_FR[normEmergedFr[1]] ?? normEmergedFr[1].replace(/_/g, ' ')}`;
    const normViolatedFr = desc.match(/^Norm violated: (.+)$/);
    if (normViolatedFr) return `Norme violée : ${LAW_DESC_FR[normViolatedFr[1]] ?? normViolatedFr[1].replace(/_/g, ' ')}`;
    const embracedCodeMatchFr = desc.match(/^(.+) embraced belief #(\d+)$/);
    if (embracedCodeMatchFr) return `${embracedCodeMatchFr[1]} a embrassé la croyance #${embracedCodeMatchFr[2]}`;
    const embracedNeutralFr = desc.match(/^(.+) embraced a belief$/);
    if (embracedNeutralFr) return `${embracedNeutralFr[1]} a embrassé une croyance`;
    const embracedMatchFr = desc.match(/^(.+) embraced (.+)$/);
    if (embracedMatchFr) {
      const [, name, label] = embracedMatchFr;
      return `${name} a embrassé la croyance ${label}`;
    }
    if (desc === 'A group split into two bands') return 'Un groupe s\'est divisé en deux bandes';
    const newLeaderMatchFr = desc.match(/^(.+) became the new leader$/);
    if (newLeaderMatchFr) return `${newLeaderMatchFr[1]} est devenu le nouveau chef`;
    const tradedMatchFr = desc.match(/^(.+) traded with (.+)$/);
    if (tradedMatchFr) return `${tradedMatchFr[1]} a échangé avec ${tradedMatchFr[2]}`;
    const soundMatchFr = desc.match(/^(.+) made a sound at (.+)$/);
    if (soundMatchFr) return `${soundMatchFr[1]} a fait un son en direction de ${soundMatchFr[2]}`;
    const pointedMatchFr = desc.match(/^(.+) pointed at (.+)$/);
    if (pointedMatchFr) return `${pointedMatchFr[1]} a pointé vers ${pointedMatchFr[2]}`;
    const exhaustedMatchFr = desc.match(/^(.+) was exhausted and fell asleep \(energy: (\d+)%\)$/);
    if (exhaustedMatchFr) return `${exhaustedMatchFr[1]} était épuisé et s'est endormi (énergie : ${exhaustedMatchFr[2]}%)`;
    const activityMatchFr = desc.match(/^(.+) is (searching for food|looking for water|resting)$/);
    if (activityMatchFr) {
      const map: Record<string, string> = { 'searching for food': 'cherche de la nourriture', 'looking for water': "cherche de l'eau", 'resting': 'se repose' };
      return `${activityMatchFr[1]} ${map[activityMatchFr[2]] ?? activityMatchFr[2]}`;
    }
    const saidMatchFr = desc.match(/^(.+) (said|gestured) ["'](.+)["'] to (.+) — (.+)$/);
    if (saidMatchFr) {
      const [, name, verb, word, target, concept] = saidMatchFr;
      const verbFr = verb === 'said' ? 'a dit' : 'a fait un geste vers';
      return `${name} ${verbFr} "${word}" à ${target} — ${CONCEPT_FR[concept] ?? concept}`;
    }
    const moodMatchFr = desc.match(/^(.+) \((\d+) yrs?, (\w+)\): (.+)$/);
    if (moodMatchFr) {
      const [, name, age, mood, activity] = moodMatchFr;
      return `${name} (${age} an, ${MOOD_FR[mood] ?? mood}) : ${ACTIVITY_FR[activity] ?? activity}`;
    }
    return desc
      .replace('Culture event:', 'Événement culturel:')
      .replace('Art event:', 'Événement artistique:')
      .replace('Astronomy event:', 'Événement astronomique:')
      .replace('Architecture event:', 'Événement architectural:')
      .replace('Law event:', 'Événement juridique:')
      .replace('Microbiome event:', 'Événement microbiotique:')
      .replace('Epigenetics event:', 'Événement épigénétique:');
  }

  if (lang === 'ar') {
    if (EXACT_DESC_AR[desc]) return EXACT_DESC_AR[desc];
    const deathMatch = desc.match(/^(.+) died: (.+)$/);
    const birthMatch = desc.match(/^Born: (.+) \((.+) & (.+)\)$/);
    const techMatch = desc.match(/^Technology discovered: (.+)$/i);
    const diseaseMatch = desc.match(/^A (.+) outbreak begins$/);
    const disasterMatch = desc.match(/^(.+) killed (\d+) individuals?$/);
    const settlementMatch = desc.match(/^(.+) completes a (.+)$/);
    const ritualMatch = desc.match(/^A (.+) ritual emerges in the group$/);
    const langStageMatch = desc.match(/^(.+) language stage advanced to (.+)$/);
    if (deathMatch) {
      const [, person, cause] = deathMatch;
      return `مات ${person}: ${CAUSE_AR[cause] ?? cause.replace(/_/g, ' ')}`;
    }
    if (birthMatch) {
      const [, bornName, p1, p2] = birthMatch;
      return `وُلد: ${bornName} (${p1} & ${p2})`;
    }
    if (techMatch) return `اكتُشفت تقنية: ${techMatch[1]}`;
    if (diseaseMatch) return `بدأ تفشي ${diseaseMatch[1]}`;
    if (disasterMatch) {
      const [, name, count] = disasterMatch;
      return `قتلت ${DISASTER_AR[name.toLowerCase().trim()] ?? name} ${count} أفراداً`;
    }
    if (settlementMatch) {
      const [, settlement, structure] = settlementMatch;
      return `أكمل ${settlement} بناء ${structure}`;
    }
    const settlementMatch2Ar = desc.match(/^(.+) completed construction of (.+)$/);
    if (settlementMatch2Ar) {
      const [, settlement, structure] = settlementMatch2Ar;
      return `أتمّ ${settlement} بناء ${structure}`;
    }
    const ritualCodeMatchAr = desc.match(/^A ritual \(belief #(\d+)\) emerges in the group$/);
    if (ritualCodeMatchAr) return `ظهرت طقوس (معتقد رقم ${ritualCodeMatchAr[1]}) في المجموعة`;
    if (desc === 'A ritual emerges in the group') return 'ظهرت طقوس في المجموعة';
    if (ritualMatch) return `ظهرت طقوس ${ritualMatch[1]} في المجموعة`;
    const roseCodeMatchAr = desc.match(/^(.+) gave rise to belief #(\d+)$/);
    if (roseCodeMatchAr) return `أوجد ${roseCodeMatchAr[1]} معتقد رقم ${roseCodeMatchAr[2]}`;
    if (desc === 'A new belief takes hold') return 'ينشأ معتقد جديد';
    const roseNeutralAr = desc.match(/^(.+) gave rise to a new belief$/);
    if (roseNeutralAr) return `أوجد ${roseNeutralAr[1]} معتقداً جديداً`;
    const roseMatchAr = desc.match(/^(.+) gave rise to (.+)$/);
    if (roseMatchAr) return `أوجد ${roseMatchAr[1]} معتقد ${roseMatchAr[2]}`;
    const newBeliefCodeMatchAr = desc.match(/^A new belief \(#(\d+)\) takes hold$/);
    if (newBeliefCodeMatchAr) return `ينشأ معتقد جديد (رقم ${newBeliefCodeMatchAr[1]})`;
    const newBeliefLabelAr = desc.match(/^A new belief, (.+), takes hold$/);
    if (newBeliefLabelAr) return `ينشأ معتقد جديد، ${newBeliefLabelAr[1]}`;
    const namedMatchAr = desc.match(/^Their belief becomes known as (.+)$/);
    if (namedMatchAr) return `أصبح معتقدهم يُعرف باسم "${namedMatchAr[1]}"`;
    const groupNamedAr = desc.match(/^The group becomes known as (.+)$/);
    if (groupNamedAr) return `أصبحت المجموعة تُعرف باسم "${groupNamedAr[1]}"`;
    const civNamedAr = desc.match(/^Their civilization becomes known as (.+)$/);
    if (civNamedAr) return `أصبحت حضارتهم تُعرف باسم "${civNamedAr[1]}"`;
    if (langStageMatch) {
      const [, person, stage] = langStageMatch;
      return `تقدم ${person} في مرحلة اللغة إلى ${STAGE_NAME_AR[stage] ?? stage}`;
    }
    const overcrowdedMatchAr = desc.match(/^(.+) is overcrowded \((\d+) of (\d+) capacity\)$/);
    if (overcrowdedMatchAr) {
      const [, settlement, cur, cap] = overcrowdedMatchAr;
      const settlementAr = settlement === 'The settlement' ? 'الاستيطان' : settlement;
      return `${settlementAr} مكتظ — ${cur} فرداً، السعة: ${cap}`;
    }
    const normEmergedAr = desc.match(/^Norm emerged: (.+)$/);
    if (normEmergedAr) return `ظهر عرف: ${LAW_DESC_AR[normEmergedAr[1]] ?? normEmergedAr[1].replace(/_/g, ' ')}`;
    const normViolatedAr = desc.match(/^Norm violated: (.+)$/);
    if (normViolatedAr) return `انتُهك عرف: ${LAW_DESC_AR[normViolatedAr[1]] ?? normViolatedAr[1].replace(/_/g, ' ')}`;
    const embracedCodeMatchAr = desc.match(/^(.+) embraced belief #(\d+)$/);
    if (embracedCodeMatchAr) return `اعتنق ${embracedCodeMatchAr[1]} معتقد رقم ${embracedCodeMatchAr[2]}`;
    const embracedNeutralAr = desc.match(/^(.+) embraced a belief$/);
    if (embracedNeutralAr) return `اعتنق ${embracedNeutralAr[1]} معتقداً`;
    const embracedMatchAr = desc.match(/^(.+) embraced (.+)$/);
    if (embracedMatchAr) {
      const [, name, label] = embracedMatchAr;
      return `اعتنق ${name} معتقد ${label}`;
    }
    if (desc === 'A group split into two bands') return 'انقسمت مجموعة إلى فرقتين';
    const newLeaderMatchAr = desc.match(/^(.+) became the new leader$/);
    if (newLeaderMatchAr) return `أصبح ${newLeaderMatchAr[1]} الزعيم الجديد`;
    const tradedMatchAr = desc.match(/^(.+) traded with (.+)$/);
    if (tradedMatchAr) return `تاجر ${tradedMatchAr[1]} مع ${tradedMatchAr[2]}`;
    const soundMatchAr = desc.match(/^(.+) made a sound at (.+)$/);
    if (soundMatchAr) return `أصدر ${soundMatchAr[1]} صوتاً تجاه ${soundMatchAr[2]}`;
    const pointedMatchAr = desc.match(/^(.+) pointed at (.+)$/);
    if (pointedMatchAr) return `أشار ${pointedMatchAr[1]} إلى ${pointedMatchAr[2]}`;
    const exhaustedMatchAr = desc.match(/^(.+) was exhausted and fell asleep \(energy: (\d+)%\)$/);
    if (exhaustedMatchAr) return `أُنهك ${exhaustedMatchAr[1]} ونام (الطاقة: ${exhaustedMatchAr[2]}%)`;
    const activityMatchAr = desc.match(/^(.+) is (searching for food|looking for water|resting)$/);
    if (activityMatchAr) {
      const map: Record<string, string> = { 'searching for food': 'يبحث عن الطعام', 'looking for water': 'يبحث عن الماء', 'resting': 'يستريح' };
      return `${activityMatchAr[1]} ${map[activityMatchAr[2]] ?? activityMatchAr[2]}`;
    }
    const saidMatchAr = desc.match(/^(.+) (said|gestured) ["'](.+)["'] to (.+) — (.+)$/);
    if (saidMatchAr) {
      const [, name, verb, word, target, concept] = saidMatchAr;
      const verbAr = verb === 'said' ? 'قال' : 'أومأ';
      return `${verbAr} ${name} "${word}" إلى ${target} — ${CONCEPT_AR[concept] ?? concept}`;
    }
    const moodMatchAr = desc.match(/^(.+) \((\d+) yrs?, (\w+)\): (.+)$/);
    if (moodMatchAr) {
      const [, name, age, mood, activity] = moodMatchAr;
      return `${name} (${age} سنة، ${MOOD_AR[mood] ?? mood}): ${ACTIVITY_AR[activity] ?? activity}`;
    }
    return desc
      .replace('Culture event:', 'حدث ثقافي:')
      .replace('Art event:', 'حدث فني:')
      .replace('Astronomy event:', 'حدث فلكي:')
      .replace('Architecture event:', 'حدث معماري:')
      .replace('Law event:', 'حدث قانوني:')
      .replace('Microbiome event:', 'حدث ميكروبيومي:')
      .replace('Epigenetics event:', 'حدث جيني:');
  }

  return desc;
}

export function translateEventType(type: string, lang: LangCode): string {
  const key = String(type ?? '').toLowerCase();
  const labels: Record<string, TranslationMap> = {
    birth:        { tr: 'doğum',      en: 'birth',         de: 'Geburt',        fr: 'naissance',       ar: 'ولادة' },
    death:        { tr: 'ölüm',       en: 'death',         de: 'Tod',           fr: 'mort',            ar: 'وفاة' },
    technology:   { tr: 'teknoloji',  en: 'technology',    de: 'Technologie',   fr: 'technologie',     ar: 'تكنولوجيا' },
    language:     { tr: 'dil',        en: 'language',      de: 'Sprache',       fr: 'langue',          ar: 'لغة' },
    discovery:    { tr: 'keşif',      en: 'discovery',     de: 'Entdeckung',    fr: 'découverte',      ar: 'اكتشاف' },
    disaster:     { tr: 'afet',       en: 'disaster',      de: 'Katastrophe',   fr: 'catastrophe',     ar: 'كارثة' },
    belief:       { tr: 'inanç',      en: 'belief',        de: 'Glaube',        fr: 'croyance',        ar: 'معتقد' },
    culture:      { tr: 'kültür',     en: 'culture',       de: 'Kultur',        fr: 'culture',         ar: 'ثقافة' },
    // "cultural_diffusion"/"cultural_meme_emerged" don't contain "culture" as a
    // substring ("cultural" has no trailing 'e'), so they fell through to the
    // raw, untranslated type string without these explicit entries.
    cultural_diffusion:    { tr: 'kültürel yayılma', en: 'cultural diffusion', de: 'kulturelle Diffusion', fr: 'diffusion culturelle', ar: 'انتشار ثقافي' },
    cultural_meme_emerged: { tr: 'kültürel motif',   en: 'cultural meme',      de: 'kulturelles Mem',      fr: 'mème culturel',        ar: 'ميم ثقافي' },
    art:          { tr: 'sanat',      en: 'art',           de: 'Kunst',         fr: 'art',             ar: 'فن' },
    astronomy:    { tr: 'astronomi',  en: 'astronomy',     de: 'Astronomie',    fr: 'astronomie',      ar: 'علم الفلك' },
    architecture: { tr: 'mimari',     en: 'architecture',  de: 'Architektur',   fr: 'architecture',    ar: 'عمارة' },
    law:          { tr: 'hukuk',      en: 'law',           de: 'Recht',         fr: 'droit',           ar: 'قانون' },
    microbiome:   { tr: 'mikrobiyom', en: 'microbiome',    de: 'Mikrobiom',     fr: 'microbiome',      ar: 'ميكروبيوم' },
    epigenetics:  { tr: 'epigenetik', en: 'epigenetics',   de: 'Epigenetik',    fr: 'épigénétique',    ar: 'علم التخلق' },
    epidemic:     { tr: 'salgın',     en: 'epidemic',      de: 'Epidemie',      fr: 'épidémie',        ar: 'وباء' },
    ritual:       { tr: 'ritüel',     en: 'ritual',        de: 'Ritual',        fr: 'rituel',          ar: 'طقوس' },
    trade:        { tr: 'ticaret',    en: 'trade',         de: 'Handel',        fr: 'commerce',        ar: 'تجارة' },
    celestial:    { tr: 'göksel',     en: 'celestial',     de: 'Himmlisch',     fr: 'céleste',         ar: 'سماوي' },
    social:       { tr: 'sosyal',     en: 'social',        de: 'Sozial',        fr: 'social',          ar: 'اجتماعي' },
    group_split:       { tr: 'grup ayrılığı',    en: 'group split',      de: 'Gruppenspaltung',  fr: 'scission de groupe',    ar: 'انقسام جماعي' },
    leadership_change: { tr: 'liderlik değişimi', en: 'leadership change', de: 'Führungswechsel', fr: 'changement de direction', ar: 'تغيير القيادة' },
    norm:         { tr: 'norm',       en: 'norm',          de: 'Norm',          fr: 'norme',           ar: 'معيار' },
    weather:      { tr: 'hava',       en: 'weather',       de: 'Wetter',        fr: 'météo',           ar: 'طقس' },
    communication:{ tr: 'iletişim',   en: 'communication', de: 'Kommunikation', fr: 'communication',   ar: 'تواصل' },
    thought:      { tr: 'düşünce',    en: 'thought',       de: 'Gedanke',       fr: 'pensée',          ar: 'فكر' },
    sleep:        { tr: 'uyku',       en: 'sleep',         de: 'Schlaf',        fr: 'sommeil',         ar: 'نوم' },
    activity:     { tr: 'etkinlik',   en: 'activity',      de: 'Aktivität',     fr: 'activité',        ar: 'نشاط' },
    mating:       { tr: 'çiftleşme',  en: 'mating',        de: 'Paarung',       fr: 'accouplement',    ar: 'تزاوج' },
    conflict:     { tr: 'çatışma',    en: 'conflict',      de: 'Konflikt',      fr: 'conflit',         ar: 'نزاع' },
    milestone:    { tr: 'kilometre taşı', en: 'milestone',  de: 'Meilenstein',   fr: 'étape',           ar: 'إنجاز' },
    structure:    { tr: 'yapı',       en: 'structure',     de: 'Bauwerk',       fr: 'structure',       ar: 'مبنى' },
    settlement:   { tr: 'yerleşim',   en: 'settlement',    de: 'Siedlung',      fr: 'colonie',         ar: 'مستوطنة' },
    migration:    { tr: 'göç',        en: 'migration',     de: 'Migration',     fr: 'migration',       ar: 'هجرة' },
  };

  for (const [needle, values] of Object.entries(labels)) {
    if (key.includes(needle)) return text(lang, values);
  }
  return type || '';
}

export function translateWords(lang: LangCode, value: string, map: TranslationMap): string {
  return text(lang, { en: value, ...map });
}

export function makeDictionaryTranslator<T extends Record<string, TranslationMap>>(lang: LangCode, dict: T) {
  return (key: keyof T) => text(lang, dict[key as string]);
}
