import { useEffect, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { translateEventDescription, text, type LangCode } from '../../utils/i18n';

const LANGUAGE_STAGES = [
  { id: 0, name: 'Pre-linguistic',  nameTr: 'Dil Öncesi',      nameDe: 'Vorsprachlich',         nameFr: 'Prélinguistique',        nameAr: 'ما قبل اللغة',
    desc: 'No symbolic communication',        descTr: 'Sembolik iletişim yok',              descDe: 'Keine symbolische Kommunikation',       descFr: 'Pas de communication symbolique',           descAr: 'لا تواصل رمزي',           color: '#8898c8' },
  { id: 1, name: 'Gestural',        nameTr: 'Jestsel',          nameDe: 'Gestural',              nameFr: 'Gestuel',                nameAr: 'إيمائي',
    desc: 'Pointing, body language',           descTr: 'İşaret, beden dili',                 descDe: 'Zeigen, Körpersprache',                 descFr: 'Pointer, langage corporel',                 descAr: 'الإشارة، لغة الجسد',      color: '#8b7355' },
  { id: 2, name: 'Emotional Sound', nameTr: 'Duygusal Ses',     nameDe: 'Emotionale Klänge',     nameFr: 'Sons émotionnels',       nameAr: 'أصوات عاطفية',
    desc: 'Shared emotional vocalizations',    descTr: 'Ortak duygusal sesler',              descDe: 'Gemeinsame emotionale Lautäußerungen',  descFr: 'Vocalisations émotionnelles partagées',     descAr: 'أصوات عاطفية مشتركة',    color: '#6b8e23' },
  { id: 3, name: 'Proto-Words',     nameTr: 'Proto-Kelimeler',  nameDe: 'Proto-Wörter',          nameFr: 'Proto-mots',             nameAr: 'كلمات أولى',
    desc: 'Consistent sound-meaning pairs',    descTr: 'Tutarlı ses-anlam eşleşmeleri',      descDe: 'Konsistente Laut-Bedeutungs-Paare',     descFr: 'Paires son-signification cohérentes',       descAr: 'أزواج صوت-معنى متسقة',   color: '#4682b4' },
  { id: 4, name: 'Syntax',          nameTr: 'Sözdizimi',        nameDe: 'Syntax',                nameFr: 'Syntaxe',                nameAr: 'نحو',
    desc: 'Grammar emerges',                   descTr: 'Dilbilgisi ortaya çıkıyor',          descDe: 'Grammatik entsteht',                    descFr: 'La grammaire émerge',                       descAr: 'ظهور القواعد النحوية',    color: '#9370db' },
  { id: 5, name: 'Abstract',        nameTr: 'Soyut',            nameDe: 'Abstrakt',              nameFr: 'Abstrait',               nameAr: 'مجرد',
    desc: 'Concepts beyond immediate world',   descTr: 'Anlık dünyayı aşan kavramlar',       descDe: 'Konzepte jenseits der unmittelbaren Welt',descFr: 'Concepts au-delà du monde immédiat',       descAr: 'مفاهيم تتخطى العالم المباشر', color: '#cd853f' },
  { id: 6, name: 'Writing',         nameTr: 'Yazı',             nameDe: 'Schrift',               nameFr: 'Écriture',               nameAr: 'كتابة',
    desc: 'Symbolic recording of language',    descTr: 'Dilin sembolik kaydı',               descDe: 'Symbolische Aufzeichnung der Sprache',  descFr: 'Enregistrement symbolique du langage',      descAr: 'التسجيل الرمزي للغة',     color: '#daa520' },
];

const C_CLASSES = [
  ['m', 'n', 'ng', 'w'],
  ['p', 't', 'k', 'b'],
  ['d', 'g', 'r', 'l'],
  ['s', 'z', 'sh', 'h'],
  ['f', 'v', 'th', 'y'],
  ['ts', 'nd', 'mb', 'rl'],
];

const V_SYSTEMS = [
  ['a', 'i', 'u'],
  ['a', 'e', 'i', 'o', 'u'],
  ['a', 'o', 'e', 'ai', 'ou'],
  ['a', 'i', 'u', 'an', 'el', 'ar'],
];

const BIOME_C_BIAS: Record<string, number> = {
  mediterranean: 0, coastal: 1, tropical_rainforest: 2, tropical_savanna: 2,
  temperate_forest: 3, boreal_forest: 4, tundra: 4, mountain: 3, grassland: 1, desert: 0,
};

type Phonology = { consonants: string[]; vowels: string[]; clanSuffix: string[] };

// Illustrative fallback only -- used when this simulation's own engine-derived
// `world_state.phoneme_palette` isn't available yet (before the first tick has
// run; tick.rs self-heals it from the founders' FOXP2/CNTNAP2 alleles on every
// subsequent tick, so this should be rare). Deliberately never labeled as
// "actual/real" in the UI -- see `phonologyFromPalette` for the genuine,
// per-simulation genetically-derived data this defers to when present.
function buildPhonology(phonologySeed: number, biome = 'mediterranean'): Phonology {
  const s = Math.abs(phonologySeed | 0);
  const biomeBias = BIOME_C_BIAS[biome] ?? 0;
  const c1 = C_CLASSES[(s + biomeBias) % C_CLASSES.length];
  const c2 = C_CLASSES[(s * 3 + biomeBias + 2) % C_CLASSES.length];
  const c3 = C_CLASSES[(s * 7 + 1) % C_CLASSES.length];
  const vowels = V_SYSTEMS[(s * 5 + biomeBias) % V_SYSTEMS.length];
  const clanSuffix = c3.slice(0, 3).map(c => c + vowels[0]);
  return { consonants: [...new Set([...c1, ...c2])], vowels, clanSuffix };
}

// This simulation's genuinely emergent phoneme inventory -- `world_state.
// phoneme_palette`, derived once at simulation creation from both founders'
// literal FOXP2_01/CNTNAP2_01 allele values (see AGENTS.md / language.rs's
// `derive_phoneme_palette`), the same palette every in-simulation word and
// personal name is actually drawn from. Unlike `buildPhonology` above, this
// is real simulation output, not a client-side template lookup.
function phonologyFromPalette(consonants: string[], vowels: string[]): Phonology {
  const c = consonants.length ? consonants : ['t'];
  const v = vowels.length ? vowels : ['a'];
  return { consonants: c, vowels: v, clanSuffix: c.slice(0, 3).map(x => x + v[0]) };
}

function buildSurfaceForms(phonology: ReturnType<typeof buildPhonology>, stage: number) {
  const { consonants, vowels } = phonology;
  const forms: string[] = [];
  if (stage >= 2) forms.push(...vowels.slice(0, 3));
  if (stage >= 3) for (const c of consonants.slice(0, 3)) for (const v of vowels.slice(0, 3)) forms.push(`${c}${v}`);
  if (stage >= 4) for (const c of consonants.slice(0, 2)) for (const v of vowels.slice(0, 2)) forms.push(`${c}${v}${c}`);
  if (stage >= 5) for (const c of consonants.slice(0, 2)) forms.push(`${c}${vowels[0]}-${c}${vowels[1] ?? vowels[0]}`);
  if (stage >= 6) forms.push(...phonology.clanSuffix);
  return [...new Set(forms)].slice(0, 9);
}

function evColor(type: string) {
  if (type === 'communication') return '#a0b4ff';
  if (type === 'word')          return '#7dd3fc';
  return '#a0c8b0';
}
function evIcon(type: string) {
  if (type === 'communication') return '🔤';
  if (type === 'word')          return '◆';
  return '◈';
}
function evLabel(type: string, lang: string) {
  if (type === 'communication') return text(lang as LangCode, { tr: 'iletişim', en: 'comm', de: 'Komm.', fr: 'comm.', ar: 'تواصل' });
  if (type === 'word')          return text(lang as LangCode, { tr: 'kelime',   en: 'word', de: 'Wort',  fr: 'mot',   ar: 'كلمة' });
  return text(lang as LangCode, { tr: 'dil', en: 'lang', de: 'Sprache', fr: 'langue', ar: 'لغة' });
}

// ── Archive Modal ────────────────────────────────────────────────────────────

const ARCHIVE_FILTERS = [
  { id: 'all',           labelTr: 'Tümü',      labelEn: 'All',      labelDe: 'Alle',         labelFr: 'Tous',         labelAr: 'الكل'     },
  { id: 'language',      labelTr: 'Dil',        labelEn: 'Language', labelDe: 'Sprache',      labelFr: 'Langue',       labelAr: 'لغة'      },
  { id: 'communication', labelTr: 'İletişim',   labelEn: 'Comm.',    labelDe: 'Komm.',        labelFr: 'Comm.',        labelAr: 'تواصل'    },
  { id: 'word',          labelTr: 'Kelime',     labelEn: 'Word',     labelDe: 'Wort',         labelFr: 'Mot',          labelAr: 'كلمة'     },
];

const PAGE = 100;

function LangArchiveModal({ simId, accessToken, lang: uiLang, onClose }: {
  simId: string; accessToken: string; lang: string; onClose: () => void;
}) {
  const [rows,    setRows]    = useState<any[]>([]);
  const [total,   setTotal]   = useState(0);
  const [offset,  setOffset]  = useState(0);
  const [filter,  setFilter]  = useState('all');
  const [search,  setSearch]  = useState('');
  const [loading, setLoading] = useState(false);

  const TYPES = 'language,word,communication';

  useEffect(() => {
    if (!simId || !accessToken) return;
    let cancelled = false;
    setLoading(true);
    const headers = { Authorization: `Bearer ${accessToken}` };
    // Load total count via summary
    axios.get(`/api/simulations/${simId}/events/summary`, { headers })
      .then(r => {
        if (cancelled) return;
        const c = r.data?.countsByType ?? {};
        setTotal((c['language'] ?? 0) + (c['word'] ?? 0) + (c['communication'] ?? 0));
      }).catch(() => {});
    // Load first page
    axios.get(`/api/simulations/${simId}/events`, {
      headers, params: { types: TYPES, limit: PAGE, offset: 0 },
    }).then(r => {
      if (cancelled) return;
      setRows(r.data ?? []);
      setOffset(PAGE);
    }).catch(() => {}).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [simId, accessToken]);

  async function loadMore() {
    if (loading) return;
    setLoading(true);
    const headers = { Authorization: `Bearer ${accessToken}` };
    try {
      const r = await axios.get(`/api/simulations/${simId}/events`, {
        headers, params: { types: TYPES, limit: PAGE, offset },
      });
      setRows(prev => [...prev, ...(r.data ?? [])]);
      setOffset(o => o + PAGE);
    } finally { setLoading(false); }
  }

  const filtered = rows.filter(ev => {
    const t = String(ev.event_type ?? '');
    if (filter !== 'all' && t !== filter) return false;
    if (search) {
      const desc = String(ev.description ?? '').toLowerCase();
      if (!desc.includes(search.toLowerCase())) return false;
    }
    return true;
  });

  return (
    <div
      style={{ position: 'fixed', inset: 0, zIndex: 60, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(4px)' }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={{ width: 640, maxWidth: '95vw', maxHeight: '88vh', display: 'flex', flexDirection: 'column', background: 'rgba(4,4,18,0.98)', border: '1px solid rgba(160,180,255,0.25)', boxShadow: '0 20px 60px rgba(0,0,0,0.8)' }}>

        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '10px 14px', borderBottom: '1px solid rgba(160,180,255,0.15)', flexShrink: 0 }}>
          <span style={{ fontSize: 13, fontFamily: 'Orbitron, monospace', color: '#a0b4ff', fontWeight: 700, letterSpacing: '0.1em', flex: 1 }}>
            {text(uiLang as LangCode, { tr: '📋 DİL & İLETİŞİM ARŞİVİ', en: '📋 LANGUAGE & COMM. ARCHIVE', de: '📋 SPRACHE & KOMM.-ARCHIV', fr: '📋 ARCHIVE LANGUE & COMM.', ar: '📋 أرشيف اللغة والتواصل' })}
          </span>
          <span style={{ fontSize: 12, color: '#6a8878', fontFamily: 'Share Tech Mono, monospace' }}>
            {total > 0 ? `${total} ${text(uiLang as LangCode, { tr: 'kayıt', en: 'records', de: 'Einträge', fr: 'entrées', ar: 'سجلات' })}` : ''}
          </span>
          <button onClick={onClose} style={{ color: '#6a8878', background: 'none', border: 'none', cursor: 'pointer', fontSize: 16, lineHeight: 1, padding: '0 4px' }}>✕</button>
        </div>

        {/* Filter + Search */}
        <div style={{ padding: '8px 14px', borderBottom: '1px solid rgba(160,180,255,0.08)', flexShrink: 0, display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center' }}>
          <div style={{ display: 'flex', gap: 4 }}>
            {ARCHIVE_FILTERS.map(f => (
              <button key={f.id} onClick={() => setFilter(f.id)} style={{
                padding: '2px 8px', fontSize: 12,
                border: `1px solid ${filter === f.id ? '#a0b4ff' : 'rgba(160,180,255,0.2)'}`,
                color: filter === f.id ? '#a0b4ff' : '#6a8878',
                background: filter === f.id ? 'rgba(160,180,255,0.1)' : 'transparent',
                fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
              }}>
                {text(uiLang as LangCode, { tr: f.labelTr, en: f.labelEn, de: f.labelDe, fr: f.labelFr, ar: f.labelAr })}
              </button>
            ))}
          </div>
          <input
            value={search} onChange={e => setSearch(e.target.value)}
            placeholder={text(uiLang as LangCode, { tr: 'Açıklamada ara…', en: 'Search descriptions…', de: 'Beschreibungen suchen…', fr: 'Rechercher dans les descriptions…', ar: 'ابحث في الأوصاف…' })}
            style={{
              flex: 1, minWidth: 120, padding: '2px 8px', fontSize: 12,
              background: 'rgba(160,180,255,0.05)', border: '1px solid rgba(160,180,255,0.2)',
              color: '#c0ccff', fontFamily: 'Share Tech Mono, monospace', outline: 'none',
            }}
          />
        </div>

        {/* Event list */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '6px 14px' }}>
          {loading && filtered.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 0', color: '#6a8878', fontSize: 12, fontFamily: 'Share Tech Mono, monospace' }}>
              {text(uiLang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' })}
            </div>
          ) : filtered.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 0', color: '#6a8878', fontSize: 12, fontStyle: 'italic' }}>
              {text(uiLang as LangCode, { tr: 'Kayıt bulunamadı.', en: 'No records found.', de: 'Keine Einträge gefunden.', fr: 'Aucune entrée trouvée.', ar: 'لا توجد سجلات.' })}
            </div>
          ) : filtered.map((ev, i) => {
            const t = String(ev.event_type ?? '');
            const color = evColor(t);
            const icon  = evIcon(t);
            const desc  = translateEventDescription(ev.description ?? '', uiLang as LangCode, ev);
            return (
              <div key={ev.id ?? i} style={{
                display: 'flex', gap: 8, alignItems: 'flex-start',
                padding: '5px 4px', borderBottom: '1px solid rgba(160,180,255,0.06)',
              }}>
                <span style={{ fontSize: 12, color, flexShrink: 0, marginTop: 1 }}>{icon}</span>
                <div style={{ flexShrink: 0, width: 72 }}>
                  <div style={{ fontSize: 12, color: '#4a6a6a', fontFamily: 'Orbitron, monospace' }}>
                    Y{String(ev.sim_year ?? 0).padStart(3,'0')}
                  </div>
                  <div style={{ fontSize: 12, color: '#3a5a5a', fontFamily: 'Orbitron, monospace' }}>
                    G{String((ev.sim_day ?? 0) % 365).padStart(3,'0')}
                  </div>
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <span style={{ fontSize: 12, color: `${color}88`, letterSpacing: '0.06em', textTransform: 'uppercase', display: 'block', marginBottom: 2 }}>
                    {evLabel(t, uiLang)}
                  </span>
                  <div style={{ fontSize: 12, color: '#a0c8b0', lineHeight: 1.5, wordBreak: 'break-word' }}>
                    {desc}
                  </div>
                </div>
              </div>
            );
          })}

          {/* Load more */}
          {rows.length >= offset && rows.length > 0 && (
            <div style={{ textAlign: 'center', padding: '10px 0' }}>
              <button onClick={loadMore} disabled={loading} style={{
                padding: '4px 16px', fontSize: 12,
                border: '1px solid rgba(160,180,255,0.3)', color: '#a0b4ff',
                background: 'transparent', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
                opacity: loading ? 0.5 : 1,
              }}>
                {loading ? text(uiLang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' }) : text(uiLang as LangCode, { tr: `Daha fazla yükle (${rows.length}/${total})`, en: `Load more (${rows.length}/${total})`, de: `Mehr laden (${rows.length}/${total})`, fr: `Charger plus (${rows.length}/${total})`, ar: `تحميل المزيد (${rows.length}/${total})` })}
              </button>
            </div>
          )}
        </div>

        {/* Footer count */}
        <div style={{ padding: '6px 14px', borderTop: '1px solid rgba(160,180,255,0.08)', flexShrink: 0 }}>
          <span style={{ fontSize: 12, color: '#4a6a6a', fontFamily: 'Share Tech Mono, monospace' }}>
            {filtered.length} / {rows.length} {text(uiLang as LangCode, { tr: 'gösteriliyor', en: 'shown', de: 'angezeigt', fr: 'affichés', ar: 'معروض' })} · {total} {text(uiLang as LangCode, { tr: 'toplam kayıt', en: 'total records', de: 'Einträge gesamt', fr: 'entrées total', ar: 'المجموع' })}
          </span>
        </div>
      </div>
    </div>
  );
}

// ── Main Panel ───────────────────────────────────────────────────────────────

export default function LanguagePanel() {
  const { stats, events, lang, currentSim, accessToken } = useSimStore();
  const [archiveOpen, setArchiveOpen] = useState(false);

  const currentStage = Math.max(0, Math.min(6, stats?.max_language_stage ?? 0));
  const langEvents = events.filter(e => {
    const t = String(e.event_type ?? '');
    return t === 'language' || t === 'word' || t === 'communication' || t.includes('language');
  });
  const worldState = currentSim?.world_state;
  const phonologySeed = Number(worldState?.phonology_seed ?? 0);
  const biome = String(worldState?.biome ?? 'mediterranean');
  const enginePalette = worldState?.phoneme_palette;
  const hasEnginePhonology = Boolean(enginePalette?.consonants?.length && enginePalette?.vowels?.length);
  const phonology = hasEnginePhonology
    ? phonologyFromPalette(enginePalette!.consonants!, enginePalette!.vowels!)
    : buildPhonology(phonologySeed, biome);
  const surfaceForms = buildSurfaceForms(phonology, currentStage);
  const nextStage = currentStage < LANGUAGE_STAGES.length - 1 ? LANGUAGE_STAGES[currentStage + 1] : null;

  return (
    <DetailPanel panelId="language" title="Language" titleTr="Dil">

      {archiveOpen && currentSim && accessToken && (
        <LangArchiveModal
          simId={currentSim.id}
          accessToken={accessToken}
          lang={lang}
          onClose={() => setArchiveOpen(false)}
        />
      )}

      <div className="bg-sim-surface rounded-lg p-3 mb-2">
        <div className="text-sim-muted text-sm mb-1">{text(lang as LangCode, { tr: 'Mevcut Aşama', en: 'Current Stage', de: 'Aktuelle Stufe', fr: 'Étape actuelle', ar: 'المرحلة الحالية' })}</div>
        <div className="text-sim-gold font-bold text-base">
          {text(lang as LangCode, { en: `Stage ${currentStage}`, tr: `Aşama ${currentStage}`, de: `Stufe ${currentStage}`, fr: `Étape ${currentStage}`, ar: `المرحلة ${currentStage}` })}: {text(lang as LangCode, { en: LANGUAGE_STAGES[currentStage].name, tr: LANGUAGE_STAGES[currentStage].nameTr, de: LANGUAGE_STAGES[currentStage].nameDe, fr: LANGUAGE_STAGES[currentStage].nameFr, ar: LANGUAGE_STAGES[currentStage].nameAr })}
        </div>
        <div className="text-sim-muted text-sm mt-1">{text(lang as LangCode, { en: LANGUAGE_STAGES[currentStage].desc, tr: LANGUAGE_STAGES[currentStage].descTr, de: LANGUAGE_STAGES[currentStage].descDe, fr: LANGUAGE_STAGES[currentStage].descFr, ar: LANGUAGE_STAGES[currentStage].descAr })}</div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-3">
        <div className="bg-sim-surface/70 rounded-lg p-3 border border-sim-border/40">
          <div className="text-sim-muted text-xs uppercase tracking-widest mb-2">
            {hasEnginePhonology
              ? text(lang as LangCode, { tr: 'Gerçek Ses Envanteri', en: 'Actual Phonology', de: 'Phonologie', fr: 'Phonologie réelle', ar: 'علم الأصوات الفعلي' })
              : text(lang as LangCode, { tr: 'Örnek Ses Envanteri', en: 'Illustrative Phonology', de: 'Beispielhafte Phonologie', fr: 'Phonologie indicative', ar: 'علم أصوات توضيحي' })}
          </div>
          <div className="text-xs text-sim-text mb-1">{text(lang as LangCode, { tr: 'Ünsüzler', en: 'Consonants', de: 'Konsonanten', fr: 'Consonnes', ar: 'الحروف الساكنة' })}</div>
          <div className="flex flex-wrap gap-1 mb-2">
            {phonology.consonants.map(item => (
              <span key={item} className="px-2 py-0.5 rounded bg-sim-accent/10 text-sim-text text-xs border border-sim-accent/20">{item}</span>
            ))}
          </div>
          <div className="text-xs text-sim-text mb-1">{text(lang as LangCode, { tr: 'Ünlüler', en: 'Vowels', de: 'Vokale', fr: 'Voyelles', ar: 'حروف العلة' })}</div>
          <div className="flex flex-wrap gap-1">
            {phonology.vowels.map(item => (
              <span key={item} className="px-2 py-0.5 rounded bg-sim-gold/10 text-sim-text text-xs border border-sim-gold/20">{item}</span>
            ))}
          </div>
        </div>

        <div className="bg-sim-surface/70 rounded-lg p-3 border border-sim-border/40">
          <div className="text-sim-muted text-xs uppercase tracking-widest mb-2">{text(lang as LangCode, { tr: 'Aşama Kanalı', en: 'Stage Channel', de: 'Stufenkanal', fr: 'Canal de stade', ar: 'قناة المرحلة' })}</div>
          <div className="text-sim-text text-sm mb-2">
            {text(lang as LangCode, { tr: LANGUAGE_STAGES[currentStage].nameTr, en: LANGUAGE_STAGES[currentStage].name, de: LANGUAGE_STAGES[currentStage].nameDe, fr: LANGUAGE_STAGES[currentStage].nameFr, ar: LANGUAGE_STAGES[currentStage].nameAr })}
          </div>
          <div className="text-sim-muted text-xs leading-relaxed">
            {text(lang as LangCode, { tr: LANGUAGE_STAGES[currentStage].descTr, en: LANGUAGE_STAGES[currentStage].desc, de: LANGUAGE_STAGES[currentStage].descDe, fr: LANGUAGE_STAGES[currentStage].descFr, ar: LANGUAGE_STAGES[currentStage].descAr })}
          </div>
        </div>

        <div className="bg-sim-surface/70 rounded-lg p-3 border border-sim-border/40">
          <div className="text-sim-muted text-xs uppercase tracking-widest mb-2">{text(lang as LangCode, { tr: 'Yüzey Biçimleri', en: 'Surface Forms', de: 'Oberflächenformen', fr: 'Formes de surface', ar: 'الأشكال السطحية' })}</div>
          <div className="flex flex-wrap gap-1">
            {surfaceForms.length === 0 ? (
              <span className="text-sim-muted text-xs italic">
                {text(lang as LangCode, { tr: 'Henüz kalıcı ses biçimi yok.', en: 'No stable surface forms yet.', de: 'Noch keine stabilen Oberflächenformen.', fr: 'Pas encore de formes de surface stables.', ar: 'لا توجد أشكال سطحية مستقرة بعد.' })}
              </span>
            ) : surfaceForms.map(item => (
              <span key={item} className="px-2 py-0.5 rounded bg-sim-green/10 text-sim-text text-xs border border-sim-green/20">{item}</span>
            ))}
          </div>
        </div>
      </div>

      <div className="bg-sim-surface/50 rounded-lg p-3 mb-3 border border-sim-border/30">
        <div className="text-sim-muted text-xs uppercase tracking-widest mb-1">
          {text(lang as LangCode, { tr: 'Bu Aşamanın Yapabildikleri', en: 'What This Stage Can Do', de: 'Was diese Stufe kann', fr: 'Ce que ce stade peut faire', ar: 'ما يمكن لهذه المرحلة فعله' })}
        </div>
        <div className="text-sim-text text-sm leading-relaxed">
          {text(lang as LangCode, { en: LANGUAGE_STAGES[currentStage].desc, tr: LANGUAGE_STAGES[currentStage].descTr, de: LANGUAGE_STAGES[currentStage].descDe, fr: LANGUAGE_STAGES[currentStage].descFr, ar: LANGUAGE_STAGES[currentStage].descAr })}
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Aşama İlerlemesi', en: 'Stage Progression', de: 'Stufenentwicklung', fr: 'Progression des stades', ar: 'تقدم المراحل' })}
        </h4>
        <div className="space-y-2">
          {LANGUAGE_STAGES.map(stage => {
            const isReached = stage.id <= currentStage;
            const isCurrent = stage.id === currentStage;
            return (
              <div key={stage.id} className={`flex items-start gap-3 p-2 rounded ${isCurrent ? 'bg-sim-accent/20 border border-sim-accent/40' : isReached ? 'bg-sim-surface/50' : 'opacity-40'}`}>
                <div className="w-2 h-2 rounded-full mt-1 flex-shrink-0" style={{ backgroundColor: isReached ? stage.color : '#5a6a7a' }} />
                <div>
                  <div className={`text-sm font-medium ${isReached ? 'text-sim-text' : 'text-sim-muted'}`}>
                    {text(lang as LangCode, { en: stage.name, tr: stage.nameTr, de: stage.nameDe, fr: stage.nameFr, ar: stage.nameAr })}
                  </div>
                  <div className="text-sim-muted text-sm">{text(lang as LangCode, { tr: stage.descTr, en: stage.desc, de: stage.descDe, fr: stage.descFr, ar: stage.descAr })}</div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {nextStage && (
        <div className="bg-sim-surface/50 rounded-lg p-3 mb-3 border border-sim-border/30">
          <div className="text-sim-muted text-xs uppercase tracking-widest mb-1">
            {text(lang as LangCode, { en: 'Next Unlock', tr: 'Sonraki Açılım', de: 'Nächste Freischaltung', fr: 'Prochain déverrouillage', ar: 'الفتح التالي' })}
          </div>
          <div className="text-sim-text text-sm">
            {text(lang as LangCode, { en: nextStage.name, tr: nextStage.nameTr, de: nextStage.nameDe, fr: nextStage.nameFr, ar: nextStage.nameAr })}: {text(lang as LangCode, { en: nextStage.desc, tr: nextStage.descTr, de: nextStage.descDe, fr: nextStage.descFr, ar: nextStage.descAr })}
          </div>
        </div>
      )}

      {stats && stats.groups > 1 && (
        <div className="bg-sim-surface/50 rounded-lg p-3 mb-3 border border-sim-border/30">
          <div className="text-sim-muted text-xs uppercase tracking-widest mb-2">
            {text(lang as LangCode, { en: 'Dialect Divergence', tr: 'Lehçe Farklılaşması', de: 'Dialektdivergenz', fr: 'Divergence dialectale', ar: 'تباعد اللهجات' })}
          </div>
          <div className="flex items-center gap-3 mb-1.5">
            <div className="flex-1">
              <div className="h-1.5 bg-sim-border rounded-full overflow-hidden">
                <div className="h-full rounded-full" style={{ width: `${Math.min(100, (stats.groups / 10) * 100)}%`, background: 'linear-gradient(90deg, #4f6ef7, #9370db)' }} />
              </div>
            </div>
            <span className="text-sim-text text-xs font-medium">{stats.groups} {text(lang as LangCode, { en: 'groups', tr: 'grup', de: 'Gruppen', fr: 'groupes', ar: 'مجموعات' })}</span>
          </div>
          <p className="text-sim-muted text-xs leading-relaxed">
            {text(lang as LangCode, {
              en: `${stats.word_count ?? 0} unique words across ${stats.groups} groups. Isolated groups develop distinct phonological shifts over generations.`,
              tr: `${stats.word_count ?? 0} benzersiz kelime ${stats.groups} gruba dağılmış. İzole gruplar nesiller boyunca farklı ses değişimleri geliştirir.`,
              de: `${stats.word_count ?? 0} einzigartige Wörter über ${stats.groups} Gruppen verteilt. Isolierte Gruppen entwickeln über Generationen hinweg unterschiedliche Lautverschiebungen.`,
              fr: `${stats.word_count ?? 0} mots uniques répartis sur ${stats.groups} groupes. Les groupes isolés développent des mutations phonologiques distinctes au fil des générations.`,
              ar: `${stats.word_count ?? 0} كلمة فريدة موزعة على ${stats.groups} مجموعة. تطوّر المجموعات المعزولة تحولات صوتية متمايزة عبر الأجيال.`,
            })}
          </p>
        </div>
      )}

      {/* Stream + Archive button */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
          <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest" style={{ margin: 0 }}>
            {text(lang as LangCode, { en: 'Live Stream', tr: 'Canlı Akış', de: 'Live-Stream', fr: 'Flux en direct', ar: 'البث المباشر' })}
            <span style={{ fontSize: 12, fontWeight: 400, color: '#6a8878', marginLeft: 6 }}>({langEvents.length})</span>
          </h4>
          {currentSim && accessToken && (
            <button onClick={() => setArchiveOpen(true)} style={{
              padding: '2px 10px', fontSize: 12,
              border: '1px solid rgba(160,180,255,0.35)', color: '#a0b4ff',
              background: 'rgba(160,180,255,0.07)', fontFamily: 'Share Tech Mono, monospace',
              cursor: 'pointer', letterSpacing: '0.05em',
            }}>
              📋 {text(lang as LangCode, { en: 'ARCHIVE', tr: 'ARŞİV', de: 'ARCHIV', fr: 'ARCHIVES', ar: 'الأرشيف' })}
            </button>
          )}
        </div>

        {langEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">{text(lang as LangCode, { en: 'No language events yet.', tr: 'Henüz dil olayı yok.', de: 'Noch keine Sprachereignisse.', fr: 'Pas encore d\'événements linguistiques.', ar: 'لا أحداث لغوية بعد.' })}</p>
        ) : (
          <div className="space-y-1.5 max-h-64 overflow-y-auto pr-1">
            {langEvents.slice(0, 30).map((ev, i) => {
              const t = String(ev.event_type ?? '');
              const color = evColor(t);
              const icon  = evIcon(t);
              const desc  = translateEventDescription(ev.description ?? '', lang as LangCode, ev);
              return (
                <div key={i} style={{
                  display: 'flex', gap: 6, alignItems: 'flex-start',
                  padding: '4px 6px',
                  background: i === 0 ? `${color}0a` : 'transparent',
                  border: `1px solid ${i < 3 ? color + '22' : 'transparent'}`,
                  opacity: Math.max(0.35, 1 - i * 0.015),
                }}>
                  <span style={{ fontSize: 12, flexShrink: 0, color, marginTop: 1 }}>{icon}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: 'flex', gap: 5, marginBottom: 1, alignItems: 'center' }}>
                      <span style={{ fontSize: 12, color: '#6a8878', fontFamily: 'Orbitron, monospace', flexShrink: 0 }}>
                        Y{String(ev.sim_year ?? 0).padStart(3,'0')} G{String((ev.sim_day ?? 0) % 365).padStart(3,'0')}
                      </span>
                      <span style={{ fontSize: 12, color: `${color}88`, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                        {evLabel(t, lang)}
                      </span>
                    </div>
                    <div style={{ fontSize: 12, color: i < 5 ? color : '#8abda0', lineHeight: 1.45, wordBreak: 'break-word' }}>
                      {desc}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
