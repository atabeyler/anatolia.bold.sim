import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { translateEventDescription, text, type LangCode } from '../../utils/i18n';

const MEME_STAGES: Record<string, number> = {
  shared_greeting: 1, mourning_ritual: 1, food_sharing_norm: 1,
  reciprocity_norm: 2, gender_roles: 2, age_hierarchy: 2, gift_exchange: 2,
  body_decoration: 3, storytelling: 3, music_drumming: 3, dance_ritual: 3, naming_ceremony: 3,
  marriage_ceremony: 4, seasonal_festival: 4, taboo_system: 4, trade_ceremony: 4,
  written_myth: 5, legal_code: 5,
};

const STAGE_COLORS = ['', '#6b8e23', '#4682b4', '#9370db', '#daa520', '#cd853f'];

export default function CulturePanel() {
  const { events, lang, stats } = useSimStore();
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });

  const cultureEvents = events.filter(e => e.event_type === 'culture' || e.event_type === 'ritual');
  const artEvents = events.filter(e => e.event_type === 'art');

  const totalMemes = cultureEvents.filter(e => e.event_type === 'culture').length;
  const totalArts = artEvents.length;
  const prestigePct = Math.round((stats?.avg_cultural_prestige ?? 0) * 100);
  // Backend meme descriptions (rust/sim-core/src/culture.rs's
  // meme_description) are unrelated prose, never containing the literal
  // id-derived phrase -- matching on data.meme_id (same pattern BeliefPanel
  // uses for data.belief_id) instead of substring-matching English text is
  // what actually finds emerged memes; the old substring check never
  // matched anything, so every stage bar below always read 0/N regardless
  // of real culture.
  const emergedMemeIds = new Set(cultureEvents.map(e => e.data?.meme_id).filter(Boolean));

  return (
    <DetailPanel panelId="culture" title="Culture" titleTr="Kültür">
      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <div className="text-purple-400 font-bold text-lg">{totalMemes}</div>
          <div className="text-sim-muted text-sm">{t('Kültürel Memler', 'Cultural Memes', 'Kulturelle Meme', 'Mèmes culturels', 'ميمات ثقافية')}</div>
        </div>
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <div className="text-pink-400 font-bold text-lg">{totalArts}</div>
          <div className="text-sim-muted text-sm">{t('Sanat Formları', 'Art Forms', 'Kunstformen', 'Formes d\'art', 'أشكال فنية')}</div>
        </div>
      </div>

      <div className="bg-sim-surface rounded-lg p-2 mb-3">
        <div className="flex items-center justify-between mb-1">
          <span className="text-sim-muted text-sm">
            {t('Kültürel Prestij', 'Cultural Prestige', 'Kulturelles Prestige', 'Prestige culturel', 'المكانة الثقافية')}
          </span>
          <span className="text-purple-400 font-bold text-sm">{prestigePct}%</span>
        </div>
        <div className="h-1.5 rounded-full overflow-hidden" style={{ background: 'rgba(168,85,247,0.15)' }}>
          <div className="h-full rounded-full transition-all" style={{ width: `${prestigePct}%`, background: '#a855f7' }} />
        </div>
        <div className="text-sim-muted text-xs mt-1">
          {t(
            'Yüksek prestijli gruplar diğer gruplara mem yayarken tercih ediliyor.',
            'Higher-prestige groups are preferentially copied from during cultural diffusion.',
            'Gruppen mit höherem Prestige werden bei kultureller Diffusion bevorzugt kopiert.',
            'Les groupes à plus fort prestige sont préférentiellement imités lors de la diffusion culturelle.',
            'يتم تقليد المجموعات ذات المكانة الأعلى تفضيلياً أثناء الانتشار الثقافي.'
          )}
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Mem Aşamaları', 'Meme Stages', 'Mem-Stufen', 'Stades des mèmes', 'مراحل الميمات')}
        </h4>
        {[1, 2, 3, 4, 5].map(stage => {
          const stageMemes = Object.entries(MEME_STAGES).filter(([, s]) => s === stage);
          const emerged = stageMemes.filter(([id]) => emergedMemeIds.has(id));
          return (
            <div key={stage} className="mb-2">
              <div className="flex items-center gap-2 mb-1">
                <div className="w-2 h-2 rounded-full" style={{ backgroundColor: STAGE_COLORS[stage] }} />
                <span className="text-sm text-sim-muted">
                  {t(`Aşama ${stage}`, `Stage ${stage}`, `Stufe ${stage}`, `Stade ${stage}`, `مرحلة ${stage}`)} ({emerged.length}/{stageMemes.length})
                </span>
              </div>
              <div className="h-1.5 bg-sim-border rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full"
                  style={{
                    width: `${(emerged.length / stageMemes.length) * 100}%`,
                    backgroundColor: STAGE_COLORS[stage],
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Kültür Olayları', 'Culture Events', 'Kulturereignisse', 'Événements culturels', 'أحداث ثقافية')}
        </h4>
        {cultureEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t('Henüz kültür olayı yok.', 'No culture events yet.', 'Noch keine Kulturereignisse.', 'Pas encore d\'événements culturels.', 'لا أحداث ثقافية بعد.')}
          </p>
        ) : (
          <div className="space-y-1 max-h-40 overflow-y-auto">
            {cultureEvents.slice(0, 10).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-purple-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
