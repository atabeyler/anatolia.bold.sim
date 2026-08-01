import { useEffect, useRef, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, translateSeason, type LangCode } from '../../utils/i18n';
import { saveFile, shareFile, openFile, type SavedFile } from '../../utils/fileExport';
import { MILESTONE_I18N } from '../simulation/MilestoneToast';

const LOCALE_MAP: Record<string, string> = { tr: 'tr-TR', en: 'en-US', de: 'de-DE', fr: 'fr-FR', ar: 'ar-SA' };

// A small population's per-module phase time is routinely sub-millisecond
// (e.g. 19 modules sharing a 0.72ms total compute budget) -- toFixed(0)
// rounded every one of those down to a misleading "0 ms", and raw
// unformatted floats (no toFixed at all) printed floating-point noise like
// "0.7199700000000002 ms". Two decimals everywhere shows real, non-zero
// sub-ms timings without either problem.
function fmtMs(value: number | null | undefined): string {
  return value != null ? value.toFixed(2) : '—';
}

interface DiagCheck { ok: boolean; name: string; detail: Record<string, unknown>; }
interface DiagEntry { day: number; ts: number; msg: string; stack: string; }
interface Diagnostics {
  sim_id?: string; current_day: number; running?: boolean; consecutive_errors?: number;
  startup?: { ts: number; day: number; checks: DiagCheck[] } | null;
  error_log?: DiagEntry[];
}
interface DbStatus {
  sim_db: {
    size_bytes: number | null;
    individuals: { total: number; alive: number };
    checkpoints: number; events: number; technologies: number;
    beliefs: number; languages: number; groups: number;
    conversations: number; publications: number; current_day: number | null;
  };
  cloud_db: { size_bytes: number | null; cloud_checkpoints: number; live_snapshots: number };
}

