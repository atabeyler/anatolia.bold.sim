import { useState, useEffect } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { Zap, Droplets, Wind, Activity, MessageCircle, Mountain, Star, Heart, Skull, RefreshCw, Shield } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

// lat/lon for the geographically-targeted disasters (earthquake/flood/
// volcano/meteor) are filled in at click-time from the population's actual
// centroid (see GodPanel's intervene() below) rather than baked in here --
// they used to be hardcoded to 0/0 ("null island"), so even once the server
// implemented these, they'd have always targeted an empty patch of ocean
// regardless of where the population actually lives.
const INTERVENTIONS = [
  { id: 'earthquake', tr: 'Deprem',   en: 'Earthquake', de: 'Erdbeben',           fr: 'Tremblement de terre', ar: 'زلزال',   icon: Zap,      color: 'text-orange-400', geo: true,  params: { magnitude: 6, radius: 100 } },
  { id: 'flood',      tr: 'Sel',      en: 'Flood',      de: 'Überschwemmung',     fr: 'Inondation',           ar: 'فيضان',   icon: Droplets, color: 'text-blue-400',   geo: true,  params: { severity: 0.5, radius: 150 } },
  { id: 'drought',    tr: 'Kuraklık', en: 'Drought',    de: 'Dürre',              fr: 'Sécheresse',           ar: 'جفاف',    icon: Wind,     color: 'text-yellow-400', geo: false, params: {} },
  { id: 'epidemic',   tr: 'Salgın',   en: 'Epidemic',   de: 'Epidemie',           fr: 'Épidémie',             ar: 'وباء',    icon: Activity, color: 'text-red-400',    geo: false, params: { mortality_rate: 0.2, spread_rate: 0.5 } },
  { id: 'volcano',    tr: 'Volkan',   en: 'Volcano',    de: 'Vulkan',             fr: 'Volcan',               ar: 'بركان',   icon: Mountain, color: 'text-red-600',    geo: true,  params: { power: 7, radius: 200 } },
  { id: 'meteor',     tr: 'Meteor',   en: 'Meteor',     de: 'Meteor',             fr: 'Météore',              ar: 'نيزك',    icon: Star,     color: 'text-yellow-200', geo: true,  params: { size: 3 } },
];

