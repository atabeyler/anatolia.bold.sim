import { useState, useRef } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { Download, FileJson, FileDown, Printer } from 'lucide-react';
import jsPDF from 'jspdf';
import html2canvas from 'html2canvas';
import { text, describeBeliefCode, beliefCodeNumber, CAUSE_LABELS, translateSeason, translateWeather, translateDrive, translateMigrationReason, translateTech, translateArtForm, translateEventType, translateEventDescription, translateRole, translateMentalState, UNNAMED_LABEL, type LangCode } from '../../utils/i18n';
import { saveFile, shareFile, openFile, type SavedFile } from '../../utils/fileExport';

// downloadPDF()'s offscreen render container -- shared style so every chunk
// (cover, section, individuals-batch) looks identical to how the old
// single-giant-canvas render looked, just assembled from smaller pieces.
const CHUNK_WRAPPER_STYLE = 'position:fixed;left:-9999px;top:0;width:794px;background:#fff;padding:40px 44px;font-family:Arial,Helvetica,sans-serif;';
// How many Individuals rows go into one html2canvas call. The Individuals
// array is the one report field that never shrinks (dead individuals stay
// in it forever) and can reach the thousands on a long-running/high-
// population simulation -- rendering it all as one canvas is what actually
// froze the tab (not the PDF library itself, but rasterizing a DOM that
// large). Batching keeps every single html2canvas call bounded regardless
// of total population, without dropping a single row from the PDF.
const INDIVIDUALS_BATCH_SIZE = 150;

// Splits a `container`'s direct children into groups, starting a new group
// at each HTML comment marker (buildReportHtml() prefixes every section with
// one, e.g. `<!-- BİREYLER -->`) -- these comments survive as real Comment
// nodes once the HTML string is parsed via innerHTML, so this needs no
// changes to buildReportHtml() beyond the markers it already had. Purely
// whitespace text nodes (template-literal newlines/indentation) are dropped
// since they'd otherwise start empty trailing chunks.
// requestAnimationFrame only fires around an actual compositor frame -- on a
// backgrounded/hidden tab (screen off, app switched away, or an Android
// WebView that quietly stops compositing while a native Filesystem/Share
// bridge call is in flight) it can go silent indefinitely, so anything
// awaiting it hangs rather than just running slower. Confirmed directly:
// swapping this in for rAF-based yields below is what turned an
// intermittent, unreproducible-looking freeze into a clean pass in a
// realistic (backgrounded-composited) test run. setTimeout is a macrotask,
// not tied to painting, so it keeps firing regardless of tab visibility --
// exactly what "yield so the browser stays responsive" actually needs here.
const yieldToMainThread = () => new Promise<void>(resolve => setTimeout(resolve, 0));

// jsPDF's own `output('datauristring')` base64-encodes the *entire* finished
// document in one uninterrupted synchronous call -- for a long report (many
// pages, each carrying an embedded raster image) that can block the main
// thread for several seconds with zero yields, right at the very end of
// downloadPDF(). Encoding from the raw arraybuffer ourselves, in bounded
// chunks with a yield in between, keeps this step exactly as responsive as
// the rendering loop above it.
async function pdfToBase64Chunked(pdf: jsPDF): Promise<string> {
  const bytes = new Uint8Array(pdf.output('arraybuffer'));
  const BYTE_CHUNK = 300_000;
  const parts: string[] = [];
  for (let i = 0; i < bytes.length; i += BYTE_CHUNK) {
    let binary = '';
    const end = Math.min(i + BYTE_CHUNK, bytes.length);
    for (let j = i; j < end; j += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(j, Math.min(j + 0x8000, end)));
    }
    parts.push(btoa(binary));
    await yieldToMainThread();
  }
  return parts.join('');
}

export function splitIntoSectionChunks(root: HTMLElement): ChildNode[][] {
  const chunks: ChildNode[][] = [];
  let current: ChildNode[] = [];
  for (const node of Array.from(root.childNodes)) {
    if (node.nodeType === Node.COMMENT_NODE) {
      if (current.length) chunks.push(current);
      current = [];
      continue;
    }
    if (node.nodeType === Node.TEXT_NODE && !(node.textContent ?? '').trim()) continue;
    current.push(node);
  }
  if (current.length) chunks.push(current);
  return chunks;
}

// Rasterizes one already-off-screen-attached node and appends it to `pdf`,
// paginating if it's taller than one page (same slicing math the original
// single-canvas downloadPDF used). Returns false always -- callers pass the
// return value in as the next call's `isFirstPageOfDoc` so only the very
// first image in the whole document skips jsPDF's own implicit first page.
async function renderNodeToPdf(node: HTMLElement, pdf: jsPDF, pageW: number, pageH: number, isFirstPageOfDoc: boolean): Promise<boolean> {
  const canvas = await html2canvas(node, { scale: 2, useCORS: true, backgroundColor: '#fff' });
  // PNG is lossless -- for a mostly-white, text-and-table report that meant
  // a single small sim's PDF (~23 individuals) came out to 117MB once every
  // chunk's raster was embedded uncompressed. That's the real reason PDF
  // generation could freeze/crash a mobile WebView right at the end: base64-
  // encoding and handing 100+MB to the native Filesystem bridge is enough to
  // exhaust a constrained device's memory outright. JPEG at a high quality
  // is visually indistinguishable for this content and shrinks the embedded
  // image by roughly an order of magnitude.
  const imgData = canvas.toDataURL('image/jpeg', 0.85);
  const imgW = pageW;
  const imgH = (canvas.height * pageW) / canvas.width;
  let posY = 0;
  let firstPage = isFirstPageOfDoc;
  while (posY < imgH) {
    if (!firstPage) pdf.addPage();
    firstPage = false;
    pdf.addImage(imgData, 'JPEG', 0, -posY, imgW, imgH);
    posY += pageH;
  }
  // html2canvas's backing pixel buffer (at scale:2, easily tens of MB per
  // chunk) otherwise lingers until the next GC pass. Rendering many chunks/
  // batches in quick succession without this was enough to exhaust memory
  // on a constrained mobile WebView and crash/reload the whole page right
  // as the last one finished -- reported as the Report panel disappearing
  // the moment the PDF progress hit its final chunk. Zeroing the canvas's
  // dimensions forces the browser to release that backing store
  // immediately instead of waiting for garbage collection.
  canvas.width = 0;
  canvas.height = 0;
  return firstPage;
}

// Renders the (potentially huge) Individuals table in row batches instead of
// one shot -- `headerNodes` (the section's title bar/any note) rides along
// with the first batch only, matching how the single-canvas render used to
// show them once, right above the table.
async function renderIndividualsInBatches(
  table: HTMLTableElement,
  headerNodes: ChildNode[],
  pdf: jsPDF,
  pageW: number,
  pageH: number,
  isFirstPageOfDoc: boolean,
  onProgress: (label: string) => void,
): Promise<boolean> {
  const allRows = Array.from(table.rows);
  const headerRow = allRows[0];
  const dataRows = allRows.slice(1);
  const totalBatches = Math.max(1, Math.ceil(dataRows.length / INDIVIDUALS_BATCH_SIZE));
  let firstPage = isFirstPageOfDoc;
  for (let b = 0; b < totalBatches; b++) {
    const batchRows = dataRows.slice(b * INDIVIDUALS_BATCH_SIZE, (b + 1) * INDIVIDUALS_BATCH_SIZE);
    const batchTable = document.createElement('table');
    batchTable.setAttribute('style', table.getAttribute('style') ?? '');
    batchTable.appendChild(headerRow.cloneNode(true));
    for (const r of batchRows) batchTable.appendChild(r.cloneNode(true));

    const wrapper = document.createElement('div');
    wrapper.style.cssText = CHUNK_WRAPPER_STYLE;
    if (b === 0) for (const n of headerNodes) wrapper.appendChild(n);
    if (totalBatches > 1) {
      const note = document.createElement('p');
      note.style.cssText = 'color:#9ca3af;font-size:10px;margin:-4px 0 8px 0;';
      note.textContent = `(${b + 1}/${totalBatches})`;
      wrapper.appendChild(note);
    }
    wrapper.appendChild(batchTable);
    document.body.appendChild(wrapper);
    onProgress(`${b + 1} / ${totalBatches}`);
    firstPage = await renderNodeToPdf(wrapper, pdf, pageW, pageH, firstPage);
    document.body.removeChild(wrapper);
    // Yields to the browser's event loop between batches so input/paint can
    // still happen -- this (plus the batching itself) is what keeps the tab
    // responsive instead of freezing for however long the whole table takes.
    await yieldToMainThread();
  }
  return firstPage;
}