export default function PerformancePanel() {
  const { activePanel, currentSim, accessToken, runtimeMetrics, setRuntimeMetrics, lang, isWarping, fastForwardTarget, wsStatus, wsLastMessageAt, wsCloseInfo, wsReconnectCount } = useSimStore();
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [dbStatus, setDbStatus] = useState<DbStatus | null>(null);
  const [reportMsg, setReportMsg] = useState('');
  const [reportText, setReportText] = useState<string | null>(null);
  const [reportFile, setReportFile] = useState<SavedFile | null>(null);
  // Re-render once a second while this panel is open so "Xs önce" (seconds
  // since the socket last heard from the server) keeps counting up live
  // instead of only updating whenever some unrelated state change happens
  // to re-render this component.
  const [, forceTick] = useState(0);
  useEffect(() => {
    if (activePanel !== 'performance') return;
    const id = setInterval(() => forceTick(n => n + 1), 1000);
    return () => clearInterval(id);
  }, [activePanel]);

  useEffect(() => {
    if (activePanel !== 'performance' || !currentSim || !accessToken) {
      if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; }
      return;
    }
    const headers = { Authorization: `Bearer ${accessToken}` };
    const fetchAll = () => {
      axios.get(`/api/simulations/${currentSim.id}/metrics`, { headers }).then(r => setRuntimeMetrics(r.data)).catch(() => {});
      axios.get(`/api/simulations/${currentSim.id}/diagnostics`, { headers }).then(r => setDiag(r.data)).catch(() => {});
      axios.get(`/api/simulations/${currentSim.id}/db-status`, { headers }).then(r => setDbStatus(r.data)).catch(() => {});
    };
    fetchAll();
    pollRef.current = setInterval(fetchAll, 5000);
    return () => { if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; } };
  }, [activePanel, currentSim?.id, accessToken]);

  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });

  // The server only ever sends structured, language-neutral values here
  // (counts, biome/season ids, day/year numbers) -- it has no notion of
  // which language this particular client is displaying in, and multiple
  // devices could be watching the same simulation in different languages
  // at once. Formatting the actual sentence has to happen here, per
  // check `name`, the same way every other label in this panel already
  // goes through `t(...)`.
  function formatCheckDetail(c: DiagCheck): string {
    const d = c.detail;
    switch (c.name) {
      case 'PATHOGEN_TYPES':
        return t(`${d.count} patogen yüklendi`, `${d.count} pathogens loaded`, `${d.count} Pathogene geladen`, `${d.count} pathogènes chargés`, `تم تحميل ${d.count} ممرض`);
      case 'population':
        return t(`toplam ${d.total}, yaşayan ${d.alive}`, `total ${d.total}, alive ${d.alive}`, `gesamt ${d.total}, lebend ${d.alive}`, `total ${d.total}, vivants ${d.alive}`, `المجموع ${d.total}، الأحياء ${d.alive}`);
      case 'world_state':
        if (!c.ok) return t('biome eksik', 'biome missing', 'Biom fehlt', 'biome manquant', 'المنطقة الحيوية مفقودة');
        return `biome=${d.biome}, ${t('mevsim', 'season', 'Jahreszeit', 'saison', 'الموسم')}=${d.season ? translateSeason(String(d.season), lang as LangCode) : '?'}`;
      case 'sim_day':
        return t(`gün ${d.day} (yıl ${d.year})`, `day ${d.day} (year ${d.year})`, `Tag ${d.day} (Jahr ${d.year})`, `jour ${d.day} (année ${d.year})`, `يوم ${d.day} (سنة ${d.year})`);
      default:
        return '';
    }
  }

  function bar(value: number, max: number, color: string) {
    const pct = max > 0 ? Math.min(100, (value / max) * 100) : 0;
    return (
      <div style={{ height: 4, background: 'rgba(255,255,255,0.08)', borderRadius: 2, overflow: 'hidden', marginTop: 3 }}>
        <div style={{ height: '100%', width: `${pct}%`, background: color, transition: 'width 0.4s ease' }} />
      </div>
    );
  }

  const m = runtimeMetrics;
  const warpYear = fastForwardTarget ? Math.floor(fastForwardTarget / 365) : null;
  const currentYear = m ? Math.floor(m.current_day / 365) : null;
  const warpPct = (warpYear && currentYear && warpYear > currentYear)
    ? Math.min(99, ((currentYear / warpYear) * 100))
    : null;
  // `engine` matches a name in sim_core::TOGGLEABLE_ENGINES exactly -- used
  // both to render this row's on/off button and to read/write it in
  // `disabledEngines` below. `undefined` (setup only) means not toggleable --
  // see TOGGLEABLE_ENGINES' own doc comment for why.
  const moduleRows: { label: string; value: number; color: string; engine?: string }[] = m?.tick_phase_setup_ms != null
    ? [
        { label: t('Kurulum / dünya', 'Setup / world', 'Setup / Welt', 'Config / monde', 'الإعداد / العالم'), value: m.tick_phase_setup_ms ?? 0, color: '#7dd3fc' },
        { label: t('Ekonomi', 'Economy', 'Wirtschaft', 'Économie', 'الاقتصاد'), value: m.tick_phase_economy_ms ?? 0, color: '#00e887', engine: 'economy' },
        { label: t('Bilinç / psikoloji', 'Consciousness / psychology', 'Bewusstsein / Psychologie', 'Conscience / psychologie', 'الوعي / علم النفس'), value: m.tick_phase_consciousness_psychology_ms ?? 0, color: '#c084fc', engine: 'consciousness_psychology' },
        { label: t('Dil / isimlendirme', 'Language / naming', 'Sprache / Namensgebung', 'Langue / dénomination', 'اللغة / التسمية'), value: m.tick_phase_language_naming_ms ?? 0, color: '#fbbf24', engine: 'language_naming' },
        { label: t('Mikrobiyom / karar alma', 'Microbiome / decision-making', 'Mikrobiom / Entscheidung', 'Microbiome / décision', 'الميكروبيوم / اتخاذ القرار'), value: m.tick_phase_microbiome_agent_ms ?? 0, color: '#38bdf8', engine: 'microbiome_agent' },
        { label: t('Hareket', 'Movement', 'Bewegung', 'Mouvement', 'الحركة'), value: m.tick_phase_movement_ms ?? 0, color: '#a0b4ff', engine: 'movement' },
        { label: t('Gözlemsel öğrenme', 'Observational learning', 'Beobachtungslernen', 'Apprentissage par observation', 'التعلم بالملاحظة'), value: m.tick_phase_observation_learning_ms ?? 0, color: '#818cf8', engine: 'observation_learning' },
        { label: t('Teknoloji ortaya çıkışı', 'Tech emergence', 'Technologie-Entstehung', 'Émergence technologique', 'ظهور التقنية'), value: m.tick_phase_tech_emergence_ms ?? 0, color: '#60a5fa', engine: 'tech_emergence' },
        { label: t('Üreme', 'Reproduction', 'Fortpflanzung', 'Reproduction', 'التكاثر'), value: m.tick_phase_reproduction_ms ?? 0, color: '#f472b6', engine: 'reproduction' },
        { label: t('Ölüm riski', 'Mortality risk', 'Sterberisiko', 'Risque de mortalité', 'خطر الوفاة'), value: m.tick_phase_mortality_roll_ms ?? 0, color: '#e05a5a', engine: 'mortality_roll' },
        { label: t('Mikrobiyom salgını', 'Microbiome outbreak', 'Mikrobiom-Ausbruch', 'Épidémie microbiome', 'تفشي الميكروبيوم'), value: m.tick_phase_microbiome_outbreak_ms ?? 0, color: '#f87171', engine: 'microbiome_outbreak' },
        { label: t('Grup temizleme', 'Group pruning', 'Gruppenbereinigung', 'Nettoyage de groupe', 'تقليم المجموعة'), value: m.tick_phase_group_pruning_ms ?? 0, color: '#fb923c', engine: 'group_pruning' },
        { label: t('İnanç', 'Belief', 'Glaube', 'Croyance', 'المعتقد'), value: m.tick_phase_belief_ms ?? 0, color: '#d4a838', engine: 'belief' },
        { label: t('Kültür / sanat', 'Culture / art', 'Kultur / Kunst', 'Culture / art', 'الثقافة / الفن'), value: m.tick_phase_culture_art_ms ?? 0, color: '#eab308', engine: 'culture_art' },
        { label: t('Sosyal roller', 'Social roles', 'Soziale Rollen', 'Rôles sociaux', 'الأدوار الاجتماعية'), value: m.tick_phase_social_ms ?? 0, color: '#a78bfa', engine: 'social' },
        { label: t('Hukuk', 'Law', 'Recht', 'Droit', 'القانون'), value: m.tick_phase_law_ms ?? 0, color: '#94a3b8', engine: 'law' },
        { label: t('Mimari / çatışma', 'Architecture / conflict', 'Architektur / Konflikt', 'Architecture / conflit', 'العمارة / الصراع'), value: m.tick_phase_architecture_conflict_ms ?? 0, color: '#fca5a5', engine: 'architecture_conflict' },
        { label: t('Astronomi', 'Astronomy', 'Astronomie', 'Astronomie', 'الفلك'), value: m.tick_phase_astronomy_ms ?? 0, color: '#67e8f9', engine: 'astronomy' },
        { label: t('Ticaret / hastalık', 'Trade / disease', 'Handel / Krankheit', 'Commerce / maladie', 'التجارة / المرض'), value: m.tick_phase_trade_disease_ms ?? 0, color: '#4ade80', engine: 'trade_disease' },
      ]
    : [];
  const moduleMax = Math.max(1, ...moduleRows.map(row => row.value ?? 0));
  const disabledEngines = new Set(m?.disabled_engines ?? []);

  function flashReport(msg: string) { setReportMsg(msg); setTimeout(() => setReportMsg(''), 3000); }

  // Plain text, not JSON: the point of this button is letting a user hand
  // over exact numbers by pasting text (into a chat, an email) instead of a
  // phone photo of this panel -- which is how every performance bug this
  // session got diagnosed before this existed, screenshot legibility and
  // all. Mirrors this panel's own section labels/order 1:1 so a report is
  // recognizable against the live screen it came from.
  function buildReportText(): string {
    const lines: string[] = [];
    const push = (s = '') => lines.push(s);
    push(`=== ${t('PERFORMANS RAPORU', 'PERFORMANCE REPORT', 'LEISTUNGSBERICHT', 'RAPPORT DE PERFORMANCE', 'تقرير الأداء')} ===`);
    push(`${t('Tarih', 'Date', 'Datum', 'Date', 'التاريخ')}: ${new Date().toLocaleString(LOCALE_MAP[lang] ?? 'en-US')}`);
    if (currentSim) push(`${t('Simülasyon', 'Simulation', 'Simulation', 'Simulation', 'المحاكاة')}: ${currentSim.name} (${currentSim.id})`);
    push();

    if (m) {
      push(`-- ${t('TİCK ZAMANLAMA', 'TICK TIMING', 'TICK-TIMING', 'TIMING TICK', 'توقيت التيك')} --`);
      push(`${t('Son ms', 'Last ms', 'Letzt ms', 'Dern. ms', 'آخر ms')}: ${fmtMs(m.tick_last_ms)}`);
      push(`${t('Ort. ms', 'Avg ms', 'Ø ms', 'Moy. ms', 'متوسط ms')}: ${fmtMs(m.tick_avg_ms)}`);
      push(`${t('Maks ms', 'Max ms', 'Max ms', 'Max ms', 'أقصى ms')}: ${fmtMs(m.tick_max_ms)}`);
      push(`${t('Min ms', 'Min ms', 'Min ms', 'Min ms', 'أدنى ms')}: ${fmtMs(m.tick_min_ms)}`);
      push();

      if (m.tick_load_ms != null || m.tick_compute_ms != null || m.tick_save_ms != null || m.tick_upsert_ms != null) {
        push(`-- ${t('SON PARÇA — AŞAMA DETAYI', 'LAST BATCH — PHASE BREAKDOWN', 'LETZTER BATCH — PHASEN', 'DERNIER LOT — DÉTAIL', 'آخر دفعة — تفصيل المراحل')} --`);
        push(`${t('Yükleme (DB)', 'Load (DB)', 'Laden (DB)', 'Chargement (DB)', 'تحميل (DB)')}: ${fmtMs(m.tick_load_ms)} ms`);
        push(`${t('Hesaplama', 'Compute', 'Berechnung', 'Calcul', 'الحساب')}: ${fmtMs(m.tick_compute_ms)} ms`);
        push(`${t('Kaydet (DB)', 'Save (DB)', 'Speichern (DB)', 'Sauvegarde (DB)', 'حفظ (DB)')}: ${fmtMs(m.tick_save_ms)} ms`);
        push(`${t('Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)')}: ${fmtMs(m.tick_upsert_ms)} ms`);
        push();
      }

      if (moduleRows.length > 0) {
        push(`-- ${t('MODÜL / PERFORMANS', 'MODULE / PERFORMANCE', 'MODUL / LEISTUNG', 'MODULE / PERFORMANCE', 'الوحدات / الأداء')} --`);
        moduleRows.forEach(({ label, value, engine }) => {
          const off = engine != null && disabledEngines.has(engine);
          push(`${label}: ${fmtMs(value)} ms${off ? ` (${t('kapalı', 'off', 'aus', 'désactivé', 'متوقف')})` : ''}`);
        });
        push();
      }

      push(`-- ${t('MOTOR', 'ENGINE', 'MOTOR', 'MOTEUR', 'المحرك')} --`);
      push(`${t('Tik/sn', 'Ticks/s', 'Ticks/s', 'Ticks/s', 'تيك/ث')}: ${m.ticks_per_second?.toFixed(1) ?? '—'}`);
      push(`${t('Hız', 'Speed', 'Tempo', 'Vitesse', 'السرعة')}: ${m.speed_multiplier ?? 1}×`);
      push(`${t('Gün', 'Day', 'Tag', 'Jour', 'اليوم')}: ${m.current_day ?? '—'}`);
      push(`${t('Yaşayan', 'Alive', 'Lebend', 'Vivants', 'أحياء')}: ${m.population ?? '—'}`);
      push(`${t('Toplam', 'Total Ever', 'Gesamt', 'Total', 'المجموع')}: ${m.total_ever ?? '—'}`);
      push(`${t('CPU Çekirdek', 'CPU Cores', 'CPU-Kerne', 'Cœurs CPU', 'أنوية المعالج')}: ${m.cpu_cores_used ?? '—'} / ${m.cpu_cores_available ?? '—'}`);
      if (m.cross_origin_isolated !== undefined) {
        push(`${t('Cross-Origin Isolation', 'Cross-Origin Isolation', 'Cross-Origin Isolation', 'Isolation Cross-Origin', 'عزل المصدر المتقاطع')}: ${m.cross_origin_isolated ? t('Aktif', 'Active', 'Aktiv', 'Actif', 'نشط') : t('Devre dışı', 'Inactive', 'Inaktiv', 'Inactif', 'غير نشط')}`);
      }
      if (m.thread_pool_error) {
        push(`${t('Çekirdek havuzu hatası', 'Thread pool error', 'Thread-Pool-Fehler', 'Erreur du pool de threads', 'خطأ تجمع الخيوط')}: ${m.thread_pool_error}`);
      }
      push();

      if (m.milestones_reached?.length > 0) {
        push(`-- ${t('ERİŞİLEN MILESTONE\'LAR', 'MILESTONES REACHED', 'ERREICHTE MEILENSTEINE', 'JALONS ATTEINTS', 'المعالم المحققة')} (${m.milestones_reached.length}) --`);
        push(m.milestones_reached.map((key: string) => MILESTONE_I18N[key] ? text(lang as LangCode, MILESTONE_I18N[key]) : key).join(', '));
        push();
      }

      if (m.centroid_trail?.length > 1) {
        const first = m.centroid_trail[0];
        const last = m.centroid_trail[m.centroid_trail.length - 1];
        push(`-- ${t('GÖÇ ROTALARI', 'MIGRATION PATH', 'MIGRATIONSPFAD', 'CHEMIN MIGRATION', 'مسار الهجرة')} (${m.centroid_trail.length} ${t('nokta', 'points', 'Punkte', 'points', 'نقاط')}) --`);
        push(`${t('Başlangıç', 'Start', 'Start', 'Départ', 'بداية')}: ${first.x.toFixed(2)}, ${first.y.toFixed(2)}`);
        push(`${t('Mevcut', 'Current', 'Aktuell', 'Actuel', 'حالي')}: ${last.x.toFixed(2)}, ${last.y.toFixed(2)}`);
        push();
      }
    }

    if (dbStatus) {
      push(`-- ${t('VERİTABANI DURUMU', 'DATABASE STATUS', 'DATENBANK STATUS', 'ÉTAT BASE DE DONNÉES', 'حالة قاعدة البيانات')} --`);
      push(`${t('SİM DB (yerel)', 'SIM DB (local)', 'SIM DB (lokal)', 'SIM DB (local)', 'قاعدة بيانات المحاكاة')}${dbStatus.sim_db.size_bytes !== null ? ` (${dbStatus.sim_db.size_bytes > 1048576 ? `${(dbStatus.sim_db.size_bytes / 1048576).toFixed(1)} MB` : `${(dbStatus.sim_db.size_bytes / 1024).toFixed(0)} KB`})` : ''}`);
      push(`  ${t('Birey (toplam/yaşayan)', 'Individuals (total/alive)', 'Individuen', 'Individus', 'الأفراد')}: ${dbStatus.sim_db.individuals.total} / ${dbStatus.sim_db.individuals.alive}`);
      push(`  Checkpoint: ${dbStatus.sim_db.checkpoints}`);
      push(`  ${t('Olay', 'Events', 'Ereignisse', 'Événements', 'أحداث')}: ${dbStatus.sim_db.events}`);
      push(`  ${t('Teknoloji', 'Technologies', 'Technologien', 'Technologies', 'تقنيات')}: ${dbStatus.sim_db.technologies}`);
      push(`  ${t('İnanç', 'Beliefs', 'Glaubenssätze', 'Croyances', 'معتقدات')}: ${dbStatus.sim_db.beliefs}`);
      push(`  ${t('Dil kaydı', 'Language records', 'Sprachaufzeichnungen', 'Enreg. langue', 'سجلات اللغة')}: ${dbStatus.sim_db.languages}`);
      push(`  ${t('Grup', 'Groups', 'Gruppen', 'Groupes', 'مجموعات')}: ${dbStatus.sim_db.groups}`);
      push(`  ${t('Konuşma', 'Conversations', 'Gespräche', 'Conversations', 'محادثات')}: ${dbStatus.sim_db.conversations}`);
      push(`${t('BULUT DB (Render)', 'CLOUD DB (Render)', 'CLOUD DB (Render)', 'CLOUD DB (Render)', 'قاعدة بيانات السحاب')}${dbStatus.cloud_db.size_bytes !== null ? ` (${dbStatus.cloud_db.size_bytes > 1048576 ? `${(dbStatus.cloud_db.size_bytes / 1048576).toFixed(1)} MB` : `${(dbStatus.cloud_db.size_bytes / 1024).toFixed(0)} KB`})` : ''}`);
      push(`  ${t('Bulut checkpoint', 'Cloud checkpoints', 'Cloud-Checkpoints', 'Sauvegardes cloud', 'نقاط حفظ السحاب')}: ${dbStatus.cloud_db.cloud_checkpoints}`);
      push(`  ${t('Canlı anlık görüntü', 'Live snapshots', 'Live-Snapshots', 'Instantanés live', 'لقطات مباشرة')}: ${dbStatus.cloud_db.live_snapshots}`);
      push(`${t('Buluta yükleme', 'Cloud upload', 'Cloud-Upload', 'Envoi cloud', 'الرفع السحابي')}: ${runtimeMetrics?.upload_paused ? t('Duraklatıldı', 'Paused', 'Pausiert', 'En pause', 'متوقف مؤقتًا') : t('Aktif', 'Active', 'Aktiv', 'Actif', 'نشط')}`);
      push();
    }

    push(`-- ${t('CANLI BAĞLANTI', 'LIVE CONNECTION', 'LIVE-VERBINDUNG', 'CONNEXION EN DIRECT', 'الاتصال المباشر')} --`);
    const secondsAgo = wsLastMessageAt ? Math.max(0, Math.round((Date.now() - wsLastMessageAt) / 1000)) : null;
    push(`${t('Durum', 'Status', 'Status', 'Statut', 'الحالة')}: ${wsStatus}${secondsAgo !== null ? `, ${t('son mesaj', 'last message', 'letzte Nachricht', 'dernier message', 'آخر رسالة')} ${secondsAgo}s` : ''}${wsReconnectCount > 0 ? `, ${wsReconnectCount}x ${t('yeniden bağlanma', 'reconnects', 'Wiederverbindungen', 'reconnexions', 'إعادة اتصال')}` : ''}${wsCloseInfo ? `, code=${wsCloseInfo.code}${wsCloseInfo.reason ? ` (${wsCloseInfo.reason})` : ''}` : ''}`);
    push();

    if (diag) {
      push(`-- ${t('BAŞLANGIÇ KONTROLÜ', 'STARTUP CHECKS', 'STARTUP-CHECKS', 'VÉRIF. DÉMARRAGE', 'فحوصات البدء')} --`);
      if (diag.startup) {
        diag.startup.checks.forEach(c => { const detail = formatCheckDetail(c); push(`${c.ok ? '✓' : '✗'} ${c.name}${detail ? ` ${detail}` : ''}`); });
        push(`${t('Gün', 'Day', 'Tag', 'Jour', 'يوم')} ${diag.startup.day} — ${new Date(diag.startup.ts).toLocaleTimeString(LOCALE_MAP[lang] ?? 'en-US')}`);
      } else {
        push(t('Simülasyon henüz başlamadı', 'Simulation not started yet', 'Noch nicht gestartet', 'Pas encore démarré', 'لم يبدأ بعد'));
      }
      push();

      push(`-- ${t('HATA KAYDI', 'ERROR LOG', 'FEHLERPROTOKOLL', "JOURNAL D'ERREURS", 'سجل الأخطاء')} (${diag.consecutive_errors ?? 0}/5) --`);
      if ((diag.error_log?.length ?? 0) === 0) {
        push(t('Hata yok', 'No errors', 'Keine Fehler', 'Aucune erreur', 'لا أخطاء'));
      } else {
        [...(diag.error_log ?? [])].reverse().forEach(e => {
          push(`${t('Gün', 'Day', 'Tag', 'Jour', 'يوم')} ${e.day} — ${new Date(e.ts).toLocaleTimeString(LOCALE_MAP[lang] ?? 'en-US')}: ${e.msg}`);
          if (e.stack) push(`  ${e.stack}`);
        });
      }
    }

    return lines.join('\n');
  }

  const [uploadToggling, setUploadToggling] = useState(false);

  // Toggles runtime.rs's per-batch DB writes without stopping the
  // simulation itself -- ticks keep computing in memory regardless (see
  // that file's should_flush_upload). Refetches metrics right after instead
  // of waiting for the next 5s poll, so the button's own label flips
  // immediately.
  async function toggleUploadPause() {
    if (!currentSim || !accessToken) return;
    const endpoint = runtimeMetrics?.upload_paused ? 'resume-upload' : 'pause-upload';
    setUploadToggling(true);
    try {
      await axios.post(`/api/simulations/${currentSim.id}/${endpoint}`, {}, { headers: { Authorization: `Bearer ${accessToken}` } });
      const r = await axios.get(`/api/simulations/${currentSim.id}/metrics`, { headers: { Authorization: `Bearer ${accessToken}` } });
      setRuntimeMetrics(r.data);
    } catch {
      flashReport(t('İşlem başarısız', 'Action failed', 'Aktion fehlgeschlagen', 'Échec de l\'action', 'فشل الإجراء'));
    }
    setUploadToggling(false);
  }

  const [fullPauseToggling, setFullPauseToggling] = useState(false);

  // Unlike toggleUploadPause, this actually stops the tick loop from
  // advancing at all (see routes.rs's pause_simulation/start_simulation) --
  // population growth, and the memory it consumes, both freeze while
  // paused. Distinct button because the two solve different problems:
  // upload-pause keeps the simulation advancing while deferring DB writes;
  // this is for when the simulation itself (not just its DB writes) needs
  // to stop, e.g. to avoid an out-of-memory crash from unchecked population
  // growth.
  async function toggleFullPause() {
    if (!currentSim || !accessToken) return;
    const endpoint = runtimeMetrics?.status === 'paused' ? 'start' : 'pause';
    setFullPauseToggling(true);
    try {
      await axios.post(`/api/simulations/${currentSim.id}/${endpoint}`, {}, { headers: { Authorization: `Bearer ${accessToken}` } });
      const r = await axios.get(`/api/simulations/${currentSim.id}/metrics`, { headers: { Authorization: `Bearer ${accessToken}` } });
      setRuntimeMetrics(r.data);
    } catch {
      flashReport(t('İşlem başarısız', 'Action failed', 'Aktion fehlgeschlagen', 'Échec de l\'action', 'فشل الإجراء'));
    }
    setFullPauseToggling(false);
  }

  const [engineToggling, setEngineToggling] = useState<string | null>(null);

  // Diagnostic-only: flips one engine on/off in the tick loop (see
  // sim_core::TOGGLEABLE_ENGINES / runtime.rs's disabled_engines). Full-
  // replace semantics on the server, so this always POSTs the whole set,
  // not just the one that changed.
  async function toggleEngine(engine: string) {
    if (!currentSim || !accessToken) return;
    const next = new Set(runtimeMetrics?.disabled_engines ?? []);
    if (next.has(engine)) {
      next.delete(engine);
    } else {
      next.add(engine);
    }
    setEngineToggling(engine);
    try {
      await axios.post(`/api/simulations/${currentSim.id}/engines`, { disabled: Array.from(next) }, { headers: { Authorization: `Bearer ${accessToken}` } });
      const r = await axios.get(`/api/simulations/${currentSim.id}/metrics`, { headers: { Authorization: `Bearer ${accessToken}` } });
      setRuntimeMetrics(r.data);
    } catch {
      flashReport(t('İşlem başarısız', 'Action failed', 'Aktion fehlgeschlagen', 'Échec de l\'action', 'فشل الإجراء'));
    }
    setEngineToggling(null);
  }

  async function generateReport() {
    const text = buildReportText();
    setReportText(text);
    setReportFile(await saveFile(`performans-raporu-${Date.now()}.txt`, 'text/plain', text, false));
  }

  async function copyReport() {
    if (!reportText) return;
    try {
      await navigator.clipboard.writeText(reportText);
      flashReport(t('Kopyalandı!', 'Copied!', 'Kopiert!', 'Copié !', 'تم النسخ!'));
    } catch {
      // Clipboard API can be unavailable/blocked in some WebView contexts --
      // the Share/Open buttons below (once a report is generated) are the
      // fallback, rather than surprising the user with a share sheet they
      // didn't ask for.
      flashReport(t('Kopyalanamadı — Paylaş/Aç butonlarını deneyin', 'Copy failed — try Share/Open below', 'Kopieren fehlgeschlagen — Teilen/Öffnen versuchen', 'Échec de la copie — essayez Partager/Ouvrir', 'فشل النسخ — جرّب مشاركة/فتح'));
    }
  }

  return (
    <DetailPanel
      panelId="performance"
      title="Performance"
      titleTr="Performans"
      titleDe="Leistung"
      titleFr="Performance"
      titleAr="الأداء"
    >
      {/* Report: every metric currently shown in this panel as plain text --
          built specifically so exact numbers can be shared (pasted into a
          chat, an email) instead of a screenshot. Generating is a separate
          step from copy/share so the text is visible before it goes
          anywhere. */}
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, flexWrap: 'wrap' }}>
          <button
            onClick={generateReport}
            style={{
              padding: '6px 14px', fontSize: 13,
              border: '1px solid rgba(0,232,135,0.5)', color: '#00e887',
              background: 'rgba(0,232,135,0.08)', fontFamily: 'Share Tech Mono, monospace',
              cursor: 'pointer', letterSpacing: '0.06em',
            }}>
            {t('Rapor Oluştur', 'Generate Report', 'Bericht erstellen', 'Générer le rapport', 'إنشاء التقرير')}
          </button>
          {reportText && (
            <>
              <button
                onClick={copyReport}
                style={{
                  padding: '6px 14px', fontSize: 13,
                  border: '1px solid rgba(160,180,255,0.5)', color: '#a0b4ff',
                  background: 'rgba(160,180,255,0.08)', fontFamily: 'Share Tech Mono, monospace',
                  cursor: 'pointer', letterSpacing: '0.06em',
                }}>
                {t('Kopyala', 'Copy', 'Kopieren', 'Copier', 'نسخ')}
              </button>
              {reportFile && (
                <>
                  <button
                    onClick={() => shareFile(reportFile)}
                    style={{
                      padding: '6px 14px', fontSize: 13,
                      border: '1px solid rgba(212,168,56,0.5)', color: '#d4a838',
                      background: 'rgba(212,168,56,0.08)', fontFamily: 'Share Tech Mono, monospace',
                      cursor: 'pointer', letterSpacing: '0.06em',
                    }}>
                    {t('Paylaş', 'Share', 'Teilen', 'Partager', 'مشاركة')}
                  </button>
                  <button
                    onClick={() => openFile(reportFile)}
                    style={{
                      padding: '6px 14px', fontSize: 13,
                      border: '1px solid rgba(212,168,56,0.5)', color: '#d4a838',
                      background: 'rgba(212,168,56,0.08)', fontFamily: 'Share Tech Mono, monospace',
                      cursor: 'pointer', letterSpacing: '0.06em',
                    }}>
                    {t('Aç', 'Open', 'Öffnen', 'Ouvrir', 'فتح')}
                  </button>
                </>
              )}
            </>
          )}
          {reportMsg && <span style={{ fontSize: 12, color: '#8abda0' }}>{reportMsg}</span>}
        </div>
        {reportText && (
          <pre style={{
            marginTop: 10, padding: 10, maxHeight: 220, overflow: 'auto',
            background: 'rgba(0,0,0,0.25)', border: '1px solid rgba(79,110,247,0.2)',
            fontSize: 11, color: '#8abda0', fontFamily: 'Share Tech Mono, monospace',
            whiteSpace: 'pre-wrap', wordBreak: 'break-word',
          }}>{reportText}</pre>
        )}
      </div>

      {/* Warp status */}
      {isWarping && (
        <div style={{ background: 'rgba(212,168,56,0.1)', border: '1px solid rgba(212,168,56,0.4)', padding: '10px 12px', marginBottom: 10 }}>
          <div style={{ fontSize: 13, color: '#d4a838', letterSpacing: '0.12em', marginBottom: 6 }}>
            ⚡ {t('WARP MOD AKTİF', 'WARP MODE ACTIVE', 'WARP-MODUS AKTIV', 'MODE WARP ACTIF', 'وضع الوارب نشط')}
          </div>
          {warpYear && <div style={{ fontSize: 13, color: '#e0c870', marginBottom: 2 }}>{t('Hedef Yıl', 'Target Year', 'Ziel-Jahr', 'Année Cible', 'السنة المستهدفة')}: {warpYear}</div>}
          {currentYear && <div style={{ fontSize: 13, color: '#e0c870', marginBottom: 4 }}>{t('Mevcut Yıl', 'Current Year', 'Aktuelles Jahr', 'Année Actuelle', 'السنة الحالية')}: {currentYear}</div>}
          {warpPct !== null && (
            <div style={{ marginTop: 6 }}>
              <div style={{ fontSize: 12, color: '#a0c8b0', marginBottom: 3 }}>{warpPct.toFixed(1)}%</div>
              <div style={{ height: 5, background: 'rgba(255,255,255,0.08)', borderRadius: 2, overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${warpPct}%`, background: 'linear-gradient(90deg,#d4a838,#fbbf24)', transition: 'width 0.4s ease', boxShadow: '0 0 6px #d4a83860' }} />
              </div>
            </div>
          )}
        </div>
      )}

      {/* Tick timing */}
      {m ? (
        <>
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
              {t('TİCK ZAMANLAMA', 'TICK TIMING', 'TICK-TIMING', 'TIMING TICK', 'توقيت التيك')}
              {m.heavy_mode && (
                <span style={{ marginLeft: 8, fontSize: 10, color: '#d4a838', background: 'rgba(212,168,56,0.12)', border: '1px solid rgba(212,168,56,0.3)', padding: '1px 5px', letterSpacing: '0.08em' }}>
                  {t('AĞIR MOD', 'HEAVY', 'HEAVY', 'HEAVY', 'ثقيل')}
                </span>
              )}
            </div>
            {[
              { label: t('Son ms', 'Last ms', 'Letzt ms', 'Dern. ms', 'آخر ms'), value: fmtMs(m.tick_last_ms), color: '#fbbf24' },
              { label: t('Ort. ms', 'Avg ms', 'Ø ms', 'Moy. ms', 'متوسط ms'), value: fmtMs(m.tick_avg_ms), color: '#00e887' },
              { label: t('Maks ms', 'Max ms', 'Max ms', 'Max ms', 'أقصى ms'), value: fmtMs(m.tick_max_ms), color: '#e05a5a' },
              { label: t('Min ms', 'Min ms', 'Min ms', 'Min ms', 'أدنى ms'), value: fmtMs(m.tick_min_ms), color: '#4ecb71' },
            ].map(({ label, value, color }) => (
              <div key={label} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5, fontSize: 13 }}>
                <span style={{ color: '#8abda0' }}>{label}</span>
                <span style={{ color, fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{value}</span>
              </div>
            ))}
            {m.tick_last_ms != null && bar(m.tick_last_ms, 500, '#fbbf24')}
            {m.tick_avg_ms !== undefined && bar(m.tick_avg_ms, 200, '#4f6ef7')}
          </div>

          {/* Phase breakdown: attributes a slow tick to DB/network latency
              (load/save/upsert) vs. actual sim computation, rather than
              leaving "why is 20x only running at 4x" a guessing game. These
              are per-batch totals, unlike the per-day figures above. */}
          {(m.tick_load_ms != null || m.tick_compute_ms != null || m.tick_save_ms != null || m.tick_upsert_ms != null) && (
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
                {t('SON PARÇA — AŞAMA DETAYI', 'LAST BATCH — PHASE BREAKDOWN', 'LETZTER BATCH — PHASEN', 'DERNIER LOT — DÉTAIL', 'آخر دفعة — تفصيل المراحل')}
              </div>
              {[
                { label: t('Yükleme (DB)', 'Load (DB)', 'Laden (DB)', 'Chargement (DB)', 'تحميل (DB)'), value: m.tick_load_ms, color: '#7dd3fc' },
                { label: t('Hesaplama', 'Compute', 'Berechnung', 'Calcul', 'الحساب'), value: m.tick_compute_ms, color: '#00e887' },
                { label: t('Kaydet (DB)', 'Save (DB)', 'Speichern (DB)', 'Sauvegarde (DB)', 'حفظ (DB)'), value: m.tick_save_ms, color: '#fbbf24' },
                { label: t('Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)', 'Upsert (DB)'), value: m.tick_upsert_ms, color: '#e05a5a' },
              ].map(({ label, value, color }) => (
                <div key={label} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5, fontSize: 13 }}>
                  <span style={{ color: '#8abda0' }}>{label}</span>
                  <span style={{ color, fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{value != null ? `${fmtMs(value)} ms` : '—'}</span>
                </div>
              ))}
              <div style={{ fontSize: 11, color: '#5a7a68', marginTop: 4, lineHeight: 1.4 }}>
                {t(
                  'Yükleme/Kaydet/Upsert yüksekse darboğaz veritabanı ağ gecikmesidir; Hesaplama yüksekse motor kendisi yavaştır.',
                  'If Load/Save/Upsert dominate, the bottleneck is DB network latency; if Compute dominates, the engine itself is slow.',
                  'Wenn Laden/Speichern/Upsert dominieren, liegt es an der DB-Latenz; wenn Berechnung dominiert, ist die Engine selbst langsam.',
                  'Si Chargement/Sauvegarde/Upsert dominent, la latence DB est en cause ; si Calcul domine, le moteur est lent.',
                  'إذا هيمن التحميل/الحفظ/Upsert فالسبب زمن استجابة قاعدة البيانات؛ وإذا هيمن الحساب فالمحرك نفسه بطيء.'
                )}
              </div>
            </div>
          )}

          {moduleRows.length > 0 && (
            <div style={{ marginBottom: 12, borderTop: '1px solid rgba(79,110,247,0.18)', paddingTop: 10 }}>
              <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
                {t('MODÜL / PERFORMANS', 'MODULE / PERFORMANCE', 'MODUL / LEISTUNG', 'MODULE / PERFORMANCE', 'الوحدات / الأداء')}
              </div>
              {moduleRows.map(({ label, value, color, engine }) => {
                const isDisabled = engine != null && disabledEngines.has(engine);
                return (
                  <div key={label} style={{ marginBottom: 7 }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 3, fontSize: 13, gap: 8 }}>
                      <span style={{ color: isDisabled ? '#5a7a68' : '#8abda0', textDecoration: isDisabled ? 'line-through' : 'none' }}>{label}</span>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
                        <span style={{ color: isDisabled ? '#5a7a68' : color, fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{fmtMs(value)} ms</span>
                        {engine && (
                          <button
                            onClick={() => toggleEngine(engine)}
                            disabled={engineToggling === engine}
                            title={t('Sadece teşhis amaçlı — motoru geçici kapatır, simülasyon tutarsız hale gelebilir', 'Diagnostic only — temporarily disables the engine, the simulation may become inconsistent', 'Nur zur Diagnose', 'Diagnostic uniquement', 'للتشخيص فقط')}
                            style={{
                              padding: '2px 8px', fontSize: 10,
                              border: isDisabled ? '1px solid rgba(0,232,135,0.5)' : '1px solid rgba(224,90,90,0.5)',
                              color: isDisabled ? '#00e887' : '#e05a5a',
                              background: isDisabled ? 'rgba(0,232,135,0.08)' : 'rgba(224,90,90,0.08)',
                              fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', letterSpacing: '0.04em',
                              opacity: engineToggling === engine ? 0.6 : 1,
                            }}>
                            {isDisabled ? t('AÇ', 'ON', 'AN', 'ON', 'تشغيل') : t('KAPAT', 'OFF', 'AUS', 'OFF', 'إيقاف')}
                          </button>
                        )}
                      </div>
                    </div>
                    {bar(value, moduleMax, isDisabled ? '#3a4a44' : color)}
                  </div>
                );
              })}
              <div style={{ fontSize: 11, color: '#5a7a68', marginTop: 4, lineHeight: 1.4 }}>
                {t(
                  'Hesaplama süresinin (yukarıdaki) motor grupları arasındaki dağılımı — son parçadaki toplam. Bir grup diğerlerine göre sürekli baskınsa yavaşlamanın kaynağı orasıdır. KAPAT butonları sadece teşhis amaçlıdır — kapatılan motor simülasyonu tutarsız bir duruma sokabilir (örn. kimse ölmez/doğmaz).',
                  'How the Compute time above splits across engine groups — totals for the last batch. Whichever group is consistently dominant is the source of a slowdown. The OFF buttons are diagnostic only — a disabled engine can leave the simulation in an inconsistent state (e.g. nobody dies/is born).',
                  'Aufteilung der obigen Berechnungszeit auf Engine-Gruppen — Summen des letzten Batches. Die AUS-Schalter dienen nur der Diagnose.',
                  'Répartition du temps de calcul ci-dessus entre les groupes de moteurs — totaux du dernier lot. Les boutons OFF sont uniquement à des fins de diagnostic.',
                  'توزيع وقت الحساب أعلاه بين مجموعات المحرك — إجماليات آخر دفعة. أزرار الإيقاف لأغراض التشخيص فقط.'
                )}
              </div>
            </div>
          )}

          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>{t('MOTOR', 'ENGINE', 'MOTOR', 'MOTEUR', 'المحرك')}</div>
            {[
              { label: t('Tik/sn', 'Ticks/s', 'Ticks/s', 'Ticks/s', 'تيك/ث'), value: m.ticks_per_second?.toFixed(1) ?? '—', color: '#d4a838' },
              { label: t('Hız', 'Speed', 'Tempo', 'Vitesse', 'السرعة'), value: `${m.speed_multiplier ?? 1}×`, color: '#a0b4ff' },
              { label: t('Gün', 'Day', 'Tag', 'Jour', 'اليوم'), value: m.current_day ?? '—', color: '#7dd3fc' },
              { label: t('Yaşayan', 'Alive', 'Lebend', 'Vivants', 'أحياء'), value: m.population ?? '—', color: '#00e887' },
              { label: t('Toplam', 'Total Ever', 'Gesamt', 'Total', 'المجموع'), value: m.total_ever ?? '—', color: '#4ecb71' },
              { label: t('CPU Çekirdek', 'CPU Cores', 'CPU-Kerne', 'Cœurs CPU', 'أنوية المعالج'), value: `${m.cpu_cores_used ?? '—'} / ${m.cpu_cores_available ?? '—'}`, color: '#f7a04f' },
              ...(m.cross_origin_isolated !== undefined
                ? [{
                    label: t('Cross-Origin Isolation', 'Cross-Origin Isolation', 'Cross-Origin Isolation', 'Isolation Cross-Origin', 'عزل المصدر المتقاطع'),
                    value: m.cross_origin_isolated
                      ? t('Aktif', 'Active', 'Aktiv', 'Actif', 'نشط')
                      : t('Devre dışı', 'Inactive', 'Inaktiv', 'Inactif', 'غير نشط'),
                    color: m.cross_origin_isolated ? '#00e887' : '#e05a5a',
                  }]
                : []),
            ].map(({ label, value, color }) => (
              <div key={label} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5, fontSize: 13 }}>
                <span style={{ color: '#8abda0' }}>{label}</span>
                <span style={{ color, fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>{value}</span>
              </div>
            ))}
            {m.thread_pool_error && (
              <div style={{ fontSize: 11, color: '#e05a5a', marginTop: 4, lineHeight: 1.4 }}>
                {t('Çekirdek havuzu hatası', 'Thread pool error', 'Thread-Pool-Fehler', 'Erreur du pool de threads', 'خطأ تجمع الخيوط')}: {m.thread_pool_error}
              </div>
            )}
          </div>

          {/* Milestones reached */}
          {m.milestones_reached?.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
                {t('ERİŞİLEN MILESTONE\'LAR', 'MILESTONES REACHED', 'ERREICHTE MEILENSTEINE', 'JALONS ATTEINTS', 'المعالم المحققة')} ({m.milestones_reached.length})
              </div>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
                {m.milestones_reached.map(key => (
                  <span key={key} style={{ fontSize: 12, color: '#d4a838', border: '1px solid rgba(212,168,56,0.3)', padding: '2px 7px', letterSpacing: '0.06em' }}>
                    {MILESTONE_I18N[key] ? text(lang as LangCode, MILESTONE_I18N[key]) : key.replace(/_/g, ' ')}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Centroid trail */}
          {m.centroid_trail?.length > 1 && (
            <div>
              <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
                {t('GÖÇ ROTALARI', 'MIGRATION PATH', 'MIGRATIONSPFAD', 'CHEMIN MIGRATION', 'مسار الهجرة')} ({m.centroid_trail.length} {t('nokta', 'points', 'Punkte', 'points', 'نقاط')})
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
                {(() => {
                  const first = m.centroid_trail[0];
                  const last = m.centroid_trail[m.centroid_trail.length - 1];
                  return [
                    { label: t('Başlangıç X', 'Start X', 'Start X', 'Départ X', 'بداية X'), value: first.x.toFixed(2) },
                    { label: t('Başlangıç Y', 'Start Y', 'Start Y', 'Départ Y', 'بداية Y'), value: first.y.toFixed(2) },
                    { label: t('Mevcut X', 'Current X', 'Aktuell X', 'Actuel X', 'حالي X'), value: last.x.toFixed(2) },
                    { label: t('Mevcut Y', 'Current Y', 'Aktuell Y', 'Actuel Y', 'حالي Y'), value: last.y.toFixed(2) },
                  ].map(({ label, value }) => (
                    <div key={label} style={{ fontSize: 12 }}>
                      <div style={{ color: '#6090a0', marginBottom: 2 }}>{label}</div>
                      <div style={{ color: '#7dd3fc', fontFamily: 'Orbitron, monospace', fontSize: 13 }}>{value}</div>
                    </div>
                  ));
                })()}
              </div>
            </div>
          )}
        </>
      ) : (
        <div style={{ fontSize: 13, color: '#6090a0', fontStyle: 'italic' }}>
          {t('Metrikler yükleniyor...', 'Loading metrics...', 'Metriken laden...', 'Chargement...', 'جارٍ التحميل...')}
        </div>
      )}

      {/* ── DB Status ── */}
      {dbStatus && (
        <div style={{ marginTop: 14, borderTop: '1px solid rgba(79,110,247,0.2)', paddingTop: 12, marginBottom: 4 }}>
          <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
            {t('VERİTABANI DURUMU', 'DATABASE STATUS', 'DATENBANK STATUS', 'ÉTAT BASE DE DONNÉES', 'حالة قاعدة البيانات')}
          </div>

          {/* Sim DB */}
          <div style={{ marginBottom: 10 }}>
            <div style={{ fontSize: 11, color: '#a0b4ff', letterSpacing: '0.1em', marginBottom: 5 }}>
              {t('SİM DB (yerel)', 'SIM DB (local)', 'SIM DB (lokal)', 'SIM DB (local)', 'قاعدة بيانات المحاكاة')}
              {dbStatus.sim_db.size_bytes !== null && (
                <span style={{ float: 'right', color: '#6090a0' }}>
                  {dbStatus.sim_db.size_bytes > 1048576
                    ? `${(dbStatus.sim_db.size_bytes / 1048576).toFixed(1)} MB`
                    : `${(dbStatus.sim_db.size_bytes / 1024).toFixed(0)} KB`}
                </span>
              )}
            </div>
            {[
              { label: t('Birey (toplam/yaşayan)', 'Individuals (total/alive)', 'Individuen', 'Individus', 'الأفراد'), value: `${dbStatus.sim_db.individuals.total} / ${dbStatus.sim_db.individuals.alive}`, color: '#00e887' },
              { label: t('Checkpoint', 'Checkpoints', 'Checkpoints', 'Sauvegardes', 'نقاط حفظ'), value: dbStatus.sim_db.checkpoints, color: '#7dd3fc' },
              { label: t('Olay', 'Events', 'Ereignisse', 'Événements', 'أحداث'), value: dbStatus.sim_db.events, color: '#7dd3fc' },
              { label: t('Teknoloji', 'Technologies', 'Technologien', 'Technologies', 'تقنيات'), value: dbStatus.sim_db.technologies, color: '#d4a838' },
              { label: t('İnanç', 'Beliefs', 'Glaubenssätze', 'Croyances', 'معتقدات'), value: dbStatus.sim_db.beliefs, color: '#d4a838' },
              { label: t('Dil kaydı', 'Language records', 'Sprachaufzeichnungen', 'Enreg. langue', 'سجلات اللغة'), value: dbStatus.sim_db.languages, color: '#d4a838' },
              { label: t('Grup', 'Groups', 'Gruppen', 'Groupes', 'مجموعات'), value: dbStatus.sim_db.groups, color: '#a0b4ff' },
              { label: t('Konuşma', 'Conversations', 'Gespräche', 'Conversations', 'محادثات'), value: dbStatus.sim_db.conversations, color: '#a0b4ff' },
            ].map(({ label, value, color }) => (
              <div key={String(label)} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4, fontSize: 12 }}>
                <span style={{ color: '#6090a0' }}>{label}</span>
                <span style={{ color, fontFamily: 'Orbitron, monospace', fontWeight: 700, fontSize: 11 }}>{value}</span>
              </div>
            ))}
          </div>

          {/* Cloud DB */}
          <div>
            <div style={{ fontSize: 11, color: '#a0b4ff', letterSpacing: '0.1em', marginBottom: 5 }}>
              {t('BULUT DB (Render)', 'CLOUD DB (Render)', 'CLOUD DB (Render)', 'CLOUD DB (Render)', 'قاعدة بيانات السحاب')}
              {dbStatus.cloud_db.size_bytes !== null && (
                <span style={{ float: 'right', color: '#6090a0' }}>
                  {dbStatus.cloud_db.size_bytes > 1048576
                    ? `${(dbStatus.cloud_db.size_bytes / 1048576).toFixed(1)} MB`
                    : `${(dbStatus.cloud_db.size_bytes / 1024).toFixed(0)} KB`}
                </span>
              )}
            </div>
            {[
              { label: t('Bulut checkpoint', 'Cloud checkpoints', 'Cloud-Checkpoints', 'Sauvegardes cloud', 'نقاط حفظ السحاب'), value: dbStatus.cloud_db.cloud_checkpoints, color: '#7dd3fc' },
              { label: t('Canlı anlık görüntü', 'Live snapshots', 'Live-Snapshots', 'Instantanés live', 'لقطات مباشرة'), value: dbStatus.cloud_db.live_snapshots, color: '#7dd3fc' },
            ].map(({ label, value, color }) => (
              <div key={String(label)} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4, fontSize: 12 }}>
                <span style={{ color: '#6090a0' }}>{label}</span>
                <span style={{ color, fontFamily: 'Orbitron, monospace', fontWeight: 700, fontSize: 11 }}>{value}</span>
              </div>
            ))}
          </div>

          {/* Pause/resume the tick loop's own per-batch DB writes -- the
              simulation itself keeps running/computing in memory either
              way, only the cloud round trips stop. A periodic safety-net
              flush still runs while paused (runtime.rs), but a crash mid-
              pause can lose up to that window. */}
          <div style={{ marginTop: 10, paddingTop: 10, borderTop: '1px solid rgba(79,110,247,0.15)', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10 }}>
            <span style={{ fontSize: 12, color: runtimeMetrics?.upload_paused ? '#e05a5a' : '#6090a0' }}>
              {runtimeMetrics?.upload_paused
                ? t('Buluta yükleme duraklatıldı', 'Cloud upload paused', 'Cloud-Upload pausiert', 'Envoi cloud en pause', 'تم إيقاف الرفع السحابي مؤقتًا')
                : t('Buluta yükleme aktif', 'Cloud upload active', 'Cloud-Upload aktiv', 'Envoi cloud actif', 'الرفع السحابي نشط')}
            </span>
            <button
              onClick={toggleUploadPause}
              disabled={uploadToggling}
              style={{
                padding: '5px 12px', fontSize: 12,
                border: runtimeMetrics?.upload_paused ? '1px solid rgba(0,232,135,0.5)' : '1px solid rgba(224,90,90,0.5)',
                color: runtimeMetrics?.upload_paused ? '#00e887' : '#e05a5a',
                background: runtimeMetrics?.upload_paused ? 'rgba(0,232,135,0.08)' : 'rgba(224,90,90,0.08)',
                fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', letterSpacing: '0.06em',
                opacity: uploadToggling ? 0.6 : 1,
              }}>
              {runtimeMetrics?.upload_paused
                ? t('Devam Et', 'Resume', 'Fortsetzen', 'Reprendre', 'استئناف')
                : t('Duraklat', 'Pause', 'Pausieren', 'Pause', 'إيقاف مؤقت')}
            </button>
          </div>

          {/* Fully stops the tick loop (see toggleFullPause's own comment) --
              population/memory growth actually freezes, unlike upload-pause
              above which only defers DB writes while ticks keep computing. */}
          {runtimeMetrics?.status !== 'completed' && (
            <div style={{ marginTop: 10, paddingTop: 10, borderTop: '1px solid rgba(79,110,247,0.15)', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10 }}>
              <span style={{ fontSize: 12, color: runtimeMetrics?.status === 'paused' ? '#e05a5a' : '#6090a0' }}>
                {runtimeMetrics?.status === 'paused'
                  ? t('Simülasyon tamamen duraklatıldı', 'Simulation fully paused', 'Simulation vollständig pausiert', 'Simulation entièrement en pause', 'المحاكاة متوقفة تمامًا')
                  : t('Simülasyon çalışıyor', 'Simulation running', 'Simulation läuft', 'Simulation en cours', 'المحاكاة قيد التشغيل')}
              </span>
              <button
                onClick={toggleFullPause}
                disabled={fullPauseToggling}
                title={t(
                  'Nüfus/hesaplama tamamen durur -- Yükleme Durdur\'un aksine, bellek büyümesi de durur.',
                  'Population/computation stops entirely -- unlike Pause Upload, memory growth stops too.',
                  'Bevölkerung/Berechnung stoppt vollständig -- im Gegensatz zu Upload pausieren stoppt auch das Speicherwachstum.',
                  'Population/calcul s\'arrêtent complètement -- contrairement à Pause upload, la croissance mémoire aussi.',
                  'يتوقف النمو السكاني/الحساب تمامًا -- على عكس إيقاف الرفع، ينمو الذاكرة يتوقف أيضًا.',
                )}
                style={{
                  padding: '5px 12px', fontSize: 12,
                  border: runtimeMetrics?.status === 'paused' ? '1px solid rgba(0,232,135,0.5)' : '1px solid rgba(224,90,90,0.5)',
                  color: runtimeMetrics?.status === 'paused' ? '#00e887' : '#e05a5a',
                  background: runtimeMetrics?.status === 'paused' ? 'rgba(0,232,135,0.08)' : 'rgba(224,90,90,0.08)',
                  fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', letterSpacing: '0.06em',
                  opacity: fullPauseToggling ? 0.6 : 1,
                }}>
                {runtimeMetrics?.status === 'paused'
                  ? t('Devam Et', 'Resume', 'Fortsetzen', 'Reprendre', 'استئناف')
                  : t('Tam Duraklat', 'Full Pause', 'Vollständig pausieren', 'Pause complète', 'إيقاف كامل')}
              </button>
            </div>
          )}
        </div>
      )}

      {/* ── Live connection status ── */}
      {/* Self-diagnosis for "the day counter looks frozen" reports: shows
          whether the live-update socket is actually open and how long ago
          it last heard from the server, without needing a USB-attached
          remote debugger to see the same thing in the console. */}
      <div style={{ marginTop: 14, borderTop: '1px solid rgba(79,110,247,0.2)', paddingTop: 12 }}>
        <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
          {t('CANLI BAĞLANTI', 'LIVE CONNECTION', 'LIVE-VERBINDUNG', 'CONNEXION EN DIRECT', 'الاتصال المباشر')}
        </div>
        {(() => {
          const statusColor = wsStatus === 'open' ? '#00e887' : wsStatus === 'connecting' ? '#c8a840' : '#e05a5a';
          const statusLabel = {
            open: t('BAĞLI', 'CONNECTED', 'VERBUNDEN', 'CONNECTÉ', 'متصل'),
            connecting: t('BAĞLANIYOR…', 'CONNECTING…', 'VERBINDE…', 'CONNEXION…', 'جارٍ الاتصال…'),
            closed: t('BAĞLANTI YOK', 'DISCONNECTED', 'GETRENNT', 'DÉCONNECTÉ', 'غير متصل'),
            error: t('HATA', 'ERROR', 'FEHLER', 'ERREUR', 'خطأ'),
          }[wsStatus];
          const secondsAgo = wsLastMessageAt ? Math.max(0, Math.round((Date.now() - wsLastMessageAt) / 1000)) : null;
          return (
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12 }}>
              <span style={{ width: 8, height: 8, borderRadius: '50%', background: statusColor, flexShrink: 0, boxShadow: `0 0 6px ${statusColor}` }} />
              <span style={{ color: statusColor }}>{statusLabel}</span>
              {secondsAgo !== null && (
                <span style={{ color: '#6090a0' }}>
                  · {t(
                    `son mesaj ${secondsAgo}sn önce`,
                    `last message ${secondsAgo}s ago`,
                    `letzte Nachricht vor ${secondsAgo}s`,
                    `dernier message il y a ${secondsAgo}s`,
                    `آخر رسالة قبل ${secondsAgo}ث`,
                  )}
                </span>
              )}
              {wsReconnectCount > 0 && (
                <span style={{ color: '#c8a840' }}>
                  · {t(`${wsReconnectCount}x yeniden bağlanma`, `${wsReconnectCount}x reconnects`, `${wsReconnectCount}x Wiederverbindungen`, `${wsReconnectCount}x reconnexions`, `${wsReconnectCount}x إعادة اتصال`)}
                </span>
              )}
              {wsStatus !== 'open' && wsCloseInfo && (
                <span style={{ color: '#6090a0' }}>
                  · code={wsCloseInfo.code}{wsCloseInfo.reason ? ` (${wsCloseInfo.reason})` : ''}
                </span>
              )}
            </div>
          );
        })()}
      </div>

      {/* ── Diagnostics ── */}
      {diag && (
        <div style={{ marginTop: 14, borderTop: '1px solid rgba(79,110,247,0.2)', paddingTop: 12 }}>
          <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
            {t('BAŞLANGIÇ KONTROLÜ', 'STARTUP CHECKS', 'STARTUP-CHECKS', 'VÉRIF. DÉMARRAGE', 'فحوصات البدء')}
          </div>

          {diag.startup ? (
            <div style={{ marginBottom: 10 }}>
              {diag.startup.checks.map((c, i) => {
                const detail = formatCheckDetail(c);
                return (
                  <div key={i} style={{ display: 'flex', alignItems: 'flex-start', gap: 6, marginBottom: 4, fontSize: 12 }}>
                    <span style={{ color: c.ok ? '#00e887' : '#e05a5a', flexShrink: 0, lineHeight: '1.4' }}>{c.ok ? '✓' : '✗'}</span>
                    <div>
                      <span style={{ color: c.ok ? '#8abda0' : '#e05a5a' }}>{c.name}</span>
                      {detail && <span style={{ color: '#6090a0', marginLeft: 5 }}>{detail}</span>}
                    </div>
                  </div>
                );
              })}
              <div style={{ fontSize: 11, color: '#4a6060', marginTop: 4 }}>
                {t('Gün', 'Day', 'Tag', 'Jour', 'يوم')} {diag.startup.day} — {new Date(diag.startup.ts).toLocaleTimeString(LOCALE_MAP[lang] ?? 'en-US')}
              </div>
            </div>
          ) : (
            <div style={{ fontSize: 12, color: '#6090a0', fontStyle: 'italic', marginBottom: 10 }}>
              {t('Simülasyon henüz başlamadı', 'Simulation not started yet', 'Noch nicht gestartet', 'Pas encore démarré', 'لم يبدأ بعد')}
            </div>
          )}

          <div style={{ fontSize: 12, color: '#4f6ef7', letterSpacing: '0.14em', marginBottom: 8 }}>
            {t('HATA KAYDI', 'ERROR LOG', 'FEHLERPROTOKOLL', 'JOURNAL D\'ERREURS', 'سجل الأخطاء')}
            {(diag.consecutive_errors ?? 0) > 0 && (
              <span style={{ marginLeft: 8, color: '#e05a5a', background: 'rgba(224,90,90,0.15)', padding: '1px 6px', borderRadius: 3 }}>
                {diag.consecutive_errors}/5
              </span>
            )}
          </div>

          {(diag.error_log?.length ?? 0) === 0 ? (
            <div style={{ fontSize: 12, color: '#4ecb71' }}>
              {t('✓ Hata yok', '✓ No errors', '✓ Keine Fehler', '✓ Aucune erreur', '✓ لا أخطاء')}
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {[...(diag.error_log ?? [])].reverse().map((e, i) => (
                <div key={i} style={{ background: 'rgba(224,90,90,0.08)', border: '1px solid rgba(224,90,90,0.25)', padding: '5px 8px' }}>
                  <div style={{ fontSize: 12, color: '#e05a5a', marginBottom: 2 }}>
                    {t('Gün', 'Day', 'Tag', 'Jour', 'يوم')} {e.day} — {new Date(e.ts).toLocaleTimeString(LOCALE_MAP[lang] ?? 'en-US')}
                  </div>
                  <div style={{ fontSize: 11, color: '#e0b0b0', wordBreak: 'break-word' }}>{e.msg}</div>
                  {e.stack && (
                    <div style={{ fontSize: 10, color: '#8a5050', marginTop: 3, wordBreak: 'break-word', opacity: 0.8 }}>{e.stack}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </DetailPanel>
  );
}
