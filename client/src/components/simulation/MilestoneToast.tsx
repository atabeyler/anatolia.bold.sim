import { useEffect, useRef, useState } from 'react';
import { useSimStore } from '../../store/simStore';
import { text, translateEventDescription, translateStageName, CAUSE_LABELS, type LangCode } from '../../utils/i18n';
import { playFounderDeathAlarm } from '../../utils/audioEngine';

interface Toast {
  id: string;
  message: string;
  sub?: string;
  icon: string;
  color: string;
}

function ToastItem({ toast }: { toast: Toast }) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const t = setTimeout(() => setVisible(true), 10);
    return () => clearTimeout(t);
  }, []);
  return (
    <div style={{
      background: 'rgba(4,4,18,0.96)',
      border: `1px solid ${toast.color}50`,
      borderLeft: `3px solid ${toast.color}`,
      padding: '8px 12px',
      minWidth: 200, maxWidth: 270,
      boxShadow: `0 4px 20px rgba(0,0,0,0.7), 0 0 8px ${toast.color}18`,
      fontFamily: 'Share Tech Mono, monospace',
      opacity: visible ? 1 : 0,
      transform: visible ? 'translateX(0)' : 'translateX(24px)',
      transition: 'opacity 0.25s ease, transform 0.25s ease',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontSize: 16, lineHeight: 1, flexShrink: 0 }}>{toast.icon}</span>
        <span style={{ fontSize: 14, color: toast.color, letterSpacing: '0.05em', lineHeight: 1.4 }}>{toast.message}</span>
      </div>
      {toast.sub && (
        <div style={{ fontSize: 12, color: '#8898c8', marginTop: 4, paddingLeft: 24, lineHeight: 1.3 }}>
          {toast.sub.length > 60 ? toast.sub.slice(0, 60) + '…' : toast.sub}
        </div>
      )}
    </div>
  );
}

const TOAST_DURATION = 5000;

