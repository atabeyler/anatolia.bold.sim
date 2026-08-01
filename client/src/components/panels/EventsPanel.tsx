import { useEffect, useMemo, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, translateEventDescription, translateEventType, type LangCode } from '../../utils/i18n';

const FILTERS = [
  { id: 'all',        tr: 'Tümü',     en: 'All',       de: 'Alle',         fr: 'Tous',         ar: 'الكل',       color: '#a0c8b0' },
  { id: 'birth',      tr: 'Doğum',    en: 'Birth',     de: 'Geburt',       fr: 'Naissance',    ar: 'ولادة',      color: '#7aff9a' },
  { id: 'death',      tr: 'Ölüm',     en: 'Death',     de: 'Tod',          fr: 'Mort',         ar: 'وفاة',       color: '#e08080' },
  { id: 'technology', tr: 'Teknoloji',en: 'Tech',      de: 'Technol.',     fr: 'Technol.',     ar: 'تقنية',      color: '#7dd3fc' },
  { id: 'language',   tr: 'Dil',      en: 'Language',  de: 'Sprache',      fr: 'Langue',       ar: 'لغة',        color: '#a0b4ff' },
  { id: 'discovery',  tr: 'Keşif',    en: 'Discovery', de: 'Entdeckung',   fr: 'Découverte',   ar: 'اكتشاف',     color: '#d4a838' },
  { id: 'disaster',   tr: 'Afet',     en: 'Disaster',  de: 'Katastrophe',  fr: 'Catastrophe',  ar: 'كارثة',      color: '#f97316' },
  { id: 'belief',     tr: 'İnanç',    en: 'Belief',    de: 'Glaube',       fr: 'Croyance',     ar: 'معتقد',      color: '#a855f7' },
  { id: 'culture',    tr: 'Kültür',   en: 'Culture',   de: 'Kultur',       fr: 'Culture',      ar: 'ثقافة',      color: '#c084fc' },
  { id: 'activity',   tr: 'Aktivite', en: 'Activity',  de: 'Aktivität',    fr: 'Activité',     ar: 'نشاط',       color: '#f59e0b' },
];

function evColor(type: string) {
  if (type?.includes('birth'))      return '#7aff9a';
  if (type?.includes('death'))      return '#e08080';
  if (type?.includes('technology')) return '#7dd3fc';
  if (type?.includes('language') || type?.includes('word') || type === 'communication') return '#a0b4ff';
  if (type?.includes('discovery'))  return '#d4a838';
  if (type?.includes('disaster'))   return '#f97316';
  if (type?.includes('belief'))     return '#a855f7';
  if (type?.includes('cultural') || type === 'norm_emerged' || type === 'norm_violation') return '#c084fc';
  if (type === 'thought' || type === 'sleep' || type === 'mating' || type === 'activity') return '#f59e0b';
  return '#8abda0';
}

function evIcon(type: string) {
  if (type?.includes('birth'))      return '+';
  if (type?.includes('death'))      return '†';
  if (type?.includes('technology')) return '⚙';
  if (type?.includes('language') || type?.includes('word') || type === 'communication') return '🔤';
  if (type?.includes('discovery'))  return '◆';
  if (type?.includes('disaster'))   return '⚠';
  if (type?.includes('belief'))     return '☽';
  if (type?.includes('cultural') || type?.includes('culture') || type === 'norm_emerged') return '◈';
  if (type === 'thought') return '◎';
  if (type === 'sleep')   return '◐';
  if (type === 'mating')  return '♦';
  return '·';
}

function matchesCategory(ev: any, id: string) {
  const type = String(ev?.event_type ?? '').toLowerCase();
  switch (id) {
    case 'birth':      return type === 'birth';
    case 'death':      return type === 'death';
    case 'technology': return type === 'technology';
    case 'language':   return type === 'language' || type === 'word' || type === 'communication' || type.includes('language');
    case 'discovery':  return type === 'discovery' || type.includes('discovery');
    case 'disaster':   return type === 'disaster' || type === 'epidemic' || type === 'epidemic_outbreak' || type.includes('disaster');
    case 'belief':     return type === 'belief' || type === 'ritual' || type.includes('belief');
    case 'culture':    return type === 'culture' || type.includes('cultural') || type === 'norm_emerged' || type === 'norm_violation';
    case 'activity':   return type === 'communication' || type === 'thought' || type === 'sleep' || type === 'mating' || type === 'activity';
    default:           return false;
  }
}

