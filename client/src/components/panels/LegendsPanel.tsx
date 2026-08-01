import { useEffect, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

interface LegendEntry {
  id: string;
  name: string;
  sex: string;
  birth_year: number;
  death_year: number | null;
  alive: boolean;
  is_founder: boolean;
  value: number;
}

interface LegendsResponse {
  highest_consciousness: LegendEntry | null;
  most_children: LegendEntry | null;
  longest_lived: LegendEntry | null;
  highest_reputation: LegendEntry | null;
  most_technologies: LegendEntry | null;
}

const CATEGORIES: { key: keyof LegendsResponse; icon: string; title: { tr: string; en: string; de: string; fr: string; ar: string }; format: (v: number, lang: LangCode) => string }[] = [
  {
    key: 'highest_consciousness',
    icon: '✨',
    title: { tr: 'En Yüksek Bilinç', en: 'Highest Consciousness', de: 'Höchstes Bewusstsein', fr: 'Conscience la plus élevée', ar: 'أعلى وعي' },
    format: v => `${Math.round(v * 100)}%`,
  },
  {
    key: 'most_children',
    icon: '👶',
    title: { tr: 'En Çok Çocuk', en: 'Most Children', de: 'Meiste Kinder', fr: "Le plus d'enfants", ar: 'أكثر عدد أطفال' },
    format: (v, lang) => `${v} ${text(lang, { tr: 'çocuk', en: 'children', de: 'Kinder', fr: 'enfants', ar: 'أطفال' })}`,
  },
  {
    key: 'longest_lived',
    icon: '⏳',
    title: { tr: 'En Uzun Yaşayan', en: 'Longest-Lived', de: 'Am längsten Gelebt', fr: 'Le plus longévif', ar: 'الأطول عمراً' },
    format: (v, lang) => `${v} ${text(lang, { tr: 'yıl', en: 'years', de: 'Jahre', fr: 'ans', ar: 'سنة' })}`,
  },
  {
    key: 'highest_reputation',
    icon: '👑',
    title: { tr: 'En Yüksek İtibar', en: 'Highest Reputation', de: 'Höchstes Ansehen', fr: 'Réputation la plus élevée', ar: 'أعلى سمعة' },
    format: v => `${Math.round(v * 100)}%`,
  },
  {
    key: 'most_technologies',
    icon: '⚙',
    title: { tr: 'En Çok Keşif Yapan', en: 'Most Prolific Discoverer', de: 'Produktivster Entdecker', fr: 'Découvreur le plus prolifique', ar: 'الأكثر اكتشافاً' },
    format: (v, lang) => `${v} ${text(lang, { tr: 'keşif', en: 'discoveries', de: 'Entdeckungen', fr: 'découvertes', ar: 'اكتشافات' })}`,
  },
];

export default function LegendsPanel() {
  const { currentSim, accessToken, lang, activePanel } = useSimStore();
  const [legends, setLegends] = useState<LegendsResponse | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (activePanel !== 'legends' || !currentSim || !accessToken) return;
    setLoading(true);
    axios
      .get<LegendsResponse>(`/api/simulations/${currentSim.id}/legends`, { headers: { Authorization: `Bearer ${accessToken}` } })
      .then(r => setLegends(r.data))
      .catch(() => setLegends(null))
      .finally(() => setLoading(false));
  }, [activePanel, currentSim?.id, accessToken]);

  return (
    <DetailPanel panelId="legends" title="Legends" titleTr="Efsaneler" titleDe="Legenden" titleFr="Légendes" titleAr="الأساطير">
      <p className="text-sim-muted text-sm italic mb-3">
        {text(lang as LangCode, {
          tr: 'Kalabalık nüfusun içinde kaybolan, gerçekten istisnai bireyler. Her kategori, o alanda hâlâ rekor sahibi olan kişiyi gösterir.',
          en: 'The truly exceptional individuals otherwise lost in a large population. Each category shows the current record holder.',
          de: 'Die wirklich außergewöhnlichen Individuen, die sonst in einer großen Bevölkerung verloren gehen. Jede Kategorie zeigt den aktuellen Rekordhalter.',
          fr: "Les individus véritablement exceptionnels autrement perdus dans une population nombreuse. Chaque catégorie montre le détenteur du record actuel.",
          ar: 'الأفراد الاستثنائيون حقاً الذين يضيعون وسط عدد كبير من السكان. تعرض كل فئة صاحب الرقم القياسي الحالي.',
        })}
      </p>

      {loading && (
        <div className="text-sim-muted text-sm text-center py-6">
          {text(lang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' })}
        </div>
      )}

      {!loading && legends && (
        <div className="space-y-2">
          {CATEGORIES.map(cat => {
            const entry = legends[cat.key];
            return (
              <div key={cat.key} className="bg-sim-surface rounded-lg p-3 border border-sim-border/40">
                <div className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-1 flex items-center gap-1.5">
                  <span>{cat.icon}</span>
                  <span>{text(lang as LangCode, cat.title)}</span>
                </div>
                {entry ? (
                  <div className="w-full flex items-center justify-between">
                    <div>
                      <div className="text-sim-text text-sm font-medium">{entry.name}</div>
                      <div className="text-sim-muted text-xs">
                        {entry.is_founder
                          ? text(lang as LangCode, { tr: 'Kurucu', en: 'Founder', de: 'Gründer', fr: 'Fondateur', ar: 'مؤسس' })
                          : `${entry.sex === 'male' ? text(lang as LangCode, { tr: 'E', en: 'M', de: 'M', fr: 'M', ar: 'ذ' }) : text(lang as LangCode, { tr: 'K', en: 'F', de: 'W', fr: 'F', ar: 'أ' })} · ${text(lang as LangCode, { tr: `Yıl ${entry.birth_year}`, en: `Year ${entry.birth_year}` })}`}
                        {!entry.alive && entry.death_year !== null ? ` – ${entry.death_year}` : ''}
                      </div>
                    </div>
                    <div className="font-orbitron font-bold text-base" style={{ color: '#4ecb71' }}>{cat.format(entry.value, lang as LangCode)}</div>
                  </div>
                ) : (
                  <div className="text-sim-muted text-sm italic">
                    {text(lang as LangCode, { tr: 'Henüz kimse yok.', en: 'No one yet.', de: 'Noch niemand.', fr: 'Personne encore.', ar: 'لا أحد بعد.' })}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </DetailPanel>
  );
}
