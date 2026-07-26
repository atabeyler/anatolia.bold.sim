import { translateEventDescription, type LangCode, text } from '../../utils/i18n';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Building2 } from 'lucide-react';

const STRUCTURE_ICONS: Record<string, string> = {
  cave_dwelling: '🪨', lean_to: '⛺', pit_house: '🏠', post_frame_hut: '🛖',
  storage_pit: '🪣', mud_brick_house: '🧱', granary: '🌾', defensive_wall: '🏰',
  stone_temple: '⛩️', stone_house: '🏚️', marketplace: '🏪', city_wall: '🧱',
};

const STRUCTURE_NAMES: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  cave_dwelling: { tr: 'Mağara Konutu', en: 'Cave Dwelling', de: 'Höhlenwohnung', fr: 'Habitation troglodytique', ar: 'سكن كهفي' },
  lean_to: { tr: 'Sığınak', en: 'Lean-To', de: 'Notunterstand', fr: 'Abris', ar: 'ملجأ بسيط' },
  pit_house: { tr: 'Çukur Ev', en: 'Pit House', de: 'Grubenhaus', fr: 'Maison semi-enterrée', ar: 'بيت محفور' },
  post_frame_hut: { tr: 'Direkli Kulübe', en: 'Post-Frame Hut', de: 'Pfostenhütte', fr: 'Hutte à poteaux', ar: 'كوخ بأعمدة' },
  storage_pit: { tr: 'Depo Çukuru', en: 'Storage Pit', de: 'Lagergrube', fr: 'Fosse de stockage', ar: 'حفرة تخزين' },
  mud_brick_house: { tr: 'Kerpiç Ev', en: 'Mud-Brick House', de: 'Lehmziegelhaus', fr: 'Maison en adobe', ar: 'بيت لبِن' },
  granary: { tr: 'Tahıl Ambarı', en: 'Granary', de: 'Getreidespeicher', fr: 'Grenier', ar: 'مخزن حبوب' },
  defensive_wall: { tr: 'Savunma Duvarı', en: 'Defensive Wall', de: 'Schutzwall', fr: 'Mur défensif', ar: 'جدار دفاعي' },
  stone_temple: { tr: 'Taş Tapınak', en: 'Stone Temple', de: 'Steintempel', fr: 'Temple de pierre', ar: 'معبد حجري' },
  stone_house: { tr: 'Taş Ev', en: 'Stone House', de: 'Steinhaus', fr: 'Maison de pierre', ar: 'بيت حجري' },
  marketplace: { tr: 'Pazar Yeri', en: 'Marketplace', de: 'Marktplatz', fr: 'Marché', ar: 'سوق' },
  city_wall: { tr: 'Şehir Surları', en: 'City Wall', de: 'Stadtmauer', fr: 'Muraille de ville', ar: 'سور المدينة' },
};

const TIER_LABELS: Record<number, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  0: { tr: 'Doğal', en: 'Natural', de: 'Natürlich', fr: 'Naturel', ar: 'طبيعي' },
  1: { tr: 'Basit', en: 'Simple', de: 'Einfach', fr: 'Simple', ar: 'بسيط' },
  2: { tr: 'Kalıcı', en: 'Permanent', de: 'Dauerhaft', fr: 'Permanent', ar: 'دائم' },
  3: { tr: 'Kentsel', en: 'Urban', de: 'Städtisch', fr: 'Urbain', ar: 'حضري' },
};

function t(lang: string, value: { tr: string; en: string; de: string; fr: string; ar: string }) {
  return text(lang as LangCode, value);
}

