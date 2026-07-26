import { useEffect, useState, useMemo } from 'react';
import axios from 'axios';
import { translateEventDescription, text, type LangCode } from '../../utils/i18n';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Telescope } from 'lucide-react';

const KNOWLEDGE_ITEMS = [
  { id: 'lunar_tracking',     label: { tr: 'Ay Takibi',         en: 'Lunar Tracking',     de: 'Mondverfolgung',        fr: 'Suivi lunaire',          ar: 'تتبع القمر' },        icon: '🌙' },
  { id: 'seasonal_calendar',  label: { tr: 'Mevsimsel Takvim',  en: 'Seasonal Calendar',  de: 'Saisonkalender',        fr: 'Calendrier saisonnier',  ar: 'التقويم الموسمي' },   icon: '📆' },
  { id: 'star_map',           label: { tr: 'Yıldız Haritası',   en: 'Star Map',           de: 'Sternenkarte',          fr: 'Carte des étoiles',      ar: 'خريطة النجوم' },      icon: '⭐' },
  { id: 'eclipse_prediction', label: { tr: 'Tutulma Tahmini',   en: 'Eclipse Prediction', de: 'Finsternisvorhersage',  fr: "Prédiction d'éclipse",   ar: 'التنبؤ بالكسوف' },    icon: '🌑' },
  { id: 'planetary_model',    label: { tr: 'Gezegen Modeli',    en: 'Planetary Model',    de: 'Planetenmodell',        fr: 'Modèle planétaire',      ar: 'النموذج الكوكبي' },   icon: '🪐' },
];

type AstroEvent = {
  sim_day: number;
  sim_year: number;
  event_type: string;
  description: string;
  importance: number;
  data?: Record<string, any>;
};

function eventKey(e: AstroEvent) {
  return `${e.sim_day}_${e.data?.knowledge_id ?? e.data?.event_id ?? e.description}`;
}

