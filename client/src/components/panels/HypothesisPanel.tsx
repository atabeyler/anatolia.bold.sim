import { useState } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { FlaskConical, CheckCircle, XCircle, HelpCircle } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

const EXAMPLES: Record<string, string[]> = {
  tr: [
    'Teknoloji nüfus artışına yol açar',
    'İnanç sistemleri çevre stresi altında ortaya çıkar',
    'Yüksek eşitsizlik (Gini > 0.5) sosyal çatışmayı önceler',
    'Sanatsal kültürler daha karmaşık dil geliştirir',
  ],
  en: [
    'Technology leads to population growth',
    'Belief systems emerge under environmental stress',
    'High inequality (Gini > 0.5) precedes social conflict',
    'Artistic cultures develop more complex language',
  ],
  de: [
    'Technologie führt zu Bevölkerungswachstum',
    'Glaubenssysteme entstehen unter Umweltstress',
    'Hohe Ungleichheit (Gini > 0.5) geht sozialen Konflikten voraus',
    'Künstlerische Kulturen entwickeln komplexere Sprache',
  ],
  fr: [
    'La technologie entraîne une croissance démographique',
    'Les systèmes de croyances émergent sous stress environnemental',
    'Une forte inégalité (Gini > 0,5) précède les conflits sociaux',
    'Les cultures artistiques développent un langage plus complexe',
  ],
  ar: [
    'التكنولوجيا تؤدي إلى نمو سكاني',
    'تنشأ أنظمة المعتقدات تحت الضغط البيئي',
    'تسبق عدم المساواة العالية (جيني > 0.5) النزاع الاجتماعي',
    'الثقافات الفنية تطور لغة أكثر تعقيداً',
  ],
};

type Result = { verdict: 'supported' | 'refuted' | 'inconclusive'; confidence: number; ci_lower?: number; ci_upper?: number; n_evidence?: number; reasoning: string };

