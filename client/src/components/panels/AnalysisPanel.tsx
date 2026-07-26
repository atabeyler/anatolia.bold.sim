import { useState, useRef, useEffect } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { Send, Bot } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

const QUICK_QUESTIONS: { tr: string; en: string; de: string; fr: string; ar: string }[] = [
  { tr: 'Nüfus neden büyümeyi durdurdu?', en: 'Why did the population stop growing?', de: 'Warum hörte das Bevölkerungswachstum auf?', fr: 'Pourquoi la population a-t-elle arrêté de croître?', ar: 'لماذا توقف نمو السكان؟' },
  { tr: 'İlk inanç sistemi neden ortaya çıktı?', en: 'What triggered the first belief system?', de: 'Was löste das erste Glaubenssystem aus?', fr: 'Qu\'est-ce qui a déclenché le premier système de croyances?', ar: 'ما الذي أطلق أول نظام معتقدات؟' },
  { tr: 'Medeniyetim çöküş riski altında mı?', en: 'Is my civilization at risk of collapse?', de: 'Ist meine Zivilisation einsturz­gefährdet?', fr: 'Ma civilisation risque-t-elle de s\'effondrer?', ar: 'هل حضارتي في خطر الانهيار؟' },
  { tr: 'Sıradaki hangi teknoloji ortaya çıkmalı?', en: 'What technology should emerge next?', de: 'Welche Technologie sollte als nächstes entstehen?', fr: 'Quelle technologie devrait émerger ensuite?', ar: 'ما التقنية التي يجب أن تظهر بعد ذلك؟' },
];

export default function AnalysisPanel() {
  const { currentSim, accessToken, lang } = useSimStore();
  const [messages, setMessages] = useState<{ role: 'user' | 'assistant'; content: string }[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => { bottomRef.current?.scrollIntoView({ behavior: 'smooth' }); }, [messages]);

  async function send(msg?: string) {
    const activeLang = lang as LangCode;
    const m = (msg ?? input).trim();
    if (!m || !currentSim) return;
    setInput('');
    setMessages(prev => [...prev, { role: 'user', content: m }]);
    setLoading(true);
    try {
      const { data } = await axios.post(
        `/api/analysis/${currentSim.id}`,
        { message: m, lang: activeLang },
        { headers: { Authorization: `Bearer ${accessToken}` } }
      );
      setMessages(prev => [...prev, { role: 'assistant', content: data.response }]);
    } catch (err: any) {
      const detail = err?.response?.data?.error ?? err?.message ?? '';
      setMessages(prev => [...prev, { role: 'assistant', content: `${text(activeLang, { tr: 'Analiz başarısız', en: 'Analysis failed', de: 'Analyse fehlgeschlagen', fr: 'Échec de l’analyse', ar: 'فشل التحليل' })}: ${detail}` }]);
    }
    setLoading(false);
  }

  return (
    <DetailPanel panelId="analysis" title="AI Analysis" titleTr="AI Analiz" titleDe="KI-Analyse" titleFr="Analyse IA" titleAr="تحليل الذكاء الاصطناعي">
      <div className="flex flex-col gap-3" style={{ minHeight: '340px' }}>
        <div className="bg-sim-accent/10 border border-sim-accent/20 rounded-lg p-2 flex items-center gap-2">
          <Bot size={13} className="text-sim-accent flex-shrink-0" />
          <span className="text-sm text-sim-muted">
            {text(lang as LangCode, { tr: 'BOLD, canlı simülasyon verilerini kullanarak medeniyetinizi analiz eder.', en: 'BOLD analyzes your civilization using live simulation data.', de: 'BOLD analysiert Ihre Zivilisation mit Live-Simulationsdaten.', fr: 'BOLD analyse votre civilisation en utilisant des données de simulation en direct.', ar: 'يحلل BOLD حضارتك باستخدام بيانات المحاكاة المباشرة.' })}
          </span>
        </div>

        {messages.length === 0 && (
          <div>
            <div className="text-sm text-sim-muted mb-2">
              {text(lang as LangCode, { tr: 'Hızlı sorular:', en: 'Quick questions:', de: 'Schnelle Fragen:', fr: 'Questions rapides:', ar: 'أسئلة سريعة:' })}
            </div>
            <div className="space-y-1">
              {QUICK_QUESTIONS.map(q => (
                <button key={q.en} onClick={() => send(text(lang as LangCode, q))} className="w-full text-left text-sm text-sim-muted hover:text-sim-text bg-sim-surface hover:bg-sim-border rounded px-2 py-1 transition-colors">
                  {text(lang as LangCode, q)}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="flex-1 space-y-2 overflow-y-auto max-h-52">
          {messages.map((m, i) => (
            <div key={i} className={`text-sm px-3 py-2 rounded-lg leading-relaxed ${m.role === 'user' ? 'bg-sim-accent/20 text-sim-text ml-6' : 'bg-sim-surface text-sim-muted mr-2'}`}>
              {m.content}
            </div>
          ))}
          {loading && (
            <div className="text-sm px-3 py-2 rounded-lg bg-sim-surface text-sim-muted mr-2 animate-pulse">
              {text(lang as LangCode, { tr: 'Analiz ediliyor…', en: 'Analyzing…', de: 'Analysiere…', fr: 'Analyse en cours…', ar: 'جارٍ التحليل…' })}
            </div>
          )}
          <div ref={bottomRef} />
        </div>

        <div className="flex gap-2">
          <input
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); } }}
            placeholder={text(lang as LangCode, { tr: "BOLD'a sor…", en: 'Ask BOLD…', de: 'BOLD fragen…', fr: 'Demander à BOLD…', ar: 'اسأل BOLD…' })}
            className="flex-1 bg-sim-bg border border-sim-border rounded-lg px-3 py-1.5 text-sm text-sim-text focus:border-sim-accent focus:outline-none"
          />
          <button onClick={() => send()} disabled={loading || !input.trim()} className="p-1.5 bg-sim-accent hover:bg-sim-accent/80 rounded-lg text-white transition-colors disabled:opacity-50 flex-shrink-0">
            <Send size={12} />
          </button>
        </div>
      </div>
    </DetailPanel>
  );
}
