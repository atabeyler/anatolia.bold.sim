import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

type LM = { tr: string; en: string; de: string; fr: string; ar: string };
// heritability/reversible mirror rust/sim-core/src/epigenetics.rs's LOCI
// table exactly (see also AGENTS.md's Epigenetics section) -- this is the
// single source of truth the "Transgenerational Inheritance" section below
// derives its numbers from, rather than a separately hand-maintained
// (and previously incorrect/stale) summary.
const LOCI: { id: string; gene: string; heritability: number; reversible: boolean; effect: LM; desc: LM }[] = [
  { id: 'HPA_AXIS',       gene: 'COMT',      heritability: 0.30, reversible: true,  effect: { tr: 'Stres Tepkisi',         en: 'Stress Reactivity',     de: 'Stressreaktivität',       fr: 'Réactivité au stress',     ar: 'رد فعل الإجهاد'       }, desc: { tr: 'Kronik stres altında zayıflar',              en: 'Blunted under chronic stress',          de: 'Abgeschwächt bei chronischem Stress',           fr: 'Atténué sous stress chronique',              ar: 'يضعف تحت الإجهاد المزمن'             } },
  { id: 'BDNF_PROMOTER',  gene: 'BDNF',      heritability: 0.20, reversible: true,  effect: { tr: 'Nöroplastisite',         en: 'Neuroplasticity',       de: 'Neuroplastizität',        fr: 'Neuroplasticité',          ar: 'اللدونة العصبية'       }, desc: { tr: 'Erken zorluk öğrenmeyi azaltır',             en: 'Early adversity reduces learning',       de: 'Frühe Schwierigkeiten verringern das Lernen',   fr: 'L\'adversité précoce réduit l\'apprentissage', ar: 'المحن المبكرة تقلل التعلم'           } },
  { id: 'MAOA_REGULATION',gene: 'MAOA',      heritability: 0.40, reversible: false, effect: { tr: 'Saldırganlık',           en: 'Aggression',            de: 'Aggression',              fr: 'Agressivité',              ar: 'العدوانية'             }, desc: { tr: 'Erken stres → kalıcı iz',                    en: 'Early stress → permanent mark',         de: 'Früher Stress → bleibende Markierung',          fr: 'Stress précoce → marque permanente',          ar: 'الإجهاد المبكر → أثر دائم'          } },
  { id: 'LEPTIN_RESIST',  gene: 'Metabolic', heritability: 0.50, reversible: true,  effect: { tr: 'Yağ Depolama',           en: 'Fat Storage',           de: 'Fettspeicherung',         fr: 'Stockage des graisses',    ar: 'تخزين الدهون'          }, desc: { tr: 'Kıtlık metabolik kaymayı tetikler',           en: 'Famine triggers metabolic shift',        de: 'Hunger löst Stoffwechselverschiebung aus',      fr: 'La famine déclenche un changement métabolique',ar: 'المجاعة تطلق تحولاً أيضياً'          } },
  { id: 'INSULIN_SENS',   gene: 'Metabolic', heritability: 0.35, reversible: true,  effect: { tr: 'İnsülin Duyarlılığı',    en: 'Insulin Sensitivity',   de: 'Insulinempfindlichkeit',  fr: 'Sensibilité à l\'insuline', ar: 'حساسية الأنسولين'     }, desc: { tr: 'Beslenme metabolik eşiği şekillendirir',      en: 'Nutrition shapes metabolic threshold',   de: 'Ernährung prägt die Stoffwechselschwelle',      fr: 'La nutrition façonne le seuil métabolique',   ar: 'التغذية تشكّل العتبة الأيضية'        } },
  // "AVPR1A" (vasopressin), not OXTR -- distinct gene from OXTR_METHYL below,
  // previously mislabeled with the same gene name as if there were two OXTR
  // marks instead of one OXTR and one vasopressin-pathway mark.
  { id: 'AVP_REGULATION', gene: 'AVPR1A',    heritability: 0.30, reversible: true,  effect: { tr: 'Sosyal Bellek',           en: 'Social Memory',         de: 'Soziales Gedächtnis',     fr: 'Mémoire sociale',          ar: 'الذاكرة الاجتماعية'   }, desc: { tr: 'Yalnızlık sosyal belleği aşındırır',          en: 'Isolation erodes social recall',         de: 'Isolation erodiert das soziale Gedächtnis',     fr: 'L\'isolement érode la mémoire sociale',       ar: 'العزلة تآكل الذاكرة الاجتماعية'     } },
  { id: 'OXTR_METHYL',    gene: 'OXTR',      heritability: 0.45, reversible: true,  effect: { tr: 'Sosyal Bağlanma',         en: 'Social Bonding',        de: 'Soziale Bindung',         fr: 'Lien social',              ar: 'الترابط الاجتماعي'    }, desc: { tr: 'Yalıtım bağlanma izlerini değiştirir',        en: 'Isolation demethylates bonding',         de: 'Isolation demethyliert Bindungsmarken',         fr: 'L\'isolement déméthyle les liens',            ar: 'العزلة تغير علامات الارتباط'         } },
  { id: 'IMMUNE_PRIMING', gene: 'Immune',    heritability: 0.60, reversible: false, effect: { tr: 'Patojen Belleği',         en: 'Pathogen Memory',       de: 'Pathogengedächtnis',      fr: 'Mémoire pathogène',        ar: 'ذاكرة الممرض'         }, desc: { tr: 'Enfeksiyon kalıcı izler bırakır',             en: 'Infection leaves lasting marks',         de: 'Infektion hinterlässt bleibende Spuren',        fr: 'L\'infection laisse des marques durables',    ar: 'العدوى تترك آثاراً دائمة'            } },
];

