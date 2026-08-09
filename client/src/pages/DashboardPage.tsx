import { useState, useEffect, useRef } from 'react';
import { text, type LangCode } from '../utils/i18n';
import FooterBar from '../components/layout/FooterBar';
import { useNavigate } from 'react-router-dom';
import { Globe, Plus, Play, LogOut, BarChart2, Trash2, DatabaseZap, Download, UploadCloud, ShieldCheck } from 'lucide-react';
import axios from 'axios';
import { useSimStore } from '../store/simStore';
import SimCreationWizard from '../components/SimCreationWizard';
import SimMenuOverlay from '../components/layout/SimMenuOverlay';
import { CLOUD_API_URL, cloudUrl, isLocalOrigin } from '../utils/cloud';
import { isWasmLocalModeActive } from '../wasmLocal/mode';

const LOCALE_MAP: Record<string, string> = { tr: 'tr-TR', en: 'en-US', de: 'de-DE', fr: 'fr-FR', ar: 'ar-SA' };

export default function DashboardPage() {
  const navigate = useNavigate();
  const { user, accessToken, logout, lang } = useSimStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPage, setMenuPage] = useState<'guide' | 'about' | 'mission' | 'contact' | null>(null);
  const [sims, setSims]             = useState<any[]>([]);
  const [cloudSims, setCloudSims]   = useState<any[]>([]);
  const [liveSims, setLiveSims]     = useState<any[]>([]);
  const [showNew, setShowNew]       = useState(false);
  const [compareMode, setCompareMode] = useState(false);
  const [loading, setLoading]       = useState(false);
  const [uploading, setUploading]   = useState<string | null>(null);
  const [cleaning, setCleaning]     = useState(false);
  const [cleanMsg, setCleanMsg]     = useState<{ ok: boolean; text: string } | null>(null);
  // The header's right-action button row (Clean DB, Admin, username, Exit,
  // Settings, Menu) doesn't wrap -- on a narrow phone it silently overflowed
  // the viewport, pushing Settings/Menu off-screen with no scroll
  // affordance to reach them. Below this breakpoint the lower-priority
  // actions move into the hamburger menu itself instead (mobileActions
  // below), same pattern SimulationPage already uses for its own
  // Exit/Terminate buttons.
  const [isMobile, setIsMobile] = useState(() => typeof window !== 'undefined' && window.innerWidth < 640);
  useEffect(() => {
    const handler = () => setIsMobile(window.innerWidth < 640);
    window.addEventListener('resize', handler);
    return () => window.removeEventListener('resize', handler);
  }, []);
  const headers = { Authorization: `Bearer ${accessToken}` };
  // In Yerel (local) mode this page is served from 127.0.0.1 and `sims`
  // below is this device's own SQLite list -- the account's cloud
  // simulations live on a different server entirely, so they're fetched
  // separately here. Browser WASM-local mode (see BrowserModeGate.tsx) is
  // the same idea with no separate origin: `sims` is this device's
  // IndexedDB list (served by wasmLocal/apiAdapter.ts), and the explicit
  // absolute-URL cloud-sims fetch below bypasses that adapter to reach the
  // real account. In Bulut mode `sims` already *is* the cloud list, so
  // there's nothing distinct to show.
  const showCloudSection = isLocalOrigin() || isWasmLocalModeActive();

  useEffect(() => {
    axios.get('/api/simulations', { headers }).then(r => setSims(r.data));
    if (showCloudSection) {
      axios.get(`${CLOUD_API_URL}/api/simulations`, { headers }).then(r => setCloudSims(r.data)).catch(() => {});
    }
    const fetchLive = () => axios.get('/api/simulations/live', { headers }).then(r => setLiveSims(r.data)).catch(() => {});
    fetchLive();
    const liveInterval = setInterval(fetchLive, 20000);
    return () => clearInterval(liveInterval);
  }, []);

  // Keep ARIA informed of wizard open/closed state
  useEffect(() => {
    (window as any).__ariaDashboardReady = true;
    (window as any).__ariaWizardOpen = showNew;
    if (!showNew) (window as any).__ariaWizardStep = -1;
    return () => {
      (window as any).__ariaDashboardReady = false;
      (window as any).__ariaWizardOpen = false;
      (window as any).__ariaWizardStep = -1;
      (window as any).__ariaWizardStepType = null;
      (window as any).__ariaWizardFounder = null;
      (window as any).__ariaWizardTraitName = null;
    };
  }, [showNew]);

  const simsRef = useRef<any[]>([]);
  simsRef.current = sims;

  useEffect(() => {
    function onAriaDashboard(e: Event) {
      const { action, index } = (e as CustomEvent).detail;
      switch (action) {
        case 'create_simulation': setShowNew(true); break;
        case 'open_simulation': {
          const sim = simsRef.current[index ?? 0];
          if (sim) navigate(`/simulation/${sim.id}`);
          break;
        }
        case 'toggle_compare': setCompareMode(c => !c); break;
        case 'wizard_exit': setShowNew(false); break;
        case 'open_menu': setMenuOpen(true); break;
        case 'open_menu_page':
          setMenuPage((e as CustomEvent).detail.menuPage ?? null);
          setMenuOpen(true);
          break;
        case 'close_menu': setMenuOpen(false); setMenuPage(null); break;
        case 'delete_simulation': {
          const sim = simsRef.current[index ?? 0];
          if (!sim) break;
          const { lang: l, accessToken: tok } = useSimStore.getState();
          if (!confirm(text(l as LangCode, { tr: `"${sim.name}" silinsin mi? Bu işlem geri alınamaz.`, en: `Delete "${sim.name}"? This cannot be undone.`, de: `"${sim.name}" löschen? Nicht rückgängig zu machen.`, fr: `Supprimer "${sim.name}"? Irréversible.`, ar: `حذف "${sim.name}"؟ لا يمكن التراجع.` }))) break;
          axios.delete(`/api/simulations/${sim.id}`, { headers: { Authorization: `Bearer ${tok}` } })
            .then(() => {
              setSims(s => s.filter((s2: any) => s2.id !== sim.id));
              setLiveSims(s => s.filter(ls => ls.simulation_id !== sim.id));
            })
            .catch(() => alert(text(l as LangCode, { tr: 'Silme başarısız.', en: 'Delete failed.', de: 'Löschen fehlgeschlagen.', fr: 'Échec de la suppression.', ar: 'فشل الحذف.' })));
          break;
        }
        case 'logout': {
          const { logout: doLogout } = useSimStore.getState();
          doLogout();
          navigate('/login');
          break;
        }
      }
    }
    window.addEventListener('aria-dashboard', onAriaDashboard);
    return () => window.removeEventListener('aria-dashboard', onAriaDashboard);
  }, [navigate]);

  async function deleteSim(id: string, name: string) {
    if (!confirm(text(lang as LangCode, { tr: `"${name}" silinsin mi? Bu işlem geri alınamaz.`, en: `Delete "${name}"? This cannot be undone.`, de: `"${name}" löschen? Nicht rückgängig zu machen.`, fr: `Supprimer "${name}"? Irréversible.`, ar: `حذف "${name}"؟ لا يمكن التراجع.` }))) return;
    try {
      await axios.delete(`/api/simulations/${id}`, { headers });
      setSims(s => s.filter(sim => sim.id !== id));
      setLiveSims(s => s.filter(ls => ls.simulation_id !== id));
    } catch (err: any) {
      // A 404 here means the row is already gone (e.g. deleted from another
      // tab/device) -- reflect that in the list instead of telling the user
      // the delete "failed" when the end state they wanted already holds.
      if (err?.response?.status === 404) {
        setSims(s => s.filter(sim => sim.id !== id));
        setLiveSims(s => s.filter(ls => ls.simulation_id !== id));
        return;
      }
      const detail = err?.response?.data?.error;
      alert(detail || text(lang as LangCode, { tr: 'Silme başarısız.', en: 'Delete failed.', de: 'Löschen fehlgeschlagen.', fr: 'Échec de la suppression.', ar: 'فشل الحذف.' }));
    }
  }

  async function deleteLiveSim(id: string, name: string) {
    // Simulations shown here come from the unfiltered "live" list (any
    // currently-running row, regardless of owner), so they can surface rows
    // that never appear in this account's own "sims" list -- e.g. orphaned
    // records predating the ownership system. This is the only delete entry
    // point such rows have, so it must succeed even without a matching
    // entry in `sims`.
    if (!confirm(text(lang as LangCode, { tr: `"${name}" silinsin mi? Bu işlem geri alınamaz.`, en: `Delete "${name}"? This cannot be undone.`, de: `"${name}" löschen? Nicht rückgängig zu machen.`, fr: `Supprimer "${name}"? Irréversible.`, ar: `حذف "${name}"؟ لا يمكن التراجع.` }))) return;
    try {
      await axios.delete(`/api/simulations/${id}`, { headers });
      setLiveSims(s => s.filter(ls => ls.simulation_id !== id));
      setSims(s => s.filter(sim => sim.id !== id));
    } catch (err: any) {
      if (err?.response?.status === 404) {
        setLiveSims(s => s.filter(ls => ls.simulation_id !== id));
        setSims(s => s.filter(sim => sim.id !== id));
        return;
      }
      const detail = err?.response?.data?.error;
      alert(detail || text(lang as LangCode, { tr: 'Silme başarısız.', en: 'Delete failed.', de: 'Löschen fehlgeschlagen.', fr: 'Échec de la suppression.', ar: 'فشل الحذف.' }));
    }
  }

  async function uploadToCloud(id: string, name: string) {
    setUploading(id);
    try {
      await axios.post(`/api/simulations/${id}/upload-to-cloud`, {}, { headers });
      alert(text(lang as LangCode, {
        tr: `"${name}" buluta yüklendi. Buluttaki kopya bu andan itibaren bağımsız ilerler.`,
        en: `"${name}" was uploaded to the cloud. The cloud copy now progresses independently.`,
        de: `"${name}" wurde in die Cloud hochgeladen.`,
        fr: `"${name}" a été téléversé vers le cloud.`,
        ar: `تم رفع "${name}" إلى السحابة.`,
      }));
    } catch {
      alert(text(lang as LangCode, { tr: 'Buluta yükleme başarısız.', en: 'Cloud upload failed.', de: 'Cloud-Upload fehlgeschlagen.', fr: 'Échec du téléversement.', ar: 'فشل الرفع إلى السحابة.' }));
    } finally {
      setUploading(null);
    }
  }

  // Mirror of uploadToCloud, pulling the other direction: fetches a cloud
  // simulation the caller owns down into this device's own local DB (see
  // routes.rs's download_from_cloud), so a cloud-started sim becomes
  // playable/watchable offline too, not just cloud-only.
  async function downloadFromCloud(id: string, name: string) {
    setUploading(id);
    try {
      await axios.post(`/api/simulations/${id}/download-from-cloud`, {}, { headers });
      alert(text(lang as LangCode, {
        tr: `"${name}" bu cihaza indirildi. Yerel kopya bu andan itibaren bağımsız ilerler.`,
        en: `"${name}" was downloaded to this device. The local copy now progresses independently.`,
        de: `"${name}" wurde auf dieses Gerät heruntergeladen.`,
        fr: `"${name}" a été téléchargé sur cet appareil.`,
        ar: `تم تنزيل "${name}" إلى هذا الجهاز.`,
      }));
      axios.get('/api/simulations', { headers }).then(r => setSims(r.data));
    } catch {
      alert(text(lang as LangCode, { tr: 'Yerele indirme başarısız.', en: 'Download to local failed.', de: 'Download fehlgeschlagen.', fr: 'Échec du téléchargement.', ar: 'فشل التنزيل.' }));
    } finally {
      setUploading(null);
    }
  }

  async function exportSim(id: string, name: string) {
    try {
      const res = await fetch(`/api/simulations/${id}/export`, { headers: { Authorization: `Bearer ${accessToken}` } });
      if (!res.ok) {
        alert(text(lang as LangCode, { tr: 'Yedek alma başarısız.', en: 'Export failed.', de: 'Export fehlgeschlagen.', fr: "Échec de l'export.", ar: 'فشل التصدير.' }));
        return;
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = `${name.replace(/[^a-z0-9_\-]/gi, '_')}_backup.json`;
      a.click(); URL.revokeObjectURL(url);
    } catch {
      alert(text(lang as LangCode, { tr: 'Yedek alma başarısız.', en: 'Export failed.', de: 'Export fehlgeschlagen.', fr: "Échec de l'export.", ar: 'فشل التصدير.' }));
    }
  }

  async function runCleanup() {
    setCleaning(true);
    setCleanMsg(null);
    try {
      const { data } = await axios.post(cloudUrl('/api/admin/cleanup'), {}, { headers });
      const total = (data.checkpoints_deleted ?? 0) + (data.events_deleted ?? 0) + (data.dead_individuals_deleted ?? 0);
      setCleanMsg({ ok: true, text: `✓ ${total} ${text(lang as LangCode, { tr: 'kayıt silindi', en: 'records deleted', de: 'Einträge gelöscht', fr: 'entrées supprimées', ar: 'سجلات محذوفة' })}` });
    } catch {
      setCleanMsg({ ok: false, text: `✗ ${text(lang as LangCode, { tr: 'Temizlik başarısız', en: 'Cleanup failed', de: 'Bereinigung fehlgeschlagen', fr: 'Échec du nettoyage', ar: 'فشل التنظيف' })}` });
    } finally {
      setCleaning(false);
      setTimeout(() => setCleanMsg(null), 4000);
    }
  }

  async function createSim(form: any, founder1: any, founder2: any) {
    setLoading(true);
    try {
      const { data } = await axios.post('/api/simulations', {
        name: form.name,
        latitude: parseFloat(form.latitude),
        longitude: parseFloat(form.longitude),
        founder_1_params: founder1,
        founder_2_params: founder2,
      }, { headers });
      useSimStore.getState().setCurrentSim(data);
      setSims(s => [data, ...s]);
      setShowNew(false);
      navigate(`/simulation/${data.id}`, {
        state: { introTarget: { lat: parseFloat(form.latitude), lon: parseFloat(form.longitude) } },
      });
    } catch (err: any) {
      alert(err?.response?.data?.error || text(lang as LangCode, { tr: 'Simülasyon oluşturulamadı, tekrar deneyin.', en: 'Failed to create simulation, please try again.', de: 'Simulation konnte nicht erstellt werden, bitte erneut versuchen.', fr: 'Échec de la création de la simulation, veuillez réessayer.', ar: 'فشل إنشاء المحاكاة، يرجى المحاولة مرة أخرى.' }));
    } finally { setLoading(false); }
  }

  const runningCount = sims.filter(s => s.status === 'running').length;
  // Terminated sims (natural extinction or manual "Sonlandır") carry an
  // archived mass-death record rather than being deleted -- they stay
  // visible here so the user can still open the report, just kept out of
  // the active registry list above so it doesn't fill up with dead sims.
  const activeSims = sims.filter(s => s.status !== 'completed');
  const completedSims = sims.filter(s => s.status === 'completed');

  function renderSimCard(sim: any, i: number, archived: boolean) {
    return (
      <div key={sim.id}
        className="relative flex items-center gap-4 cursor-pointer transition-all duration-200"
        style={{
          background: 'rgba(4,4,15,0.9)', border: `1px solid ${archived ? 'rgba(96,112,160,0.4)' : 'rgba(200,34,34,0.6)'}`, padding: '14px 16px',
          clipPath: 'polygon(0 0, calc(100% - 8px) 0, 100% 8px, 100% 100%, 8px 100%, 0 calc(100% - 8px))',
          animation: `boot-in 0.4s ease-out ${i * 60}ms both`,
          opacity: archived ? 0.75 : 1,
        }}
        onMouseEnter={e => (e.currentTarget as HTMLDivElement).style.borderColor = archived ? 'rgba(96,112,160,0.7)' : 'rgba(200,34,34,0.9)'}
        onMouseLeave={e => (e.currentTarget as HTMLDivElement).style.borderColor = archived ? 'rgba(96,112,160,0.4)' : 'rgba(200,34,34,0.6)'}
        onClick={() => navigate(`/simulation/${sim.id}`)}>

        <div className="flex-shrink-0 w-8 h-8 flex items-center justify-center"
          style={{
            background: sim.status === 'running' ? 'rgba(78,203,113,0.1)' : 'rgba(79,110,247,0.1)',
            border: `1px solid ${sim.status === 'running' ? 'rgba(78,203,113,0.3)' : 'rgba(79,110,247,0.2)'}`,
            clipPath: 'polygon(0 0, calc(100% - 4px) 0, 100% 4px, 100% 100%, 4px 100%, 0 calc(100% - 4px))',
          }}>
          {sim.status === 'running' ? <div className="w-2 h-2 rounded-full bg-sim-green pulse-live" /> : <Globe size={14} className="text-sim-accent" />}
        </div>

        <div className="flex-1 min-w-0">
          <p className="font-orbitron font-bold tracking-[0.1em] truncate" style={{ fontSize: 14, color: '#d0d8f8' }}>{sim.name}</p>
          <p className="font-share-tech mt-0.5 tracking-widest" style={{ fontSize: 14, color: '#e0e0f0' }}>
            {sim.start_latitude?.toFixed(2)}°N {sim.start_longitude?.toFixed(2)}°E
            <span className="mx-2" style={{ color: 'rgba(200,34,34,0.4)' }}>·</span>
            {text(lang as LangCode, { tr: 'YIL', en: 'YEAR', de: 'JAHR', fr: 'ANNÉE', ar: 'سنة' })} <span style={{ color: '#e0e0f0' }}>{sim.current_year}</span>
          </p>
        </div>

        <div className="flex-shrink-0 px-3 py-1 font-share-tech tracking-widest" style={{
          fontSize: 13,
          background: sim.status === 'running' ? 'rgba(78,203,113,0.1)' : 'rgba(22,22,58,0.6)',
          border: `1px solid ${sim.status === 'running' ? 'rgba(78,203,113,0.35)' : 'rgba(79,110,247,0.15)'}`,
          color: '#e0e0f0',
        }}>
          {sim.status.toUpperCase()}
        </div>

        {showCloudSection && !archived && (
          <button
            disabled={uploading === sim.id}
            onClick={e => { e.stopPropagation(); uploadToCloud(sim.id, sim.name); }}
            className="flex-shrink-0 p-2 transition-all duration-150"
            title={text(lang as LangCode, { tr: 'Buluta Yükle', en: 'Upload to Cloud', de: 'In die Cloud hochladen', fr: 'Téléverser vers le cloud', ar: 'رفع إلى السحابة' })}
            style={{ background: 'transparent', border: '1px solid rgba(124,58,237,0.3)', color: uploading === sim.id ? 'rgba(167,139,250,0.4)' : '#a78bfa', lineHeight: 0,
              clipPath: 'polygon(0 0, calc(100% - 4px) 0, 100% 4px, 100% 100%, 4px 100%, 0 calc(100% - 4px))' }}>
            <UploadCloud size={13} />
          </button>
        )}
        <button
          onClick={e => { e.stopPropagation(); exportSim(sim.id, sim.name); }}
          className="flex-shrink-0 p-2 transition-all duration-150"
          title={text(lang as LangCode, { tr: 'Yedek Al', en: 'Export', de: 'Exportieren', fr: 'Exporter', ar: 'تصدير' })}
          style={{ background: 'transparent', border: '1px solid rgba(79,158,247,0.2)', color: '#3a6070', lineHeight: 0,
            clipPath: 'polygon(0 0, calc(100% - 4px) 0, 100% 4px, 100% 100%, 4px 100%, 0 calc(100% - 4px))' }}
          onMouseEnter={e => { (e.currentTarget as HTMLButtonElement).style.color = '#4f9ef7'; (e.currentTarget as HTMLButtonElement).style.borderColor = 'rgba(79,158,247,0.6)'; }}
          onMouseLeave={e => { (e.currentTarget as HTMLButtonElement).style.color = '#3a6070'; (e.currentTarget as HTMLButtonElement).style.borderColor = 'rgba(79,158,247,0.2)'; }}>
          <Download size={13} />
        </button>
        <button
          onClick={e => { e.stopPropagation(); deleteSim(sim.id, sim.name); }}
          className="flex-shrink-0 p-2 transition-all duration-150"
          title={text(lang as LangCode, { tr: 'Sil', en: 'Delete', de: 'Löschen', fr: 'Supprimer', ar: 'حذف' })}
          style={{ background: 'transparent', border: '1px solid rgba(224,90,90,0.25)', color: '#7a3030', lineHeight: 0,
            clipPath: 'polygon(0 0, calc(100% - 4px) 0, 100% 4px, 100% 100%, 4px 100%, 0 calc(100% - 4px))' }}
          onMouseEnter={e => { (e.currentTarget as HTMLButtonElement).style.color = '#e05a5a'; (e.currentTarget as HTMLButtonElement).style.borderColor = 'rgba(224,90,90,0.6)'; }}
          onMouseLeave={e => { (e.currentTarget as HTMLButtonElement).style.color = '#7a3030'; (e.currentTarget as HTMLButtonElement).style.borderColor = 'rgba(224,90,90,0.25)'; }}>
          <Trash2 size={13} />
        </button>
        <button
          onClick={e => { e.stopPropagation(); navigate(`/simulation/${sim.id}`); }}
          className="flex-shrink-0 p-2 transition-all duration-150 hover:brightness-125"
          style={{ background: 'rgba(79,110,247,0.15)', border: '1px solid rgba(79,110,247,0.35)', color: '#4f6ef7', lineHeight: 0,
            clipPath: 'polygon(0 0, calc(100% - 4px) 0, 100% 4px, 100% 100%, 4px 100%, 0 calc(100% - 4px))' }}>
          {archived
            ? <BarChart2 size={13} />
            : <Play size={13} />}
        </button>
      </div>
    );
  }

  return (
    <div className="min-h-screen text-sim-text" style={{ background: '#030310' }}>

      {/* Scanlines overlay */}
      <div className="pointer-events-none fixed inset-0 z-0"
        style={{ background: 'repeating-linear-gradient(to bottom, transparent 0px, transparent 2px, rgba(0,0,0,0.06) 2px, rgba(0,0,0,0.06) 4px)' }} />

      {/* Header */}
      <div className="sticky top-0 z-10"
        style={{
          background: 'rgba(3,3,16,0.97)',
          borderBottom: '1px solid rgba(200,34,34,0.7)',
          backdropFilter: 'blur(20px)',
          boxShadow: '0 2px 20px rgba(200,34,34,0.5), 0 0 8px rgba(200,34,34,0.3)',
        }}>
        <div className="w-full px-3 sm:px-6 h-14 sm:h-16 flex items-center justify-between gap-2">
          {/* Brand */}
          <div className="flex items-center gap-2 flex-shrink-0">
            <div className="relative w-7 h-7 flex items-center justify-center">
              <div className="absolute inset-0 neon-breathe"
                style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '1.5px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }} />
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
                <circle cx="12" cy="12" r="10" />
                <ellipse cx="12" cy="12" rx="6" ry="14.29" />
                <path d="M2 12h20" />
                <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
              </svg>
            </div>
            <div className="flex flex-col leading-none gap-0.5 items-center">
              <span className="font-orbitron font-bold tracking-[0.2em]" style={{ fontSize: 'clamp(12px, 3.8vw, 18px)', color: '#e0e0f0' }}>{lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'}</span>
              <span className="font-share-tech tracking-[0.25em]" style={{ fontSize: 'clamp(10px, 3vw, 16px)', color: '#cc2222' }}>{text(lang as LangCode, { tr: 'MEDENİYET', en: 'CIVILIZATION', de: 'ZIVILISATION', fr: 'CIVILISATION', ar: 'الحضارة' })}</span>
            </div>
          </div>

          {/* Right actions */}
          <div className="flex items-center gap-1 sm:gap-2 flex-shrink-0">
            {runningCount > 0 && (
              <div className="hidden sm:flex items-center gap-2 px-3 py-1"
                style={{ background: 'rgba(78,203,113,0.1)', border: '1px solid rgba(78,203,113,0.3)' }}>
                <div className="w-1.5 h-1.5 rounded-full bg-sim-green pulse-live" />
                <span className="font-share-tech text-sim-green tracking-widest" style={{ fontSize: 10 }}>
                  {runningCount} {text(lang as LangCode, { tr: 'AKTİF', en: 'ACTIVE', de: 'AKTIV', fr: 'ACTIF', ar: 'نشط' })}
                </span>
              </div>
            )}
            {cleanMsg && (
              <span className="hidden sm:inline font-share-tech tracking-widest" style={{ fontSize: 12, color: cleanMsg.ok ? '#4ecb71' : '#e05a5a' }}>
                {cleanMsg.text}
              </span>
            )}
            <button onClick={runCleanup} disabled={cleaning} title={text(lang as LangCode, { tr: 'Veritabanı temizle', en: 'Clean up database', de: 'Datenbank bereinigen', fr: 'Nettoyer la base de données', ar: 'تنظيف قاعدة البيانات' })}
              className="hidden sm:flex items-center gap-1.5 transition-colors"
              style={{ fontFamily: 'Share Tech Mono,monospace', fontSize: 13, letterSpacing: '0.08em', color: cleaning ? 'rgba(212,168,56,0.4)' : 'rgba(212,168,56,0.85)', border: 'none', background: 'transparent', padding: '4px 8px', cursor: cleaning ? 'wait' : 'pointer' }}>
              <DatabaseZap size={13} />
              <span className="hidden sm:inline">{cleaning ? text(lang as LangCode, { tr: 'TEMİZLENİYOR...', en: 'CLEANING...', de: 'BEREINIGT...', fr: 'NETTOYAGE...', ar: 'جارٍ التنظيف...' }) : text(lang as LangCode, { tr: 'DB TEMİZLE', en: 'CLEAN DB', de: 'DB BEREINIGEN', fr: 'NETTOYER DB', ar: 'تنظيف قاعدة البيانات' })}</span>
            </button>
            {user?.role === 'admin' && (
              <button onClick={() => navigate('/admin')} title={text(lang as LangCode, { tr: 'Yönetim Paneli', en: 'Admin Panel', de: 'Admin-Panel', fr: "Panneau d'administration", ar: 'لوحة الإدارة' })}
                className="hidden sm:flex items-center gap-1.5 transition-colors"
                style={{ fontFamily: 'Share Tech Mono,monospace', fontSize: 13, letterSpacing: '0.08em', color: 'rgba(200,34,34,0.85)', border: 'none', background: 'transparent', padding: '4px 8px', cursor: 'pointer' }}>
                <ShieldCheck size={13} />
                <span className="hidden sm:inline">{text(lang as LangCode, { tr: 'YÖNETİM', en: 'ADMIN', de: 'ADMIN', fr: 'ADMIN', ar: 'الإدارة' })}</span>
              </button>
            )}
            <span className="hidden sm:block font-share-tech tracking-widest font-bold" style={{ fontSize: 14, color: '#ffffff' }}>{user?.username?.toUpperCase()}</span>
            <button onClick={() => { logout(); navigate('/login'); }}
              className="hidden sm:flex items-center gap-1.5 transition-colors"
              style={{ fontFamily: 'Share Tech Mono,monospace', fontSize: 14, fontWeight: 700, letterSpacing: '0.1em', color: '#ffffff', background: 'transparent', border: 'none', padding: '4px 10px' }}>
              <LogOut size={13} />
              <span className="hidden sm:inline">{text(lang as LangCode, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'AUSGANG', fr: 'QUITTER', ar: 'خروج' })}</span>
            </button>
            <button onClick={() => setMenuOpen(true)}
              style={{ display: 'flex', alignItems: 'center', gap: 3, padding: '4px 10px', border: 'none', color: '#ffffff', background: 'transparent', fontSize: 14, letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', flexShrink: 0 }}>
              ☰ {text(lang as LangCode, { tr: 'MENÜ', en: 'MENU', de: 'MENÜ', fr: 'MENU', ar: 'القائمة' })}
            </button>
          </div>
        </div>
      </div>

      <div className="max-w-5xl mx-auto px-3 sm:px-6 py-5 sm:py-8 relative z-1">

        {/* Wizard overlay — shown instead of list when creating */}
        {showNew ? (
          <SimCreationWizard
            lang={lang}
            loading={loading}
            accessToken={accessToken}
            onSubmit={createSim}
            onExit={() => setShowNew(false)}
          />
        ) : (
          <>
            {/* Title row */}
            <div className="flex flex-wrap items-center justify-between gap-2 mb-5 sm:mb-8">
              <div className="flex items-center gap-2">
                <div className="w-1 h-5 bg-sim-accent" style={{ boxShadow: '0 0 8px rgba(79,110,247,0.8)' }} />
                <h2 className="font-orbitron font-bold tracking-[0.12em] text-sim-text" style={{ fontSize: 'clamp(13px, 3.5vw, 16px)' }}>
                  {text(lang as LangCode, { tr: 'SİMÜLASYON KAYITLARI', en: 'SIMULATION REGISTRY', de: 'SIMULATIONS-REGISTER', fr: 'REGISTRE DES SIMULATIONS', ar: 'سجل المحاكاة' })}
                </h2>
              </div>

              <div className="flex items-center gap-2">
                {sims.length >= 2 && (
                  <button onClick={() => setCompareMode(c => !c)}
                    className="flex items-center gap-1.5 font-share-tech tracking-widest transition-all duration-150"
                    style={{
                      padding: '7px 10px', fontSize: 'clamp(12px, 3vw, 14px)',
                      background: compareMode ? 'rgba(79,110,247,0.2)' : 'rgba(22,22,58,0.6)',
                      border: `1px solid ${compareMode ? 'rgba(79,110,247,0.5)' : 'rgba(79,110,247,0.15)'}`,
                      color: '#e0e0f0',
                      clipPath: 'polygon(0 0, calc(100% - 5px) 0, 100% 5px, 100% 100%, 5px 100%, 0 calc(100% - 5px))',
                    }}>
                    <BarChart2 size={13} />
                    <span className="hidden sm:inline">{text(lang as LangCode, { tr: 'KARŞILAŞTIR', en: 'COMPARE', de: 'VERGLEICHEN', fr: 'COMPARER', ar: 'مقارنة' })}</span>
                    <span className="sm:hidden">{text(lang as LangCode, { tr: 'KAR', en: 'CMP', de: 'VGL', fr: 'CMP', ar: 'مقا' })}</span>
                  </button>
                )}
                <button onClick={() => setShowNew(true)}
                  className="flex items-center gap-1.5 font-share-tech tracking-widest transition-all duration-150 hover:brightness-110"
                  style={{
                    padding: '7px 10px', fontSize: 'clamp(12px, 3vw, 14px)',
                    background: 'rgba(79,110,247,0.2)',
                    border: '1px solid rgba(79,110,247,0.5)',
                    color: '#e0e0f0',
                    clipPath: 'polygon(0 0, calc(100% - 6px) 0, 100% 6px, 100% 100%, 6px 100%, 0 calc(100% - 6px))',
                    boxShadow: '0 0 15px rgba(79,110,247,0.2)',
                  }}>
                  <Plus size={13} />
                  <span className="hidden sm:inline">{text(lang as LangCode, { tr: 'YENİ SİMÜLASYON', en: 'NEW SIMULATION', de: 'NEUE SIMULATION', fr: 'NOUVELLE SIMULATION', ar: 'محاكاة جديدة' })}</span>
                  <span className="sm:hidden">{text(lang as LangCode, { tr: 'YENİ', en: 'NEW', de: 'NEU', fr: 'NOUVEAU', ar: 'جديد' })}</span>
                </button>
              </div>
            </div>

            {/* Compare mode */}
            {compareMode && sims.length >= 2 && (
              <div className="mb-6 relative" style={{
                background: 'rgba(4,4,15,0.97)', border: '1px solid rgba(200,34,34,0.6)', animation: 'boot-in 0.4s ease-out both',
              }}>
                <div className="px-4 py-2.5 border-b flex items-center gap-2" style={{ borderColor: 'rgba(200,34,34,0.4)' }}>
                  <BarChart2 size={13} className="text-sim-accent" />
                  <span className="font-orbitron text-xs font-semibold tracking-[0.2em] text-sim-accent">
                    {text(lang as LangCode, { tr: 'PARALEL KARŞILAŞTIRMA', en: 'PARALLEL COMPARISON', de: 'PARALLELVERGLEICH', fr: 'COMPARAISON PARALLÈLE', ar: 'مقارنة متوازية' })}
                  </span>
                </div>
                <div className="p-4 overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr>
                        <th className="text-left pb-2 pr-4">
                          <span className="font-share-tech tracking-widest" style={{ fontSize: 14, color: '#e0e0f0' }}>{text(lang as LangCode, { tr: 'METRİK', en: 'METRIC', de: 'METRIK', fr: 'MÉTRIQUE', ar: 'مقياس' })}</span>
                        </th>
                        {sims.slice(0, 4).map(s => (
                          <th key={s.id} className="text-left pb-2 pr-4">
                            <span className="font-share-tech text-sim-accent tracking-widest" style={{ fontSize: 9 }}>{s.name.toUpperCase()}</span>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {[
                        { label: text(lang as LangCode, { tr: 'YIL', en: 'YEAR', de: 'JAHR', fr: 'ANNÉE', ar: 'سنة' }), key: 'current_year' },
                        { label: text(lang as LangCode, { tr: 'DURUM', en: 'STATUS', de: 'STATUS', fr: 'STATUT', ar: 'الحالة' }), key: 'status' },
                        { label: text(lang as LangCode, { tr: 'KONUM', en: 'LOCATION', de: 'STANDORT', fr: 'EMPLACEMENT', ar: 'الموقع' }), key: '_coord' },
                      ].map(row => (
                        <tr key={row.key} style={{ borderBottom: '1px solid rgba(200,34,34,0.2)' }}>
                          <td className="py-1.5 pr-4">
                            <span className="font-share-tech tracking-widest" style={{ fontSize: 14, color: '#e0e0f0' }}>{row.label}</span>
                          </td>
                          {sims.slice(0, 4).map(s => (
                            <td key={s.id} className="py-1.5 pr-4">
                              {row.key === '_coord'
                                ? <span className="font-share-tech text-sim-text" style={{ fontSize: 10 }}>{s.start_latitude?.toFixed(1)}°N {s.start_longitude?.toFixed(1)}°E</span>
                                : row.key === 'status'
                                  ? <span className="font-share-tech tracking-widest" style={{ fontSize: 10, color: s.status === 'running' ? '#4ecb71' : '#6070a0' }}>{s.status.toUpperCase()}</span>
                                  : <span className="font-orbitron font-bold text-sim-text" style={{ fontSize: 11 }}>{s[row.key]}</span>
                              }
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {/* Canlı simülasyonlar (masaüstünde şu an çalışan) */}
            {liveSims.length > 0 && (
              <div className="mb-6">
                <div className="flex items-center gap-2 mb-3" style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.15em', color: '#22c55e' }}>
                  <div className="w-2 h-2 rounded-full bg-sim-green pulse-live" />
                  <span>{text(lang as LangCode, { tr: 'CANLI SİMÜLASYONLAR', en: 'LIVE SIMULATIONS', de: 'LIVE-SIMULATIONEN', fr: 'SIMULATIONS EN DIRECT', ar: 'محاكاة مباشرة' })}</span>
                </div>
                <div className="grid gap-2">
                  {liveSims.map(ls => (
                    <div key={ls.simulation_id} className="flex items-center justify-between gap-4"
                      style={{ background: 'rgba(34,197,94,0.05)', border: '1px solid rgba(34,197,94,0.3)', padding: '12px 16px', borderRadius: 8 }}>
                      <div>
                        <div style={{ fontSize: 14, fontWeight: 600, color: '#e2e8f0' }}>{ls.simulation_name}</div>
                        <div style={{ fontSize: 11, color: '#64748b', marginTop: 2 }}>
                          {text(lang as LangCode, { tr: 'Gün', en: 'Day', de: 'Tag', fr: 'Jour', ar: 'يوم' })} {ls.current_day} · {text(lang as LangCode, { tr: 'Yıl', en: 'Year', de: 'Jahr', fr: 'Année', ar: 'سنة' })} {ls.current_year} · {ls.population_count} {text(lang as LangCode, { tr: 'birey', en: 'individuals', de: 'Individuen', fr: 'individus', ar: 'أفراد' })}
                          {' · '}{new Date(ls.updated_at).toLocaleTimeString(LOCALE_MAP[lang] ?? 'en-US')}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => navigate(`/watch/${ls.simulation_id}`)}
                          style={{ background: 'rgba(34,197,94,0.15)', border: '1px solid rgba(34,197,94,0.5)', color: '#86efac', padding: '6px 14px', borderRadius: 6, fontSize: 12, fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
                          📺 {text(lang as LangCode, { tr: 'Canlı İzle', en: 'Watch Live', de: 'Live ansehen', fr: 'Regarder en direct', ar: 'مشاهدة مباشرة' })}
                        </button>
                        <button
                          onClick={() => deleteLiveSim(ls.simulation_id, ls.simulation_name)}
                          title={text(lang as LangCode, { tr: 'Sil', en: 'Delete', de: 'Löschen', fr: 'Supprimer', ar: 'حذف' })}
                          style={{ background: 'rgba(239,68,68,0.1)', border: '1px solid rgba(239,68,68,0.4)', color: '#f87171', padding: '6px 10px', borderRadius: 6, fontSize: 12, cursor: 'pointer' }}>
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Cloud simulations (bu hesabın buluttaki kayıtları — masaüstü her zaman
                internete bağlı olduğu için Yerel moddayken de görünür ve devam edilebilir) */}
            {showCloudSection && cloudSims.length > 0 && (
              <div className="mb-6">
                <div className="flex items-center gap-2 mb-3" style={{ color: '#7c3aed', fontSize: 12, fontWeight: 700, letterSpacing: '0.15em' }}>
                  <span>☁️</span>
                  <span>{text(lang as LangCode, { tr: 'BULUT SİMÜLASYONLARI', en: 'CLOUD SIMULATIONS', de: 'CLOUD-SIMULATIONEN', fr: 'SIMULATIONS CLOUD', ar: 'محاكاة سحابية' })}</span>
                </div>
                <div className="grid gap-2">
                  {cloudSims.map(cs => (
                    <div key={cs.id} className="flex items-center justify-between gap-4"
                      style={{ background: 'rgba(124,58,237,0.06)', border: '1px solid rgba(124,58,237,0.3)', padding: '12px 16px', borderRadius: 8 }}>
                      <div>
                        <div style={{ fontSize: 14, fontWeight: 600, color: '#e2e8f0' }}>{cs.name}</div>
                        <div style={{ fontSize: 11, color: '#64748b', marginTop: 2 }}>
                          {text(lang as LangCode, { tr: 'Yıl', en: 'Year', de: 'Jahr', fr: 'Année', ar: 'سنة' })} {cs.current_year} · {cs.status?.toUpperCase()}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => {
                            // Continues against the cloud backend directly (desktop is
                            // always online) -- a full cross-origin navigation, since
                            // this page's own server (local) never touches cloud data.
                            // The token travels as a one-time query param; App.tsx on
                            // the cloud side picks it up and signs the session in.
                            window.location.href = `${CLOUD_API_URL}/simulation/${cs.id}?token=${encodeURIComponent(accessToken || '')}`;
                          }}
                          style={{ background: 'rgba(124,58,237,0.2)', border: '1px solid rgba(124,58,237,0.5)', color: '#a78bfa', padding: '6px 14px', borderRadius: 6, fontSize: 12, fontWeight: 600, cursor: 'pointer', whiteSpace: 'nowrap' }}>
                          ☁️ {text(lang as LangCode, { tr: 'Devam Et', en: 'Continue', de: 'Fortsetzen', fr: 'Continuer', ar: 'متابعة' })}
                        </button>
                        <button
                          disabled={uploading === cs.id}
                          onClick={() => downloadFromCloud(cs.id, cs.name)}
                          title={text(lang as LangCode, { tr: 'Bu cihaza indir', en: 'Download to this device', de: 'Auf dieses Gerät herunterladen', fr: 'Télécharger sur cet appareil', ar: 'تنزيل إلى هذا الجهاز' })}
                          style={{ background: 'rgba(34,197,94,0.12)', border: '1px solid rgba(34,197,94,0.4)', color: '#4ade80', padding: '6px 10px', borderRadius: 6, fontSize: 12, cursor: uploading === cs.id ? 'wait' : 'pointer', whiteSpace: 'nowrap' }}>
                          <Download size={14} />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Simulation list */}
            {activeSims.length === 0 && completedSims.length === 0 ? (
              <div className="hud-panel flex flex-col items-center justify-center py-20">
                <span className="hud-corner-tr" /><span className="hud-corner-bl" />
                <div className="relative w-16 h-16 flex items-center justify-center mb-5">
                  <div className="absolute inset-0 rounded-full" style={{
                    border: '1.5px solid rgba(200,34,34,0.7)',
                    boxShadow: '0 0 10px rgba(200,34,34,0.5), inset 0 0 10px rgba(200,34,34,0.1)',
                    animation: 'ring-expand 2.4s ease-out infinite',
                  }} />
                  <div className="absolute inset-0 rounded-full" style={{
                    border: '1px solid rgba(200,34,34,0.45)',
                    boxShadow: '0 0 14px rgba(200,34,34,0.35)',
                    animation: 'ring-expand 2.4s ease-out 0.8s infinite',
                  }} />
                  <div className="absolute inset-0 rounded-full" style={{
                    border: '1px solid rgba(200,34,34,0.25)',
                    animation: 'ring-expand 2.4s ease-out 1.6s infinite',
                  }} />
                  <div className="relative w-10 h-10 flex items-center justify-center neon-breathe"
                    style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '2px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }}>
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
                      <circle cx="12" cy="12" r="10" />
                      <ellipse cx="12" cy="12" rx="6" ry="14.29" />
                      <path d="M2 12h20" />
                      <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
                    </svg>
                  </div>
                </div>
                <p className="font-share-tech tracking-[0.3em]" style={{ fontSize: 14, color: '#e0e0f0' }}>
                  {text(lang as LangCode, { tr: 'SİMÜLASYON BULUNAMADI', en: 'NO SIMULATIONS FOUND', de: 'KEINE SIMULATIONEN GEFUNDEN', fr: 'AUCUNE SIMULATION TROUVÉE', ar: 'لا توجد محاكاة' })}
                </p>
              </div>
            ) : (
              <>
                {activeSims.length > 0 && (
                  <div className="grid gap-3">
                    {activeSims.map((sim, i) => renderSimCard(sim, i, false))}
                  </div>
                )}

                {/* Sonlandırılan simülasyonlar -- ayrı bir alt bölüm, arşivlenmiş
                    toplu-ölüm kaydıyla birlikte, aktif listeyi doldurmasın diye. */}
                {completedSims.length > 0 && (
                  <div className={activeSims.length > 0 ? 'mt-8' : ''}>
                    <div className="flex items-center gap-2 mb-3" style={{ color: '#6070a0', fontSize: 12, fontWeight: 700, letterSpacing: '0.15em' }}>
                      <span>🪦</span>
                      <span>{text(lang as LangCode, { tr: 'SONLANDIRILAN SİMÜLASYONLAR', en: 'TERMINATED SIMULATIONS', de: 'BEENDETE SIMULATIONEN', fr: 'SIMULATIONS TERMINÉES', ar: 'محاكاة منتهية' })}</span>
                    </div>
                    <div className="grid gap-3">
                      {completedSims.map((sim, i) => renderSimCard(sim, i, true))}
                    </div>
                  </div>
                )}
              </>
            )}
          </>
        )}
      </div>

      {/* Menu Overlay */}
      <SimMenuOverlay
        isOpen={menuOpen}
        onClose={() => { setMenuOpen(false); setMenuPage(null); }}
        menuPage={menuPage}
        onMenuPageChange={setMenuPage}
        mobileActions={isMobile ? (
          <div style={{ padding: '8px 14px', borderTop: '1px solid #0a1a10', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div className="font-share-tech tracking-widest" style={{ fontSize: 13, color: '#ffffff', textAlign: 'center' }}>{user?.username?.toUpperCase()}</div>
            <div style={{ display: 'flex', gap: 8 }}>
              {user?.role === 'admin' && (
                <button onClick={() => { setMenuOpen(false); navigate('/admin'); }}
                  style={{ flex: 1, padding: '7px 0', fontSize: 13, border: '1px solid #4a1a1a', color: 'rgba(200,34,34,0.85)', background: 'transparent', letterSpacing: '0.06em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer' }}>
                  {text(lang as LangCode, { tr: 'YÖNETİM', en: 'ADMIN', de: 'ADMIN', fr: 'ADMIN', ar: 'الإدارة' })}
                </button>
              )}
              <button onClick={runCleanup} disabled={cleaning}
                style={{ flex: 1, padding: '7px 0', fontSize: 13, border: '1px solid #4a1a1a', color: 'rgba(212,168,56,0.85)', background: 'transparent', letterSpacing: '0.06em', fontFamily: 'Share Tech Mono, monospace', cursor: cleaning ? 'wait' : 'pointer' }}>
                {cleaning ? text(lang as LangCode, { tr: 'TEMİZLENİYOR...', en: 'CLEANING...', de: 'BEREINIGT...', fr: 'NETTOYAGE...', ar: 'جارٍ التنظيف...' }) : text(lang as LangCode, { tr: 'DB TEMİZLE', en: 'CLEAN DB', de: 'DB BEREINIGEN', fr: 'NETTOYER DB', ar: 'تنظيف قاعدة البيانات' })}
              </button>
              <button onClick={() => { logout(); navigate('/login'); }}
                style={{ flex: 1, padding: '7px 0', fontSize: 13, border: '1px solid #4a1a1a', color: '#a0c8b0', background: 'transparent', letterSpacing: '0.06em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer' }}>
                {text(lang as LangCode, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'AUSGANG', fr: 'QUITTER', ar: 'خروج' })}
              </button>
            </div>
          </div>
        ) : undefined}
      />

      {/* Footer — fixed to bottom of viewport */}
      <FooterBar mode="fixed" />
      {/* spacer so fixed footer doesn't cover last card */}
      <div style={{ height: 36 }} />
    </div>
  );
}
