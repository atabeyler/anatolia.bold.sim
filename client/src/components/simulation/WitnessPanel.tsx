import { useEffect, useRef, useState } from 'react';
import { Eye, EyeOff, GripHorizontal } from 'lucide-react';
import axios from 'axios';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

function useDrag(initial: { x: number; y: number }) {
  const [pos, setPos] = useState(initial);
  const dragging = useRef(false);
  const origin = useRef({ clientX: 0, clientY: 0, posX: 0, posY: 0 });
  const moved = useRef(false);
  useEffect(() => {
    function onMove(e: MouseEvent) {
      if (!dragging.current) return;
      const dx = e.clientX - origin.current.clientX;
      const dy = e.clientY - origin.current.clientY;
      if (Math.abs(dx) + Math.abs(dy) > 4) moved.current = true;
      setPos({ x: Math.max(0, origin.current.posX + dx), y: Math.max(0, origin.current.posY + dy) });
    }
    function onUp() { dragging.current = false; }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
  }, []);
  function startDrag(clientX: number, clientY: number) {
    dragging.current = true; moved.current = false;
    origin.current = { clientX, clientY, posX: pos.x, posY: pos.y };
  }
  return { pos, startDrag, wasMoved: () => moved.current };
}

function generateNarration(ind: any, lang: string): string[] {
  const lines: string[] = [];
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });
  const health = ind.health ?? {};
  const mind = ind.mind ?? {};
  const language = ind.language ?? {};
  const fears = ind._fears ?? {};
  const cal = health.calories ?? 0.7;
  const hyd = health.hydration ?? 0.7;
  const hp = health.hp ?? 0.8;
  const stress = ind.psychology?.stress_level ?? 0.3;
  const consciousness = mind.consciousness ?? 0;
  const matingUrge = ind.mating_urge ?? 0;
  const satiation = ind.satiation ?? 0.5;

  // Activity
  if (ind._inWater) {
    lines.push(t('Suda ilerliyor — tehlike yüksek.', 'Moving through water — high danger.', 'Bewegt sich durchs Wasser — hohe Gefahr.', "Se déplace dans l'eau — danger élevé.", 'يتحرك عبر الماء — خطر مرتفع.'));
  } else if (cal < 0.15) {
    lines.push(t('Açlığın eşiğinde, yiyecek peşinde.', 'On the verge of starvation, searching for food.', 'Am Rande des Verhungerns, auf der Suche nach Nahrung.', 'Au bord de la famine, à la recherche de nourriture.', 'على حافة المجاعة، يبحث عن الطعام.'));
  } else if (hyd < 0.15) {
    lines.push(t('Susuzluktan zayıf, su arıyor.', 'Weakened by thirst, searching for water.', 'Geschwächt durch Durst, sucht Wasser.', "Affaibli par la soif, cherche de l'eau.", 'أضعفه العطش، يبحث عن الماء.'));
  } else if (hp < 0.25) {
    lines.push(t('Ağır hasta, zar zor ayakta.', 'Seriously ill, barely standing.', 'Schwer krank, kann sich kaum auf den Beinen halten.', 'Gravement malade, tient à peine debout.', 'مريض بشدة، بالكاد يقف.'));
  } else if (health.pregnancy) {
    lines.push(t('Doğuma yakın, yorgun ama güçlü.', 'Near birth, tired but strong.', 'Kurz vor der Geburt, müde aber stark.', "Proche de l'accouchement, fatiguée mais forte.", 'قريبة من الولادة، متعبة لكن قوية.'));
  } else if (matingUrge > 0.8 && cal > 0.4) {
    lines.push(t('Eş arayışında, etrafı tarıyor.', 'Searching for a mate, scanning the surroundings.', 'Auf Partnersuche, mustert die Umgebung.', "À la recherche d'un partenaire, scrute les environs.", 'يبحث عن شريك، يتفحص المحيط.'));
  } else if (satiation > 0.75 && hp > 0.7) {
    lines.push(t('Tok ve sağlıklı, grubuyla yürüyor.', 'Well-fed and healthy, walking with the group.', 'Satt und gesund, geht mit der Gruppe.', 'Rassasié et en bonne santé, marche avec le groupe.', 'شبعان وبصحة جيدة، يسير مع المجموعة.'));
  } else {
    lines.push(t('Günlük hayatta kalma rutininde.', 'Going through the daily survival routine.', 'Folgt der täglichen Überlebensroutine.', 'Suit la routine quotidienne de survie.', 'يمارس روتين البقاء اليومي.'));
  }

  // Fears
  if ((fears.predator ?? 0) > 0.4) {
    lines.push(t('Yırtıcı kokusu var — sinirli ve tetikte.', 'Senses predators — nervous and alert.', 'Wittert Raubtiere — nervös und wachsam.', 'Sent des prédateurs — nerveux et vigilant.', 'يستشعر مفترسات — متوتر ومتيقظ.'));
  } else if ((fears.scarcity ?? 0) > 0.4) {
    lines.push(t('Kıtlık korkusuyla yiyecek biriktiriyor.', 'Hoarding food, haunted by scarcity.', 'Hortet Nahrung, geplagt von der Angst vor Mangel.', 'Accumule de la nourriture, hanté par la pénurie.', 'يخزن الطعام، يطارده الخوف من الندرة.'));
  } else if ((fears.disaster ?? 0) > 0.3) {
    lines.push(t('Son afetin gölgesi hâlâ üzerinde.', 'Still shadowed by the recent disaster.', 'Noch immer überschattet von der jüngsten Katastrophe.', 'Encore marqué par la récente catastrophe.', 'لا يزال في ظل الكارثة الأخيرة.'));
  }

  // Language & social
  const stage = language.stage ?? 0;
  const wordCount = Object.keys(language.vocabulary ?? {}).length;
  if (stage >= 3 && wordCount > 0) {
    const firstWord = Object.keys(language.vocabulary)[0];
    lines.push(t(`"${firstWord}" dahil ${wordCount} kelimeyle iletişim kuruyor.`,
      `Communicates with ${wordCount} words including "${firstWord}".`,
      `Kommuniziert mit ${wordCount} Wörtern, darunter "${firstWord}".`,
      `Communique avec ${wordCount} mots dont "${firstWord}".`,
      `يتواصل بـ ${wordCount} كلمة بما فيها "${firstWord}".`));
  } else if (stage === 2) {
    lines.push(t('Duygusal seslerle çevresine mesaj veriyor.', 'Sending messages to others with emotional sounds.', 'Sendet Botschaften mit emotionalen Lauten.', 'Envoie des messages avec des sons émotionnels.', 'يرسل رسائل بأصوات عاطفية.'));
  } else if (stage === 1) {
    lines.push(t('El işaretleri ve seslerle anlaşıyor.', 'Communicating with gestures and simple sounds.', 'Verständigt sich mit Gesten und einfachen Lauten.', 'Communique par gestes et sons simples.', 'يتواصل بالإيماءات والأصوات البسيطة.'));
  }

  // Consciousness
  if (consciousness > 0.6) {
    const pct = Math.round(consciousness * 100);
    lines.push(t(`Derin bir iç dünya — bilinç %${pct}.`, `Deep inner world — consciousness ${pct}%.`, `Tiefe Innenwelt — Bewusstsein ${pct}%.`, `Monde intérieur profond — conscience ${pct}%.`, `عالم داخلي عميق — الوعي ${pct}%.`));
  } else if (consciousness > 0.3) {
    lines.push(t('Çevresini giderek daha fazla sorguluyor.', 'Increasingly questioning its surroundings.', 'Hinterfragt zunehmend seine Umgebung.', 'Interroge de plus en plus son environnement.', 'يتساءل بشكل متزايد عن محيطه.'));
  }

  // Stress
  if (stress > 0.7) {
    lines.push(t('Yüksek stres altında — vücut alarma geçmiş.', 'Under heavy stress — body on high alert.', 'Unter starkem Stress — Körper in Alarmbereitschaft.', 'Sous stress intense — corps en alerte.', 'تحت ضغط شديد — الجسم في حالة تأهب قصوى.'));
  }

  return lines.slice(0, 4);
}

