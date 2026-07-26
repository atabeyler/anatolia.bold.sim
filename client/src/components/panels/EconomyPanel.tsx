import { translateEventDescription, text, type LangCode } from '../../utils/i18n';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

export default function EconomyPanel() {
  const { stats, events, lang } = useSimStore();
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });

  const meanWealth = (stats as any)?.mean_wealth ?? 0;
  const gini = (stats as any)?.gini ?? 0;
  const foodAb = (stats as any)?.food_abundance ?? 0;
  const waterAb = (stats as any)?.water_abundance ?? 0;

  const tradeEvents = events.filter(e => e.event_type === 'trade');

  const resourceData = [
    { name: t('Yiyecek', 'Food', 'Nahrung', 'Nourriture', 'الطعام'), value: Math.round(foodAb * 100) },
    { name: t('Su', 'Water', 'Wasser', 'Eau', 'الماء'), value: Math.round(waterAb * 100) },
    { name: t('Servet', 'Wealth', 'Reichtum', 'Richesse', 'الثروة'), value: Math.round(Math.min(meanWealth / 10, 100)) },
    { name: t('Eşitlik', 'Equality', 'Gleichheit', 'Égalité', 'المساواة'), value: Math.round((1 - gini) * 100) },
  ];

  return (
    <DetailPanel panelId="economy" title="Economy" titleTr="Ekonomi">
      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <div className="text-sim-gold font-bold text-lg">{meanWealth.toFixed(1)}</div>
          <div className="text-sim-muted text-sm">{t('Ort. Servet', 'Mean Wealth', 'Ø Reichtum', 'Richesse moy.', 'متوسط الثروة')}</div>
        </div>
        <div className="bg-sim-surface rounded-lg p-2 text-center">
          <div className={`font-bold text-lg ${gini > 0.5 ? 'text-red-400' : gini > 0.3 ? 'text-yellow-400' : 'text-green-400'}`}>
            {gini.toFixed(2)}
          </div>
          <div className="text-sim-muted text-sm">{t('Gini Endeksi', 'Gini Index', 'Gini-Index', 'Indice Gini', 'معامل جيني')}</div>
        </div>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Kaynak Durumu', 'Resource Status', 'Ressourcenstatus', 'État des ressources', 'حالة الموارد')}
        </h4>
        <ResponsiveContainer width="100%" height={110}>
          <BarChart data={resourceData} margin={{ top: 0, right: 0, bottom: 0, left: -20 }}>
            <XAxis dataKey="name" tick={{ fontSize: 11, fill: '#888' }} />
            <YAxis domain={[0, 100]} tick={{ fontSize: 11, fill: '#888' }} />
            <Tooltip
              contentStyle={{ backgroundColor: '#0f1117', border: '1px solid #2a2a3a', fontSize: 12 }}
              formatter={(v: any) => [`${v}%`]}
            />
            <Bar dataKey="value" fill="#7c3aed" radius={[2, 2, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Ekonomi Modeli', 'Economic Model', 'Wirtschaftsmodell', 'Modèle économique', 'النموذج الاقتصادي')}
        </h4>
        <p className="text-sim-muted text-sm italic">
          {t(
            // economy.rs::attempt_trade weighs trade likelihood on the two
            // traders' averaged altruism alone -- the separate "cooperation"
            // phenotype trait is never actually read by the trade code (or
            // anywhere else in the engine), so the previous copy claimed a
            // mechanic that doesn't exist.
            'Takas, fazlalık tespitiyle yönlendirilir. İki tarafın özgecilik (altruism) geninin ortalaması → ticaret sıklığı. Gini, uzmanlaşmadan doğan eşitsizliği ölçer.',
            'Barter driven by surplus detection. The two traders\' averaged altruism gene → trade frequency. Gini measures inequality from specialization.',
            'Tauschhandel durch Überschusserkennung. Der gemittelte Altruismus-Genwert beider Händler → Handelsfrequenz. Gini misst Ungleichheit durch Spezialisierung.',
            'Troc piloté par détection des surplus. La moyenne du gène altruisme des deux commerçants → fréquence commerciale. Le Gini mesure l\'inégalité due à la spécialisation.',
            'المقايضة مدفوعة باكتشاف الفائض. متوسط جين الإيثار لدى الطرفين ← تكرار التبادل. يقيس معامل جيني عدم المساواة الناتج عن التخصص.'
          )}
        </p>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {t('Ticaret Günlüğü', 'Trade Log', 'Handelsprotokoll', 'Journal de commerce', 'سجل التجارة')}
        </h4>
        {tradeEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">
            {t('Henüz ticaret olayı yok.', 'No trade events yet.', 'Noch keine Handelsereignisse.', 'Pas encore d\'événements commerciaux.', 'لا أحداث تجارية بعد.')}
          </p>
        ) : (
          <div className="space-y-1 max-h-32 overflow-y-auto">
            {tradeEvents.slice(0, 8).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5 border-b border-sim-border/30">
                <span className="text-yellow-400 font-mono text-sm">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
