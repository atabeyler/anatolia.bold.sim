import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Music } from 'lucide-react';
import { text, translateEventDescription, type LangCode } from '../../utils/i18n';

const ART_CATEGORIES = [
  {
    medium: 'visual',
    en: 'Visual Art', tr: 'Görsel Sanat', de: 'Visuelle Kunst', fr: 'Art visuel', ar: 'الفن البصري',
    emoji: '🎨',
    items: [
      { en: 'Cave Painting',      tr: 'Mağara Resmi',     de: 'Höhlenmalerei',         fr: 'Peinture rupestre',     ar: 'رسم الكهف' },
      { en: 'Sculpture',          tr: 'Heykelcilik',      de: 'Skulptur',              fr: 'Sculpture',             ar: 'النحت' },
      { en: 'Pottery Decoration', tr: 'Çömlek Süslemesi', de: 'Töpferdekoration',      fr: 'Décoration de poterie', ar: 'زخرفة الفخار' },
      { en: 'Textile Pattern',    tr: 'Dokuma Deseni',    de: 'Textilmuster',          fr: 'Motif textile',         ar: 'نمط نسيج' },
      { en: 'Architecture Art',   tr: 'Mimari Sanatı',   de: 'Architekturkunst',      fr: 'Art architectural',     ar: 'الفن المعماري' },
    ],
  },
  {
    medium: 'music',
    en: 'Music', tr: 'Müzik', de: 'Musik', fr: 'Musique', ar: 'الموسيقى',
    emoji: '🎵',
    items: [
      { en: 'Rhythmic Percussion', tr: 'Ritimli Vurma', de: 'Rhythmisches Schlagzeug', fr: 'Percussion rythmique', ar: 'الإيقاع' },
      { en: 'Vocal Melody',        tr: 'Sesli Melodi',  de: 'Gesangsmelodie',          fr: 'Mélodie vocale',       ar: 'لحن صوتي' },
      { en: 'Bone Flute',          tr: 'Kemik Flüt',   de: 'Knochenflöte',            fr: 'Flûte en os',          ar: 'ناي العظم' },
      { en: 'String Instrument',   tr: 'Telli Çalgı',  de: 'Saiteninstrument',        fr: 'Instrument à cordes',  ar: 'آلة وترية' },
    ],
  },
  {
    medium: 'narrative',
    en: 'Narrative', tr: 'Anlatı', de: 'Erzählung', fr: 'Narration', ar: 'السرد',
    emoji: '📖',
    items: [
      { en: 'Oral Story',    tr: 'Sözlü Hikâye',  de: 'Mündliche Geschichte',  fr: 'Histoire orale',  ar: 'قصة شفهية' },
      { en: 'Epic Poem',     tr: 'Epik Şiir',     de: 'Epos',                  fr: 'Poème épique',    ar: 'قصيدة ملحمية' },
      { en: 'Written Story', tr: 'Yazılı Hikâye', de: 'Schriftliche Geschichte',fr: 'Histoire écrite', ar: 'قصة مكتوبة' },
    ],
  },
];

export default function ArtPanel() {
  const { events, stats, lang } = useSimStore();

  const artEvents = events.filter(e => e.event_type === 'art');
  const totalForms = (stats as any)?.art_forms ?? artEvents.length;

  return (
    <DetailPanel panelId="art" title="Art & Music" titleTr="Sanat & Müzik" titleDe="Kunst & Musik" titleFr="Art & Musique" titleAr="الفن والموسيقى">
      <div className="bg-sim-surface rounded-lg p-3 mb-3 flex items-center gap-3">
        <Music size={24} className="text-pink-400" />
        <div>
          <div className="text-pink-400 font-bold text-lg">{totalForms}</div>
          <div className="text-sim-muted text-sm">
            {text(lang as LangCode, { en: 'Art Forms Discovered', tr: 'Keşfedilen Sanat Formları', de: 'Kunstformen entdeckt', fr: 'Formes artistiques découvertes', ar: 'أشكال فنية مكتشفة' })}
          </div>
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Emergence Requirements', tr: 'Ortaya Çıkış Gereksinimleri', de: 'Entstehungsvoraussetzungen', fr: 'Conditions d\'émergence', ar: 'متطلبات الظهور' })}
        </h4>
        <p className="text-sim-muted text-sm italic">
          {text(lang as LangCode, {
            en: 'Art requires food surplus + artistic_sense gene × intelligence > threshold. Higher forms need cognitive prerequisites.',
            tr: 'Sanat; gıda fazlası + artistik_duyarlılık geni × zekâ > eşik gerektirir. Yüksek formlar bilişsel önkoşullar ister.',
            de: 'Kunst erfordert Nahrungsüberschuss + Kunstsinn-Gen × Intelligenz > Schwelle. Höhere Formen benötigen kognitive Voraussetzungen.',
            fr: 'L\'art requiert surplus alimentaire + gène sens_artistique × intelligence > seuil. Les formes supérieures nécessitent des prérequis cognitifs.',
            ar: 'الفن يتطلب فائض غذائي + جين الحس الفني × الذكاء > العتبة. الأشكال الأعلى تحتاج متطلبات إدراكية.',
          })}
        </p>
      </div>

      {ART_CATEGORIES.map(cat => {
        const discovered = artEvents.filter(e =>
          cat.items.some(item => e.description?.toLowerCase().includes(item.en.toLowerCase()))
        );
        return (
          <div key={cat.medium} className="mb-3">
            <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 flex items-center gap-1.5">
              <span>{cat.emoji}</span>
              <span>{text(lang as LangCode, { en: cat.en, tr: cat.tr, de: cat.de, fr: cat.fr, ar: cat.ar })}</span>
              <span className="text-sim-muted font-normal normal-case tracking-normal">
                ({discovered.length}/{cat.items.length})
              </span>
            </h4>
            <div className="space-y-0.5">
              {cat.items.map(item => {
                const isDiscovered = discovered.some(e => e.description?.toLowerCase().includes(item.en.toLowerCase()));
                return (
                  <div
                    key={item.en}
                    className={`text-sm px-2 py-1 rounded ${isDiscovered ? 'text-sim-text bg-pink-500/10' : 'text-sim-muted opacity-40'}`}
                  >
                    {isDiscovered ? '✓' : '○'} {text(lang as LangCode, { en: item.en, tr: item.tr, de: item.de, fr: item.fr, ar: item.ar })}
                  </div>
                );
              })}
            </div>
          </div>
        );
      })}

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Art Events', tr: 'Sanat Olayları', de: 'Kunstereignisse', fr: 'Événements artistiques', ar: 'أحداث فنية' })}
        </h4>
        {artEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {text(lang as LangCode, { en: 'No art events yet.', tr: 'Henüz sanat olayı yok.', de: 'Noch keine Kunstereignisse.', fr: 'Pas encore d\'événements artistiques.', ar: 'لا أحداث فنية بعد.' })}
          </p>
        ) : (
          <div className="space-y-1 max-h-36 overflow-y-auto">
            {artEvents.slice(0, 8).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-pink-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