export default function ArchitecturePanel() {
  const { events, lang } = useSimStore();

  const archEvents = events.filter(e => e.event_type === 'structure_built');
  const builtStructures = archEvents.map(e => (e as any).structure_type ?? null).filter(Boolean) as string[];
  const uniqueStructures = [...new Set(builtStructures)];

  return (
    <DetailPanel panelId="architecture" title="Architecture" titleTr="Mimari" titleDe="Architektur" titleFr="Architecture" titleAr="العمارة">
      <div className="bg-sim-surface rounded-lg p-3 mb-3 flex items-center gap-3">
        <Building2 size={24} className="text-orange-400" />
        <div>
          <div className="text-orange-400 font-bold text-lg">{uniqueStructures.length}</div>
          <div className="text-sim-muted text-sm">
            {t(lang, { tr: 'İnşa Edilen Yapı Türleri', en: 'Structure Types Built', de: 'Gebaute Strukturtypen', fr: 'Types de structures construites', ar: 'أنواع المنشآت المبنية' })}
          </div>
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t(lang, { tr: 'İnşa Edilmiş Yapılar', en: 'Built Structures', de: 'Errichtete Strukturen', fr: 'Structures construites', ar: 'المنشآت المبنية' })}
        </h4>
        {uniqueStructures.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t(lang, { tr: 'Henüz yapı inşa edilmedi.', en: 'No structures built yet.', de: 'Noch keine Strukturen gebaut.', fr: 'Aucune structure construite pour le moment.', ar: 'لم يتم بناء أي منشآت بعد.' })}
          </p>
        ) : (
          <div className="grid grid-cols-2 gap-1">
            {uniqueStructures.map(s => (
              <div key={s} className="flex items-center gap-1.5 bg-sim-surface rounded p-1.5">
                <span>{STRUCTURE_ICONS[s] ?? '🏗️'}</span>
                <span className="text-sm text-sim-text capitalize">
                  {t(lang, STRUCTURE_NAMES[s] ?? { tr: s.replace(/_/g, ' '), en: s.replace(/_/g, ' '), de: s.replace(/_/g, ' '), fr: s.replace(/_/g, ' '), ar: s.replace(/_/g, ' ') })}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t(lang, { tr: 'Mevcut Yapılar', en: 'Available Structures', de: 'Verfügbare Strukturen', fr: 'Structures disponibles', ar: 'المنشآت المتاحة' })}
        </h4>
        <div className="space-y-2">
          {[0, 1, 2, 3].map(tier => (
            <div key={tier}>
              <div className="text-sm text-sim-muted mb-1">
                {t(lang, { tr: `Seviye ${tier}: ${TIER_LABELS[tier].tr}`, en: `Tier ${tier}: ${TIER_LABELS[tier].en}`, de: `Stufe ${tier}: ${TIER_LABELS[tier].de}`, fr: `Niveau ${tier}: ${TIER_LABELS[tier].fr}`, ar: `المستوى ${tier}: ${TIER_LABELS[tier].ar}` })}
              </div>
              <div className="flex flex-wrap gap-1">
                {Object.entries(STRUCTURE_ICONS)
                  .slice(tier * 3, tier * 3 + 3)
                  .map(([id, icon]) => (
                    <div key={id} className="text-sm bg-sim-surface/50 rounded px-1.5 py-0.5 text-sim-muted">
                      {icon} {t(lang, STRUCTURE_NAMES[id] ?? { tr: id.replace(/_/g, ' '), en: id.replace(/_/g, ' '), de: id.replace(/_/g, ' '), fr: id.replace(/_/g, ' '), ar: id.replace(/_/g, ' ') })}
                    </div>
                  ))}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t(lang, { tr: 'İnşaat Günlüğü', en: 'Construction Log', de: 'Bautagebuch', fr: 'Journal de construction', ar: 'سجل البناء' })}
        </h4>
        {archEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t(lang, { tr: 'Henüz inşaat yok.', en: 'No construction yet.', de: 'Noch keine Bauaktivität.', fr: 'Aucune construction pour le moment.', ar: 'لا يوجد بناء بعد.' })}
          </p>
        ) : (
          <div className="space-y-1 max-h-36 overflow-y-auto">
            {archEvents.slice(0, 8).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-orange-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