export const MILESTONE_I18N: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  pop_10:        { tr: 'Nüfus 10\'a ulaştı!',          en: 'Population reached 10!',             de: 'Bevölkerung erreicht 10!',            fr: 'Population atteint 10!',             ar: 'بلغ عدد السكان 10!' },
  pop_25:        { tr: 'Nüfus 25\'e ulaştı!',          en: 'Population reached 25!',             de: 'Bevölkerung erreicht 25!',            fr: 'Population atteint 25!',             ar: 'بلغ عدد السكان 25!' },
  pop_50:        { tr: 'Nüfus 50\'ye ulaştı!',         en: 'Population reached 50!',             de: 'Bevölkerung erreicht 50!',            fr: 'Population atteint 50!',             ar: 'بلغ عدد السكان 50!' },
  pop_100:       { tr: 'Nüfus 100\'e ulaştı!',         en: 'Population reached 100!',            de: 'Bevölkerung erreicht 100!',           fr: 'Population atteint 100!',            ar: 'بلغ عدد السكان 100!' },
  pop_250:       { tr: 'Nüfus 250\'ye ulaştı!',        en: 'Population reached 250!',            de: 'Bevölkerung erreicht 250!',           fr: 'Population atteint 250!',            ar: 'بلغ عدد السكان 250!' },
  pop_500:       { tr: 'Nüfus 500\'e ulaştı!',         en: 'Population reached 500!',            de: 'Bevölkerung erreicht 500!',           fr: 'Population atteint 500!',            ar: 'بلغ عدد السكان 500!' },
  tech_5:        { tr: '5 teknoloji keşfedildi!',      en: '5 technologies discovered!',         de: '5 Technologien entdeckt!',            fr: '5 technologies découvertes!',        ar: 'اكتُشفت 5 تقنيات!' },
  tech_10:       { tr: '10 teknoloji keşfedildi!',     en: '10 technologies discovered!',        de: '10 Technologien entdeckt!',           fr: '10 technologies découvertes!',       ar: 'اكتُشفت 10 تقنيات!' },
  tech_15:       { tr: '15 teknoloji keşfedildi!',     en: '15 technologies discovered!',        de: '15 Technologien entdeckt!',           fr: '15 technologies découvertes!',       ar: 'اكتُشفت 15 تقنية!' },
  belief_first:  { tr: 'İlk inanç sistemi doğdu!',     en: 'First belief system emerged!',       de: 'Erstes Glaubenssystem entstanden!',   fr: 'Premier système de croyances né!',   ar: 'ظهر أول نظام عقيدي!' },
  belief_5:      { tr: '5 inanç sistemi kaydedildi!',  en: '5 belief systems recorded!',         de: '5 Glaubenssysteme erfasst!',          fr: '5 systèmes de croyances enregistrés!', ar: 'سُجِّلت 5 أنظمة عقائدية!' },
  art_first:     { tr: 'İlk sanat eseri yaratıldı!',   en: 'First art form created!',            de: 'Erste Kunstform erschaffen!',         fr: 'Première forme d\'art créée!',       ar: 'خُلق أول شكل فني!' },
  lang_stage2:   { tr: 'İlk fonemik dil aşaması!',     en: 'First phonemic language stage!',     de: 'Erste phonemische Sprachstufe!',      fr: 'Premier stade phonémique!',          ar: 'أول مرحلة لغوية صوتية!' },
  lang_stage3:   { tr: 'Morfolojik dil bilgisi gelişti!', en: 'Morphemic grammar emerged!',      de: 'Morphemische Grammatik entstanden!',  fr: 'Grammaire morphémique apparue!',     ar: 'نشأت قواعد صرفية!' },
  lang_stage4:   { tr: 'Karmaşık sözdizimi gelişti!',  en: 'Complex syntax achieved!',           de: 'Komplexe Syntax erreicht!',           fr: 'Syntaxe complexe atteinte!',         ar: 'تحقّق تركيب نحوي معقد!' },
  lang_stage5:   { tr: 'Yazı sistemi icat edildi!',    en: 'Writing system invented!',           de: 'Schriftsystem erfunden!',             fr: 'Système d\'écriture inventé!',       ar: 'اختُرع نظام كتابة!' },
  lang_stage6:   { tr: 'Edebiyat çağı başladı!',       en: 'Literature era begins!',             de: 'Literaturzeitalter beginnt!',         fr: 'L\'ère de la littérature commence!', ar: 'بدأ عصر الأدب!' },
  year_10:       { tr: 'Uygarlık 10 yıl ayakta!',      en: 'Civilization survived 10 years!',    de: 'Zivilisation überlebt 10 Jahre!',     fr: 'Civilisation survit 10 ans!',        ar: 'الحضارة أتمّت 10 سنوات!' },
  year_100:      { tr: 'Uygarlık 100 yıl ayakta!',     en: 'Civilization survived 100 years!',   de: 'Zivilisation überlebt 100 Jahre!',    fr: 'Civilisation survit 100 ans!',       ar: 'الحضارة أتمّت 100 عام!' },
  year_500:      { tr: 'Uygarlık 500 yıl ayakta!',     en: 'Civilization survived 500 years!',   de: 'Zivilisation überlebt 500 Jahre!',    fr: 'Civilisation survit 500 ans!',       ar: 'الحضارة أتمّت 500 عام!' },
  year_1000:     { tr: 'Uygarlık 1000 yıl ayakta!',    en: 'Civilization survived 1000 years!',  de: 'Zivilisation überlebt 1000 Jahre!',   fr: 'Civilisation survit 1000 ans!',      ar: 'الحضارة أتمّت 1000 عام!' },
};

