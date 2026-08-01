import { useEffect, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, translateStageName, type LangCode, type TranslationMap } from '../../utils/i18n';

const GENE_LABELS: Record<string, { tr: string; en: string; de: string; fr: string; ar: string; phenoKey: string }> = {
  FOXP2:              { tr: 'Dil (FOXP2)',       en: 'Language (FOXP2)',       de: 'Sprache (FOXP2)',       fr: 'Langage (FOXP2)',        ar: 'اللغة (FOXP2)',       phenoKey: 'language_capacity' },
  OXTR:               { tr: 'Bağlanma (OXTR)',   en: 'Bonding (OXTR)',         de: 'Bindung (OXTR)',        fr: 'Lien (OXTR)',            ar: 'الترابط (OXTR)',      phenoKey: 'social_bonding' },
  DRD4:               { tr: 'Yenilik (DRD4)',    en: 'Novelty (DRD4)',         de: 'Neuheit (DRD4)',        fr: 'Nouveauté (DRD4)',       ar: 'الجدة (DRD4)',        phenoKey: 'curiosity' },
  MAOA:               { tr: 'Dürtü (MAOA)',      en: 'Impulse (MAOA)',         de: 'Impuls (MAOA)',         fr: 'Impulsion (MAOA)',       ar: 'الاندفاع (MAOA)',     phenoKey: 'aggression' },
  BDNF:               { tr: 'Esneklik (BDNF)',   en: 'Plasticity (BDNF)',      de: 'Plastizität (BDNF)',   fr: 'Plasticité (BDNF)',      ar: 'اللدونة (BDNF)',      phenoKey: 'learning_rate' },
  fluid_intelligence: { tr: 'Zekâ',             en: 'Intelligence',           de: 'Intelligenz',           fr: 'Intelligence',           ar: 'الذكاء',             phenoKey: 'fluid_intelligence' },
  physical_strength:  { tr: 'Güç',              en: 'Strength',               de: 'Stärke',                fr: 'Force',                  ar: 'القوة',              phenoKey: 'physical_strength' },
  empathy:            { tr: 'Empati',            en: 'Empathy',                de: 'Empathie',              fr: 'Empathie',               ar: 'التعاطف',            phenoKey: 'empathy' },
  curiosity:          { tr: 'Merak',             en: 'Curiosity',              de: 'Neugier',               fr: 'Curiosité',              ar: 'الفضول',             phenoKey: 'curiosity' },
  conscientiousness:  { tr: 'Özenlilik',         en: 'Conscientiousness',      de: 'Gewissenhaftigkeit',   fr: 'Conscience',             ar: 'الضمير',             phenoKey: 'conscientiousness' },
  aggression:         { tr: 'Saldırganlık',      en: 'Aggression',             de: 'Aggression',            fr: 'Agressivité',            ar: 'العدوانية',           phenoKey: 'aggression' },
  immune_strength:    { tr: 'Bağışıklık',        en: 'Immunity',               de: 'Immunität',             fr: 'Immunité',               ar: 'المناعة',            phenoKey: 'immune_strength' },
  artistic_sense:     { tr: 'Sanat Duyusu',      en: 'Art Sense',              de: 'Kunstsinn',             fr: 'Sens artistique',        ar: 'الحس الفني',          phenoKey: 'artistic_sense' },
};

const HAIR_I18N: Record<string, TranslationMap> = {
  black:        { tr: 'Siyah',          en: 'Black',         de: 'Schwarz',        fr: 'Noir',         ar: 'أسود' },
  brown:        { tr: 'Kahverengi',     en: 'Brown',         de: 'Braun',          fr: 'Brun',         ar: 'بني' },
  blonde:       { tr: 'Sarı',           en: 'Blonde',        de: 'Blond',          fr: 'Blond',        ar: 'أشقر' },
  red:          { tr: 'Kızıl',          en: 'Red',           de: 'Rot',            fr: 'Roux',         ar: 'أحمر' },
  grey:         { tr: 'Gri',            en: 'Grey',          de: 'Grau',           fr: 'Gris',         ar: 'رمادي' },
  white:        { tr: 'Beyaz',          en: 'White',         de: 'Weiß',           fr: 'Blanc',        ar: 'أبيض' },
  dark:         { tr: 'Koyu',           en: 'Dark',          de: 'Dunkel',         fr: 'Foncé',        ar: 'داكن' },
  light:        { tr: 'Açık',           en: 'Light',         de: 'Hell',           fr: 'Clair',        ar: 'فاتح' },
  medium:       { tr: 'Orta',           en: 'Medium',        de: 'Mittel',         fr: 'Moyen',        ar: 'متوسط' },
  'dark brown': { tr: 'Koyu Kahverengi', en: 'Dark Brown',  de: 'Dunkelbraun',    fr: 'Brun foncé',   ar: 'بني داكن' },
  'light brown':{ tr: 'Açık Kahverengi', en: 'Light Brown', de: 'Hellbraun',      fr: 'Brun clair',   ar: 'بني فاتح' },
};

