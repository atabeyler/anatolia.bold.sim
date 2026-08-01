import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';
import { useSimStore } from '../../store/simStore';
import { useState, useEffect, useRef } from 'react';
import { BarChart2, X, GripHorizontal } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

type Metric = 'pop' | 'food' | 'water' | 'happiness' | 'consciousness' | 'lang';

const METRICS: { key: Metric; label: string; labelTr: string; labelDe: string; labelFr: string; labelAr: string; color: string }[] = [
  { key: 'pop',           label: 'Population',   labelTr: 'Nüfus',      labelDe: 'Bevölkerung',      labelFr: 'Population',       labelAr: 'السكان', color: '#4f6ef7' },
  { key: 'food',          label: 'Food',         labelTr: 'Besin',       labelDe: 'Nahrung',          labelFr: 'Nourriture',       labelAr: 'الغذاء', color: '#4ecb71' },
  { key: 'water',         label: 'Water',        labelTr: 'Su',          labelDe: 'Wasser',           labelFr: 'Eau',              labelAr: 'الماء', color: '#7dd3fc' },
  { key: 'happiness',     label: 'Happiness',    labelTr: 'Mutluluk',    labelDe: 'Glück',            labelFr: 'Bonheur',          labelAr: 'السعادة', color: '#ff8ab0' },
  { key: 'consciousness', label: 'Conscious.',    labelTr: 'Bilinç',      labelDe: 'Bewusstsein',      labelFr: 'Conscience',       labelAr: 'الوعي', color: '#c8b4ff' },
  { key: 'lang',          label: 'Language',     labelTr: 'Dil',         labelDe: 'Sprache',          labelFr: 'Langue',           labelAr: 'اللغة', color: '#00d4ff' },
];

function GaugeBar({ label, value, color }: { label: string; value: number; color: string }) {
  const pct = Math.min(100, value * 100);
  return (
    <div className="mb-1.5">
      <div className="flex justify-between items-center mb-0.5">
        <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color: '#a0c8b0', letterSpacing: '0.08em' }}>{label}</span>
        <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color }}>{(value * 100).toFixed(0)}%</span>
      </div>
      <div style={{ height: 3, background: 'rgba(255,255,255,0.08)', borderRadius: 2, overflow: 'hidden' }}>
        <div style={{ height: '100%', width: `${pct}%`, background: `linear-gradient(90deg, ${color}88, ${color})`, transition: 'width 0.7s ease', boxShadow: `0 0 4px ${color}80` }} />
      </div>
    </div>
  );
}

function useDrag(initial: { x: number; y: number }) {
  const [pos, setPos] = useState(initial);
  const dragging = useRef(false);
  const origin = useRef({ clientX: 0, clientY: 0, posX: 0, posY: 0 });
  const moved = useRef(false);

  useEffect(() => {
    function onMove(e: MouseEvent | TouchEvent) {
      if (!dragging.current) return;
      if ('touches' in e) e.preventDefault();
      const cx = 'touches' in e ? e.touches[0].clientX : (e as MouseEvent).clientX;
      const cy = 'touches' in e ? e.touches[0].clientY : (e as MouseEvent).clientY;
      const dx = cx - origin.current.clientX;
      const dy = cy - origin.current.clientY;
      if (Math.abs(dx) + Math.abs(dy) > 4) moved.current = true;
      setPos({ x: Math.max(0, origin.current.posX + dx), y: Math.max(0, origin.current.posY + dy) });
    }
    function onUp() { dragging.current = false; }
    function onResize() {
      setPos(p => ({
        x: Math.max(8, Math.min(window.innerWidth - 52, p.x)),
        y: Math.max(8, Math.min(window.innerHeight - 52, p.y)),
      }));
    }
    window.addEventListener('mousemove', onMove as any);
    window.addEventListener('mouseup', onUp);
    window.addEventListener('touchmove', onMove as any, { passive: false });
    window.addEventListener('touchend', onUp);
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('mousemove', onMove as any);
      window.removeEventListener('mouseup', onUp);
      window.removeEventListener('touchmove', onMove as any);
      window.removeEventListener('touchend', onUp);
      window.removeEventListener('resize', onResize);
    };
  }, []);

  function startDrag(clientX: number, clientY: number) {
    dragging.current = true;
    moved.current = false;
    origin.current = { clientX, clientY, posX: pos.x, posY: pos.y };
  }

  return { pos, startDrag, wasMoved: () => moved.current };
}