const STAGE_NAMES: Record<number, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  0: { tr: 'Dil Öncesi',    en: 'Pre-linguistic',   de: 'Vorsprachlich',    fr: 'Prélinguistique',  ar: 'ما قبل اللغة'  },
  1: { tr: 'Jest',          en: 'Gestural',          de: 'Gestural',         fr: 'Gestuel',          ar: 'إيمائي'         },
  2: { tr: 'Duygusal Ses',  en: 'Emotional Sound',   de: 'Emotionaler Laut', fr: 'Son émotionnel',   ar: 'صوت عاطفي'     },
  3: { tr: 'Proto Kelime',  en: 'Proto-word',        de: 'Protowort',        fr: 'Proto-mot',        ar: 'كلمة أولى'     },
  4: { tr: 'Sözdizimi',     en: 'Syntax',            de: 'Syntax',           fr: 'Syntaxe',          ar: 'نحو'            },
  5: { tr: 'Soyut',         en: 'Abstract',          de: 'Abstrakt',         fr: 'Abstrait',         ar: 'مجرد'           },
  6: { tr: 'Yazı',          en: 'Writing',           de: 'Schrift',          fr: 'Écriture',         ar: 'كتابة'          },
};

const DEATH_CAUSE_I18N: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  starvation:                  { tr: 'Açlık',                    en: 'Starvation',             de: 'Verhungern',                       fr: 'Famine',                        ar: 'الجوع'                   },
  dehydration:                 { tr: 'Susuzluk',                 en: 'Dehydration',            de: 'Dehydrierung',                     fr: 'Déshydratation',               ar: 'الجفاف'                  },
  old_age:                     { tr: 'Yaşlılık',                 en: 'Old age',                de: 'Altersschwäche',                  fr: 'Vieillesse',                    ar: 'الشيخوخة'                },
  predator:                    { tr: 'Yırtıcı hayvan',           en: 'Predator',               de: 'Raubtier',                         fr: 'Prédateur',                    ar: 'مفترس'                   },
  genetic_disease:             { tr: 'Genetik hastalık',         en: 'Genetic disease',        de: 'Erbkrankheit',                     fr: 'Maladie génétique',           ar: 'مرض وراثي'               },
  infection:                   { tr: 'Enfeksiyon',               en: 'Infection',              de: 'Infektion',                        fr: 'Infection',                     ar: 'عدوى'                    },
  trauma:                      { tr: 'Travma',                   en: 'Trauma',                 de: 'Trauma',                           fr: 'Traumatisme',                   ar: 'صدمة'                    },
  birth_complications:         { tr: 'Doğum komplikasyonu',      en: 'Birth complications',    de: 'Geburtskomplikationen',            fr: 'Complications à la naissance',  ar: 'مضاعفات الولادة'         },
  conflict:                    { tr: 'Çatışma',                  en: 'Conflict',               de: 'Konflikt',                         fr: 'Conflit',                       ar: 'صراع'                    },
  drowning:                    { tr: 'Boğulma',                  en: 'Drowning',               de: 'Ertrinken',                        fr: 'Noyade',                        ar: 'الغرق'                   },
  meteor_tsunami:              { tr: 'Meteor çarpması ve tsunami', en: 'Meteor impact and tsunami', de: 'Meteoriteneinschlag und Tsunami', fr: 'Impact de météore et tsunami', ar: 'اصطدام نيزك وتسونامي' },
  disease_intestinal_parasite: { tr: 'Bağırsak paraziti',        en: 'Intestinal parasite',    de: 'Darmparasit',                      fr: 'Parasite intestinal',           ar: 'طفيل معوي'               },
  disease_cholera_like:        { tr: 'Kolera benzeri hastalık',  en: 'Cholera-like disease',   de: 'Choleraähnliche Krankheit',        fr: 'Maladie type choléra',          ar: 'مرض شبيه بالكوليرا'      },
  disease_respiratory_common:  { tr: 'Solunum yolu hastalığı',   en: 'Respiratory illness',    de: 'Atemwegserkrankung',               fr: 'Maladie respiratoire',          ar: 'مرض تنفسي'               },
  disease_pneumonia_like:      { tr: 'Zatürre benzeri hastalık', en: 'Pneumonia-like illness', de: 'Lungenentzündungsähnliche Erkrankung', fr: 'Maladie type pneumonie',     ar: 'مرض شبيه بالالتهاب الرئوي' },
  disease_plague_like:         { tr: 'Veba benzeri salgın',      en: 'Plague-like epidemic',   de: 'Pestähnliche Epidemie',            fr: 'Épidémie type peste',           ar: 'وباء شبيه بالطاعون'      },
  disease_malaria_like:        { tr: 'Sıtma benzeri hastalık',   en: 'Malaria-like disease',   de: 'Malariaähnliche Krankheit',        fr: 'Maladie type paludisme',        ar: 'مرض شبيه بالملاريا'      },
  disease_fever_tick:          { tr: 'Kene ateşi',               en: 'Tick fever',             de: 'Zeckenfieber',                     fr: 'Fièvre à tiques',               ar: 'حمى القراد'              },
  disease_wound_infection:     { tr: 'Yara enfeksiyonu',         en: 'Wound infection',        de: 'Wundinfektion',                    fr: 'Infection de plaie',            ar: 'عدوى الجرح'              },
  disease_fungal_skin:         { tr: 'Mantar enfeksiyonu',       en: 'Fungal skin infection',  de: 'Pilzinfektion der Haut',           fr: 'Infection fongique cutanée',    ar: 'عدوى فطرية جلدية'       },
};