export default function ReportPanel() {
  const { currentSim, accessToken, lang, stats, events } = useSimStore();
  const [loading, setLoading] = useState(false);
  const [pdfLoading, setPdfLoading] = useState(false);
  const [pdfProgress, setPdfProgress] = useState('');
  const [msg, setMsg] = useState('');
  const [jsonFile, setJsonFile] = useState<SavedFile | null>(null);
  const [pdfFile, setPdfFile] = useState<SavedFile | null>(null);
  const reportRef = useRef<HTMLDivElement>(null);

  function flash(text: string) { setMsg(text); setTimeout(() => setMsg(''), 4000); }

  async function downloadJSON() {
    if (!currentSim || !accessToken) return;
    setLoading(true);
    try {
      const { data } = await axios.get(`/api/simulations/${currentSim.id}/report`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      });
      const filename = `anatolia-sim-${currentSim.name ?? currentSim.id}-Y${stats?.year ?? 0}.json`;
      setJsonFile(await saveFile(filename, 'application/json', JSON.stringify(data, null, 2), false));
      flash(text(lang as LangCode, { en: '✓ JSON downloaded.', tr: '✓ JSON indirildi.', de: '✓ JSON heruntergeladen.', fr: '✓ JSON téléchargé.', ar: '✓ تم تنزيل JSON.' }));
    } catch {
      flash(text(lang as LangCode, { en: '✗ Download failed.', tr: '✗ İndirme başarısız.', de: '✗ Download fehlgeschlagen.', fr: '✗ Échec du téléchargement.', ar: '✗ فشل التنزيل.' }));
    }
    setLoading(false);
  }

  async function buildReportHtml(): Promise<string> {
    const sim = currentSim!;
    const { data: r } = await axios.get(`/api/simulations/${sim.id}/report`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });

      const S = r.current_stats ?? {};
      const localeMap: Record<string, string> = { tr: 'tr-TR', en: 'en-US', de: 'de-DE', fr: 'fr-FR', ar: 'ar-SA' };
      const now = new Date().toLocaleString(localeMap[lang] ?? 'en-US');
      const rt = (tr: string, en: string, de?: string, fr?: string, ar?: string) => {
        if (lang === 'tr') return tr;
        if (lang === 'de') return de ?? en;
        if (lang === 'fr') return fr ?? en;
        if (lang === 'ar') return ar ?? en;
        return en;
      };
      const th = (s: string) => `<th style="border:1px solid #bbb;padding:4px 6px;background:#f0f0f0;font-size:11px;text-align:left;">${s}</th>`;
      const td = (s: unknown) => `<td style="border:1px solid #ddd;padding:3px 6px;font-size:11px;">${s != null ? String(s) : '—'}</td>`;
      const tr2 = (...cells: unknown[]) => `<tr>${cells.map(td).join('')}</tr>`;
      const sec = (title: string) => `<h2 style="font-size:14px;border-bottom:2px solid #333;padding-bottom:4px;margin:24px 0 8px 0;">${title}</h2>`;
      const tbl = (headers: string[], rows: string) =>
        `<table style="width:100%;border-collapse:collapse;margin-bottom:16px;"><tr>${headers.map(th).join('')}</tr>${rows}</table>`;
      const pct = (v: number|null|undefined) => v != null ? `${Math.round(v*100)}%` : '—';
      const coord = (v: number|null|undefined) => v != null ? v.toFixed(4) : '—';

      // Population history rows (every 4th checkpoint to keep compact — annual)
      const popHistory = (r.population_history ?? [])
        .filter((_: unknown, i: number) => i % 4 === 0 || i === (r.population_history.length - 1));

      const deathByCause = r.death_statistics?.by_cause ?? {};
      const deathTotal = r.death_statistics?.total ?? 0;
      const deathByAge = r.death_statistics?.by_age_group ?? {};

      // Auto-generated intro stats
      const totalYears = r.simulation?.current_year ?? S.year ?? 0;
      const totalEver  = (r.individuals ?? []).length;
      const peakPop    = (r.population_history ?? []).reduce((mx: number, c: Record<string,unknown>) => Math.max(mx, (c.population as number) ?? 0), 0);
      const langStageNamesMap: Record<string, string[]> = {
        tr: ['dil öncesi', 'jest dili', 'proto-dil', 'sözel dil', 'karmaşık dil', 'sembolik dil'],
        en: ['pre-linguistic', 'gestural', 'proto-language', 'verbal', 'complex language', 'symbolic'],
        de: ['vorsprachlich', 'Gestik', 'Protosprache', 'verbal', 'komplexe Sprache', 'symbolisch'],
        fr: ['prélinguistique', 'gestuelle', 'proto-langage', 'verbal', 'langage complexe', 'symbolique'],
        ar: ['ما قبل اللغة', 'إيمائي', 'لغة أولية', 'لفظي', 'لغة معقدة', 'رمزي'],
      };
      const langStageNames = langStageNamesMap[lang] ?? langStageNamesMap.en;
      const langStageName  = langStageNames[S.max_language_stage ?? 0] ?? '?';
      const totalMigDist   = (r.migration_history ?? []).reduce((s: number, m: Record<string,unknown>) => s + ((m.distance_km as number) ?? 0), 0);
      const epicCount  = (r.notable_events ?? []).filter((e: Record<string,unknown>) => e.event_type === 'epidemic').length;
      const disCount   = (r.notable_events ?? []).filter((e: Record<string,unknown>) => e.event_type === 'disaster').length;
      const deadAvgAge = (() => {
        const dead = (r.individuals ?? []).filter((i: Record<string,unknown>) => i.is_dead && i.age_at_death != null);
        if (!dead.length) return null;
        return Math.round(dead.reduce((s: number, i: Record<string,unknown>) => s + (i.age_at_death as number), 0) / dead.length * 10) / 10;
      })();

      const introMap: Record<string, string> = {
        tr: `Bu rapor, ANATOLİA-SİM evrimsel medeniyet simülasyonunda "${r.simulation?.name ?? sim.id}" adıyla oluşturulan medeniyetin ${totalYears} yıllık tarihsel kaydını içermektedir. Simülasyon ${r.simulation?.start_latitude ?? '?'}° enlem, ${r.simulation?.start_longitude ?? '?'}° boylamda, ${r.simulation?.biome ?? '?'} biome'unda başlatılmıştır.\n\nMedeniyet tarihi boyunca toplam ${totalEver} birey doğmuş, nüfus en yüksek ${peakPop} kişiye ulaşmıştır. Topluluk ${(r.technology_timeline ?? []).length} teknoloji ve ${(r.belief_timeline ?? []).length} inanç geliştirmiş; dil ${langStageName} aşamasına ilerlemiştir. ${deathTotal} ölümün gerçekleştiği bu süreçte ortalama ölüm yaşı ${deadAvgAge ?? '?'} yıl olarak hesaplanmıştır${epicCount > 0 ? `; ${epicCount} salgın ve ${disCount} doğal afet kaydedilmiştir` : ''}. ${totalMigDist > 0 ? `Bant toplamda yaklaşık ${totalMigDist} km yer değiştirmiştir.` : ''}`,
        en: `This report contains the ${totalYears}-year historical record of the civilization "${r.simulation?.name ?? sim.id}" created in the ANATOLIA-SIM evolutionary civilization simulation. The simulation was initialized at latitude ${r.simulation?.start_latitude ?? '?'}°, longitude ${r.simulation?.start_longitude ?? '?'}° in the ${r.simulation?.biome ?? '?'} biome.\n\nThroughout its history, a total of ${totalEver} individuals were born, with peak population reaching ${peakPop}. The civilization developed ${(r.technology_timeline ?? []).length} technologies and ${(r.belief_timeline ?? []).length} beliefs; language progressed to the ${langStageName} stage. Of the ${deathTotal} deaths recorded, the average age at death was ${deadAvgAge ?? '?'} years${epicCount > 0 ? `; ${epicCount} epidemic(s) and ${disCount} disaster(s) occurred` : ''}. ${totalMigDist > 0 ? `The band migrated approximately ${totalMigDist} km in total.` : ''}`,
        de: `Dieser Bericht enthält die ${totalYears}-jährige historische Aufzeichnung der Zivilisation „${r.simulation?.name ?? sim.id}", die in der ANATOLIA-SIM-Evolutionssimulation erstellt wurde. Die Simulation wurde bei ${r.simulation?.start_latitude ?? '?'}° Breite, ${r.simulation?.start_longitude ?? '?'}° Länge im Biom „${r.simulation?.biome ?? '?'}" gestartet.\n\nInsgesamt wurden ${totalEver} Individuen geboren, die Höchstbevölkerung betrug ${peakPop}. Die Zivilisation entwickelte ${(r.technology_timeline ?? []).length} Technologien und ${(r.belief_timeline ?? []).length} Glaubenssätze; die Sprache erreichte die Stufe ${langStageName}. Von den ${deathTotal} Todesfällen betrug das durchschnittliche Sterbealter ${deadAvgAge ?? '?'} Jahre${epicCount > 0 ? `; ${epicCount} Epidemie(n) und ${disCount} Katastrophe(n) wurden verzeichnet` : ''}. ${totalMigDist > 0 ? `Die Gruppe wanderte insgesamt ca. ${totalMigDist} km.` : ''}`,
        fr: `Ce rapport contient l'enregistrement historique de ${totalYears} ans de la civilisation « ${r.simulation?.name ?? sim.id} » créée dans la simulation ANATOLIA-SIM. La simulation a été initialisée à ${r.simulation?.start_latitude ?? '?'}° de latitude, ${r.simulation?.start_longitude ?? '?'}° de longitude dans le biome « ${r.simulation?.biome ?? '?'} ».\n\nAu total, ${totalEver} individus sont nés, la population maximale atteignant ${peakPop}. La civilisation a développé ${(r.technology_timeline ?? []).length} technologies et ${(r.belief_timeline ?? []).length} croyances ; le langage a progressé jusqu'au stade ${langStageName}. Sur les ${deathTotal} décès enregistrés, l'âge moyen au décès était de ${deadAvgAge ?? '?'} ans${epicCount > 0 ? ` ; ${epicCount} épidémie(s) et ${disCount} catastrophe(s) ont eu lieu` : ''}. ${totalMigDist > 0 ? `Le groupe a migré d'environ ${totalMigDist} km au total.` : ''}`,
        ar: `يحتوي هذا التقرير على السجل التاريخي لـ ${totalYears} عامًا من حضارة "${r.simulation?.name ?? sim.id}" التي أُنشئت في محاكاة ANATOLIA-SIM. بدأت المحاكاة عند خط عرض ${r.simulation?.start_latitude ?? '?'}° وخط طول ${r.simulation?.start_longitude ?? '?'}° في منطقة ${r.simulation?.biome ?? '?'}.\n\nوُلد ما مجموعه ${totalEver} فردًا، وبلغ الحد الأقصى للسكان ${peakPop}. طورت الحضارة ${(r.technology_timeline ?? []).length} تقنية و${(r.belief_timeline ?? []).length} معتقدًا؛ وتقدمت اللغة إلى مرحلة ${langStageName}. من أصل ${deathTotal} حالة وفاة مسجلة، كان متوسط العمر عند الوفاة ${deadAvgAge ?? '?'} عامًا${epicCount > 0 ? `؛ جرى تسجيل ${epicCount} وباء و${disCount} كارثة` : ''}. ${totalMigDist > 0 ? `هاجرت المجموعة ما يقارب ${totalMigDist} كم إجمالاً.` : ''}`,
      };
      const intro = introMap[lang] ?? introMap.en;

      // ── SVG Chart Helpers ──────────────────────────────────────────────────
      const popChartSvg = (() => {
        const data = popHistory as Record<string, unknown>[];
        if (data.length < 2) return '';
        const W = 680, H = 180, PL = 42, PR = 16, PT = 14, PB = 32;
        const maxP = Math.max(...data.map(d => (d.population as number) ?? 0), 1);
        const minY = (data[0]?.year as number) ?? 0;
        const maxY = (data[data.length - 1]?.year as number) ?? 1;
        const xS = (y: number) => PL + ((y - minY) / Math.max(1, maxY - minY)) * (W - PL - PR);
        const yS = (p: number) => PT + (H - PT - PB) * (1 - p / maxP);
        const pts = data.map(d => `${xS(d.year as number).toFixed(1)},${yS(d.population as number).toFixed(1)}`).join(' ');
        const area = `${xS(minY)},${H - PB} ${pts} ${xS(maxY)},${H - PB}`;
        const grids = [0, 0.25, 0.5, 0.75, 1].map(f => {
          const v = Math.round(maxP * f); const y = yS(v).toFixed(1);
          return `<line x1="${PL}" y1="${y}" x2="${W - PR}" y2="${y}" stroke="#e5e7eb" stroke-width="0.8"/>
                  <text x="${PL - 4}" y="${(parseFloat(y) + 3).toFixed(1)}" text-anchor="end" font-size="9" fill="#9ca3af">${v}</text>`;
        }).join('');
        const step = Math.max(1, Math.floor(data.length / 8));
        const xLbls = data.filter((_, i) => i % step === 0 || i === data.length - 1).map(d =>
          `<text x="${xS(d.year as number).toFixed(1)}" y="${H - PB + 13}" text-anchor="middle" font-size="9" fill="#9ca3af">${d.year}</text>`).join('');
        const dots = data.map(d =>
          `<circle cx="${xS(d.year as number).toFixed(1)}" cy="${yS(d.population as number).toFixed(1)}" r="2.5" fill="#f59e0b"/>`).join('');
        return `<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg">
          <rect width="${W}" height="${H}" fill="#fafafa" rx="6"/>
          ${grids}
          <polygon points="${area}" fill="#f59e0b" fill-opacity="0.12"/>
          <polyline points="${pts}" fill="none" stroke="#f59e0b" stroke-width="2.5" stroke-linejoin="round"/>
          ${dots}
          <line x1="${PL}" y1="${H - PB}" x2="${W - PR}" y2="${H - PB}" stroke="#d1d5db"/>
          <line x1="${PL}" y1="${PT}" x2="${PL}" y2="${H - PB}" stroke="#d1d5db"/>
          ${xLbls}
        </svg>`;
      })();

      const deathCauseChartSvg = (() => {
        const entries = Object.entries(deathByCause).sort(([, a], [, b]) => (b as number) - (a as number)).slice(0, 10);
        if (!entries.length) return '';
        const maxV = Math.max(...entries.map(([, v]) => v as number), 1);
        const W = 680, BH = 18, GAP = 5, PL = 170, PR = 60, PT = 8;
        const H = PT + entries.length * (BH + GAP);
        const bars = entries.map(([cause, count], i) => {
          const y = PT + i * (BH + GAP);
          const bw = ((count as number) / maxV) * (W - PL - PR);
          const pctStr = deathTotal ? ` (${Math.round((count as number) / deathTotal * 100)}%)` : '';
          const causeLabel = CAUSE_LABELS[cause]?.[lang as LangCode] ?? cause.replace(/_/g, ' ');
          return `<text x="${PL - 6}" y="${y + BH - 4}" text-anchor="end" font-size="10" fill="#374151">${causeLabel}</text>
                  <rect x="${PL}" y="${y}" width="${bw.toFixed(1)}" height="${BH}" fill="#ef4444" fill-opacity="0.75" rx="3"/>
                  <text x="${PL + bw + 5}" y="${y + BH - 4}" font-size="10" fill="#374151">${count}${pctStr}</text>`;
        }).join('');
        return `<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg"><rect width="${W}" height="${H}" fill="#fafafa" rx="6"/>${bars}</svg>`;
      })();

      const ageChartSvg = (() => {
        const groups = [
          { label: '0–1', val: deathByAge.infant_0_1 ?? 0, color: '#ef4444' },
          { label: '1–15', val: deathByAge.child_1_15 ?? 0, color: '#f97316' },
          { label: '15–30', val: deathByAge.young_adult_15_30 ?? 0, color: '#eab308' },
          { label: '30–50', val: deathByAge.adult_30_50 ?? 0, color: '#22c55e' },
          { label: '50+', val: deathByAge.elder_50plus ?? 0, color: '#3b82f6' },
        ];
        const maxV = Math.max(...groups.map(g => g.val), 1);
        const BW = 52, GAP = 16, PL = 20, PT = 10, PB = 28, H = 150;
        const W = PL * 2 + groups.length * (BW + GAP) - GAP;
        const bars = groups.map((g, i) => {
          const x = PL + i * (BW + GAP);
          const bh = ((g.val / maxV) * (H - PT - PB));
          const y = H - PB - bh;
          const pct = deathTotal ? Math.round(g.val / deathTotal * 100) : 0;
          return `<rect x="${x}" y="${y.toFixed(1)}" width="${BW}" height="${bh.toFixed(1)}" fill="${g.color}" fill-opacity="0.8" rx="4"/>
                  <text x="${x + BW / 2}" y="${(y - 4).toFixed(1)}" text-anchor="middle" font-size="9" fill="#374151" font-weight="bold">${g.val}</text>
                  <text x="${x + BW / 2}" y="${H - PB + 13}" text-anchor="middle" font-size="9" fill="#6b7280">${g.label}</text>
                  <text x="${x + BW / 2}" y="${H - PB + 23}" text-anchor="middle" font-size="8" fill="#9ca3af">${pct}%</text>`;
        }).join('');
        return `<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg">
          <rect width="${W}" height="${H}" fill="#fafafa" rx="6"/>
          <line x1="${PL}" y1="${H - PB}" x2="${W - PL}" y2="${H - PB}" stroke="#d1d5db"/>
          ${bars}
        </svg>`;
      })();

      // ── CSS & Layout helpers ───────────────────────────────────────────────
      const secColor = (title: string, color: string, icon: string) =>
        `<div style="margin:28px 0 10px 0;padding:8px 14px;background:${color}18;border-left:4px solid ${color};border-radius:0 6px 6px 0;">
           <span style="font-size:15px;font-weight:bold;color:${color};">${icon} ${title}</span>
         </div>`;
      const styledTbl = (headers: string[], rows: string, hdrColor: string, tableId?: string) => {
        const ths = headers.map(h => `<th style="padding:6px 8px;background:${hdrColor};color:#fff;font-size:10px;text-align:left;font-weight:600;">${h}</th>`).join('');
        return `<table${tableId ? ` id="${tableId}"` : ''} style="width:100%;border-collapse:collapse;margin-bottom:16px;border-radius:6px;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,0.1);">
          <tr>${ths}</tr>${rows}</table>`;
      };
      const stRow = (cells: unknown[], i: number) => {
        const bg = i % 2 === 0 ? '#ffffff' : '#f9fafb';
        return `<tr style="background:${bg};">${cells.map(c => `<td style="padding:4px 8px;font-size:10px;border-bottom:1px solid #f0f0f0;">${c != null ? String(c) : '—'}</td>`).join('')}</tr>`;
      };
      const statCard = (label: string, value: string, color: string) =>
        `<div style="flex:1;background:${color}12;border:1px solid ${color}30;border-radius:8px;padding:12px 14px;min-width:120px;">
           <div style="font-size:10px;color:#6b7280;margin-bottom:4px;">${label}</div>
           <div style="font-size:20px;font-weight:bold;color:${color};">${value}</div>
         </div>`;
      const badge = (text: string, color: string) =>
        `<span style="display:inline-block;background:${color}20;color:${color};border:1px solid ${color}40;border-radius:12px;padding:2px 10px;font-size:10px;margin:2px;">${text}</span>`;
      // A belief with no generated name yet (see belief.rs's cardinal rule --
      // the raw archetype string is never shown) falls back to its opaque
      // code plus a short mechanically-derived description, e.g. "#5 —
      // a sophisticated belief that requires...".
      const beliefDisplay = (e: Record<string, unknown>) =>
        (e.name as string | null | undefined) ?? `#${beliefCodeNumber(String(e.code ?? ''))} — ${describeBeliefCode(String(e.code ?? ''), lang as LangCode)}`;

      const html = `<!DOCTYPE html><html lang="${lang}"><head><meta charset="UTF-8">
<title>${lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'} — ${sim.name ?? sim.id}</title>
<style>
  *{box-sizing:border-box;}
  body{font-family:Arial,Helvetica,sans-serif;background:#fff;color:#1f2937;margin:0;font-size:12px;line-height:1.5;}
  @media print{body{margin:0;}}
</style></head><body>

<!-- ═══ KAPAK SAYFASI ═══ -->
<div id="rpt-cover" style="background:#0d1b2a;color:#fff;padding:64px 52px;min-height:1060px;display:flex;flex-direction:column;justify-content:space-between;">
  <div>
    <div style="font-size:10px;letter-spacing:5px;text-transform:uppercase;color:#64748b;margin-bottom:52px;">
      ${lang === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'} &middot; ${rt('EVRİMSEL MEDENİYET MOTORU', 'EVOLUTIONARY CIVILIZATION ENGINE', 'EVOLUTIONÄRE ZIVILISATIONS-ENGINE', 'MOTEUR DE CIVILISATION ÉVOLUTIVE', 'محرك الحضارة التطورية')}
    </div>
    <div style="font-size:56px;font-weight:900;letter-spacing:3px;line-height:1;color:#f59e0b;margin-bottom:6px;">${lang === 'tr' ? 'ANATOLİA' : 'ANATOLIA'}</div>
    <div style="font-size:56px;font-weight:900;letter-spacing:3px;line-height:1;color:#fff;margin-bottom:16px;">${lang === 'tr' ? 'SİM' : 'SIM'}</div>
    <div style="font-size:18px;color:#94a3b8;letter-spacing:2px;margin-bottom:40px;">${rt('MEDENİYET RAPORU','CIVILIZATION REPORT')}</div>
    <div style="font-size:30px;font-weight:700;color:#f1f5f9;border-top:1px solid #334155;border-bottom:1px solid #334155;padding:14px 0;margin-bottom:36px;">
      ${r.simulation?.name ?? sim.id}
    </div>
    <div style="display:flex;gap:32px;flex-wrap:wrap;">
      <div>
        <div style="font-size:10px;color:#64748b;text-transform:uppercase;letter-spacing:1px;">${rt('Biyom','Biome')}</div>
        <div style="font-size:14px;color:#e2e8f0;font-weight:600;">${r.simulation?.biome ?? '?'}</div>
      </div>
      <div>
        <div style="font-size:10px;color:#64748b;text-transform:uppercase;letter-spacing:1px;">${rt('Koordinatlar','Coordinates')}</div>
        <div style="font-size:14px;color:#e2e8f0;font-weight:600;">${r.simulation?.start_latitude ?? '?'}°, ${r.simulation?.start_longitude ?? '?'}°</div>
      </div>
      <div>
        <div style="font-size:10px;color:#64748b;text-transform:uppercase;letter-spacing:1px;">${rt('Süre','Duration')}</div>
        <div style="font-size:14px;color:#e2e8f0;font-weight:600;">${totalYears} ${rt('yıl','yr')} · ${r.simulation?.current_day ?? S.day ?? '?'} ${rt('gün','days')}</div>
      </div>
    </div>
    <div style="display:flex;gap:16px;margin-top:32px;flex-wrap:wrap;">
      ${statCard(rt('Zirve Nüfus','Peak Pop.'), String(peakPop), '#f59e0b')}
      ${statCard(rt('Toplam Birey','Total Lived'), String(totalEver), '#38bdf8')}
      ${statCard(rt('Teknoloji','Technologies'), String((r.technology_timeline ?? []).length), '#34d399')}
      ${statCard(rt('Toplam Ölüm','Total Deaths'), String(deathTotal), '#f87171')}
    </div>
  </div>
  <div style="font-size:10px;color:#475569;border-top:1px solid #1e293b;padding-top:12px;margin-top:40px;">
    Bold Askeri Teknoloji ve Savunma Sanayi A.Ş. &copy; 2026 &middot; RST Q-Nation 200120401018 &nbsp;&nbsp;|&nbsp;&nbsp; ${rt('Oluşturuldu','Generated')}: ${now}
  </div>
</div>

<!-- ═══ İÇERİK SAYFASI ═══ -->
<div id="rpt-content" style="padding:40px 44px;">

${r.simulation?.intervened ? `<!-- MÜDAHALELİ KOŞU UYARISI -->
<div style="background:#fef3c7;border:2px solid #d97706;border-radius:8px;padding:20px 24px;margin-bottom:28px;display:flex;align-items:flex-start;gap:16px;">
  <div style="font-size:28px;line-height:1;">⚠️</div>
  <div>
    <div style="font-weight:bold;color:#92400e;font-size:13px;margin-bottom:6px;">${rt('MÜDAHALELİ DENEY KOŞUSU','GOD MODE INTERVENTION DETECTED')}</div>
    <div style="color:#78350f;font-size:11px;line-height:1.7;">${rt('Bu simülasyonda God Mode müdahalesi kullanılmıştır. Bu koşu doğal hipotez verisi değildir; rapordaki istatistikler deneysel kontrol grubu verisi olarak kullanılmamalıdır.','God Mode interventions were applied during this simulation run. This is not a clean natural-hypothesis dataset; statistics in this report should not be used as experimental control data.')
    }</div>
  </div>
