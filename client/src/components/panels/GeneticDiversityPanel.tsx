import { useEffect, useState } from 'react';
import axios from 'axios';
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

function t(lang: string, trStr: string, enStr: string, deStr = enStr, frStr = enStr, arStr = enStr) {
  return text(lang as LangCode, { tr: trStr, en: enStr, de: deStr, fr: frStr, ar: arStr });
}

interface CheckpointRow {
  sim_day: number;
  sim_year: number;
  stats?: {
    genetic_diversity?: {
      avg_heterozygosity: number;
      allelic_variance: number;
      effective_population_size: number;
      avg_inbreeding_coefficient: number;
    };
  };
}

export default function GeneticDiversityPanel() {
  const { currentSim, accessToken, lang, stats, activePanel } = useSimStore();
  const gd = stats?.genetic_diversity;
  const [history, setHistory] = useState<CheckpointRow[]>([]);

  useEffect(() => {
    if (activePanel !== 'genetics' || !currentSim || !accessToken) return;
    axios
      .get(`/api/simulations/${currentSim.id}/checkpoints`, { headers: { Authorization: `Bearer ${accessToken}` } })
      // list_checkpoints returns newest-first; the trend chart needs chronological order.
      .then(r => setHistory([...(r.data as CheckpointRow[])].reverse()))
      .catch(() => setHistory([]));
  }, [activePanel, currentSim?.id, accessToken]);

  const trend = history
    .filter(cp => cp.stats?.genetic_diversity)
    .map(cp => ({
      year: cp.sim_year,
      heterozygosity: Math.round((cp.stats!.genetic_diversity!.avg_heterozygosity ?? 0) * 1000) / 10,
      ne: cp.stats!.genetic_diversity!.effective_population_size ?? 0,
      inbreeding: Math.round((cp.stats!.genetic_diversity!.avg_inbreeding_coefficient ?? 0) * 1000) / 10,
    }));

  const statCard = (label: string, value: string, color: string) => (
    <div className="bg-sim-surface rounded-lg p-2 text-center">
      <div className="font-orbitron font-bold text-base" style={{ color }}>{value}</div>
      <div className="text-sim-muted text-sm">{label}</div>
    </div>
  );

  return (
    <DetailPanel panelId="genetics" title="Genetic Diversity" titleTr="Genetik Çeşitlilik" titleDe="Genetische Vielfalt" titleFr="Diversité génétique" titleAr="التنوع الجيني">
      <p className="text-sim-muted text-sm italic mb-3">
        {t(
          lang,
          'Nesiller boyunca gen havuzunun ne kadar çeşitli kaldığını (veya darboğaz/yakın akraba çiftleşmesiyle daraldığını) ölçer.',
          'Measures how diverse the gene pool has stayed across generations (or how much a bottleneck/inbreeding has narrowed it).',
          'Misst, wie vielfältig der Genpool über Generationen geblieben ist (oder wie stark ein Engpass/Inzucht ihn verengt hat).',
          'Mesure la diversité du bassin génétique à travers les générations (ou son rétrécissement dû à un goulot d\'étranglement/à la consanguinité).',
          'يقيس مدى تنوع المجمع الجيني عبر الأجيال (أو مدى تضييقه بسبب الاختناق أو زواج الأقارب).'
        )}
      </p>

      <div className="grid grid-cols-2 gap-2 mb-4">
        {statCard(
          t(lang, 'Ort. Heterozigotluk', 'Avg Heterozygosity', 'Ø Heterozygotie', 'Hétérozygotie moy.', 'متوسط التغاير الزيجوتي'),
          gd ? `${Math.round(gd.avg_heterozygosity * 100)}%` : '—',
          '#4ecb71'
        )}
        {statCard(
          t(lang, 'Etkin Nüfus (Ne)', 'Effective Pop. (Ne)', 'Effektive Pop. (Ne)', 'Pop. efficace (Ne)', 'السكان الفعّالون'),
          gd ? gd.effective_population_size.toFixed(1) : '—',
          '#7dd3fc'
        )}
        {statCard(
          t(lang, 'Alel Varyansı', 'Allelic Variance', 'Allel-Varianz', 'Variance allélique', 'التباين الأليلي'),
          gd ? gd.allelic_variance.toFixed(3) : '—',
          '#d4a838'
        )}
        {statCard(
          t(lang, 'Ort. Akrabalık Katsayısı', 'Avg Inbreeding Coeff.', 'Ø Inzuchtkoeffizient', 'Coeff. de consanguinité moy.', 'متوسط معامل زواج الأقارب'),
          gd ? `${Math.round(gd.avg_inbreeding_coefficient * 100)}%` : '—',
          gd && gd.avg_inbreeding_coefficient > 0.1 ? '#e05a5a' : '#a0b4ff'
        )}
      </div>

      <div className="mb-2">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t(lang, 'Zaman İçinde Eğilim', 'Trend Over Time', 'Verlauf über die Zeit', 'Évolution dans le temps', 'الاتجاه عبر الزمن')}
        </h4>
        {trend.length >= 2 ? (
          <ResponsiveContainer width="100%" height={160}>
            <LineChart data={trend} margin={{ top: 4, right: 8, bottom: 0, left: -20 }}>
              <XAxis dataKey="year" tick={{ fontSize: 11, fill: '#888' }} />
              {/* heterozygosity/inbreeding (0-100%) and Ne (an unbounded raw
                  population count) previously shared one Y axis -- once Ne
                  exceeded ~100 (any healthy population) it dwarfed the two
                  percentage lines, flattening them unreadably at the bottom
                  of exactly the chart meant to surface inbreeding/bottleneck
                  trends. Ne now gets its own right-hand axis, scaled to its
                  own range. */}
              <YAxis yAxisId="pct" tick={{ fontSize: 11, fill: '#888' }} unit="%" />
              <YAxis yAxisId="ne" orientation="right" tick={{ fontSize: 11, fill: '#7dd3fc' }} />
              <Tooltip contentStyle={{ backgroundColor: '#0f1117', border: '1px solid #2a2a3a', fontSize: 12 }} />
              <Line yAxisId="pct" type="monotone" dataKey="heterozygosity" name={t(lang, 'Heterozigotluk %', 'Heterozygosity %')} stroke="#4ecb71" dot={false} strokeWidth={2} />
              <Line yAxisId="ne" type="monotone" dataKey="ne" name="Ne" stroke="#7dd3fc" dot={false} strokeWidth={2} />
              <Line yAxisId="pct" type="monotone" dataKey="inbreeding" name={t(lang, 'Akrabalık %', 'Inbreeding %')} stroke="#e05a5a" dot={false} strokeWidth={2} />
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <p className="text-sim-muted text-sm text-center py-6 italic">
            {t(
              lang,
              'Eğilim için en az iki kontrol noktası gerekir. Zaman Makinesi\'nden kaydedin veya simülasyonun otomatik kaydetmesini bekleyin.',
              'The trend needs at least two checkpoints. Save one from Time Machine, or wait for an automatic one.',
              'Der Verlauf benötigt mindestens zwei Kontrollpunkte.',
              'La tendance nécessite au moins deux points de contrôle.',
              'يتطلب الاتجاه نقطتي تحقق على الأقل.'
            )}
          </p>
        )}
      </div>
    </DetailPanel>
  );
}