export default function StatsPanel() {
  const { stats, lang } = useSimStore();
  const [open, setOpen] = useState(false);
  const [activeMetrics, setActiveMetrics] = useState<Set<Metric>>(new Set(['pop', 'food']));
  const [history, setHistory] = useState<any[]>([]);
  const lastYear = useRef(-1);

  const fab = useDrag({ x: window.innerWidth - 60, y: window.innerHeight - 80 });
  const panel = useDrag({ x: Math.max(16, window.innerWidth - 320), y: Math.max(60, window.innerHeight - 480) });

  useEffect(() => {
    if (!stats || stats.year === lastYear.current) return;
    lastYear.current = stats.year;
    setHistory(h => [
      ...h,
      {
        year: stats.year,
        pop: stats.population,
        food: Math.round(stats.food_abundance * 100),
        water: Math.round((stats.water_abundance ?? 0) * 100),
        happiness: Math.round(((stats as any).happiness_index ?? 0) * 100),
        consciousness: Math.round(((stats as any).avg_consciousness ?? 0) * 100),
        lang: Math.round(((stats.max_language_stage ?? 0) / 6) * 100),
      },
    ].slice(-80));
  }, [stats?.year]);

  function toggleMetric(key: Metric) {
    setActiveMetrics(prev => {
      const next = new Set(prev);
      if (next.has(key) && next.size > 1) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  const panelW = Math.min(296, window.innerWidth - 32);
  const avgAge = stats?.avg_age ?? 0;
  const avgIntel = stats?.avg_intelligence ?? 0;
  const population = stats?.population ?? 0;

  return (
    <>
      {open && (
        <div style={{ position: 'fixed', inset: 0, zIndex: 38 }} onClick={() => setOpen(false)} />
      )}

      <div
        onClick={e => e.stopPropagation()}
        style={{
          position: 'fixed',
          left: panel.pos.x,
          top: panel.pos.y,
          zIndex: 39,
          width: panelW,
          background: 'rgba(2,6,4,0.96)',
          border: '1px solid rgba(79,110,247,0.4)',
          borderRadius: 4,
          backdropFilter: 'blur(16px)',
          boxShadow: '0 8px 40px rgba(0,0,0,0.7), 0 0 20px rgba(79,110,247,0.1)',
          padding: '12px 14px',
          fontFamily: 'Share Tech Mono, monospace',
          transform: open ? 'scale(1)' : 'scale(0.97)',
          opacity: open ? 1 : 0,
          pointerEvents: open ? 'auto' : 'none',
          transition: 'transform 0.22s cubic-bezier(0.22,1,0.36,1), opacity 0.18s ease',
        }}
      >
        <span style={{ position: 'absolute', top: -1, left: -1, width: 10, height: 10, borderTop: '2px solid rgba(79,110,247,0.8)', borderLeft: '2px solid rgba(79,110,247,0.8)' }} />
        <span style={{ position: 'absolute', bottom: -1, right: -1, width: 10, height: 10, borderBottom: '2px solid rgba(79,110,247,0.8)', borderRight: '2px solid rgba(79,110,247,0.8)' }} />

        <div
          onMouseDown={e => panel.startDrag(e.clientX, e.clientY)}
          onTouchStart={e => panel.startDrag(e.touches[0].clientX, e.touches[0].clientY)}
          style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 10, cursor: 'grab', userSelect: 'none' }}
        >
          <GripHorizontal size={12} style={{ color: '#6a9a80', flexShrink: 0 }} />
          <div style={{ width: 3, height: 14, background: '#4f6ef7', boxShadow: '0 0 6px rgba(79,110,247,0.8)', flexShrink: 0 }} />
          <span style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.22em', fontWeight: 700 }}>
            {text(lang as LangCode, { tr: 'NÜFUS TELEMETRİSİ', en: 'POPULATION TELEMETRY', de: 'BEVÖLKERUNGSTELEMETRIE', fr: 'TÉLÉMÉTRIE DE LA POPULATION', ar: 'قياس السكان' })}
          </span>
          <div style={{ flex: 1 }} />
          <div style={{ width: 6, height: 6, borderRadius: '50%', background: stats ? '#00e887' : '#a0c8b0', boxShadow: stats ? '0 0 6px #00e887' : 'none', animation: stats ? 'pulse 1.5s infinite' : 'none' }} />
          <span style={{ fontSize: 12, color: stats ? '#00e887' : '#a0c8b0', letterSpacing: '0.1em' }}>
            {text(lang as LangCode, { tr: 'CANLI', en: 'LIVE', de: 'LIVE', fr: 'EN DIRECT', ar: 'مباشر' })}
          </span>
          <button
            onClick={e => { e.stopPropagation(); setOpen(false); }}
            style={{ marginLeft: 6, background: 'transparent', border: 'none', color: '#a0c8b0', cursor: 'pointer', lineHeight: 0, padding: 2 }}
          >
            <X size={12} />
          </button>
        </div>

        {stats?.civilization_name && (
          <div style={{ textAlign: 'center', marginBottom: 8, padding: '4px 0', borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
            <span style={{ fontSize: 10, color: '#a0c8b0', letterSpacing: '0.15em' }}>
              {text(lang as LangCode, { tr: 'MEDENİYET', en: 'CIVILIZATION', de: 'ZIVILISATION', fr: 'CIVILISATION', ar: 'الحضارة' })}
            </span>
            <div style={{ fontSize: 16, color: '#d4a838', fontFamily: 'Orbitron, monospace', fontWeight: 700, letterSpacing: '0.05em' }}>
              {stats.civilization_name}
            </div>
          </div>
        )}

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 4, marginBottom: 8 }}>
          {METRICS.map(m => (
            <button key={m.key} onClick={() => toggleMetric(m.key)}
              style={{
                padding: '2px 0', fontSize: 11,
                border: `1px solid ${activeMetrics.has(m.key) ? m.color : 'rgba(160,200,176,0.3)'}`,
                color: activeMetrics.has(m.key) ? m.color : '#a0c8b0',
                background: activeMetrics.has(m.key) ? `${m.color}15` : 'transparent',
                cursor: 'pointer', borderRadius: 2, letterSpacing: '0.03em', transition: 'all 0.15s',
                fontFamily: 'Share Tech Mono, monospace',
              }}>
              {text(lang as LangCode, { tr: m.labelTr, en: m.label, de: m.labelDe, fr: m.labelFr, ar: m.labelAr })}
            </button>
          ))}
        </div>

        {history.length > 1 ? (
          <div style={{ marginBottom: 10 }}>
            <ResponsiveContainer width="100%" height={72}>
              <LineChart data={history}>
                <XAxis dataKey="year" hide />
                <YAxis hide />
                <Tooltip
                  contentStyle={{ background: '#07071a', border: '1px solid rgba(79,110,247,0.3)', borderRadius: 2, fontSize: 12, fontFamily: 'Share Tech Mono' }}
                  labelStyle={{ color: '#a0c8b0' }}
                  formatter={(val: any, name: string) => {
                    const m = METRICS.find(x => x.key === name);
                    return [name === 'pop' ? Number(val).toLocaleString() : `${val}%`, m ? text(lang as LangCode, { tr: m.labelTr, en: m.label, de: m.labelDe, fr: m.labelFr, ar: m.labelAr }) : name];
                  }}
                />
                {METRICS.filter(m => activeMetrics.has(m.key)).map(m => (
                  <Line key={m.key} type="monotone" dataKey={m.key} stroke={m.color} dot={false} strokeWidth={1.5} />
                ))}
              </LineChart>
            </ResponsiveContainer>
          </div>
        ) : (
          <div style={{ height: 72, display: 'flex', alignItems: 'center', justifyContent: 'center', marginBottom: 10 }}>
            <span style={{ fontSize: 12, color: 'rgba(160,200,180,0.7)', letterSpacing: '0.1em' }}>
              {text(lang as LangCode, { tr: 'VERİ BEKLENİYOR…', en: 'AWAITING DATA…', de: 'DATEN WARTEN…', fr: 'EN ATTENTE…', ar: 'في انتظار البيانات…' })}
            </span>
          </div>
        )}

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, marginBottom: 10 }}>
          {[
            { l: text(lang as LangCode, { tr: 'NÜFUS', en: 'POP', de: 'BEV.', fr: 'POP.', ar: 'السكان' }),      v: population.toLocaleString(), c: '#4f6ef7' },
            { l: text(lang as LangCode, { tr: 'ORT YAŞ', en: 'AGE', de: 'ALTER', fr: 'ÂGE', ar: 'العمر' }),    v: `${avgAge.toFixed(1)} yr`,   c: '#e0e0f0' },
            { l: text(lang as LangCode, { tr: 'ZEKA', en: 'INTEL', de: 'INTEL.', fr: 'INTEL.', ar: 'الذكاء' }), v: `${(avgIntel * 100).toFixed(0)}%`, c: '#d4a838' },
            { l: text(lang as LangCode, { tr: 'TEKNOLOJİ', en: 'TECH', de: 'TECH.', fr: 'TECH.', ar: 'التقنية' }), v: stats?.technologies ?? '—', c: '#4ecb71' },
          ].map(({ l, v, c }) => (
            <div key={l} style={{ background: 'rgba(4,4,15,0.8)', border: '1px solid rgba(255,255,255,0.07)', padding: '5px 8px', borderRadius: 2 }}>
              <div style={{ fontSize: 12, color: '#a0c8b0', letterSpacing: '0.1em', marginBottom: 2 }}>{l}</div>
              <div style={{ fontSize: 15, color: c, fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{v}</div>
            </div>
          ))}
        </div>

        <div style={{ borderTop: '1px solid rgba(255,255,255,0.06)', paddingTop: 8 }}>
          <GaugeBar label={text(lang as LangCode, { tr: 'BESİN', en: 'FOOD', de: 'NAHRUNG', fr: 'NOURRITURE', ar: 'الغذاء' })} value={stats?.food_abundance ?? 0} color="#4ecb71" />
          <GaugeBar label={text(lang as LangCode, { tr: 'SU', en: 'WATER', de: 'WASSER', fr: 'EAU', ar: 'الماء' })} value={stats?.water_abundance ?? 0} color="#7dd3fc" />
          <GaugeBar label={text(lang as LangCode, { tr: 'MUTLULUK', en: 'HAPPINESS', de: 'GLÜCK', fr: 'BONHEUR', ar: 'السعادة' })} value={(stats as any)?.happiness_index ?? 0} color="#ff8ab0" />
        </div>
      </div>

      <button
        onMouseDown={e => fab.startDrag(e.clientX, e.clientY)}
        onTouchStart={e => fab.startDrag(e.touches[0].clientX, e.touches[0].clientY)}
        onClick={() => { if (!fab.wasMoved()) setOpen(v => !v); }}
        style={{
          position: 'fixed',
          left: fab.pos.x,
          top: fab.pos.y,
          zIndex: 40,
          width: 44, height: 44, borderRadius: '50%',
          background: open ? 'rgba(79,110,247,0.25)' : 'rgba(0,232,135,0.15)',
          border: `1px solid ${open ? '#4f6ef7' : '#00e887'}`,
          color: open ? '#4f6ef7' : '#00e887',
          boxShadow: open
            ? '0 0 16px rgba(79,110,247,0.4), 0 4px 20px rgba(0,0,0,0.6)'
            : '0 0 16px rgba(0,232,135,0.3), 0 4px 20px rgba(0,0,0,0.6)',
          cursor: 'grab', display: 'flex', alignItems: 'center', justifyContent: 'center',
          transition: 'background 0.2s, border-color 0.2s, box-shadow 0.2s',
          backdropFilter: 'blur(8px)',
        }}
        title={text(lang as LangCode, { tr: 'Nüfus Telemetrisi', en: 'Population Telemetry', de: 'Bevölkerungstelemetrie', fr: 'Télémétrie de la population', ar: 'قياس السكان' })}
      >
        {open ? <X size={18} /> : <BarChart2 size={18} />}
      </button>
    </>
  );
}