export default function EpigeneticsPanel() {
  const { lang, stats } = useSimStore();
  const epi = stats?.epigenetics ?? {};
  const hasStats = stats?.epigenetics != null;
  const L = lang as LangCode;

  return (
    <DetailPanel panelId="epigenetics" title="Epigenetics" titleTr="Epigenetik" titleDe="Epigenetik" titleFr="Épigénétique" titleAr="علم التخلق">
      <div className="bg-sim-surface rounded-lg p-3 mb-3">
        <p className="text-sim-muted text-sm italic">
          {text(L, { tr: 'Deneyim, DNA dizisini değiştirmeden gen ifadesini değiştirir. Bazı izler nesiller arasında aktarılır.', en: 'Experience modifies gene expression without changing DNA sequence. Some marks are heritable across generations.', de: 'Erfahrung verändert die Genexpression ohne die DNA-Sequenz zu ändern. Einige Markierungen sind generationsübergreifend vererbbar.', fr: 'L\'expérience modifie l\'expression génique sans changer la séquence d\'ADN. Certaines marques sont héritables entre générations.', ar: 'تعدّل التجربة التعبير الجيني دون تغيير تسلسل الحمض النووي. بعض العلامات قابلة للتوارث عبر الأجيال.' })}
        </p>
      </div>

      <div className="mb-3">
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(L, { tr: 'İzlenen Lokuslar', en: 'Monitored Loci', de: 'Überwachte Loci', fr: 'Loci surveillés', ar: 'المواضع المراقبة' })}
        </h4>
        <div className="space-y-2">
          {LOCI.map(locus => {
            // No fabricated "50%" placeholder before real stats arrive --
            // that used to be visually indistinguishable from a genuinely
            // measured neutral methylation reading, which matters for a
            // panel whose whole premise is showing real measurements.
            const hasLocusData = hasStats && locus.id in epi;
            if (!hasLocusData) {
              return (
                <div key={locus.id} className="bg-sim-surface/50 rounded p-2 opacity-60">
                  <div className="flex justify-between mb-1">
                    <span className="text-sim-text text-sm font-medium">{locus.gene}</span>
                    <span className="text-sim-accent text-sm">{text(L, locus.effect)}</span>
                  </div>
                  <div className="text-sim-muted text-sm italic">
                    {text(L, { tr: 'Henüz veri yok', en: 'No data yet', de: 'Noch keine Daten', fr: 'Pas encore de données', ar: 'لا توجد بيانات بعد' })}
                  </div>
                </div>
              );
            }
            const methylation: number = epi[locus.id];
            const pct = Math.round(methylation * 100);
            const barColor = methylation > 0.65
              ? `hsl(${270 - (methylation - 0.65) * 200}, 70%, 60%)`
              : `hsl(${220 + methylation * 50}, 70%, 60%)`;
            return (
              <div key={locus.id} className="bg-sim-surface/50 rounded p-2">
                <div className="flex justify-between mb-1">
                  <span className="text-sim-text text-sm font-medium">{locus.gene}</span>
                  <span className="text-sim-accent text-sm">{text(L, locus.effect)}</span>
                </div>
                <div className="text-sim-muted text-sm italic mb-1">
                  {text(L, locus.desc)}
                </div>
                <div className="mt-1 h-1.5 bg-sim-border rounded-full overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all duration-700"
                    style={{ width: `${pct}%`, background: barColor }}
                  />
                </div>
                <div className="flex justify-between mt-0.5">
                  <span className="text-xs text-sim-muted">{text(L, { tr: 'Aktif', en: 'Active', de: 'Aktiv', fr: 'Actif', ar: 'نشط' })}</span>
                  <span className="text-xs text-sim-accent font-mono">{pct}%</span>
                  <span className="text-xs text-sim-muted">{text(L, { tr: 'Sessiz', en: 'Silenced', de: 'Stumm', fr: 'Silencieux', ar: 'صامت' })}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(L, { tr: 'Nesiller Arası Aktarım', en: 'Transgenerational Inheritance', de: 'Transgenerationale Vererbung', fr: 'Héritage transgénérationnel', ar: 'التوارث عبر الأجيال' })}
        </h4>
        <div className="space-y-1">
          {/* One row per actual locus (see the LOCI table above), not a
              separately hand-maintained summary that had drifted from the
              real per-locus heritability coefficients (e.g. showing a
              single "50%" for two metabolic loci that are actually 35%/50%,
              and a "20-35%" reversible range that excluded LEPTIN_RESIST's
              real 50%). */}
          {LOCI.map(locus => (
            <div key={locus.id} className="flex justify-between py-0.5 border-b border-sim-border/30 text-sm">
              <span className="text-sim-muted">
                {text(L, locus.effect)} ({locus.gene})
                <span className="text-sim-muted/70 ml-1" style={{ fontSize: 11 }}>
                  {locus.reversible
                    ? text(L, { tr: 'geri dönüşümlü', en: 'reversible', de: 'reversibel', fr: 'réversible', ar: 'قابل للعكس' })
                    : text(L, { tr: 'kalıcı', en: 'permanent', de: 'dauerhaft', fr: 'permanent', ar: 'دائم' })}
                </span>
              </span>
              <span className="text-sim-accent">{Math.round(locus.heritability * 100)}%</span>
            </div>
          ))}
        </div>
      </div>
    </DetailPanel>
  );
}