export default function AstronomyPanel() {
  const { events, stats, lang, currentSim, accessToken } = useSimStore();
  const [dbEvents, setDbEvents] = useState<AstroEvent[]>([]);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [discoverers, setDiscoverers] = useState<Record<string, string>>({});

  const L = lang as LangCode;
  const t = (tr: string, en: string, de = en, fr = en, ar = en) =>
    text(L, { tr, en, de, fr, ar });

  // Fetch full astronomy history from DB on mount
  useEffect(() => {
    if (!currentSim || !accessToken) return;
    axios.get(`/api/simulations/${currentSim.id}/events`, {
      headers: { Authorization: `Bearer ${accessToken}` },
      params: { types: 'astronomy', limit: 1000 },
    }).then(res => setDbEvents(res.data ?? [])).catch(() => {});
  }, [currentSim?.id, accessToken]);

  // Live store events (may include events not yet checkpointed to DB)
  const liveAstro = useMemo(
    () => events.filter(e => e.event_type === 'astronomy'),
    [events]
  );

  // Merge DB + live, deduplicated, sorted newest first
  const allAstro = useMemo(() => {
    const seen = new Set<string>();
    const merged: AstroEvent[] = [];
    for (const e of [...liveAstro, ...dbEvents]) {
      const k = eventKey(e);
      if (!seen.has(k)) { seen.add(k); merged.push(e); }
    }
    merged.sort((a, b) => b.sim_day - a.sim_day);
    return merged;
  }, [liveAstro, dbEvents]);

  const discoveries = useMemo(
    () => allAstro.filter(e => e.data?.type === 'astronomy_discovery'),
    [allAstro]
  );
  const observations = useMemo(
    () => allAstro.filter(e => e.data?.type === 'celestial_observation'),
    [allAstro]
  );

  // Fetch discoverer names for known discovery events
  useEffect(() => {
    if (!currentSim || !accessToken) return;
    const ids = discoveries
      .map(e => e.data?.discoverer_id)
      .filter((id): id is string => !!id && !discoverers[id]);
    for (const id of ids) {
      axios.get(`/api/simulations/${currentSim.id}/population/${id}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      }).then(res => {
        const name = res.data?.phenotype?.name ?? res.data?.name ?? id.slice(-4).toUpperCase();
        setDiscoverers(prev => ({ ...prev, [id]: name }));
      }).catch(() => {});
    }
  }, [discoveries.length, currentSim?.id]);

  return (
    <DetailPanel panelId="astronomy" title="Astronomy" titleTr="Astronomi">

      {/* Header */}
      <div className="bg-sim-surface rounded-lg p-3 mb-3 flex items-center gap-3">
        <Telescope size={24} className="text-blue-400" />
        <div className="flex-1">
          <div className="text-blue-400 font-bold text-lg">
            {(stats as any)?.astronomy_knowledge ?? discoveries.length}
          </div>
          <div className="text-sim-muted text-sm">
            {t('Astronomik Keşifler', 'Astronomical Discoveries', 'Astronomische Entdeckungen', 'Découvertes astronomiques', 'اكتشافات فلكية')}
          </div>
        </div>
        <div className="text-right">
          <div className="text-sim-muted font-bold text-base">{observations.length}</div>
          <div className="text-sim-muted text-xs">
            {t('Gözlem', 'Observations', 'Beobachtungen', 'Observations', 'ملاحظات')}
          </div>
        </div>
      </div>

      {/* Knowledge tree */}
      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Bilgi Ağacı', 'Knowledge Tree', 'Wissensbaum', 'Arbre des connaissances', 'شجرة المعرفة')}
        </h4>
        <div className="space-y-1.5">
          {KNOWLEDGE_ITEMS.map(item => {
            const disc = discoveries.find(
              e => e.data?.knowledge_id === item.id ||
                   e.description?.toLowerCase().includes(item.id.replace(/_/g, ' '))
            );
            const discovered = !!disc;
            return (
              <div
                key={item.id}
                className={`flex items-center gap-2 p-2 rounded ${discovered ? 'bg-blue-500/15 border border-blue-500/30' : 'bg-sim-surface/30 opacity-50'}`}
              >
                <span className="text-base">{item.icon}</span>
                <span className={`text-sm flex-1 ${discovered ? 'text-sim-text' : 'text-sim-muted'}`}>
                  {text(L, item.label)}
                </span>
                {disc && (
                  <span className="text-sim-muted text-sm font-mono text-right">
                    <span>Y{disc.sim_year}</span>
                    {disc.data?.discoverer_id && discoverers[disc.data.discoverer_id] && (
                      <span className="block text-blue-300">{discoverers[disc.data.discoverer_id]}</span>
                    )}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Last 5 discoveries */}
      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Son Keşifler', 'Recent Discoveries', 'Neueste Entdeckungen', 'Découvertes récentes', 'أحدث الاكتشافات')}
        </h4>
        {discoveries.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t('Henüz keşif yok.', 'No discoveries yet.', 'Noch keine Entdeckungen.', 'Aucune découverte.', 'لا اكتشافات بعد.')}
          </p>
        ) : (
          <div className="space-y-1">
            {discoveries.slice(0, 5).map((ev, i) => {
              const discName = ev.data?.discoverer_id ? discoverers[ev.data.discoverer_id] : null;
              return (
                <div key={i} className="flex gap-2 items-start py-1 border-b border-sim-border/30">
                  <span className="text-blue-400 font-mono text-sm shrink-0 mt-0.5">Y{ev.sim_year}</span>
                  <div className="flex-1 min-w-0">
                    <div className="text-sim-text text-sm leading-snug">
                      {translateEventDescription(ev.description ?? '', L, ev)}
                    </div>
                    {discName && (
                      <div className="text-blue-300 text-xs mt-0.5">
                        {t('Keşfeden', 'Discovered by', 'Entdeckt von', 'Découvert par', 'اكتشف بواسطة')}: {discName}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Archive */}
      <div>
        <button
          onClick={() => setArchiveOpen(o => !o)}
          className="w-full flex items-center justify-between text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 hover:text-sim-text transition-colors"
        >
          <span>
            {t('Arşiv', 'Archive', 'Archiv', 'Archives', 'الأرشيف')} ({allAstro.length})
          </span>
          <span className="text-sim-muted">{archiveOpen ? '▲' : '▼'}</span>
        </button>
        {archiveOpen && (
          <div className="space-y-0.5 max-h-60 overflow-y-auto pr-1">
            {allAstro.length === 0 ? (
              <p className="text-sim-muted italic text-sm">
                {t('Kayıtlı astronomi olayı yok.', 'No astronomical events recorded.', 'Keine astronomischen Ereignisse aufgezeichnet.', 'Aucun événement astronomique enregistré.', 'لا توجد أحداث فلكية مسجلة.')}
              </p>
            ) : (
              allAstro.map((ev, i) => {
                const isDisc = ev.data?.type === 'astronomy_discovery';
                const discName = isDisc && ev.data?.discoverer_id ? discoverers[ev.data.discoverer_id] : null;
                return (
                  <div key={i} className="flex gap-2 items-start py-1 border-b border-sim-border/20">
                    <span className={`font-mono text-sm shrink-0 mt-0.5 ${isDisc ? 'text-blue-400' : 'text-sim-muted'}`}>
                      Y{ev.sim_year}
                    </span>
                    <div className="flex-1 min-w-0">
                      <div className={`text-sm leading-snug ${isDisc ? 'text-sim-text' : 'text-sim-muted'}`}>
                        {translateEventDescription(ev.description ?? '', L, ev)}
                      </div>
                      {discName && <div className="text-blue-300 text-sm">{discName}</div>}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>

    </DetailPanel>
  );
}