const EYE_I18N: Record<string, TranslationMap> = {
  brown: { tr: 'Kahverengi', en: 'Brown', de: 'Braun', fr: 'Marron', ar: 'بني' },
  blue:  { tr: 'Mavi',       en: 'Blue',  de: 'Blau',  fr: 'Bleu',   ar: 'أزرق' },
  green: { tr: 'Yeşil',      en: 'Green', de: 'Grün',  fr: 'Vert',   ar: 'أخضر' },
  hazel: { tr: 'Ela',        en: 'Hazel', de: 'Haselnussbraun', fr: 'Noisette', ar: 'بندقي' },
  grey:  { tr: 'Gri',        en: 'Grey',  de: 'Grau',  fr: 'Gris',   ar: 'رمادي' },
  amber: { tr: 'Kehribar',   en: 'Amber', de: 'Bernstein', fr: 'Ambre', ar: 'كهرماني' },
  black: { tr: 'Siyah',      en: 'Black', de: 'Schwarz', fr: 'Noir', ar: 'أسود' },
};

const SKIN_I18N: Record<string, TranslationMap> = {
  fair:      { tr: 'Açık',      en: 'Fair',      de: 'Hell',        fr: 'Clair',       ar: 'فاتح' },
  light:     { tr: 'Açık',      en: 'Light',     de: 'Hell',        fr: 'Clair',       ar: 'فاتح' },
  medium:    { tr: 'Orta',      en: 'Medium',    de: 'Mittel',      fr: 'Moyen',       ar: 'متوسط' },
  olive:     { tr: 'Zeytinî',   en: 'Olive',     de: 'Olivfarben',  fr: 'Olive',       ar: 'زيتوني' },
  tan:       { tr: 'Bronz',     en: 'Tan',       de: 'Gebräunt',    fr: 'Bronzé',      ar: 'أسمر' },
  dark:      { tr: 'Koyu',      en: 'Dark',      de: 'Dunkel',      fr: 'Foncé',       ar: 'داكن' },
  'very dark': { tr: 'Çok Koyu', en: 'Very Dark', de: 'Sehr dunkel', fr: 'Très foncé', ar: 'داكن جداً' },
  very_dark:   { tr: 'Çok Koyu', en: 'Very Dark', de: 'Sehr dunkel', fr: 'Très foncé', ar: 'داكن جداً' },
};

const LANG_STAGE_I18N: Record<string, TranslationMap> = {
  'pre-linguistic':  { tr: 'Dil öncesi',      en: 'Pre-linguistic', de: 'Vorsprachlich', fr: 'Prélinguistique', ar: 'ما قبل اللغة' },
  'gestural':        { tr: 'Jestsel',          en: 'Gestural',       de: 'Gestisch',      fr: 'Gestuel',         ar: 'إيمائي' },
  'emotional sound': { tr: 'Duygusal ses',     en: 'Emotional sound', de: 'Emotionaler Laut', fr: 'Son émotionnel', ar: 'صوت عاطفي' },
  'proto-words':     { tr: 'Proto-kelimeler',  en: 'Proto-words',    de: 'Protowörter',   fr: 'Proto-mots',      ar: 'كلمات أولية' },
  'syntax':          { tr: 'Sözdizimi',        en: 'Syntax',         de: 'Syntax',        fr: 'Syntaxe',         ar: 'نحو' },
  'abstract':        { tr: 'Soyut',            en: 'Abstract',       de: 'Abstrakt',      fr: 'Abstrait',        ar: 'مجرد' },
  'writing':         { tr: 'Yazı',             en: 'Writing',        de: 'Schrift',       fr: 'Écriture',        ar: 'كتابة' },
};