// Maps category id → event_type values for the API types filter
function categoryToTypes(id: string): string {
  switch (id) {
    case 'birth':      return 'birth';
    case 'death':      return 'death';
    case 'technology': return 'technology';
    case 'language':   return 'language,word,communication';
    case 'discovery':  return 'discovery';
    case 'disaster':   return 'disaster,epidemic,epidemic_outbreak';
    case 'belief':     return 'belief,ritual';
    case 'culture':    return 'culture,cultural_meme_emerged,cultural_diffusion,norm_emerged,norm_violation';
    case 'activity':   return 'communication,thought,sleep,mating,activity';
    default:           return '';
  }
}

// ── Events Archive Modal ─────────────────────────────────────────────────────

const PAGE = 100;

function EventsArchiveModal({ simId, accessToken, lang: uiLang, initialFilter, onClose }: {
  simId: string; accessToken: string; lang: string; initialFilter: string; onClose: () => void;
}) {
  const [rows,    setRows]    = useState<any[]>([]);
  const [total,   setTotal]   = useState(0);
  const [offset,  setOffset]  = useState(0);
  const [filter,  setFilter]  = useState(initialFilter);
  const [search,  setSearch]  = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setRows([]);
    setOffset(0);
    const headers = { Authorization: `Bearer ${accessToken}` };
    const types = categoryToTypes(filter);
    const params: any = { limit: PAGE, offset: 0 };
    if (types) params.types = types;

    // Fetch count from summary
    axios.get(`/api/simulations/${simId}/events/summary`, { headers })
      .then(r => {
        if (cancelled) return;
        const c = r.data?.countsByType ?? {};
        if (filter === 'all') {
          setTotal(r.data?.total ?? 0);
        } else {
          const fakeEvts = Object.entries(c).filter(([t]) => matchesCategory({ event_type: t }, filter));
          setTotal(fakeEvts.reduce((s, [, n]) => s + (n as number), 0));
        }
      }).catch(() => {});

    axios.get(`/api/simulations/${simId}/events`, { headers, params })
      .then(r => {
        if (cancelled) return;
        setRows(r.data ?? []);
        setOffset(PAGE);
      }).catch(() => {}).finally(() => { if (!cancelled) setLoading(false); });

    return () => { cancelled = true; };
  }, [filter, simId, accessToken]);

  async function loadMore() {
    if (loading) return;
    setLoading(true);
    const headers = { Authorization: `Bearer ${accessToken}` };
    const types = categoryToTypes(filter);
    const params: any = { limit: PAGE, offset };
    if (types) params.types = types;
    try {
      const r = await axios.get(`/api/simulations/${simId}/events`, { headers, params });
      setRows(prev => [...prev, ...(r.data ?? [])]);
      setOffset(o => o + PAGE);
    } finally { setLoading(false); }
  }

  const filtered = search
    ? rows.filter(ev => String(ev.description ?? '').toLowerCase().includes(search.toLowerCase()))
    : rows;

  const activeFilter = FILTERS.find(f => f.id === filter) ?? FILTERS[0];

  return (
    <div
      style={{ position: 'fixed', inset: 0, zIndex: 60, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.75)', backdropFilter: 'blur(4px)' }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={{ width: 680, maxWidth: '96vw', maxHeight: '90vh', display: 'flex', flexDirection: 'column', background: 'rgba(4,4,18,0.98)', border: `1px solid ${activeFilter.color}33`, boxShadow: `0 20px 60px rgba(0,0,0,0.85), 0 0 40px ${activeFilter.color}08` }}>

        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '10px 14px', borderBottom: `1px solid ${activeFilter.color}18`, flexShrink: 0 }}>
          <span style={{ fontSize: 13, fontFamily: 'Orbitron, monospace', color: activeFilter.color, fontWeight: 700, letterSpacing: '0.1em', flex: 1 }}>
            📋 {text(uiLang as LangCode, { tr: 'OLAY KAYDI ARŞİVİ', en: 'EVENT LOG ARCHIVE', de: 'EREIGNISPROTOKOLL-ARCHIV', fr: 'ARCHIVE DES ÉVÉNEMENTS', ar: 'أرشيف سجل الأحداث' })}
          </span>
          <span style={{ fontSize: 12, color: '#6a8878', fontFamily: 'Share Tech Mono, monospace' }}>
            {total > 0 ? `${total.toLocaleString()} ${text(uiLang as LangCode, { tr: 'kayıt', en: 'records', de: 'Einträge', fr: 'entrées', ar: 'سجلات' })}` : ''}
          </span>
          <button onClick={onClose} style={{ color: '#6a8878', background: 'none', border: 'none', cursor: 'pointer', fontSize: 16, lineHeight: 1, padding: '0 4px' }}>✕</button>
        </div>

        {/* Category filters */}
        <div style={{ padding: '6px 14px 0', borderBottom: `1px solid rgba(160,200,176,0.08)`, flexShrink: 0 }}>
          <div style={{ display: 'flex', gap: 3, flexWrap: 'wrap', paddingBottom: 6 }}>
            {FILTERS.map(f => (
              <button key={f.id} onClick={() => setFilter(f.id)} style={{
                padding: '2px 7px', fontSize: 12,
                border: `1px solid ${filter === f.id ? f.color : 'rgba(160,200,176,0.2)'}`,
                color: filter === f.id ? f.color : '#6a8878',
                background: filter === f.id ? `${f.color}12` : 'transparent',
                fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
              }}>
                {text(uiLang as LangCode, { tr: f.tr, en: f.en, de: f.de, fr: f.fr, ar: f.ar })}
              </button>
            ))}
          </div>
          {/* Search */}
          <input
            value={search} onChange={e => setSearch(e.target.value)}
            placeholder={text(uiLang as LangCode, { tr: 'Açıklamada ara…', en: 'Search descriptions…', de: 'Beschreibungen suchen…', fr: 'Rechercher dans les descriptions…', ar: 'ابحث في الأوصاف…' })}
            style={{
              width: '100%', boxSizing: 'border-box', marginBottom: 6, padding: '3px 8px', fontSize: 12,
              background: 'rgba(160,200,176,0.04)', border: '1px solid rgba(160,200,176,0.15)',
              color: '#a0c8b0', fontFamily: 'Share Tech Mono, monospace', outline: 'none',
            }}
          />
        </div>

        {/* Event list */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '4px 14px' }}>
          {loading && filtered.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 0', color: '#6a8878', fontSize: 12, fontFamily: 'Share Tech Mono, monospace' }}>
              {text(uiLang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' })}
            </div>
          ) : filtered.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 0', color: '#6a8878', fontSize: 12, fontStyle: 'italic' }}>
              {text(uiLang as LangCode, { tr: 'Kayıt bulunamadı.', en: 'No records found.', de: 'Keine Einträge gefunden.', fr: 'Aucune entrée trouvée.', ar: 'لا توجد سجلات.' })}
            </div>
          ) : filtered.map((ev, i) => {
            const color = evColor(ev.event_type);
            const icon  = evIcon(ev.event_type);
            const desc  = translateEventDescription(ev.description ?? '', uiLang as LangCode, ev);
            return (
              <div key={ev.id ?? i} style={{
                display: 'flex', gap: 8, alignItems: 'flex-start',
                padding: '5px 4px', borderBottom: '1px solid rgba(160,200,176,0.05)',
              }}>
                <span style={{ fontSize: 12, color, flexShrink: 0, marginTop: 2 }}>{icon}</span>
                <div style={{ flexShrink: 0, width: 72 }}>
                  <div style={{ fontSize: 12, color: '#4a6a6a', fontFamily: 'Orbitron, monospace' }}>
                    Y{String(ev.sim_year ?? 0).padStart(4,'0')}
                  </div>
                  <div style={{ fontSize: 12, color: '#3a5a5a', fontFamily: 'Orbitron, monospace' }}>
                    G{String((ev.sim_day ?? 0) % 365).padStart(3,'0')}
                  </div>
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <span style={{ fontSize: 12, color: `${color}88`, letterSpacing: '0.06em', textTransform: 'uppercase', display: 'block', marginBottom: 2 }}>
                    {translateEventType(ev.event_type, uiLang as LangCode)}
                  </span>
                  <div style={{ fontSize: 12, color: '#a0c8b0', lineHeight: 1.5, wordBreak: 'break-word' }}>
                    {desc}
                  </div>
                </div>
              </div>
            );
          })}

          {/* Load more */}
          {rows.length > 0 && rows.length < total && (
            <div style={{ textAlign: 'center', padding: '10px 0' }}>
              <button onClick={loadMore} disabled={loading} style={{
                padding: '4px 16px', fontSize: 12,
                border: `1px solid ${activeFilter.color}55`, color: activeFilter.color,
                background: 'transparent', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
                opacity: loading ? 0.5 : 1,
              }}>
                {loading
                  ? text(uiLang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' })
                  : text(uiLang as LangCode, { tr: `Daha fazla yükle (${rows.length.toLocaleString()} / ${total.toLocaleString()})`, en: `Load more (${rows.length.toLocaleString()} / ${total.toLocaleString()})`, de: `Mehr laden (${rows.length.toLocaleString()} / ${total.toLocaleString()})`, fr: `Charger plus (${rows.length.toLocaleString()} / ${total.toLocaleString()})`, ar: `تحميل المزيد (${rows.length.toLocaleString()} / ${total.toLocaleString()})` })}
              </button>
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{ padding: '5px 14px', borderTop: `1px solid rgba(160,200,176,0.08)`, flexShrink: 0 }}>
          <span style={{ fontSize: 12, color: '#3a5a5a', fontFamily: 'Share Tech Mono, monospace' }}>
            {filtered.length.toLocaleString()} / {rows.length.toLocaleString()} {text(uiLang as LangCode, { tr: 'gösteriliyor', en: 'shown', de: 'angezeigt', fr: 'affichés', ar: 'معروض' })} · {total.toLocaleString()} {text(uiLang as LangCode, { tr: 'toplam', en: 'total', de: 'gesamt', fr: 'total', ar: 'المجموع' })}
          </span>
        </div>
      </div>
    </div>
  );
}