export default function HypothesisPanel() {
  const { currentSim, accessToken, lang, stats, events } = useSimStore();
  const [hypothesis, setHypothesis] = useState('');
  const [result, setResult] = useState<Result | null>(null);
  const [loading, setLoading] = useState(false);

  async function test() {
    if (!hypothesis.trim() || !currentSim) return;
    setLoading(true);
    setResult(null);
    try {
      // The server only ever consumes events.length (a count, for n_evidence
      // and the heuristic verdict threshold) -- it never inspects event
      // content (see rust/sim-server/src/analysis.rs). Sending the full
      // store-buffered array (already capped at 200) instead of an arbitrary
      // 50-event slice makes n_evidence reflect the true buffered history
      // rather than an arbitrary recency-biased subset.
      const { data } = await axios.post(`/api/analysis/${currentSim.id}/hypothesis`, { hypothesis, lang, stats, events }, { headers: { Authorization: `Bearer ${accessToken}` } });
      setResult(data);
    } catch { setResult({ verdict: 'inconclusive', confidence: 0, reasoning: text(lang as LangCode, { en: 'Test failed.', tr: 'Test başarısız.', de: 'Test fehlgeschlagen.', fr: 'Test échoué.', ar: 'فشل الاختبار.' }) }); }
    setLoading(false);
  }

  const examples = EXAMPLES[lang] ?? EXAMPLES.en;

  const verdictStyle = result ? {
    supported:    { border: 'border-green-500/30',  bg: 'bg-green-500/10',  color: 'text-green-400',  icon: CheckCircle },
    refuted:      { border: 'border-red-500/30',    bg: 'bg-red-500/10',    color: 'text-red-400',    icon: XCircle },
    inconclusive: { border: 'border-yellow-500/30', bg: 'bg-yellow-500/10', color: 'text-yellow-400', icon: HelpCircle },
  }[result.verdict] : null;

  const VERDICT_LABELS: Record<string, { en: string; tr: string; de: string; fr: string; ar: string }> = {
    supported:    { en: 'Supported',    tr: 'Destekleniyor', de: 'Bestätigt',      fr: 'Confirmée',   ar: 'مدعومة' },
    refuted:      { en: 'Refuted',      tr: 'Çürütüldü',     de: 'Widerlegt',      fr: 'Réfutée',     ar: 'مدحوضة' },
    inconclusive: { en: 'Inconclusive', tr: 'Belirsiz',      de: 'Nicht eindeutig', fr: 'Non concluante', ar: 'غير حاسمة' },
  };
  const verdictLabel = result ? (VERDICT_LABELS[result.verdict] ? text(lang as LangCode, VERDICT_LABELS[result.verdict]) : result.verdict) : '';

  return (
    <DetailPanel panelId="hypothesis" title="Hypothesis Test" titleTr="Hipotez Testi" titleDe="Hypothesentest" titleFr="Test d'hypothèse" titleAr="اختبار الفرضية">
      {/* Adam & Eve Core Metrics */}
      {stats && (
        <div className="bg-sim-surface rounded-lg p-3 mb-3">
          <div className="text-sim-gold text-xs font-semibold uppercase tracking-widest mb-2">
            {text(lang as LangCode, { en: 'Adam & Eve Hypothesis Metrics', tr: 'Adem & Havva Hipotez Metrikleri', de: 'Adam & Eva Hypothesenmetriken', fr: 'Métriques Hypothèse Adam & Ève', ar: 'مقاييس فرضية آدم وحواء' })}
          </div>
          <div className="grid grid-cols-2 gap-1.5">
            {[
              { label: text(lang as LangCode, { en: 'Avg Consciousness', tr: 'Ort. Bilinç', de: 'Ø Bewusstsein', fr: 'Conscience moy.', ar: 'متوسط الوعي' }), value: ((stats.avg_consciousness ?? 0) * 100).toFixed(1) + '%' },
              { label: text(lang as LangCode, { en: 'Max ToM Stage', tr: 'Max ZihinKur.', de: 'Max ToM-Stufe', fr: 'Stade ToM max', ar: 'أقصى مرحلة ToM' }), value: stats.max_tom_stage ?? 0 },
              { label: text(lang as LangCode, { en: 'Word Count', tr: 'Kelime Sayısı', de: 'Wortanzahl', fr: 'Nb. de mots', ar: 'عدد الكلمات' }), value: stats.word_count ?? 0 },
              { label: text(lang as LangCode, { en: 'Lang Stage', tr: 'Dil Aşaması', de: 'Sprachstufe', fr: 'Niveau langue', ar: 'مرحلة اللغة' }), value: stats.max_language_stage ?? 0 },
              { label: text(lang as LangCode, { en: 'Technologies', tr: 'Teknolojiler', de: 'Technologien', fr: 'Technologies', ar: 'التقنيات' }), value: stats.technologies ?? 0 },
              { label: text(lang as LangCode, { en: 'Beliefs', tr: 'İnançlar', de: 'Überzeugungen', fr: 'Croyances', ar: 'المعتقدات' }), value: stats.beliefs ?? 0 },
              { label: text(lang as LangCode, { en: 'Art Forms', tr: 'Sanat Biçimleri', de: 'Kunstformen', fr: "Formes d'art", ar: 'أشكال الفن' }), value: stats.art_forms ?? 0 },
              { label: text(lang as LangCode, { en: 'QoL Index', tr: 'YYK Endeksi', de: 'Lebensqualität', fr: 'Indice Q.V.', ar: 'مؤشر جودة الحياة' }), value: ((stats.qol_index ?? 0) * 100).toFixed(1) + '%' },
            ].map(({ label, value }) => (
              <div key={label} className="bg-sim-bg rounded p-1.5">
                <div className="text-sim-muted" style={{ fontSize: 12 }}>{label}</div>
                <div className="text-sim-text font-medium text-sm">{value}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="bg-sim-surface rounded-lg p-3 mb-3 flex items-start gap-2">
        <FlaskConical size={16} className="text-green-400 mt-0.5 flex-shrink-0" />
        <p className="text-sim-muted text-sm">
          {text(lang as LangCode, {
            en: 'State a hypothesis and Aria evaluates it against live simulation data.',
            tr: 'Bir hipotez belirtin; Aria bunu canlı simülasyon verileriyle değerlendirir.',
            de: 'Formulieren Sie eine Hypothese, und Aria bewertet sie anhand der Live-Simulationsdaten.',
            fr: 'Formulez une hypothèse et Aria l\'évalue à partir des données de simulation en direct.',
            ar: 'اذكر فرضية وسيقوم Aria بتقييمها مقابل بيانات المحاكاة الحية.',
          })}
        </p>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Examples', tr: 'Örnekler', de: 'Beispiele', fr: 'Exemples', ar: 'أمثلة' })}
        </h4>
        <div className="space-y-1">
          {examples.map(ex => (
            <button key={ex} onClick={() => setHypothesis(ex)}
              className="w-full text-left text-sm text-sim-muted hover:text-sim-text bg-sim-surface hover:bg-sim-border rounded px-2 py-1 transition-colors">
              "{ex}"
            </button>
          ))}
        </div>
      </div>

      <textarea
        value={hypothesis}
        onChange={e => setHypothesis(e.target.value)}
        placeholder={text(lang as LangCode, { en: 'State your hypothesis…', tr: 'Hipotezinizi belirtin…', de: 'Geben Sie Ihre Hypothese ein…', fr: 'Énoncez votre hypothèse…', ar: 'اذكر فرضيتك…' })}
        className="w-full bg-sim-bg border border-sim-border rounded-lg px-3 py-2 text-sm text-sim-text resize-none h-16 focus:border-sim-accent focus:outline-none mb-2"
      />
      <button onClick={test} disabled={loading || !hypothesis.trim()}
        className="w-full px-3 py-1.5 bg-green-700 hover:bg-green-600 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 mb-3">
        {loading ? text(lang as LangCode, { en: 'Testing…', tr: 'Test ediliyor…', de: 'Testen…', fr: 'Test en cours…', ar: 'جارٍ الاختبار…' }) : text(lang as LangCode, { en: 'Test Hypothesis', tr: 'Hipotezi Test Et', de: 'Hypothese testen', fr: "Tester l'hypothèse", ar: 'اختبار الفرضية' })}
      </button>

      {result && verdictStyle && (() => {
        const Icon = verdictStyle.icon;
        return (
          <div className={`rounded-lg p-3 border ${verdictStyle.border} ${verdictStyle.bg}`}>
            <div className="flex items-center gap-2 mb-2">
              <Icon size={14} className={verdictStyle.color} />
              <span className={`text-sm font-semibold uppercase ${verdictStyle.color}`}>
                {verdictLabel} ({(result.confidence * 100).toFixed(0)}% {text(lang as LangCode, { en: 'confidence', tr: 'güven', de: 'Konfidenz', fr: 'confiance', ar: 'ثقة' })})
              </span>
            </div>
            {result.ci_lower !== undefined && result.ci_upper !== undefined && (
              <p className="text-xs text-sim-muted mb-2">
                95% CI: [{(result.ci_lower * 100).toFixed(1)}%, {(result.ci_upper * 100).toFixed(1)}%]
                {result.n_evidence !== undefined && (
                  <span className="ml-2 opacity-60">n={result.n_evidence} {text(lang as LangCode, { en: 'events', tr: 'olay', de: 'Ereignisse', fr: 'événements', ar: 'أحداث' })}</span>
                )}
              </p>
            )}
            <p className="text-sm text-sim-muted leading-relaxed">{result.reasoning}</p>
          </div>
        );
      })()}
    </DetailPanel>
  );
}