function deathCauseLabel(cause: string | null | undefined, lang: string): string {
  if (!cause) return text(lang as LangCode, { tr: 'Bilinmeyen', en: 'Unknown', de: 'Unbekannt', fr: 'Inconnue', ar: 'غير معروف' });
  const entry = DEATH_CAUSE_I18N[cause];
  if (entry) return text(lang as LangCode, entry);
  return cause.replace(/_/g, ' ');
}

export default function WitnessPanel() {
  const { watchedIndividualId, setWatchedIndividual, currentSim, accessToken, lang, stats } = useSimStore();
  const [ind, setInd] = useState<any>(null);
  const drag = useDrag({ x: 16, y: 120 });

  const confirmedDeadRef = useRef(false);
  useEffect(() => { confirmedDeadRef.current = false; }, [watchedIndividualId]);

  useEffect(() => {
    if (!watchedIndividualId || !currentSim || !accessToken) { setInd(null); return; }
    // Stop polling once we confirmed the individual is dead — no need to keep fetching
    if (confirmedDeadRef.current) return;
    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval> | null = null;
    async function fetch_() {
      if (confirmedDeadRef.current) return;
      try {
        const res = await axios.get(
          `/api/simulations/${currentSim!.id}/population/${watchedIndividualId}`,
          { headers: { Authorization: `Bearer ${accessToken}` } }
        );
        if (!cancelled) {
          setInd(res.data);
          if (res.data.alive === false) {
            confirmedDeadRef.current = true;
            if (intervalId) clearInterval(intervalId);
          }
        }
      } catch {
        // fallback: try the population list (alive=true only — if not found, individual is dead)
        try {
          const res = await axios.get(
            `/api/simulations/${currentSim!.id}/population?alive=true&limit=200`,
            { headers: { Authorization: `Bearer ${accessToken}` } }
          );
          const found = (res.data as any[]).find((i: any) => i.id === watchedIndividualId);
          if (!cancelled) {
            if (found) setInd(found);
            else if (ind) {
              // Not in alive list — mark as dead using last known data
              confirmedDeadRef.current = true;
              if (intervalId) clearInterval(intervalId);
              setInd({ ...ind, alive: false });
            }
          }
        } catch {}
      }
    }
    fetch_();
    intervalId = setInterval(fetch_, 8000);
    return () => { cancelled = true; if (intervalId) clearInterval(intervalId); };
  }, [watchedIndividualId, currentSim?.id, stats?.day]);

  if (!watchedIndividualId) return null;

  const name = ind ? (ind.phenotype?.name ?? ind.name ?? `ID:${watchedIndividualId.slice(-6)}`) : '…';
  const age = ind ? Math.floor(ind.age_years ?? (ind.age ?? 0) / 365) : null;
  const isDead = ind && (ind.is_dead || ind.alive === false || !!ind.death_cause);
  const stage = ind?.language?.stage ?? 0;
  const narration = ind && !isDead ? generateNarration(ind, lang) : [];

  return (
    <div
      style={{
        position: 'fixed', left: drag.pos.x, top: drag.pos.y, zIndex: 42, width: 240,
        background: 'rgba(2,6,4,0.96)', border: '1px solid rgba(0,212,255,0.35)',
        backdropFilter: 'blur(14px)', boxShadow: '0 8px 32px rgba(0,0,0,0.7), 0 0 16px rgba(0,212,255,0.08)',
        fontFamily: 'Share Tech Mono, monospace',
      }}
    >
      {/* Drag handle */}
      <div
        onMouseDown={e => drag.startDrag(e.clientX, e.clientY)}
        style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '7px 10px', borderBottom: '1px solid rgba(0,212,255,0.15)', cursor: 'grab', userSelect: 'none' }}
      >
        <GripHorizontal size={10} style={{ color: '#6a9a80', flexShrink: 0 }} />
        <Eye size={10} style={{ color: '#00d4ff', flexShrink: 0 }} />
        <span style={{ fontSize: 10, color: '#00d4ff', letterSpacing: '0.2em', flex: 1 }}>
          {text(lang as LangCode, { tr: 'TANİK MODU', en: 'WITNESS MODE', de: 'ZEUGENMODUS', fr: 'MODE TÉMOIN', ar: 'وضع الشاهد' })}
        </span>
        <button
          onClick={() => setWatchedIndividual(null)}
          style={{ background: 'transparent', border: 'none', color: '#a0c8b0', cursor: 'pointer', lineHeight: 0, padding: 2 }}
          title={text(lang as LangCode, { tr: 'Takibi bırak', en: 'Stop watching', de: 'Beobachtung beenden', fr: 'Arrêter de regarder', ar: 'إيقاف المتابعة' })}
        >
          <EyeOff size={10} />
        </button>
      </div>

      <div style={{ padding: '8px 10px' }}>
        {/* Identity */}
        <div style={{ marginBottom: 8, paddingBottom: 6, borderBottom: '1px solid rgba(0,212,255,0.1)' }}>
          <div style={{ fontSize: 13, color: ind?.sex === 'male' ? '#6090ff' : '#ff8ab0', fontFamily: 'Orbitron, monospace', fontWeight: 700 }}>
            {name}
            {isDead && <span style={{ color: '#e05a5a', marginLeft: 6, fontSize: 11 }}>†</span>}
          </div>
          {age !== null && (
            <div style={{ fontSize: 11, color: '#a0b4ff', marginTop: 2 }}>
              {age} {text(lang as LangCode, { tr: 'yaş', en: 'yr', de: 'J.', fr: 'ans', ar: 'سنة' })} · {text(lang as LangCode, STAGE_NAMES[stage] ?? { en: '—' })}
              {ind?.group_id && <span style={{ color: '#4f6ef7', marginLeft: 6 }}>· {text(lang as LangCode, { tr: 'grupta', en: 'in group', de: 'in Gruppe', fr: 'dans groupe', ar: 'في المجموعة' })}</span>}
            </div>
          )}
        </div>

        {/* Narration */}
        {!ind && (
          <div style={{ fontSize: 11, color: '#6a8878', letterSpacing: '0.08em' }}>
            {text(lang as LangCode, { tr: 'Birey yükleniyor…', en: 'Loading individual…', de: 'Individuum lädt…', fr: 'Chargement de l\'individu…', ar: 'جارٍ تحميل الفرد…' })}
          </div>
        )}
        {isDead && (
          <div style={{ fontSize: 11, color: '#a05050' }}>
            <div>† {text(lang as LangCode, { tr: 'Bu birey hayatini kaybetti.', en: 'This individual has died.', de: 'Dieses Individuum ist gestorben.', fr: 'Cet individu est decede.', ar: 'لقي هذا الفرد حتفه.' })}</div>
            {ind.death_cause && (
              <div style={{ marginTop: 3, color: '#c06060', fontSize: 10 }}>
                {text(lang as LangCode, { tr: 'Sebep:', en: 'Cause:', de: 'Ursache:', fr: 'Cause:', ar: 'السبب:' })} {deathCauseLabel(ind.death_cause, lang)}
              </div>
            )}
          </div>
        )}
        {narration.map((line, i) => (
          <div key={i} style={{ fontSize: 11, color: i === 0 ? '#c8d8e8' : '#8898c8', lineHeight: 1.55, marginBottom: 3 }}>
            {i === 0 ? '▸ ' : '  '}{line}
          </div>
        ))}

        {/* Vitals */}
        {ind && !isDead && (
          <div style={{ marginTop: 8, paddingTop: 6, borderTop: '1px solid rgba(0,212,255,0.1)', display: 'flex', gap: 8 }}>
            {[
              { l: 'HP', v: ind.health?.hp ?? 0, c: '#4ecb71' },
              { l: text(lang as LangCode, { tr: 'Kal', en: 'Cal', de: 'Kal.', fr: 'Cal', ar: 'سعر' }), v: ind.health?.calories ?? 0, c: '#d4a838' },
              { l: text(lang as LangCode, { tr: 'Su', en: 'H₂O', de: 'H₂O', fr: 'H₂O', ar: 'ماء' }), v: ind.health?.hydration ?? 0, c: '#7dd3fc' },
            ].map(({ l, v, c }) => (
              <div key={l} style={{ flex: 1, textAlign: 'center' }}>
                <div style={{ fontSize: 10, color: '#6a8878', marginBottom: 2 }}>{l}</div>
                <div style={{ height: 2, background: 'rgba(255,255,255,0.08)', borderRadius: 1, overflow: 'hidden' }}>
                  <div style={{ height: '100%', width: `${Math.round(v * 100)}%`, background: c }} />
                </div>
                <div style={{ fontSize: 10, color: c, marginTop: 1 }}>{Math.round(v * 100)}%</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
