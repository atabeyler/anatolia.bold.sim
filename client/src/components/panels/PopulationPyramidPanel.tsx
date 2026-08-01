import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

function t(lang: string, trStr: string, enStr: string, deStr = enStr, frStr = enStr, arStr = enStr) {
  return text(lang as LangCode, { tr: trStr, en: enStr, de: deStr, fr: frStr, ar: arStr });
}

export default function PopulationPyramidPanel() {
  const { stats, lang } = useSimStore();
  const pyramid = stats?.age_pyramid ?? [];

  // Transform: male = negative for left side
  const data = [...pyramid].reverse().map(r => ({
    group: r.group,
    male: -r.male,
    female: r.female,
    maleRaw: r.male,
  }));

  const maxVal = Math.max(1, ...pyramid.map(r => Math.max(r.male, r.female)));

  const totalMale = pyramid.reduce((s, r) => s + r.male, 0);
  const totalFemale = pyramid.reduce((s, r) => s + r.female, 0);
  const total = totalMale + totalFemale;

  return (
    <DetailPanel panelId="pyramid" title="Population Pyramid" titleTr="Nüfus Piramidi" titleDe="Bevölkerungspyramide" titleFr="Pyramide des âges" titleAr="هرم السكان">
      {/* Summary */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        {[
          { l: t(lang, 'Toplam', 'Total', 'Gesamt', 'Total', 'المجموع'),    v: total,       c: '#e0e0f0' },
          { l: t(lang, 'Erkek', 'Male', 'Männlich', 'Masculin', 'ذكر'),   v: totalMale,   c: '#7dd3fc' },
          { l: t(lang, 'Kadın', 'Female', 'Weiblich', 'Féminin', 'أنثى'), v: totalFemale, c: '#ff8ab0' },
        ].map(({ l, v, c }) => (
          <div key={l} className="bg-sim-surface rounded p-2 text-center">
            <div className="text-sim-muted text-sm mb-0.5">{l}</div>
            <div className="font-orbitron font-bold text-base" style={{ color: c }}>{v}</div>
          </div>
        ))}
      </div>

      {/* Sex ratio bar */}
      {total > 0 && (
        <div className="mb-4">
          <div className="flex justify-between text-sm text-sim-muted mb-1">
            <span>♂ {((totalMale / total) * 100).toFixed(1)}%</span>
            <span>{t(lang, 'Cinsiyet Oranı', 'Sex Ratio', 'Geschlechterverhältnis', 'Rapport des sexes', 'نسبة الجنسين')}</span>
            <span>{((totalFemale / total) * 100).toFixed(1)}% ♀</span>
          </div>
          <div className="h-2 rounded-full overflow-hidden flex">
            <div style={{ width: `${(totalMale / total) * 100}%`, background: '#7dd3fc' }} />
            <div style={{ flex: 1, background: '#ff8ab0' }} />
          </div>
        </div>
      )}

      {/* Pyramid chart */}
      {data.length === 0 || total === 0 ? (
        <div className="flex items-center justify-center py-12 text-sim-muted italic text-sm">
          {t(lang, 'Henüz nüfus verisi yok.', 'No population data yet.', 'Noch keine Bevölkerungsdaten.', 'Pas encore de données démographiques.', 'لا توجد بيانات سكانية بعد.')}
        </div>
      ) : (
        <>
          <div className="flex justify-between text-sm mb-1 px-1">
            <span style={{ color: '#7dd3fc' }}>♂ {t(lang, 'Erkek', 'Male', 'Männlich', 'Masculin', 'ذكر')}</span>
            <span className="text-sim-muted text-sm">{t(lang, 'Yaş', 'Age', 'Alter', 'Âge', 'عمر')}</span>
            <span style={{ color: '#ff8ab0' }}>{t(lang, 'Kadın', 'Female', 'Weiblich', 'Féminin', 'أنثى')} ♀</span>
          </div>
          <ResponsiveContainer width="100%" height={320}>
            <BarChart
              data={data}
              layout="vertical"
              margin={{ top: 0, right: 8, left: 32, bottom: 0 }}
              barCategoryGap="15%"
            >
              <XAxis
                type="number"
                domain={[-maxVal, maxVal]}
                tickFormatter={v => Math.abs(v).toString()}
                tick={{ fill: '#6a9a78', fontSize: 11, fontFamily: 'Share Tech Mono' }}
              />
              <YAxis
                type="category"
                dataKey="group"
                tick={{ fill: '#a0c8b0', fontSize: 11, fontFamily: 'Share Tech Mono' }}
                width={32}
              />
              <Tooltip
                formatter={(value: any, name: string) => {
                  const v = Math.abs(value);
                  const label = name === 'male'
                    ? t(lang, 'Erkek', 'Male', 'Männlich', 'Masculin', 'ذكر')
                    : t(lang, 'Kadın', 'Female', 'Weiblich', 'Féminin', 'أنثى');
                  const pct = total > 0 ? ((v / total) * 100).toFixed(1) : '0';
                  return [`${v} (${pct}%)`, label];
                }}
                contentStyle={{ background: '#07071a', border: '1px solid rgba(160,200,180,0.3)', borderRadius: 2, fontSize: 12, fontFamily: 'Share Tech Mono' }}
                labelStyle={{ color: '#a0c8b0' }}
              />
              <Bar dataKey="male" fill="#7dd3fc" radius={[0, 2, 2, 0]} />
              <Bar dataKey="female" fill="#ff8ab0" radius={[0, 2, 2, 0]} />
            </BarChart>
          </ResponsiveContainer>

          {/* Age group summary */}
          <div className="mt-3 border-t border-sim-border/30 pt-3">
            {[
              { l: t(lang, 'Çocuk (0-14)', 'Children (0-14)', 'Kinder (0-14)', 'Enfants (0-14)', 'أطفال (0-14)'),           groups: ['0-4','5-9','10-14'] },
              { l: t(lang, 'Genç (15-29)', 'Youth (15-29)', 'Jugend (15-29)', 'Jeunes (15-29)', 'شباب (15-29)'),           groups: ['15-19','20-24','25-29'] },
              { l: t(lang, 'Yetişkin (30-59)', 'Adults (30-59)', 'Erwachsene (30-59)', 'Adultes (30-59)', 'بالغون (30-59)'), groups: ['30-34','35-39','40-44','45-49','50-54','55-59'] },
              { l: t(lang, 'Yaşlı (60+)', 'Elders (60+)', 'Ältere (60+)', 'Aînés (60+)', 'كبار السن (60+)'),               groups: ['60-64','65+'] },
            ].map(({ l, groups }) => {
              const count = pyramid.filter(r => groups.includes(r.group)).reduce((s, r) => s + r.male + r.female, 0);
              const pct = total > 0 ? ((count / total) * 100).toFixed(0) : '0';
              return (
                <div key={l} className="flex justify-between items-center py-0.5 border-b border-sim-border/20 text-sm">
                  <span className="text-sim-muted">{l}</span>
                  <span className="text-sim-text font-mono">{count} <span className="text-sim-muted">({pct}%)</span></span>
                </div>
              );
            })}
          </div>
        </>
      )}
    </DetailPanel>
  );
}
