import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Flame } from 'lucide-react';
import { translateEventDescription, text, describeBeliefCode, beliefCodeNumber, type LangCode } from '../../utils/i18n';

// Cardinal rule: belief archetypes are internal, opaque engine bucketing
// keys (see sim-core's belief.rs) -- never a real-world religion name, and
// never a made-up name either. A belief only gets a display label once the
// population's own language has actually coined one for it (a
// "belief_named" event, built from belief::try_label_belief); until then
// it renders as its bare opaque code (e.g. "#5") plus a short,
// mechanically-derived description (see describeBeliefCode in i18n.ts) so
// we can roughly tell what kind of belief it is without ever naming it
// ourselves. Colors are assigned by discovery order, not tied to any
// specific archetype.
const PALETTE = ['#6b8e23', '#8b7355', '#9370db', '#daa520', '#4682b4', '#cd853f', '#e07a5f', '#3d5a80'];

const BELIEF_EVENT_TYPES = new Set(['belief_formed', 'belief_spread', 'ritual_emerged', 'belief_named']);

export default function BeliefPanel() {
  const { events, lang } = useSimStore();
  const L = lang as LangCode;
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(L, { tr, en, de, fr, ar });

  const beliefEvents = events.filter(e => BELIEF_EVENT_TYPES.has(e.event_type));

  const labels: Record<string, string> = {};
  for (const ev of events) {
    if (ev.event_type === 'belief_named' && ev.data?.belief_id && ev.data?.label) {
      labels[ev.data.belief_id] = ev.data.label;
    }
  }

  const discoveredIds: string[] = [];
  for (const ev of events) {
    if (ev.event_type === 'belief_formed' && ev.data?.belief_id && !discoveredIds.includes(ev.data.belief_id)) {
      discoveredIds.push(ev.data.belief_id);
    }
  }

  return (
    <DetailPanel panelId="belief" title="Belief" titleTr="İnanç">
      <div className="bg-sim-surface rounded-lg p-3 mb-2 text-center">
        <Flame size={24} className="text-orange-400 mx-auto mb-1" />
        <div className="text-sim-gold font-bold text-lg">{discoveredIds.length}</div>
        <div className="text-sim-muted text-sm">
          {t('Ortaya çıkan inanç sistemleri', 'Belief systems emerged', 'Aufgetauchte Glaubenssysteme', 'Systèmes de croyances apparus', 'الأنظمة العقدية الناشئة')}
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Ortaya Çıkan İnançlar', 'Emerged Beliefs', 'Entstandene Glaubenssysteme', 'Croyances apparues', 'المعتقدات الناشئة')}
        </h4>
        {discoveredIds.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t('Henüz bir inanç oluşmadı.', 'No belief has formed yet.', 'Noch kein Glaube entstanden.', "Aucune croyance n'est encore apparue.", 'لم تتشكل أي معتقدات بعد.')}
          </p>
        ) : (
          <div className="space-y-1.5">
            {discoveredIds.map((id, i) => {
              const label = labels[id];
              return (
                <div key={id} className="p-2 rounded border border-sim-accent/40 bg-sim-accent/10">
                  <span className="text-sm font-medium" style={{ color: PALETTE[i % PALETTE.length] }}>
                    {label ?? `#${beliefCodeNumber(id)}`}
                  </span>
                  {!label && (
                    <p className="text-sim-muted text-xs mt-0.5">{describeBeliefCode(id, L)}</p>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Ortaya Çıkış Koşulları', 'Emergence Conditions', 'Entstehungsbedingungen', "Conditions d'émergence", 'شروط الظهور')}
        </h4>
        <p className="text-sim-muted text-sm italic">
          {t('İnanç; dindar gen + kaygı + çevre stresi şüphecilik eşiğini aştığında oluşur. Bir inanca isim ise ancak topluluğun dili buna izin verdiğinde (proto-kelime aşaması) ortaya çıkar.',
             'Belief forms when religiosity gene + anxiety + environmental stress overcome the skepticism threshold. A belief only gets a name once the population\'s own language allows one (proto-words stage).',
             'Glaube entsteht, wenn Religiosität + Angst + Umweltstress den Skeptizismuswert überschreiten. Ein Name entsteht erst, sobald die Sprache der Bevölkerung das zulässt (Proto-Wort-Stufe).',
             "La croyance se forme quand gène religiosité + anxiété + stress dépasse le seuil de scepticisme. Un nom n'apparaît que lorsque la langue de la population le permet (étape des proto-mots).",
             'تتشكل المعتقدات عندما يتجاوز جين التدين + القلق + الضغط البيئي عتبة الشك. لا يظهر اسم للمعتقد إلا عندما تسمح لغة المجتمع بذلك (مرحلة الكلمات الأولية).')}
        </p>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Ritüel Olayları', 'Ritual Events', 'Ritualereignisse', 'Événements rituels', 'أحداث طقسية')}
        </h4>
        {beliefEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t('Henüz inanç olayı yok.', 'No belief events yet.', 'Noch keine Glaubensereignisse.', "Pas encore d'événements de croyance.", 'لا أحداث عقدية بعد.')}
          </p>
        ) : (
          <div className="space-y-1 max-h-40 overflow-y-auto">
            {beliefEvents.slice(0, 8).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-orange-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', L, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