function translateSkin(value: any, lang: LangCode): string {
  if (!value && value !== 0) return '-';
  if (typeof value === 'number') {
    if (value < 0.25) return text(lang, { tr: 'Açık', en: 'Fair', de: 'Hell', fr: 'Clair', ar: 'فاتح' });
    if (value < 0.5)  return text(lang, { tr: 'Orta', en: 'Medium', de: 'Mittel', fr: 'Moyen', ar: 'متوسط' });
    if (value < 0.75) return text(lang, { tr: 'Koyu', en: 'Dark', de: 'Dunkel', fr: 'Foncé', ar: 'داكن' });
    return text(lang, { tr: 'Çok Koyu', en: 'Very Dark', de: 'Sehr dunkel', fr: 'Très foncé', ar: 'داكن جداً' });
  }
  const key = String(value).toLowerCase();
  const entry = SKIN_I18N[key];
  return entry ? text(lang, entry) : String(value).slice(0, 12);
}


export default function BiologyPanel() {
  const { stats, lang, currentSim, accessToken, milestones } = useSimStore();
  const [individuals, setIndividuals] = useState<any[]>([]);
  const [loadError, setLoadError] = useState(false);

  useEffect(() => {
    if (!currentSim || !accessToken) return;
    const headers = { Authorization: `Bearer ${accessToken}` };
    const load = () => axios.get(`/api/simulations/${currentSim.id}/population?alive=true`, { headers })
      .then(r => { setIndividuals(r.data); setLoadError(false); })
      .catch(() => setLoadError(true));
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  }, [currentSim?.id, accessToken]);

  const avgIntel = stats?.avg_intelligence ?? 0;
  const pop = stats?.population ?? 0;
  const avgAge = stats?.avg_age ?? 0;
  const sexRatio = stats?.sex_ratio ?? 0.5;

  return (
    <DetailPanel panelId="biology" title="Biology" titleTr="Biyoloji">
      {loadError && (
        <div className="mb-2 px-3 py-2 text-xs font-share-tech text-sim-red bg-sim-red/10 border border-sim-red/30 rounded">
          {text(lang as LangCode, { en: 'Failed to load individual data. Retrying…', tr: 'Bireysel veri yüklenemedi. Yeniden deneniyor…', de: 'Individualdaten konnten nicht geladen werden. Wiederholen…', fr: 'Échec du chargement des données. Réessai…', ar: 'فشل تحميل البيانات الفردية. إعادة المحاولة…' })}
        </div>
      )}
      <Section title={text(lang as LangCode, { en: 'Population Vitals', tr: 'Nüfus Değerleri', de: 'Bevölkerungsdaten', fr: 'Données démographiques', ar: 'البيانات الحيوية للسكان' })}>
        <MetricRow label={text(lang as LangCode, { en: 'Population', tr: 'Nüfus', de: 'Bevölkerung', fr: 'Population', ar: 'السكان' })} value={pop} />
        <MetricRow label={text(lang as LangCode, { en: 'Avg Age', tr: 'Ortalama Yaş', de: 'Ø Alter', fr: 'Âge moy.', ar: 'متوسط العمر' })} value={`${avgAge.toFixed(1)} yr`} />
        <MetricRow label={text(lang as LangCode, { en: 'Sex Ratio M:F', tr: 'Cinsiyet Oranı E:K', de: 'Geschl.-verh. M:W', fr: 'Ratio H:F', ar: 'نسبة ذ:أ' })} value={`${(sexRatio * 100).toFixed(0)}% / ${((1 - sexRatio) * 100).toFixed(0)}%`} />
        <MetricRow label={text(lang as LangCode, { en: 'Avg Intelligence', tr: 'Ortalama Zekâ', de: 'Ø Intelligenz', fr: 'Intelligence moy.', ar: 'متوسط الذكاء' })} value={(avgIntel * 100).toFixed(1) + '%'} />
        <MetricRow label={text(lang as LangCode, { en: 'Sick Rate', tr: 'Hastalık Oranı', de: 'Krankenquote', fr: 'Taux de maladie', ar: 'معدل المرض' })} value={`${((stats?.sick_rate ?? 0) * 100).toFixed(1)}%`} />
        <MetricRow label={text(lang as LangCode, { en: 'Avg Consciousness', tr: 'Ort. Bilinç', de: 'Ø Bewusstsein', fr: 'Conscience moy.', ar: 'متوسط الوعي' })} value={`${((stats?.avg_consciousness ?? 0) * 100).toFixed(2)}%`} />
        <MetricRow label={text(lang as LangCode, { en: 'Max ToM Stage', tr: 'Maks. Zihin Teorisi', de: 'Max ToM-Stufe', fr: 'Stade ToM max', ar: 'أقصى مرحلة ToM' })} value={`${stats?.max_tom_stage ?? 0} / 3`} />
      </Section>

      <Section title={text(lang as LangCode, { en: 'Genome Activity', tr: 'Genom Aktivitesi', de: 'Genomaktivität', fr: 'Activité génomique', ar: 'نشاط الجينوم' })}>
        <p className="text-sim-muted text-sm italic mb-2">
          {text(lang as LangCode, { en: 'Gene loci drive behavior. Values emerge from meiosis and mutation.', tr: 'Gen lokusları davranışları yönlendirir. Değerler mayoz ve mutasyondan ortaya çıkar.', de: 'Genloci steuern das Verhalten. Werte entstehen durch Meiose und Mutation.', fr: 'Les loci géniques guident le comportement. Les valeurs émergent de la méiose et des mutations.', ar: 'مواضع الجينات تحدد السلوك. تظهر القيم من الانقسام الاختزالي والطفرات.' })}
        </p>
        {Object.entries(GENE_LABELS).slice(0, 6).map(([key, labels]) => {
          // Read the server-authoritative average (derive_stats' own
          // allele_frequencies, keyed exactly to these GENE_LABELS entries)
          // instead of recomputing it here from the `individuals` list --
          // that list comes from a separate, paginated/capped fetch
          // (SimulationPage's population poll), so a client-side average
          // over it could silently diverge from the true population-wide
          // value the server already computes once, correctly, for
          // everyone else on this panel (avg_intelligence, etc.).
          const pct = Math.round((stats?.allele_frequencies?.[key] ?? 0) * 100);
          return (
            <div key={key} className="mb-1">
              <div className="flex justify-between mb-0.5">
                <span className="text-sim-muted">{text(lang as LangCode, labels)}</span>
                <span className="text-sim-accent">{pct}%</span>
              </div>
              <div className="h-1 bg-sim-border rounded-full overflow-hidden">
                <div
                  className="h-full bg-sim-accent rounded-full"
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          );
        })}
      </Section>

      <Section title={text(lang as LangCode, { en: 'Living Individuals', tr: 'Yaşayan Bireyler', de: 'Lebende Individuen', fr: 'Individus vivants', ar: 'الأفراد الأحياء' })}>
        {individuals.length === 0 ? (
          <p className="text-sim-muted text-sm italic">
            {text(lang as LangCode, { en: 'No live population loaded yet.', tr: 'Canlı nüfus henüz yüklenmedi.', de: 'Noch keine lebende Bevölkerung geladen.', fr: 'Aucune population vivante chargée.', ar: 'لم يتم تحميل أي سكان أحياء بعد.' })}
          </p>
        ) : (
          <div className="space-y-2 max-h-80 overflow-auto pr-1">
            {individuals.slice(0, 80).map(ind => {
              const sexLocalized = text(lang as LangCode, { en: ind.sex, tr: ind.sex === 'male' ? 'Erkek' : ind.sex === 'female' ? 'Kadın' : ind.sex, de: ind.sex === 'male' ? 'Männlich' : ind.sex === 'female' ? 'Weiblich' : ind.sex, fr: ind.sex === 'male' ? 'Mâle' : ind.sex === 'female' ? 'Femelle' : ind.sex, ar: ind.sex === 'male' ? 'ذكر' : ind.sex === 'female' ? 'أنثى' : ind.sex });
              const hairVal = ind.phenotype?.hair_color ?? '-';
              const eyeVal  = ind.phenotype?.eye_color ?? '-';
              const skinVal = ind.phenotype?.skin_color ?? ind.phenotype?.skin_tone;
              const langStage = ind.language?.stage_name ?? '-';
              return (
                <div key={ind.id} className="bg-sim-bg/60 border border-sim-border/40 rounded p-2">
                  <div className="flex justify-between gap-2 mb-1">
                    <span className="text-sim-text font-semibold truncate">
                      {ind.name ?? ind.phenotype?.name ?? text(lang as LangCode, { en: 'Unnamed', tr: 'İsimsiz', de: 'Unbenannt', fr: 'Sans nom', ar: 'بلا اسم' })}
                    </span>
                    <span className="text-sim-accent font-mono text-[12px]">{Number(ind.age_years ?? 0).toFixed(1)} yr</span>
                  </div>
                  <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-[12px]">
                    <Mini label={text(lang as LangCode, { en: 'Sex', tr: 'Cinsiyet', de: 'Geschlecht', fr: 'Sexe', ar: 'الجنس' })} value={sexLocalized} />
                    <Mini label={text(lang as LangCode, { en: 'Location', tr: 'Konum', de: 'Standort', fr: 'Emplacement', ar: 'الموقع' })} value={`${Number(ind.y ?? 0).toFixed(2)}, ${Number(ind.x ?? 0).toFixed(2)}`} />
                    <Mini label={text(lang as LangCode, { en: 'Hair', tr: 'Saç', de: 'Haare', fr: 'Cheveux', ar: 'الشعر' })} value={HAIR_I18N[String(hairVal).toLowerCase()] ? text(lang as LangCode, HAIR_I18N[String(hairVal).toLowerCase()]) : hairVal} />
                    <Mini label={text(lang as LangCode, { en: 'Eye', tr: 'Göz', de: 'Auge', fr: 'Œil', ar: 'العين' })} value={EYE_I18N[String(eyeVal).toLowerCase()] ? text(lang as LangCode, EYE_I18N[String(eyeVal).toLowerCase()]) : eyeVal} />
                    <Mini label={text(lang as LangCode, { en: 'Skin', tr: 'Ten', de: 'Haut', fr: 'Peau', ar: 'البشرة' })} value={translateSkin(skinVal, lang as LangCode)} />
                    <Mini label={text(lang as LangCode, { en: 'Height', tr: 'Boy', de: 'Größe', fr: 'Taille', ar: 'الطول' })} value={`${ind.phenotype?.height_cm ?? '-'} cm`} />
                    <Mini label={text(lang as LangCode, { en: 'IQ', tr: 'Zekâ', de: 'IQ', fr: 'QI', ar: 'معدل الذكاء' })} value={`${((ind.phenotype?.fluid_intelligence ?? 0) * 100).toFixed(0)}%`} />
                    <Mini
                      label={text(lang as LangCode, { en: 'Language', tr: 'Dil', de: 'Sprache', fr: 'Langage', ar: 'اللغة' })}
                      value={LANG_STAGE_I18N[langStage.toLowerCase()] ? text(lang as LangCode, LANG_STAGE_I18N[langStage.toLowerCase()]) : translateStageName(langStage, lang as LangCode)}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Section>

      <Section title={text(lang as LangCode, { en: 'Life Stages', tr: 'Yaşam Evreleri', de: 'Lebensphasen', fr: 'Étapes de vie', ar: 'مراحل الحياة' })}>
        {(() => {
          const LIFE_STAGE_I18N: Record<string, { tr: string; de: string; fr: string; ar: string }> = {
            infant:     { tr: 'Bebek',    de: 'Säugling',    fr: 'Nourrisson', ar: 'رضيع' },
            child:      { tr: 'Çocuk',    de: 'Kind',         fr: 'Enfant',     ar: 'طفل' },
            adolescent: { tr: 'Ergen',    de: 'Jugendlicher', fr: 'Adolescent', ar: 'مراهق' },
            adult:      { tr: 'Yetişkin', de: 'Erwachsener',  fr: 'Adulte',     ar: 'بالغ' },
            elder:      { tr: 'Yaşlı',    de: 'Älterer',      fr: 'Aîné',       ar: 'شيخ' },
          };
          const counts: Record<string, number> = { infant: 0, child: 0, adolescent: 0, adult: 0, elder: 0 };
          for (const ind of individuals) {
            const stage = (ind.life_stage ?? '').toLowerCase();
            if (stage in counts) counts[stage]++;
          }
          return ['infant', 'child', 'adolescent', 'adult', 'elder'].map(stage => (
            <div key={stage} className="flex justify-between py-0.5 border-b border-sim-border/30">
              <span className="text-sim-muted">
                {text(lang as LangCode, { en: stage, tr: LIFE_STAGE_I18N[stage]?.tr ?? stage, de: LIFE_STAGE_I18N[stage]?.de ?? stage, fr: LIFE_STAGE_I18N[stage]?.fr ?? stage, ar: LIFE_STAGE_I18N[stage]?.ar ?? stage })}
              </span>
              <span className="text-sim-text font-mono">{counts[stage]}</span>
            </div>
          ));
        })()}
      </Section>

      {/* Feature 15: Allele Frequency Snapshot */}
      {stats?.allele_frequencies && Object.keys(stats.allele_frequencies).length > 0 && (
        <Section title={text(lang as LangCode, { en: 'Allele Frequencies', tr: 'Alel Frekansları', de: 'Allelfrequenzen', fr: 'Fréquences Alléliques', ar: 'ترددات الأليل' })}>
          <p style={{ fontSize: 12, color: '#6090a0', marginBottom: 8, lineHeight: 1.4 }}>
            {text(lang as LangCode, { en: 'Population-wide mean phenotype per locus (checkpoint snapshot)', tr: 'Lokus başına popülasyon ortalaması (kontrol noktası anlık görüntüsü)', de: 'Bevölkerungsweiter Phänotyp-Mittelwert je Locus', fr: 'Moyenne phénotypique par locus (instantané checkpoint)', ar: 'متوسط الفينوتيب للسكان لكل موضع (لقطة نقطة التفتيش)' })}
          </p>
          {Object.entries(stats.allele_frequencies)
            .sort(([, a], [, b]) => b - a)
            .map(([locus, freq]) => {
              const label = GENE_LABELS[locus];
              const displayName = label ? text(lang as LangCode, label) : locus.replace(/_/g, ' ');
              const pct = Math.round(freq * 100);
              const color = freq > 0.7 ? '#4ecb71' : freq > 0.4 ? '#d4a838' : '#e05a5a';
              return (
                <div key={locus} style={{ marginBottom: 6 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 2, fontSize: 12 }}>
                    <span style={{ color: '#8abda0', maxWidth: '70%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{displayName}</span>
                    <span style={{ color, fontFamily: 'Orbitron, monospace', fontSize: 12 }}>{pct}%</span>
                  </div>
                  <div style={{ height: 3, background: 'rgba(255,255,255,0.07)', borderRadius: 2, overflow: 'hidden' }}>
                    <div style={{ height: '100%', width: `${pct}%`, background: color, transition: 'width 1s ease' }} />
                  </div>
                </div>
              );
            })}
        </Section>
      )}

      {/* Genetic Milestone Log */}
      {milestones.length > 0 && (
        <Section title={text(lang as LangCode, { en: 'Genetic Milestones', tr: 'Genetik Kilometre Taşları', de: 'Genetische Meilensteine', fr: 'Jalons Génétiques', ar: 'المعالم الجينية' })}>
          <div style={{ maxHeight: 160, overflowY: 'auto' }}>
            {milestones.filter(m => m.key.startsWith('pop') || m.key.startsWith('gen') || m.key.startsWith('allele')).slice(0, 10).map(m => (
              <div key={`${m.key}-${m.day}`} style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '3px 0', borderBottom: '1px solid rgba(160,200,176,0.08)', fontSize: 12 }}>
                <span>{m.icon}</span>
                <span style={{ color: '#8abda0', flex: 1 }}>{m.description}</span>
                <span style={{ color: '#6090a0', flexShrink: 0 }}>G{m.day}</span>
              </div>
            ))}
          </div>
        </Section>
      )}
    </DetailPanel>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">{title}</h4>
      {children}
    </div>
  );
}

function MetricRow({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex justify-between py-1 border-b border-sim-border/30">
      <span className="text-sim-muted">{label}</span>
      <span className="text-sim-text font-mono">{value}</span>
    </div>
  );
}

function Mini({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex justify-between gap-2 border-b border-sim-border/20 pb-0.5">
      <span className="text-sim-muted">{label}</span>
      <span className="text-sim-text text-right truncate">{value}</span>
    </div>
  );
}