// ── Main Panel ───────────────────────────────────────────────────────────────

export default function EventsPanel() {
  const { events, lang, currentSim, accessToken, stats } = useSimStore();
  const [filter, setFilter] = useState('all');
  const [summaryCounts, setSummaryCounts] = useState<Record<string, number>>({});
  const [summaryTotal, setSummaryTotal] = useState(0);
  const [archiveOpen, setArchiveOpen] = useState(false);

  useEffect(() => {
    if (!currentSim || !accessToken) {
      setSummaryCounts({});
      setSummaryTotal(0);
      return;
    }
    let active = true;
    const headers = { Authorization: `Bearer ${accessToken}` };
    const load = () => axios.get(`/api/simulations/${currentSim.id}/events/summary`, { headers })
      .then(r => {
        if (!active) return;
        setSummaryCounts(r.data?.countsByType ?? {});
        setSummaryTotal(r.data?.total ?? 0);
      })
      .catch(() => { setSummaryCounts({}); });
    load();
    // Poll less aggressively — birth/death now come from WebSocket stats
    const timer = setInterval(load, 15000);
    return () => { active = false; clearInterval(timer); };
  }, [currentSim?.id, accessToken]);

  const visible = filter === 'all'
    ? events
    : events.filter(ev => matchesCategory(ev, filter));

  const counts = useMemo(() => {
    const next: Record<string, number> = {};
    for (const f of FILTERS.slice(1)) next[f.id] = 0;
    // birth/death: use authoritative WebSocket stats for consistency with TopBar and PopulationPanel
    next['birth'] = stats?.births ?? 0;
    next['death'] = stats?.deaths ?? 0;
    // other categories: derive from DB summary (less time-sensitive)
    for (const [eventType, count] of Object.entries(summaryCounts)) {
      const lower = eventType.toLowerCase();
      if (lower === 'birth' || lower === 'death') continue;
      const fakeEvent = { event_type: lower };
      for (const f of FILTERS.slice(1)) {
        if (f.id === 'birth' || f.id === 'death') continue;
        if (matchesCategory(fakeEvent, f.id)) next[f.id] += count;
      }
    }
    return next;
  }, [summaryCounts, stats?.births, stats?.deaths]);

  const activeFilterObj = FILTERS.find(f => f.id === filter) ?? FILTERS[0];

  return (
    <DetailPanel panelId="olaylar" title="Event Log" titleTr="Olay Kaydı" titleDe="Ereignisprotokoll" titleFr="Journal des événements" titleAr="سجل الأحداث">

      {archiveOpen && currentSim && accessToken && (
        <EventsArchiveModal
          simId={currentSim.id}
          accessToken={accessToken}
          lang={lang}
          initialFilter={filter}
          onClose={() => setArchiveOpen(false)}
        />
      )}

      {/* Summary row */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 3, marginBottom: 8 }}>
        {FILTERS.slice(1).map(f => (
          <div key={f.id} style={{ background: 'rgba(15,0,0,0.7)', border: `1px solid ${f.color}22`, padding: '3px 5px' }}>
            <div style={{ fontSize: 12, color: f.color, letterSpacing: '0.08em', opacity: 0.7, whiteSpace: 'nowrap', overflow: 'hidden' }}>
              {text(lang as LangCode, { tr: f.tr.toUpperCase(), en: f.en.toUpperCase(), de: f.de?.toUpperCase(), fr: f.fr?.toUpperCase(), ar: f.ar })}
            </div>
            <div style={{ fontSize: 14, color: f.color, fontFamily: 'Orbitron, monospace', fontWeight: 700, lineHeight: 1 }}>
              {counts[f.id] ?? 0}
            </div>
          </div>
        ))}
      </div>

      {/* Filter chips */}
      <div style={{ display: 'flex', gap: 3, flexWrap: 'wrap', marginBottom: 8 }}>
        {FILTERS.map(f => (
          <button key={f.id} onClick={() => setFilter(f.id)}
            style={{
              padding: '2px 6px', fontSize: 12,
              border: `1px solid ${filter === f.id ? f.color : 'rgba(160,200,176,0.3)'}`,
              color: filter === f.id ? f.color : '#8abda0',
              background: filter === f.id ? `${f.color}14` : 'transparent',
              fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer',
            }}>
            {text(lang as LangCode, { tr: f.tr, en: f.en, de: f.de, fr: f.fr, ar: f.ar })}
            {f.id !== 'all' && ` ${counts[f.id] ?? 0}`}
          </button>
        ))}
      </div>

      {/* Total + Archive button */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
        <span style={{ fontSize: 12, color: '#8abda0', letterSpacing: '0.06em' }}>
          {visible.length} / {summaryTotal || events.length} {text(lang as LangCode, { tr: 'olay', en: 'events', de: 'Ereignisse', fr: 'événements', ar: 'أحداث' })}
        </span>
        {currentSim && accessToken && (
          <button onClick={() => setArchiveOpen(true)} style={{
            padding: '2px 10px', fontSize: 12,
            border: `1px solid ${activeFilterObj.color}55`,
            color: activeFilterObj.color,
            background: `${activeFilterObj.color}08`,
            fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', letterSpacing: '0.05em',
          }}>
            📋 {text(lang as LangCode, { tr: 'ARŞİV', en: 'ARCHIVE', de: 'ARCHIV', fr: 'ARCHIVE', ar: 'الأرشيف' })}
          </button>
        )}
      </div>

      {/* Event list */}
      {visible.length === 0 ? (
        <div style={{ fontSize: 12, color: '#8abda0', textAlign: 'center', padding: '24px 0', fontStyle: 'italic' }}>
          {text(lang as LangCode, { tr: 'Olay bulunamadı.', en: 'No events found.', de: 'Keine Ereignisse gefunden.', fr: 'Aucun événement trouvé.', ar: 'لم يتم العثور على أحداث.' })}
        </div>
      ) : visible.map((ev, i) => {
        const color = evColor(ev.event_type);
        const icon = evIcon(ev.event_type);
        const rawDesc = ev.description ?? ev.event_type;
        const desc = translateEventDescription(rawDesc, lang as LangCode, ev);
        return (
          <div key={i} style={{
            display: 'flex', gap: 6, alignItems: 'flex-start',
            padding: '5px 6px', marginBottom: 2,
            background: i === 0 ? `${color}0c` : 'transparent',
            border: `1px solid ${i < 3 ? color + '28' : 'rgba(80,40,40,0.3)'}`,
            opacity: Math.max(0.35, 1 - i * 0.012),
          }}>
            <span style={{ fontSize: 12, flexShrink: 0, lineHeight: 1.1, color, marginTop: 1 }}>{icon}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: 'flex', gap: 6, marginBottom: 2, alignItems: 'center' }}>
                <span style={{ fontSize: 12, color: '#8abda0', fontFamily: 'Orbitron, monospace', flexShrink: 0 }}>
                  Y{String(ev.sim_year).padStart(4, '0')} G{String(ev.sim_day % 365).padStart(3, '0')}
                </span>
                <span style={{ fontSize: 12, color: `${color}88`, letterSpacing: '0.06em', textTransform: 'uppercase' }}>
                  {translateEventType(ev.event_type, lang as LangCode)}
                </span>
              </div>
              <div style={{ fontSize: 12, color: i < 5 ? color : '#a0c8b0', lineHeight: 1.45, wordBreak: 'break-word' }}>
                {desc}
              </div>
            </div>
          </div>
        );
      })}
    </DetailPanel>
  );
}