export default function GodPanel() {
  const { currentSim, accessToken, lang, stats } = useSimStore();
  const [message, setMessage] = useState('');
  const [response, setResponse] = useState('');
  const [chatLoading, setChatLoading] = useState(false);
  const [status, setStatus] = useState('');
  const [population, setPopulation] = useState<any[]>([]);
  const [selectedIndId, setSelectedIndId] = useState('');
  const [quarantine, setQuarantine] = useState(false);
  const [migrateSourceSimId, setMigrateSourceSimId] = useState('');
  const [migrateIndividualId, setMigrateIndividualId] = useState('');
  const [migrateStatus, setMigrateStatus] = useState('');

  useEffect(() => {
    if (!currentSim || !accessToken) return;
    axios.get(`/api/simulations/${currentSim.id}/population?alive=true&limit=50`, { headers: { Authorization: `Bearer ${accessToken}` } })
      .then(r => { setPopulation(r.data); if (r.data.length > 0) setSelectedIndId(r.data[0].id); })
      .catch(() => setPopulation([]));
  }, [currentSim?.id, accessToken]);

  async function intervene(type: string, params: any) {
    if (!currentSim) return;
    try {
      const { data } = await axios.post(`/api/god/${currentSim.id}/intervene`, { type, params, user_note: `Manual: ${type}` }, { headers: { Authorization: `Bearer ${accessToken}` } });
      setStatus(`✓ ${data.affected_individuals} ${text(lang as LangCode, { tr: 'etkilendi', en: 'affected', de: 'betroffen', fr: 'affectés', ar: 'متأثر' })} · ${data.deaths} ${text(lang as LangCode, { tr: 'öldü', en: 'died', de: 'gestorben', fr: 'décédés', ar: 'ماتوا' })}`);
    } catch { setStatus(`✗ ${text(lang as LangCode, { tr: 'Başarısız (simülasyon çalışmıyor olabilir)', en: 'Failed (simulation may not be running)', de: 'Fehlgeschlagen (Simulation läuft möglicherweise nicht)', fr: "Échec (la simulation n'est peut-être pas en cours)", ar: 'فشل (قد لا تكون المحاكاة قيد التشغيل)' })}`); }
    setTimeout(() => setStatus(''), 4000);
  }

  async function applyLongevity() {
    if (!currentSim || !selectedIndId) return;
    await intervene('longevity', { individual_id: selectedIndId, extra_years: 50 });
  }

  async function applyDeath() {
    if (!currentSim || !selectedIndId) return;
    if (!window.confirm(text(lang as LangCode, { en: 'Instantly kill this individual?', tr: 'Bu bireyi anında öldür?', de: 'Dieses Individuum sofort töten?', fr: 'Tuer instantanément cet individu?', ar: 'قتل هذا الفرد على الفور؟' }))) return;
    await intervene('instant_death', { individual_id: selectedIndId });
  }

  async function toggleQuarantine() {
    if (!currentSim) return;
    const next = !quarantine;
    try {
      await axios.post(`/api/god/${currentSim.id}/quarantine`, { enabled: next }, { headers: { Authorization: `Bearer ${accessToken}` } });
      setQuarantine(next);
      setStatus(next ? text(lang as LangCode, { en: '✓ Quarantine ON — disasters suppressed', tr: '✓ Karantina AKTİF — afetler bastırıldı', de: '✓ Quarantäne AN — Katastrophen unterdrückt', fr: '✓ Quarantaine ACTIVE — catastrophes supprimées', ar: '✓ الحجر الصحي نشط — الكوارث مكبوتة' }) : text(lang as LangCode, { en: '✓ Quarantine OFF', tr: '✓ Karantina KAPALI', de: '✓ Quarantäne AUS', fr: '✓ Quarantaine INACTIVE', ar: '✓ الحجر الصحي غير نشط' }));
    } catch { setStatus(`✗ ${text(lang as LangCode, { tr: 'Başarısız', en: 'Failed', de: 'Fehlgeschlagen', fr: 'Échec', ar: 'فشل' })}`); }
    setTimeout(() => setStatus(''), 4000);
  }

  async function refreshPopulation() {
    if (!currentSim || !accessToken) return;
    try {
      const r = await axios.get(`/api/simulations/${currentSim.id}/population?alive=true&limit=50`, { headers: { Authorization: `Bearer ${accessToken}` } });
      setPopulation(r.data);
      if (r.data.length > 0 && !r.data.find((i: any) => i.id === selectedIndId)) setSelectedIndId(r.data[0].id);
    } catch {
      setStatus(`✗ ${text(lang as LangCode, { tr: 'Başarısız', en: 'Failed', de: 'Fehlgeschlagen', fr: 'Échec', ar: 'فشل' })}`);
      setTimeout(() => setStatus(''), 4000);
    }
  }

  async function speakToIndividual() {
    if (!currentSim || !message.trim()) return;
    const targetId = selectedIndId || population[0]?.id;
    if (!targetId) { setResponse(text(lang as LangCode, { en: 'No living individuals.', tr: 'Yaşayan birey yok.', de: 'Keine lebenden Individuen.', fr: 'Aucun individu vivant.', ar: 'لا يوجد أفراد أحياء.' })); return; }
    setChatLoading(true);
    try {
      const { data } = await axios.post(`/api/god/${currentSim.id}/talk/${targetId}`, { message, lang }, { headers: { Authorization: `Bearer ${accessToken}` } });
      setResponse(data.response);
    } catch { setResponse('...'); }
    setChatLoading(false);
  }

  async function migrateIndividual() {
    if (!currentSim || !migrateSourceSimId.trim() || !migrateIndividualId.trim()) return;
    setMigrateStatus('…');
    try {
      const { data } = await axios.post(
        `/api/god/${currentSim.id}/migrate-individual`,
        { source_simulation_id: migrateSourceSimId.trim(), individual_id: migrateIndividualId.trim() },
        { headers: { Authorization: `Bearer ${accessToken}` } }
      );
      setMigrateStatus(`✓ ${text(lang as LangCode, { tr: 'Yeni birey geldi', en: 'New arrival' })}: ${data.arrived_individual_id.slice(0, 8)}`);
      refreshPopulation();
    } catch (err: any) {
      setMigrateStatus(`✗ ${err?.response?.data?.error ?? text(lang as LangCode, { tr: 'Başarısız', en: 'Failed' })}`);
    }
    setTimeout(() => setMigrateStatus(''), 5000);
  }

  const selectedInd = population.find(i => i.id === selectedIndId);

  return (
    <DetailPanel panelId="god" title="God Mode" titleTr="Tanrı Modu">
      <div className="bg-orange-500/10 border border-orange-500/30 rounded-lg p-3 mb-3 text-center">
        <div className="text-2xl mb-1">⚡</div>
        <div className="text-orange-300 text-sm font-medium">
          {text(lang as LangCode, { en: 'Divine interventions override physics.', tr: 'Tanrısal müdahaleler fiziği geçersiz kılar.', de: 'Göttliche Eingriffe überschreiben die Physik.', fr: 'Les interventions divines ignorent la physique.', ar: 'التدخلات الإلهية تتجاوز قوانين الفيزياء.' })}
        </div>
      </div>

      {status && (
        <div className="bg-sim-accent/20 border border-sim-accent/40 rounded px-3 py-2 text-sm text-sim-text mb-3">
          {status}
        </div>
      )}

      {/* Natural Disasters */}
      <div className="mb-4">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { en: 'Natural Disasters', tr: 'Doğal Afetler', de: 'Naturkatastrophen', fr: 'Catastrophes naturelles', ar: 'الكوارث الطبيعية' })}
        </h4>
        <div className="grid grid-cols-3 gap-1.5">
          {INTERVENTIONS.map(iv => {
            const Icon = iv.icon;
            // Population's actual centroid (see routes.rs's derive_stats),
            // not a hardcoded coordinate -- falls back to 0/0 only if the
            // simulation has no live centroid yet (e.g. an extinct population).
            const params = iv.geo ? { ...iv.params, lat: stats?.centroid_y ?? 0, lon: stats?.centroid_x ?? 0 } : iv.params;
            return (
              <button key={iv.id} onClick={() => intervene(iv.id, params)}
                className="flex flex-col items-center gap-1 p-2 bg-sim-surface hover:bg-sim-border rounded-lg transition-colors border border-sim-border hover:border-sim-accent/40">
                <Icon size={14} className={iv.color} />
                <span className={`text-sm font-medium ${iv.color}`} style={{ fontSize: 12 }}>{text(lang as LangCode, { en: iv.en, tr: iv.tr, de: iv.de, fr: iv.fr, ar: iv.ar })}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Individual Anomalies */}
      <div className="mb-4">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 flex items-center gap-1.5">
          <Zap size={10} />
          {text(lang as LangCode, { en: 'Individual Anomalies', tr: 'Bireysel Anomali', de: 'Individuelle Anomalien', fr: 'Anomalies individuelles', ar: 'الشذوذات الفردية' })}
        </h4>
        <div className="flex items-center gap-1.5 mb-2">
          <select
            value={selectedIndId}
            onChange={e => setSelectedIndId(e.target.value)}
            className="flex-1 bg-sim-bg border border-sim-border rounded px-2 py-1 text-sm text-sim-text focus:border-sim-accent focus:outline-none"
            style={{ fontSize: 12 }}>
            {population.length === 0
              ? <option value="">{text(lang as LangCode, { en: 'No population', tr: 'Nüfus yok', de: 'Keine Bevölkerung', fr: 'Aucune population', ar: 'لا توجد سكان' })}</option>
              : population.map(ind => (
                <option key={ind.id} value={ind.id}>
                  {ind.name ?? ind.id.slice(0, 8)} · {ind.sex === 'male' ? ({'tr':'E','en':'M','de':'M','fr':'M','ar':'ذ'} as any)[lang] ?? 'M' : ({'tr':'K','en':'F','de':'W','fr':'F','ar':'أ'} as any)[lang] ?? 'F'} · {Math.floor(ind.age_years ?? 0)}y
                </option>
              ))}
          </select>
          <button onClick={refreshPopulation} className="p-1.5 bg-sim-surface border border-sim-border rounded text-sim-muted hover:text-sim-accent transition-colors" title={text(lang as LangCode, { tr: 'Yenile', en: 'Refresh', de: 'Aktualisieren', fr: 'Actualiser', ar: 'تحديث' })}>
            <RefreshCw size={10} />
          </button>
        </div>
        <div className="grid grid-cols-2 gap-1.5">
          <button onClick={applyLongevity} disabled={!selectedIndId}
            className="flex items-center justify-center gap-1.5 p-2 bg-sim-surface hover:bg-sim-border rounded-lg transition-colors border border-sim-border hover:border-green-500/40 disabled:opacity-40">
            <Heart size={12} className="text-green-400" />
            <span className="text-sm text-green-400" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'Longevity', tr: 'Uzun Ömür', de: 'Langlebigkeit', fr: 'Longévité', ar: 'طول العمر' })}</span>
          </button>
          <button onClick={applyDeath} disabled={!selectedIndId}
            className="flex items-center justify-center gap-1.5 p-2 bg-sim-surface hover:bg-red-900/20 rounded-lg transition-colors border border-sim-border hover:border-red-500/40 disabled:opacity-40">
            <Skull size={12} className="text-red-400" />
            <span className="text-sm text-red-400" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'Instant Death', tr: 'Anında Ölüm', de: 'Sofortiger Tod', fr: 'Mort instantanée', ar: 'موت فوري' })}</span>
          </button>
        </div>
        {selectedInd && (
          <div className="mt-1.5 text-sim-muted text-center" style={{ fontSize: 12 }}>
            {text(lang as LangCode, { en: 'Selected', tr: 'Seçili', de: 'Ausgewählt', fr: 'Sélectionné', ar: 'محدد' })}: {selectedInd.name ?? selectedInd.id.slice(0,8)} · IQ {((selectedInd.phenotype?.fluid_intelligence ?? 0.5) * 100).toFixed(0)}
          </div>
        )}
      </div>

      {/* Speak to Individual */}
      <div className="mb-4">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 flex items-center gap-1.5">
          <MessageCircle size={12} />
          {text(lang as LangCode, { en: 'Speak to Individual', tr: 'Bireyyle Konuş', de: 'Mit Individuum sprechen', fr: 'Parler à un individu', ar: 'التحدث إلى الفرد' })}
        </h4>
        <textarea
          value={message}
          onChange={e => setMessage(e.target.value)}
          placeholder={text(lang as LangCode, { en: 'They respond in their language stage…', tr: 'Dil seviyelerinde yanıt verirler…', de: 'Sie antworten in ihrer Sprachstufe…', fr: 'Ils répondent à leur niveau de langage…', ar: 'يستجيبون بمستوى لغتهم…' })}
          className="w-full bg-sim-bg border border-sim-border rounded-lg px-3 py-2 text-sm text-sim-text resize-none h-14 focus:border-sim-accent focus:outline-none mb-2"
        />
        <button onClick={speakToIndividual} disabled={chatLoading || !message.trim()}
          className="w-full px-3 py-1.5 bg-sim-accent hover:bg-sim-accent/80 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
          {chatLoading ? '…' : text(lang as LangCode, { en: 'Speak', tr: 'Konuş', de: 'Sprechen', fr: 'Parler', ar: 'تحدث' })}
        </button>
        {response && (
          <div className="mt-2 bg-sim-surface rounded-lg p-2 text-sim-muted text-sm italic border-l-2 border-sim-accent">
            {response}
          </div>
        )}
      </div>

      {/* Quarantine Mode */}
      <div className="mb-4">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 flex items-center gap-1.5">
          <Shield size={10} />
          {text(lang as LangCode, { en: 'Quarantine Mode', tr: 'Karantina Modu', de: 'Quarantäne-Modus', fr: 'Mode quarantaine', ar: 'وضع الحجر الصحي' })}
        </h4>
        <button
          onClick={toggleQuarantine}
          className={`w-full flex items-center justify-center gap-2 p-2 rounded-lg transition-colors border ${
            quarantine
              ? 'bg-green-700/30 border-green-500/50 text-green-300'
              : 'bg-sim-surface border-sim-border text-sim-muted hover:border-sim-accent/40'
          }`}
        >
          <Shield size={12} />
          <span style={{ fontSize: 12 }}>
            {quarantine
              ? text(lang as LangCode, { en: 'Quarantine: ON (disasters suppressed)', tr: 'Karantina: AKTİF (afetler bastırıldı)', de: 'Quarantäne: AN (Katastrophen unterdrückt)', fr: 'Quarantaine: ACTIVE (catastrophes supprimées)', ar: 'الحجر الصحي: نشط (الكوارث مكبوتة)' })
              : text(lang as LangCode, { en: 'Quarantine: OFF', tr: 'Karantina: KAPALI', de: 'Quarantäne: AUS', fr: 'Quarantaine: INACTIVE', ar: 'الحجر الصحي: غير نشط' })}
          </span>
        </button>
      </div>

      {/* Cross-Simulation Migration */}
      <div className="mb-4">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2 flex items-center gap-1.5">
          {text(lang as LangCode, { en: 'Cross-Simulation Migration', tr: 'Simülasyonlar Arası Göç', de: 'Simulationsübergreifende Migration', fr: 'Migration inter-simulations', ar: 'الهجرة بين المحاكاة' })}
        </h4>
        <p className="text-sim-muted text-sm italic mb-2">
          {text(lang as LangCode, {
            tr: 'Sahip olduğunuz başka bir simülasyondan bir bireyi (tüm genomu ile) buraya yeni bir gelen olarak taşıyın.',
            en: 'Bring one individual (full genome and all) from another simulation you own into this one as a new arrival.',
          })}
        </p>
        <input
          value={migrateSourceSimId}
          onChange={e => setMigrateSourceSimId(e.target.value)}
          placeholder={text(lang as LangCode, { tr: 'Kaynak simülasyon ID', en: 'Source simulation ID' })}
          className="w-full bg-sim-bg border border-sim-border rounded px-2 py-1 text-sm text-sim-text focus:border-sim-accent focus:outline-none mb-1.5"
          style={{ fontSize: 12 }}
        />
        <input
          value={migrateIndividualId}
          onChange={e => setMigrateIndividualId(e.target.value)}
          placeholder={text(lang as LangCode, { tr: 'Birey ID', en: 'Individual ID' })}
          className="w-full bg-sim-bg border border-sim-border rounded px-2 py-1 text-sm text-sim-text focus:border-sim-accent focus:outline-none mb-1.5"
          style={{ fontSize: 12 }}
        />
        <button
          onClick={migrateIndividual}
          disabled={!migrateSourceSimId.trim() || !migrateIndividualId.trim()}
          className="w-full px-3 py-1.5 bg-sim-surface hover:bg-sim-border rounded-lg text-sm text-sim-text transition-colors disabled:opacity-40 border border-sim-border hover:border-sim-accent/40"
        >
          {text(lang as LangCode, { tr: 'Göç Ettir', en: 'Migrate' })}
        </button>
        {migrateStatus && <div className="mt-1.5 text-sim-muted text-center" style={{ fontSize: 12 }}>{migrateStatus}</div>}
      </div>

      <div className="text-sm text-sim-muted text-center pt-2 border-t border-sim-border/30">
        {text(lang as LangCode, { en: `Year ${stats?.year ?? 0} · Pop ${stats?.population ?? 0}`, tr: `Yıl ${stats?.year ?? 0} · Nüfus ${stats?.population ?? 0}`, de: `Jahr ${stats?.year ?? 0} · Bev. ${stats?.population ?? 0}`, fr: `An ${stats?.year ?? 0} · Pop. ${stats?.population ?? 0}`, ar: `سنة ${stats?.year ?? 0} · سكان ${stats?.population ?? 0}` })}
      </div>
    </DetailPanel>
  );
}