</div>` : ''}

<!-- GİRİŞ -->
${secColor(rt('Giriş','Introduction'), '#475569', '📋')}
<div style="background:#f8fafc;border-radius:8px;padding:18px 22px;font-size:12px;line-height:1.9;color:#374151;white-space:pre-wrap;border:1px solid #e2e8f0;">${intro}</div>

<!-- ANLIK DURUM -->
${secColor(rt('Anlık Durum','Current Snapshot'), '#6366f1', '📊')}
<div style="display:flex;gap:10px;flex-wrap:wrap;margin-bottom:16px;">
  ${statCard(rt('Nüfus','Population'), String(S.population ?? '—'), '#6366f1')}
  ${statCard(rt('Ort. Yaş','Avg Age'), S.avg_age ? S.avg_age + rt(' yaş',' yr',' J.',' an',' سنة') : '—', '#8b5cf6')}
  ${statCard(rt('Mutluluk','Happiness'), pct(S.happiness_index), '#10b981')}
  ${statCard('Gini', String(S.gini ?? '—'), '#f59e0b')}
  ${statCard(rt('Zeka','Intelligence'), pct(S.avg_intelligence), '#3b82f6')}
  ${statCard('QoL', String(S.qol_index ?? '—'), '#06b6d4')}
</div>

<!-- HORMONAL SİSTEM -->
${secColor(rt('Hormonal Sistem (Nüfus Ort.)','Hormonal System (Pop. Avg.)'), '#ec4899', '🧬')}
<div style="display:flex;gap:10px;flex-wrap:wrap;margin-bottom:16px;">
  ${(() => {
    const H = (S.mean_hormones ?? {}) as Record<string, number>;
    // A representative subset, one per axis (see AGENTS.md's Hormones
    // section) -- the full 49-hormone breakdown is in PsychologyPanel/
    // PopulationPanel in the live app; a printed report stays a summary.
    return [
      [rt('Kortizol','Cortisol'), H.cortisol, '#ef4444'],
      [rt('Adrenalin','Adrenaline'), H.adrenaline, '#f97316'],
      [rt('Testosteron','Testosterone'), H.testosterone, '#3b82f6'],
      [rt('Östrojen','Estrogen'), H.estrogen, '#ec4899'],
      [rt('Tiroid','Thyroid'), H.thyroid, '#06b6d4'],
      [rt('Dopamin','Dopamine'), H.dopamine, '#eab308'],
      [rt('Oksitosin','Oxytocin'), H.oxytocin, '#22c55e'],
      [rt('İnsülin','Insulin'), H.insulin, '#84cc16'],
    ].map(([label, value, color]) => statCard(label as string, pct(value as number), color as string)).join('');
  })()}
</div>
${styledTbl(
  [rt('Gösterge','Metric'), rt('Değer','Value'), rt('Gösterge','Metric'), rt('Değer','Value')],
  (() => {
    const rows2 = [
      [rt('Besin Bolluğu','Food Abundance'), pct(S.food_abundance), rt('Su Bolluğu','Water Abundance'), pct(S.water_abundance)],
      [rt('Hastalık Oranı','Sick Rate'), pct(S.sick_rate), rt('Sıcaklık','Temperature'), S.temperature ? S.temperature + '°C' : '—'],
      [rt('Dil Aşaması','Lang Stage'), S.max_language_stage, rt('Kelime Sayısı','Word Count'), S.word_count],
      [rt('Teknoloji','Technologies'), S.technologies, rt('İnanç','Beliefs'), S.beliefs],
      [rt('Sanat','Art Forms'), S.art_forms, rt('Gruplar','Groups'), S.groups],
      [rt('Mevsim','Season'), translateSeason(String(S.season ?? ''), lang as LangCode), rt('Hava','Weather'), S.weather ? translateWeather(String(S.weather), lang as LangCode) : '—'],
      [rt('Toplam Doğum','Total Births'), S.births, rt('Toplam Ölüm','Total Deaths'), S.deaths],
    ];
    return rows2.map((r2, i) => stRow(r2, i)).join('');
  })(),
  '#6366f1'
)}

<!-- NÜFUS TARİHİ -->
${secColor(rt('Nüfus Tarihi','Population History'), '#f59e0b', '📈')}
${(r.population_history?.length ?? 0) > popHistory.length ? `<p style="color:#9ca3af;font-size:10px;margin:-4px 0 10px 0;">${rt(
  `${popHistory.length} / ${r.population_history.length} kontrol noktası gösteriliyor (her 4.).`,
  `Showing ${popHistory.length} of ${r.population_history.length} checkpoints (every 4th).`
)}</p>` : ''}
<div style="margin-bottom:12px;">${popChartSvg}</div>
${styledTbl(
  [rt('Yıl','Year'), rt('Nüfus','Pop'), rt('Ort.Yaş','Avg Age'), rt('Mutluluk','Happiness'), 'Gini',
   rt('Besin','Food'), rt('Su','Water'), rt('Konum','Location'), rt('Hareket Sebebi','Move Reason'), rt('Hava','Weather')],
  popHistory.map((c: Record<string,unknown>, i: number) => stRow([
    c.year, c.population, c.avg_age ? c.avg_age + 'yr' : '—',
    pct(c.happiness_index as number|undefined), c.gini,
    pct(c.food_abundance as number|undefined), pct(c.water_abundance as number|undefined),
    (c.centroid_x != null) ? coord(c.centroid_x as number) + ',' + coord(c.centroid_y as number) : '—',
    c.dominant_drive ? translateDrive(String(c.dominant_drive), lang as LangCode) : '—', c.weather ? translateWeather(String(c.weather), lang as LangCode) : '—',
  ], i)).join(''),
  '#d97706'
)}

<!-- TEKNOLOJİ ZAMAN ÇİZELGESİ -->
${secColor(rt('Teknoloji Zaman Çizelgesi','Technology Timeline'), '#3b82f6', '⚙️')}
<div style="margin-bottom:10px;">
  ${(r.technology_timeline as Record<string,unknown>[] ?? []).map(e => badge(translateTech(String(e.name ?? '?'), lang as LangCode), '#3b82f6')).join('')}
</div>
${r.technology_timeline?.length ? styledTbl(
  [rt('Yıl','Year'), rt('Teknoloji','Technology'), rt('Keşif Sebebi','Discovery Reason'), rt('Nüfus','Pop'), rt('Mevsim','Season'), rt('Hava','Weather')],
  (r.technology_timeline as Record<string,unknown>[]).map((e, i) => stRow([
    e.year, translateTech(String(e.name ?? '?'), lang as LangCode), e.trigger_reason ?? '—', e.population, translateSeason(String(e.season ?? ''), lang as LangCode), e.weather ? translateWeather(String(e.weather), lang as LangCode) : '—'
  ], i)).join(''),
  '#2563eb'
) : '<p style="color:#9ca3af;font-size:11px;padding:8px;">—</p>'}

<!-- İNANÇ & KÜLTÜR -->
${secColor(rt('İnanç & Kültür Zaman Çizelgesi','Belief & Culture Timeline'), '#8b5cf6', '🌀')}
<div style="margin-bottom:10px;">
  ${(r.belief_timeline as Record<string,unknown>[] ?? []).map(e => badge(beliefDisplay(e), '#8b5cf6')).join('')}
  ${(r.art_timeline as Record<string,unknown>[] ?? []).map(e => badge(translateArtForm(String(e.name ?? '?'), lang as LangCode), '#ec4899')).join('')}
</div>
${(r.belief_timeline?.length || r.art_timeline?.length) ? styledTbl(
  [rt('Yıl','Year'), rt('Tür','Type'), rt('İsim','Name'), rt('Oluşum Sebebi','Reason'), rt('Nüfus','Pop'), rt('Mevsim','Season')],
  [
    ...(r.belief_timeline as Record<string,unknown>[] ?? []).map((e, i) => stRow([e.year, rt('İnanç','Belief'), beliefDisplay(e), e.trigger_reason ?? '—', e.population, translateSeason(String(e.season ?? ''), lang as LangCode)], i)),
    ...(r.art_timeline as Record<string,unknown>[] ?? []).map((e, i) => stRow([e.year, translateEventType(String(e.type ?? ''), lang as LangCode), translateArtForm(String(e.name ?? '?'), lang as LangCode), '—', '—', '—'], i + (r.belief_timeline?.length ?? 0))),
  ].join(''),
  '#7c3aed'
) : '<p style="color:#9ca3af;font-size:11px;padding:8px;">—</p>'}

<!-- GÖÇ TARİHİ -->
${secColor(rt('Göç Tarihi','Migration History'), '#10b981', '🧭')}
${r.migration_history?.length ? styledTbl(
  [rt('Yıl','Year'), rt('Mesafe','Distance'), rt('Göç Sebebi','Reason'), rt('Önceki','From'), rt('Yeni','To'), rt('Besin','Food'), rt('Su','Water'), rt('Mevsim','Season')],
  (r.migration_history as Record<string,unknown>[]).map((e, i) => {
    const from = e.from as Record<string,number>|undefined;
    const to   = e.to   as Record<string,number>|undefined;
    return stRow([
      e.year, e.distance_km ? e.distance_km + ' km' : '—', e.reason ? translateMigrationReason(String(e.reason), lang as LangCode) : '—',
      from ? coord(from.x) + ',' + coord(from.y) : '—',
      to   ? coord(to.x)   + ',' + coord(to.y)   : '—',
      pct(e.food_abundance as number|undefined), pct(e.water_abundance as number|undefined), translateSeason(String(e.season ?? ''), lang as LangCode),
    ], i);
  }).join(''),
  '#059669'
) : `<p style="color:#9ca3af;font-size:11px;padding:8px;">${rt('Göç kaydı bulunamadı — yeni checkpoint\'lerden itibaren toplanacak.','No migration records yet — will accumulate from future checkpoints.','Keine Migrationsdaten vorhanden.','Aucune donnée de migration.','لا توجد سجلات هجرة بعد.')}</p>`}

<!-- ÖLÜM İSTATİSTİKLERİ -->
${secColor(rt('Ölüm İstatistikleri','Death Statistics'), '#ef4444', '💀')}
<div style="display:flex;gap:12px;margin-bottom:10px;">
  ${statCard(rt('Toplam Ölüm','Total Deaths'), String(deathTotal), '#ef4444')}
  ${statCard(rt('Ort. Ölüm Yaşı','Avg Death Age'), deadAvgAge != null ? deadAvgAge + rt(' yaş',' yr',' J.',' an',' سنة') : '—', '#f97316')}
  ${statCard(rt('Bebek Ölümü','Infant Deaths'), String(deathByAge.infant_0_1 ?? 0), '#fbbf24')}
</div>
<div style="margin-bottom:14px;">${deathCauseChartSvg}</div>
<div style="display:flex;gap:20px;align-items:flex-start;">
  <div style="flex:2;">
  <div style="font-size:11px;font-weight:600;color:#6b7280;margin-bottom:6px;">${rt('Nedene Göre','By Cause')}</div>
  ${styledTbl(
    [rt('Sebep','Cause'), rt('Sayı','Count'), '%'],
    Object.entries(deathByCause).sort(([,a],[,b]) => (b as number) - (a as number))
      .map(([cause, count], i) => stRow([CAUSE_LABELS[cause]?.[lang as LangCode] ?? cause.replace(/_/g,' '), count, deathTotal ? Math.round((count as number)/deathTotal*100)+'%' : '—'], i))
      .join('') || stRow([rt('Veri yok','No data'),'',''], 0),
    '#dc2626'
  )}
  </div>
  <div style="flex:1;">
  <div style="font-size:11px;font-weight:600;color:#6b7280;margin-bottom:6px;">${rt('Yaş Grubuna Göre','By Age')}</div>
  <div>${ageChartSvg}</div>
  </div>
</div>

<!-- ÖNEMLİ OLAYLAR -->
${secColor(rt('Önemli Olaylar (önem ≥ 3)','Notable Events (importance ≥ 3)'), '#f97316', '⚡')}
${r.notable_events?.length ? styledTbl(
  [rt('Yıl','Year'), rt('Gün','Day'), rt('Tür','Type'), rt('Açıklama','Description')],
  (r.notable_events as Record<string,unknown>[]).map((e, i) => stRow([e.sim_year, e.sim_day, translateEventType(String(e.event_type ?? ''), lang as LangCode), translateEventDescription(String(e.description ?? ''), lang as LangCode, e)], i)).join(''),
  '#ea580c'
) : '<p style="color:#9ca3af;font-size:11px;padding:8px;">—</p>'}

<!-- BİREYLER -->
${secColor(rt('Bireyler','Individuals'), '#64748b', '👥')}
${(() => {
  const mentalStateLabel = (s: unknown) => s ? translateMentalState(String(s), lang as LangCode) : '—';
  const topBond = (ind: Record<string, unknown>) => {
    const rels = ind.relationships as Array<{ name?: string; bond?: number }> | undefined;
    if (!rels?.length) return '—';
    const top = rels[0];
    const sign = (top.bond ?? 0) >= 0 ? '+' : '';
    const name = String(top.name ?? '?') === 'Unnamed' ? UNNAMED_LABEL[lang as LangCode] : (top.name ?? '?');
    return `${name} (${sign}${(top.bond ?? 0).toFixed(2)})`;
  };
  // This table grows with total-ever-born (never shrinks -- dead
  // individuals stay in the array), which for a long-running/high-population
  // simulation can reach the thousands. Full data is kept here (not
  // truncated) -- downloadPDF() instead renders this specific table
  // (identified by id) in row-batches with a yield between each, so
  // html2canvas never rasterizes more than one batch's worth of rows at
  // once, avoiding the freeze without dropping any individual from the PDF.
  const allIndividuals = (r.individuals as Record<string,unknown>[]) ?? [];
  return styledTbl(
    [rt('İsim','Name'), rt('Cin.','Sex'), rt('Kurucu','Fnd'), rt('Doğum Yılı','Born'), rt('Ölüm Yılı','Died'), rt('Ölüm Yaşı','Age@Death'), rt('Ölüm Sebebi','Cause'), rt('Zeka','IQ'),
     rt('Ruh Hali','Mood'), rt('Empati Kur.','ToM'), rt('İtibar','Rep.'), rt('Rol','Role'), rt('En Güçlü Bağ','Top Bond')],
    allIndividuals.map((ind, i) => stRow([
      String(ind.name ?? '?') === 'Unnamed' ? UNNAMED_LABEL[lang as LangCode] : ind.name, ind.sex === 'male' ? '♂' : '♀', ind.is_founder ? '★' : '',
      ind.birth_year,
      ind.is_dead ? ind.death_year : rt('(yaşıyor)','(alive)'),
      ind.age_at_death ?? (ind.is_dead ? '—' : ''),
      ind.death_cause ? (CAUSE_LABELS[String(ind.death_cause)]?.[lang as LangCode] ?? String(ind.death_cause).replace(/_/g, ' ')) : (ind.is_dead ? '—' : ''),
      ind.intelligence != null ? Math.round((ind.intelligence as number)*100)+'%' : '—',
      mentalStateLabel(ind.mental_state),
      ind.theory_of_mind ?? 0,
      ind.reputation != null ? Math.round((ind.reputation as number)*100)+'%' : '—',
      ind.role ? translateRole(String(ind.role), lang as LangCode) : '—',
      topBond(ind),
    ], i)).join(''),
    '#475569',
    'rpt-individuals-table'
  );
})()}

<!-- YAŞAM GÜNLÜĞÜ ÖNE ÇIKANLARI -->
${secColor(rt('Yaşam Günlüğü Öne Çıkanları','Life Log Highlights'), '#a855f7', '💭')}
${(() => {
  const kindLabel = (k: string): string => {
    const labels: Record<string, [string, string]> = {
      first_word: ['İlk Kelime', 'First Word'], first_thought: ['İlk Düşünce', 'First Thought'],
      first_abstract: ['İlk Soyut Kavram', 'First Abstract'], consciousness_10: ['Bilinç %10', 'Consciousness 10%'],
      consciousness_25: ['Bilinç %25', 'Consciousness 25%'], consciousness_50: ['Bilinç %50', 'Consciousness 50%'],
      consciousness_75: ['Bilinç %75', 'Consciousness 75%'], death_proximity: ['Ölüme Yakın', 'Near Death'],
      grief: ['Yas', 'Grief'],
    };
    const row = labels[k];
    return row ? rt(row[0], row[1]) : k;
  };
  type LogEntry = { day: number; kind: string; thought?: { proto?: string; annotated?: string } };
  const rows: { name: string; entry: LogEntry }[] = [];
  for (const ind of (r.individuals as Record<string, unknown>[] ?? [])) {
    const log = (ind.inner_thought_log as LogEntry[] | undefined) ?? [];
    for (const entry of log) rows.push({ name: String(ind.name ?? '?'), entry });
  }
  rows.sort((a, b) => a.entry.day - b.entry.day);
  const capped = rows.slice(0, 60);
  const truncNote = rows.length > capped.length
    ? `<p style="color:#9ca3af;font-size:10px;margin:-4px 0 8px 0;">${rt(
        `${capped.length} / ${rows.length} kilometre taşı gösteriliyor (en eski 60).`,
        `Showing ${capped.length} of ${rows.length} milestones (earliest 60).`
      )}</p>`
    : '';
  return capped.length ? truncNote + styledTbl(
    [rt('Gün','Day'), rt('Yıl','Year'), rt('Birey','Individual'), rt('Kilometre Taşı','Milestone'), rt('Söylenen','Words')],
    capped.map((r2, i) => stRow([r2.entry.day, Math.floor(r2.entry.day / 365), r2.name, kindLabel(r2.entry.kind), r2.entry.thought?.proto ?? '—'], i)).join(''),
    '#7c3aed'
  ) : `<p style="color:#9ca3af;font-size:11px;padding:8px;">${rt('Henüz kaydedilmiş bir kilometre taşı yok.','No milestones recorded yet.')}</p>`;
})()}

<div style="margin-top:40px;padding-top:12px;border-top:1px solid #e5e7eb;font-size:10px;color:#9ca3af;text-align:center;">
  Bold Askeri Teknoloji ve Savunma Sanayi A.Ş. &copy; 2026 &middot; RST Q-Nation 200120401018
</div>
</div>
</body></html>`;

    return html;
  }

  async function printReport() {
    if (!currentSim || !accessToken) return;
    // window.open must happen synchronously, in direct response to the
    // click -- before any `await`. Once this function yields to the
    // microtask queue (the report fetch inside buildReportHtml() below),
    // some browsers/mobile WebViews no longer consider a later window.open
    // to be a direct result of the original user gesture, and silently
    // block it as a popup even though a real click triggered this. Opening
    // the (still-blank) window first, then filling it in once the report is
    // ready, keeps it inside the gesture window every time.
    const w = window.open('', '_blank', 'width=900,height=1000');
    if (!w) {
      flash(text(lang as LangCode, { en: '✗ Popup blocked.', tr: '✗ Popup engellendi.', de: '✗ Popup blockiert.', fr: '✗ Fenêtre contextuelle bloquée.', ar: '✗ تم حظر النافذة المنبثقة.' }));
      return;
    }
    setPdfLoading(true);
    try {
      const html = await buildReportHtml();
      w.document.write(html);
      w.document.close();
      w.focus();
      setTimeout(() => w.print(), 500);
    } catch {
      flash(text(lang as LangCode, { en: '✗ Failed.', tr: '✗ Başarısız.', de: '✗ Fehlgeschlagen.', fr: '✗ Échec.', ar: '✗ فشل.' }));
      w.close();
    }
    setPdfLoading(false);
  }

  async function downloadPDF() {
    if (!currentSim || !accessToken) return;
    setPdfLoading(true);
    setPdfProgress('');
    try {
      const html = await buildReportHtml();
      const container = document.createElement('div');
      container.style.cssText = 'position:fixed;left:-9999px;top:0;width:794px;background:#fff;font-family:Arial,Helvetica,sans-serif;';
      container.innerHTML = html;
      document.body.appendChild(container);

      const pdf = new jsPDF({ orientation: 'portrait', unit: 'mm', format: 'a4' });
      const pageW = pdf.internal.pageSize.getWidth();
      const pageH = pdf.internal.pageSize.getHeight();

      // Renders the report as a sequence of small chunks (cover, then each
      // section, with the Individuals table further split into row
      // batches) instead of one giant canvas -- see the module-level
      // helpers' own doc comments for why: this is what actually froze the
      // tab on a long-running/high-population simulation, without any of
      // this data (every individual, event, migration record) being
      // dropped from the PDF -- it's just assembled from more, smaller
      // html2canvas calls with a yield in between.
      let firstPage = true;
      const cover = container.querySelector<HTMLElement>('#rpt-cover');
      if (cover) {
        firstPage = await renderNodeToPdf(cover, pdf, pageW, pageH, firstPage);
        await yieldToMainThread();
      }

      const content = container.querySelector<HTMLElement>('#rpt-content');
      const chunks = content ? splitIntoSectionChunks(content) : [];
      for (let i = 0; i < chunks.length; i++) {
        const chunkNodes = chunks[i];
        setPdfProgress(`${i + 1} / ${chunks.length}`);

        let individualsTable: HTMLTableElement | null = null;
        for (const n of chunkNodes) {
          if (n.nodeType !== Node.ELEMENT_NODE) continue;
          const el = n as HTMLElement;
          individualsTable = el.id === 'rpt-individuals-table' ? (el as HTMLTableElement) : el.querySelector<HTMLTableElement>('#rpt-individuals-table');
          if (individualsTable) break;
        }

        if (individualsTable) {
          const headerNodes = chunkNodes.filter(n => n !== individualsTable && !(n.nodeType === Node.ELEMENT_NODE && (n as HTMLElement).contains(individualsTable)));
          firstPage = await renderIndividualsInBatches(individualsTable, headerNodes, pdf, pageW, pageH, firstPage, (label) => setPdfProgress(`${i + 1} / ${chunks.length} (${label})`));
          continue;
        }

        const wrapper = document.createElement('div');
        wrapper.style.cssText = CHUNK_WRAPPER_STYLE;
        for (const n of chunkNodes) wrapper.appendChild(n);
        document.body.appendChild(wrapper);
        firstPage = await renderNodeToPdf(wrapper, pdf, pageW, pageH, firstPage);
        document.body.removeChild(wrapper);
        await yieldToMainThread();
      }

      document.body.removeChild(container);

      const fname = `anatolia-sim-${currentSim.name ?? currentSim.id}-Y${stats?.year ?? 0}.pdf`;
      // pdf.save() reports success on Android even when the file is
      // genuinely unfindable afterward -- it's the same blob-URL trick
      // saveFile replaces. Pulling the raw bytes out ourselves lets Android
      // route through the native Filesystem/Share plugins instead.
      setPdfProgress(text(lang as LangCode, { en: 'Finishing…', tr: 'Tamamlanıyor…', de: 'Wird fertiggestellt…', fr: 'Finalisation…', ar: 'جارٍ الإنهاء…' }));
      const base64 = await pdfToBase64Chunked(pdf);
      setPdfFile(await saveFile(fname, 'application/pdf', base64, true));
      flash(text(lang as LangCode, { en: '✓ PDF downloaded.', tr: '✓ PDF indirildi.', de: '✓ PDF heruntergeladen.', fr: '✓ PDF téléchargé.', ar: '✓ تم تنزيل PDF.' }));
    } catch (err) {
      console.error(err);
      flash(text(lang as LangCode, { en: '✗ PDF generation failed.', tr: '✗ PDF oluşturulamadı.', de: '✗ PDF-Erstellung fehlgeschlagen.', fr: '✗ Échec de la génération du PDF.', ar: '✗ فشل إنشاء ملف PDF.' }));
    }
    setPdfProgress('');
    setPdfLoading(false);
  }

  return (
    <DetailPanel panelId="report" title="Report" titleTr="Rapor">
      <div className="bg-sim-surface rounded-lg p-3 mb-4">
        <p className="text-sim-muted text-sm italic">
          {text(lang as LangCode, {
            en: 'Export the current simulation state as a JSON file or print a formatted PDF report.',
            tr: 'Mevcut simülasyon durumunu JSON dosyası olarak dışa aktarın veya biçimlendirilmiş PDF raporu yazdırın.',
            de: 'Exportieren Sie den aktuellen Simulationszustand als JSON-Datei oder drucken Sie einen formatierten PDF-Bericht.',
            fr: 'Exportez l’état actuel de la simulation en fichier JSON ou imprimez un rapport PDF formaté.',
            ar: 'صدّر حالة المحاكاة الحالية كملف JSON أو اطبع تقرير PDF منسقًا.',
          })}
        </p>
      </div>

      {msg && (
        <div className="bg-sim-accent/20 border border-sim-accent/40 rounded px-3 py-2 text-sm text-sim-text mb-3">
          {msg}
        </div>
      )}

      <div className="space-y-3">
        {/* JSON */}
        <div className="bg-sim-surface rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <FileJson size={16} className="text-sim-accent" />
            <span className="text-sim-text text-sm font-semibold">JSON</span>
          </div>
          <p className="text-sim-muted text-sm mb-3">
              {text(lang as LangCode, {
              en: 'Full simulation data: stats, events, checkpoints, technologies, beliefs.',
              tr: 'Tam simülasyon verisi: istatistikler, olaylar, kontrol noktaları, teknolojiler, inançlar.',
              de: 'Vollständige Simulationsdaten: Statistiken, Ereignisse, Checkpoints, Technologien, Glaubenssysteme.',
              fr: 'Données complètes de simulation : statistiques, événements, points de contrôle, technologies, croyances.',
              ar: 'بيانات المحاكاة الكاملة: الإحصاءات، الأحداث، نقاط التفقد، التقنيات، المعتقدات.',
            })}
          </p>
          <button
            onClick={downloadJSON}
            disabled={loading || !currentSim}
            className="w-full flex items-center justify-center gap-2 py-2 rounded border border-sim-accent/50 bg-sim-accent/10 hover:bg-sim-accent/25 text-sim-accent transition-colors text-sm font-share-tech disabled:opacity-50"
          >
            <Download size={14} className={loading ? 'animate-bounce' : ''} />
            {loading ? text(lang as LangCode, { en: 'Preparing…', tr: 'Hazırlanıyor…', de: 'Wird vorbereitet…', fr: 'Préparation…', ar: 'جارٍ التحضير…' }) : text(lang as LangCode, { en: 'Download JSON', tr: 'JSON İndir', de: 'JSON herunterladen', fr: 'Télécharger JSON', ar: 'تنزيل JSON' })}
          </button>
          {jsonFile && (
            <div className="flex gap-2 mt-2">
              <button
                onClick={() => shareFile(jsonFile)}
                className="flex-1 py-1.5 rounded border border-sim-accent/30 bg-sim-accent/5 hover:bg-sim-accent/15 text-sim-accent/80 transition-colors text-xs font-share-tech"
              >
                {text(lang as LangCode, { en: 'Share', tr: 'Paylaş', de: 'Teilen', fr: 'Partager', ar: 'مشاركة' })}
              </button>
              <button
                onClick={() => openFile(jsonFile)}
                className="flex-1 py-1.5 rounded border border-sim-accent/30 bg-sim-accent/5 hover:bg-sim-accent/15 text-sim-accent/80 transition-colors text-xs font-share-tech"
              >
                {text(lang as LangCode, { en: 'Open', tr: 'Aç', de: 'Öffnen', fr: 'Ouvrir', ar: 'فتح' })}
              </button>
            </div>
          )}
        </div>

        {/* PDF */}
        <div className="bg-sim-surface rounded-lg p-3">
          <div className="flex items-center gap-2 mb-2">
            <FileDown size={16} className="text-orange-400" />
            <span className="text-sim-text text-sm font-semibold">PDF</span>
          </div>
          <p className="text-sim-muted text-sm mb-3">
            {text(lang as LangCode, {
              en: 'Generates and downloads a .pdf file directly — no dialog needed.',
              tr: 'Kapak + giriş + tüm bölümler dahil .pdf dosyası olarak indirir.',
              de: 'Erzeugt und lädt direkt eine .pdf-Datei herunter — kein Dialog nötig.',
              fr: 'Génère et télécharge directement un fichier .pdf — sans dialogue.',
              ar: 'ينشئ ملف .pdf ويحمّله مباشرةً دون الحاجة إلى مربع حوار.',
            })}
          </p>
          <div className="flex gap-2">
            <button
              onClick={downloadPDF}
              disabled={pdfLoading || !currentSim}
              className="flex-1 flex items-center justify-center gap-2 py-2 rounded border border-orange-400/50 bg-orange-400/10 hover:bg-orange-400/25 text-orange-400 transition-colors text-sm font-share-tech disabled:opacity-50"
            >
              <FileDown size={14} className={pdfLoading ? 'animate-bounce' : ''} />
              {pdfLoading
                ? `${text(lang as LangCode, { en: 'Generating…', tr: 'Oluşturuluyor…', de: 'Wird erstellt…', fr: 'Génération…', ar: 'جارٍ الإنشاء…' })}${pdfProgress ? ` ${pdfProgress}` : ''}`
                : text(lang as LangCode, { en: 'Download PDF', tr: 'PDF İndir', de: 'PDF herunterladen', fr: 'Télécharger le PDF', ar: 'تنزيل PDF' })}
            </button>
            <button
              onClick={printReport}
              disabled={!currentSim}
              title={text(lang as LangCode, { en: 'Open Print View', tr: 'Yazdırma Görünümünü Aç', de: 'Druckansicht öffnen', fr: 'Ouvrir l’aperçu avant impression', ar: 'فتح عرض الطباعة' })}
              className="px-3 py-2 rounded border border-orange-400/30 bg-orange-400/5 hover:bg-orange-400/15 text-orange-400/70 transition-colors text-sm disabled:opacity-50"
            >
              <Printer size={14} />
            </button>
          </div>
          {pdfFile && (
            <div className="flex gap-2 mt-2">
              <button
                onClick={() => shareFile(pdfFile)}
                className="flex-1 py-1.5 rounded border border-orange-400/30 bg-orange-400/5 hover:bg-orange-400/15 text-orange-400/80 transition-colors text-xs font-share-tech"
              >
                {text(lang as LangCode, { en: 'Share', tr: 'Paylaş', de: 'Teilen', fr: 'Partager', ar: 'مشاركة' })}
              </button>
              <button
                onClick={() => openFile(pdfFile)}
                className="flex-1 py-1.5 rounded border border-orange-400/30 bg-orange-400/5 hover:bg-orange-400/15 text-orange-400/80 transition-colors text-xs font-share-tech"
              >
                {text(lang as LangCode, { en: 'Open', tr: 'Aç', de: 'Öffnen', fr: 'Ouvrir', ar: 'فتح' })}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Current snapshot */}
      {stats && (
        <div className="mt-4 border-t border-sim-border/30 pt-3">
          <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
            {text(lang as LangCode, { en: 'Current Snapshot', tr: 'Anlık Görüntü', de: 'Aktueller Überblick', fr: 'Instantané actuel', ar: 'اللقطة الحالية' })}
          </h4>
          <div className="space-y-1">
            {[
              [text(lang as LangCode, { en: 'Year', tr: 'Yıl', de: 'Jahr', fr: 'Année', ar: 'السنة' }), stats.year],
              [text(lang as LangCode, { en: 'Population', tr: 'Nüfus', de: 'Bevölkerung', fr: 'Population', ar: 'السكان' }), stats.population.toLocaleString()],
              [text(lang as LangCode, { en: 'Technologies', tr: 'Teknoloji', de: 'Technologien', fr: 'Technologies', ar: 'التقنيات' }), stats.technologies],
              [text(lang as LangCode, { en: 'Beliefs', tr: 'İnanç', de: 'Glaubenssätze', fr: 'Croyances', ar: 'المعتقدات' }), stats.beliefs],
              [text(lang as LangCode, { en: 'Language Stage', tr: 'Dil Aşaması', de: 'Sprachstufe', fr: 'Stade linguistique', ar: 'مرحلة اللغة' }), stats.max_language_stage],
            ].map(([l, v]) => (
              <div key={String(l)} className="flex justify-between text-sm border-b border-sim-border/20 py-0.5">
                <span className="text-sim-muted">{l}</span>
                <span className="text-sim-text font-mono">{v}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </DetailPanel>
  );
}