export default function MilestoneToast() {
  const { stats, events, lang, addMoment, currentSim, milestones, soundSettings } = useSimStore();
  const [toasts, setToasts] = useState<Toast[]>([]);
  const fired = useRef<Set<string>>(new Set());
  const prevEventCount = useRef(-1);
  const simIdRef = useRef<string | null>(null);

  const L = lang as LangCode;
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(L, { tr, en, de, fr, ar });

  useEffect(() => {
    if (!currentSim?.id) return;
    if (simIdRef.current === currentSim.id) return;
    simIdRef.current = currentSim.id;
    try {
      const saved = JSON.parse(sessionStorage.getItem(`milestones_${currentSim.id}`) ?? '[]');
      fired.current = new Set(saved);
    } catch { fired.current = new Set(); }
    prevEventCount.current = events.length;
  }, [currentSim?.id, events.length]);

  // `toast.message` is always already localized (t()/text()/MILESTONE_I18N);
  // callers used to also pass the raw, untranslated `ev.description` in as
  // a `momentTitle` override that took priority over it here -- so the
  // Moments panel's prominent, colored title line (MomentsPanel.tsx renders
  // `m.title` large/colored and `m.description` small/secondary) showed raw
  // English/engine text for every non-English language, while the properly
  // translated `translateEventDescription()` text sat unnoticed in the
  // smaller `description` line below it. `toast.message` is the only title
  // source now; `toast.sub` (already translated where present) is the detail.
  function push(toast: Omit<Toast, 'id'>) {
    const id = Math.random().toString(36).slice(2);
    setToasts(prev => [...prev.slice(-2), { ...toast, id }]);
    setTimeout(() => setToasts(prev => prev.filter(x => x.id !== id)), TOAST_DURATION);
    if (simIdRef.current) {
      try { sessionStorage.setItem(`milestones_${simIdRef.current}`, JSON.stringify([...fired.current])); } catch {}
    }
    if (stats) {
      addMoment({
        day: stats.day,
        year: stats.year,
        icon: toast.icon,
        title: toast.message,
        description: toast.sub,
        color: toast.color,
      });
    }
  }

  function fireOnce(key: string, toast: Omit<Toast, 'id'>) {
    if (fired.current.has(key)) return;
    fired.current.add(key);
    push(toast);
  }

  // Backend milestone events (from WebSocket — server already curates these)
  const prevMilestoneCount = useRef(0);
  useEffect(() => {
    if (milestones.length <= prevMilestoneCount.current) { prevMilestoneCount.current = milestones.length; return; }
    const newest = milestones[0];
    prevMilestoneCount.current = milestones.length;
    if (!newest) return;
    const label = MILESTONE_I18N[newest.key];
    const message = label ? text(L, label) : newest.description;
    fireOnce(`backend_${newest.key}`, { icon: newest.icon, color: '#fbbf24', message });
  }, [milestones.length]);

  // First technology
  useEffect(() => {
    if (!stats || !simIdRef.current) return;
    if ((stats.technologies ?? 0) > 2) {
      fireOnce('first_tech', { icon: '⚙', color: '#4ecb71', message: t('İlk teknoloji keşfedildi!', 'First technology discovered!', 'Erste Technologie entdeckt!', 'Première technologie découverte!', 'اكتُشفت أول تقنية!') });
    }
  }, [stats?.technologies]);

  // Event-based first-only notifications
  useEffect(() => {
    if (prevEventCount.current === -1) return;
    const newCount = events.length;
    if (newCount <= prevEventCount.current) { prevEventCount.current = newCount; return; }
    const newEvents = events.slice(0, newCount - prevEventCount.current);
    prevEventCount.current = newCount;

    for (const ev of newEvents) {
      if (ev.event_type === 'birth' && !ev.data?.is_twin) {
        const name = ev.data?.name ?? '?';
        fireOnce('first_birth', {
          icon: '✦', color: '#ff8ab0',
          message: t(`İlk doğum: ${name}`, `First birth: ${name}`, `Erste Geburt: ${name}`, `Première naissance: ${name}`, `أول مولود: ${name}`),
        });
      }
      if (ev.event_type === 'birth' && ev.data?.is_twin) {
        fireOnce('first_twin', {
          icon: '✦', color: '#ff8ab0',
          message: t('İlk ikizler doğdu!', 'First twins born!', 'Erste Zwillinge geboren!', 'Premiers jumeaux nés!', 'وُلد أول توأم!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'death') {
        const name = ev.data?.name ?? '?';
        const rawCause = ev.data?.cause ?? '';
        const causeMap = rawCause ? CAUSE_LABELS[rawCause] : null;
        const causeStr = causeMap ? text(L, causeMap) : (rawCause ? rawCause.replace(/_/g, ' ') : '');
        const causePrefix = t('Sebep', 'Cause', 'Ursache', 'Cause', 'السبب');
        fireOnce('first_death', {
          icon: '†', color: '#e05a5a',
          message: t(`İlk ölüm: ${name}`, `First death: ${name}`, `Erster Todesfall: ${name}`, `Premier décès: ${name}`, `أول وفاة: ${name}`),
          sub: causeStr ? `${causePrefix}: ${causeStr}` : undefined,
        });

        // A founder's death ends the experiment's entire two-founder premise,
        // so it gets a distinct ~3s audio alarm (not the ordinary chime other
        // milestones use) and its own always-shown toast, on top of (not
        // instead of) the generic "first_death" toast above. Keyed per
        // individual id, not a single shared key, since both founders can
        // die independently.
        if (ev.data?.is_founder && ev.data?.individual_id) {
          if (soundSettings.notificationEnabled) playFounderDeathAlarm();
          fireOnce(`founder_death_${ev.data.individual_id}`, {
            icon: '☠', color: '#ff2d55',
            message: t(`KURUCU ÖLDÜ: ${name}`, `FOUNDER DIED: ${name}`, `GRÜNDER GESTORBEN: ${name}`, `FONDATEUR DÉCÉDÉ: ${name}`, `تُوفِّي المؤسِّس: ${name}`),
            sub: causeStr ? `${causePrefix}: ${causeStr}` : undefined,
          });
        }
      }
      if (ev.event_type === 'disaster') {
        fireOnce('first_disaster', {
          icon: '⚠', color: '#f97316',
          message: t('İlk doğal afet!', 'First natural disaster!', 'Erste Naturkatastrophe!', 'Première catastrophe naturelle!', 'أول كارثة طبيعية!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'epidemic') {
        fireOnce('first_epidemic', {
          icon: '⊗', color: '#c084fc',
          message: t('İlk salgın hastalık!', 'First epidemic!', 'Erste Seuche!', 'Première épidémie!', 'أول وباء!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'language') {
        const rawStage = ev.data?.stage_name ?? `stage ${ev.data?.stage}`;
        const stageName = translateStageName(rawStage, L);
        fireOnce('first_language', {
          icon: '◆', color: '#00d4ff',
          message: t(`İlk dil evrimi: ${stageName}`, `First language: ${stageName}`, `Erste Sprache: ${stageName}`, `Première langue: ${stageName}`, `أول لغة: ${stageName}`),
        });
      }
      if (ev.event_type === 'belief') {
        fireOnce('first_belief', {
          icon: '◆', color: '#a78bfa',
          message: t('İlk inanç oluştu', 'First belief formed', 'Erster Glaube geformt', 'Première croyance formée', 'تشكّل أول معتقد'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'art') {
        fireOnce('first_art', {
          icon: '◈', color: '#fb923c',
          message: t('İlk sanat eseri!', 'First art created!', 'Erstes Kunstwerk!', "Première œuvre d'art!", 'أول عمل فني!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'architecture') {
        fireOnce('first_architecture', {
          icon: '▣', color: '#94a3b8',
          message: t('İlk yapı inşa edildi!', 'First structure built!', 'Erstes Bauwerk errichtet!', 'Première structure construite!', 'أول مبنى شُيِّد!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'law') {
        fireOnce('first_law', {
          icon: '⊢', color: '#fbbf24',
          message: t('İlk yasa oluştu!', 'First law established!', 'Erstes Gesetz gegründet!', 'Première loi établie!', 'أول قانون أُسِّس!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
      if (ev.event_type === 'astronomy') {
        fireOnce('first_astronomy', {
          icon: '★', color: '#93c5fd',
          message: t('İlk astronomi keşfi!', 'First astronomy discovery!', 'Erste Astronomieentdeckung!', 'Première découverte astronomique!', 'أول اكتشاف فلكي!'),
          sub: ev.description ? translateEventDescription(ev.description, L) : undefined,
        });
      }
    }
  }, [events.length]);

  if (!toasts.length) return null;

  return (
    <div style={{
      position: 'fixed', top: 64, right: 16, zIndex: 55,
      display: 'flex', flexDirection: 'column', gap: 8,
      pointerEvents: 'none',
    }}>
      {toasts.map(t => <ToastItem key={t.id} toast={t} />)}
    </div>
  );
}
