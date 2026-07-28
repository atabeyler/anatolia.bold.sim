import { useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { translateEventDescription, text, type LangCode } from '../../utils/i18n';
import { Film } from 'lucide-react';

interface Scene {
  year: number;
  title: string;
  narration: string;
}

interface DocumentaryResponse {
  civilization_name: string;
  scenes: Scene[];
  generated_by: 'gemini' | 'heuristic';
}

export default function DocumentaryPanel() {
  const { currentSim, accessToken, lang } = useSimStore();
  const [doc, setDoc] = useState<DocumentaryResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  async function generate() {
    if (!currentSim) return;
    setLoading(true);
    setError('');
    try {
      const { data } = await axios.get<DocumentaryResponse>(`/api/simulations/${currentSim.id}/documentary`, {
        params: { lang },
        headers: { Authorization: `Bearer ${accessToken}` },
      });
      setDoc(data);
    } catch (err: any) {
      setError(err?.response?.data?.error ?? err?.message ?? 'failed');
    }
    setLoading(false);
  }

  return (
    <DetailPanel panelId="documentary" title="Documentary" titleTr="Belgesel" titleDe="Dokumentation" titleFr="Documentaire" titleAr="وثائقي">
      <p className="text-sim-muted text-sm italic mb-3">
        {text(lang as LangCode, {
          tr: 'Medeniyetinizin tarihini, gerçekten yaşanmış olaylardan (kurgusuz) oluşan sahnelik bir anlatıya dönüştürür.',
          en: "Turns your civilization's history into a scene-by-scene narrative built entirely from real, tracked events.",
          de: 'Verwandelt die Geschichte Ihrer Zivilisation in eine szenische Erzählung, die vollständig auf realen, aufgezeichneten Ereignissen basiert.',
          fr: "Transforme l'histoire de votre civilisation en un récit scène par scène, construit entièrement à partir d'événements réels et enregistrés.",
          ar: 'يحوّل تاريخ حضارتك إلى سرد مشهدي مبني بالكامل على أحداث حقيقية مسجَّلة.',
        })}
      </p>

      <button
        onClick={generate}
        disabled={loading || !currentSim}
        className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-sim-accent hover:bg-sim-accent/80 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 mb-3"
      >
        <Film size={14} />
        {loading
          ? text(lang as LangCode, { tr: 'Oluşturuluyor…', en: 'Generating…', de: 'Wird erstellt…', fr: 'Génération…', ar: 'جارٍ الإنشاء…' })
          : text(lang as LangCode, { tr: 'Belgesel Oluştur', en: 'Generate Documentary', de: 'Dokumentation erstellen', fr: 'Générer le documentaire', ar: 'إنشاء وثائقي' })}
      </button>

      {error && <p className="text-sm text-red-400 mb-3">{error}</p>}

      {doc && (
        <div>
          <div className="flex items-center justify-between mb-2">
            <h3 className="font-orbitron font-bold text-sim-gold text-base">{doc.civilization_name}</h3>
            {doc.generated_by === 'heuristic' && (
              <span className="text-xs text-sim-muted italic">
                {text(lang as LangCode, { tr: 'basit mod', en: 'basic mode', de: 'einfacher Modus', fr: 'mode simple', ar: 'وضع أساسي' })}
              </span>
            )}
          </div>
          <div className="space-y-3">
            {doc.scenes.map((scene, i) => (
              <div key={i} className="relative pl-4 border-l-2 border-sim-accent/30">
                <div className="absolute -left-[5px] top-1 w-2 h-2 rounded-full bg-sim-accent" />
                <div className="text-sim-muted text-xs font-mono mb-0.5">
                  {text(lang as LangCode, { tr: `Yıl ${scene.year}`, en: `Year ${scene.year}` })}
                </div>
                <div className="text-sim-gold text-sm font-semibold capitalize mb-1">{scene.title}</div>
                <p className="text-sim-text text-sm leading-relaxed">
                  {/* Gemini-authored scenes are already written in the target
                      language (see the system prompt); only the heuristic
                      fallback reuses raw English event descriptions that
                      still need this app's own template-based translator. */}
                  {doc.generated_by === 'heuristic' ? translateEventDescription(scene.narration, lang as LangCode) : scene.narration}
                </p>
              </div>
            ))}
          </div>
        </div>
      )}
    </DetailPanel>
  );
}
