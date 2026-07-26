import { useState, useEffect, useCallback } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { RefreshCw } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

interface IndividualNode {
  id: string;
  name?: string;
  sex: string;
  birth_day: number;
  death_day?: number;
  is_dead?: boolean;
  parent_1_id?: string;
  parent_2_id?: string;
  phenotype?: { name?: string; fluid_intelligence?: number };
}

function buildTree(pop: IndividualNode[], rootId: string, depth = 0, maxDepth = 4): any {
  if (depth > maxDepth) return null;
  const node = pop.find(i => i.id === rootId);
  if (!node) return null;
  const name = node.phenotype?.name ?? node.id.slice(-6).toUpperCase();
  const children = pop.filter(i => i.parent_1_id === rootId || i.parent_2_id === rootId);
  return {
    id: rootId,
    name,
    sex: node.sex,
    is_dead: node.is_dead,
    iq: node.phenotype?.fluid_intelligence ?? 0.5,
    children: children.slice(0, 6).map(c => buildTree(pop, c.id, depth + 1, maxDepth)).filter(Boolean),
  };
}

function TreeNode({ node, lang }: { node: any; lang: string }) {
  const color = node.sex === 'male' ? '#4f6ef7' : '#f76f9e';
  const opacity = node.is_dead ? 0.4 : 1;
  return (
    <div className="flex flex-col items-center" style={{ opacity }}>
      <div
        className="rounded-lg px-2 py-1 text-center border"
        style={{
          background: `${color}18`,
          borderColor: `${color}55`,
          minWidth: 60,
          maxWidth: 80,
        }}
      >
        <div className="font-medium text-sim-text" style={{ fontSize: 12 }}>{node.name}</div>
        <div className="text-sim-muted" style={{ fontSize: 12 }}>
          {node.sex === 'male' ? text(lang as LangCode, { tr: 'E', en: 'M', de: 'M', fr: 'M', ar: 'ذ' }) : text(lang as LangCode, { tr: 'K', en: 'F', de: 'W', fr: 'F', ar: 'أ' })}
          {' · '}IQ {((node.iq ?? 0.5) * 100).toFixed(0)}
        </div>
      </div>
      {node.children && node.children.length > 0 && (
        <div className="flex flex-col items-center">
          <div className="w-px h-3" style={{ background: `${color}55` }} />
          <div className="flex gap-2 items-start">
            {node.children.map((child: any, i: number) => (
              <div key={child.id} className="flex flex-col items-center">
                {i > 0 && <div className="hidden" />}
                <div className="w-px h-2" style={{ background: '#4f6ef744' }} />
                <TreeNode node={child} lang={lang} />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default function GenealogyPanel() {
  const { currentSim, accessToken, lang, stats } = useSimStore();
  const [population, setPopulation] = useState<IndividualNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedRoot, setSelectedRoot] = useState<string>('');

  const fetchPop = useCallback(async () => {
    if (!currentSim || !accessToken) return;
    setLoading(true);
    try {
      const { data } = await axios.get(`/api/simulations/${currentSim.id}/population?limit=200`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      });
      setPopulation(data);
      const founders = data.filter((i: IndividualNode) => !i.parent_1_id && !i.parent_2_id);
      if (founders.length > 0 && !selectedRoot) setSelectedRoot(founders[0].id);
    } catch {
      setPopulation([]);
    }
    setLoading(false);
  }, [currentSim?.id, accessToken]);

  useEffect(() => { fetchPop(); }, [fetchPop]);

  const founders = population.filter(i => !i.parent_1_id && !i.parent_2_id);
  const rootNode = selectedRoot ? buildTree(population, selectedRoot) : null;
  // fetchPop's own query is hardcoded to limit=200 -- for any simulation
  // with more people ever born than that, later generations are silently
  // missing from the tree with no indication it's truncated rather than
  // an unusually shallow family. total_population (derive_stats) lets us
  // at least surface that instead of leaving it looking like a complete tree.
  const totalPopulation = stats?.total_population ?? 0;
  const isTruncated = totalPopulation > population.length;

  return (
    <DetailPanel panelId="genealogy" title="Genealogy" titleTr="Soy Ağacı">
      {isTruncated && (
        <div className="mb-2 px-2 py-1.5 text-xs bg-yellow-500/10 border border-yellow-500/30 rounded text-yellow-400">
          {text(lang as LangCode, {
            tr: `${population.length} / ${totalPopulation} kişi gösteriliyor — soy ağacı kesilmiş olabilir.`,
            en: `Showing ${population.length} of ${totalPopulation} individuals — the tree may be truncated.`,
            de: `${population.length} von ${totalPopulation} Personen angezeigt — der Baum könnte abgeschnitten sein.`,
            fr: `Affichage de ${population.length} sur ${totalPopulation} individus — l'arbre est peut-être tronqué.`,
            ar: `يتم عرض ${population.length} من ${totalPopulation} فرداً — قد تكون الشجرة مقتطعة.`,
          })}
        </div>
      )}
      <div className="flex items-center gap-2 mb-3">
        <select
          value={selectedRoot}
          onChange={e => setSelectedRoot(e.target.value)}
          className="flex-1 bg-sim-bg border border-sim-border rounded px-2 py-1 text-sm text-sim-text focus:border-sim-accent focus:outline-none"
          style={{ fontSize: 12 }}
        >
          {founders.length === 0 && <option value="">{text(lang as LangCode, { tr: 'Kurucu yok', en: 'No founders', de: 'Keine Gründer', fr: 'Pas de fondateurs', ar: 'لا يوجد مؤسسون' })}</option>}
          {founders.map(f => (
            <option key={f.id} value={f.id}>
              {f.phenotype?.name ?? f.id.slice(-8)} ({f.sex === 'male' ? text(lang as LangCode, { tr: 'Erkek', en: 'Male', de: 'Männlich', fr: 'Masculin', ar: 'ذكر' }) : text(lang as LangCode, { tr: 'Kadın', en: 'Female', de: 'Weiblich', fr: 'Féminin', ar: 'أنثى' })})
            </option>
          ))}
          {population.filter(i => i.parent_1_id || i.parent_2_id).slice(0, 20).map(i => (
            <option key={i.id} value={i.id}>
              {i.phenotype?.name ?? i.id.slice(-8)}
            </option>
          ))}
        </select>
        <button
          onClick={fetchPop}
          disabled={loading}
          className="p-1.5 bg-sim-surface border border-sim-border rounded text-sim-muted hover:text-sim-accent transition-colors disabled:opacity-40"
        >
          <RefreshCw size={10} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      <div className="bg-sim-surface rounded-lg p-3 mb-3">
        <div className="text-sim-muted text-xs mb-2">
          {text(lang as LangCode, { tr: `Toplam ${population.length} birey · ${founders.length} kurucu`, en: `${population.length} individuals total · ${founders.length} founders`, de: `${population.length} Individuen · ${founders.length} Gründer`, fr: `${population.length} individus · ${founders.length} fondateurs`, ar: `${population.length} فرد · ${founders.length} مؤسس` })}
        </div>
        <div className="flex gap-3 text-xs text-sim-muted">
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-sm" style={{ background: '#4f6ef7' }} />
            {text(lang as LangCode, { tr: 'Erkek', en: 'Male', de: 'Männlich', fr: 'Masculin', ar: 'ذكر' })}
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-sm" style={{ background: '#f76f9e' }} />
            {text(lang as LangCode, { tr: 'Kadın', en: 'Female', de: 'Weiblich', fr: 'Féminin', ar: 'أنثى' })}
          </span>
          <span className="flex items-center gap-1 opacity-40">
            <span className="inline-block w-2 h-2 rounded-sm" style={{ background: '#888' }} />
            {text(lang as LangCode, { tr: 'Ölü', en: 'Deceased', de: 'Verstorben', fr: 'Décédé', ar: 'متوفى' })}
          </span>
        </div>
      </div>

      {loading ? (
        <div className="text-sim-muted text-sm text-center py-6">
          {text(lang as LangCode, { tr: 'Yükleniyor…', en: 'Loading…', de: 'Lädt…', fr: 'Chargement…', ar: 'جارٍ التحميل…' })}
        </div>
      ) : rootNode ? (
        <div className="overflow-x-auto overflow-y-auto" style={{ maxHeight: 400 }}>
          <div className="p-2" style={{ minWidth: 200 }}>
            <TreeNode node={rootNode} lang={lang} />
          </div>
        </div>
      ) : (
        <div className="text-sim-muted text-sm text-center py-6 italic">
          {text(lang as LangCode, { tr: 'Henüz soy verisi yok.', en: 'No lineage data yet.', de: 'Noch keine Abstammungsdaten.', fr: 'Pas encore de données de lignée.', ar: 'لا توجد بيانات نسب بعد.' })}
        </div>
      )}
    </DetailPanel>
  );
}
