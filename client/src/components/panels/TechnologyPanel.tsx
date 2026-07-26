import { useEffect, useState } from 'react';
import axios from 'axios';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Check, Lock } from 'lucide-react';
import { text, translateTech, type LangCode } from '../../utils/i18n';

type TM = { tr: string; en: string; de: string; fr: string; ar: string };
const TECH_TIERS: { tier: number; label: TM; techs: string[] }[] = [
  { tier: 0, label: { tr: 'Anlık Hayatta Kalma', en: 'Immediate Survival',  de: 'Sofortiges Überleben', fr: 'Survie immédiate',     ar: 'البقاء الفوري'       }, techs: ['fire_making', 'stone_tools', 'foraging'] },
  { tier: 1, label: { tr: 'Temel Geçim',          en: 'Basic Subsistence',   de: 'Grundversorgung',      fr: 'Subsistance de base',  ar: 'الكفاف الأساسي'      }, techs: ['hunting_spear', 'shelter_basic', 'water_container', 'animal_trap', 'clothing_basic'] },
  { tier: 2, label: { tr: 'Gıda Güvenliği',       en: 'Food Security',       de: 'Ernährungssicherheit', fr: 'Sécurité alimentaire',  ar: 'الأمن الغذائي'       }, techs: ['fishing', 'plant_cultivation', 'animal_herding', 'food_preservation', 'bow_arrow'] },
  { tier: 3, label: { tr: 'Karmaşıklık',           en: 'Complexity',          de: 'Komplexität',          fr: 'Complexité',            ar: 'التعقيد'             }, techs: ['pottery', 'weaving', 'metallurgy_copper', 'writing_system', 'calendar', 'mathematics_basic'] },
  { tier: 4, label: { tr: 'Uygarlık',              en: 'Civilization',        de: 'Zivilisation',         fr: 'Civilisation',          ar: 'الحضارة'             }, techs: ['architecture_stone', 'wheel', 'irrigation', 'sailing', 'metallurgy_iron'] },
];

const TECH_NAMES: Record<string, TM> = {
  fire_making:        { tr: 'Ateş Yakma',      en: 'Fire Making',        de: 'Feuermachen',          fr: 'Faire du feu',           ar: 'إشعال النار'        },
  stone_tools:        { tr: 'Taş Aletler',     en: 'Stone Tools',        de: 'Steinwerkzeuge',       fr: 'Outils en pierre',       ar: 'أدوات حجرية'        },
  foraging:           { tr: 'Toplayıcılık',    en: 'Foraging',           de: 'Sammeln',              fr: 'Cueillette',             ar: 'الجمع والقطف'       },
  hunting_spear:      { tr: 'Av Mızrağı',      en: 'Hunting Spear',      de: 'Jagdspeer',            fr: 'Lance de chasse',        ar: 'رمح الصيد'          },
  shelter_basic:      { tr: 'Temel Barınak',   en: 'Basic Shelter',      de: 'Einfache Unterkunft',  fr: 'Abri de base',           ar: 'مأوى أساسي'         },
  water_container:    { tr: 'Su Kabı',         en: 'Water Container',    de: 'Wasserbehälter',       fr: 'Récipient à eau',        ar: 'وعاء الماء'         },
  animal_trap:        { tr: 'Hayvan Tuzağı',   en: 'Animal Trap',        de: 'Tierfalle',            fr: 'Piège à animaux',        ar: 'فخ الحيوانات'       },
  clothing_basic:     { tr: 'Giysi',           en: 'Clothing',           de: 'Kleidung',             fr: 'Vêtements',              ar: 'الملبس'             },
  fishing:            { tr: 'Balıkçılık',      en: 'Fishing',            de: 'Fischerei',            fr: 'Pêche',                  ar: 'الصيد'              },
  plant_cultivation:  { tr: 'Tarım',           en: 'Plant Cultivation',  de: 'Pflanzenzucht',        fr: 'Culture végétale',       ar: 'زراعة النباتات'     },
  animal_herding:     { tr: 'Hayvancılık',     en: 'Animal Herding',     de: 'Viehhaltung',          fr: 'Élevage',                ar: 'رعي الحيوانات'      },
  food_preservation:  { tr: 'Gıda Saklama',    en: 'Food Preservation',  de: 'Lebensmittelkonservierung', fr: 'Conservation des aliments', ar: 'حفظ الطعام'    },
  bow_arrow:          { tr: 'Yay ve Ok',       en: 'Bow & Arrow',        de: 'Bogen und Pfeil',      fr: 'Arc et flèche',          ar: 'القوس والسهم'       },
  pottery:            { tr: 'Çömlekçilik',     en: 'Pottery',            de: 'Töpferei',             fr: 'Poterie',                ar: 'الفخار'             },
  weaving:            { tr: 'Dokumacılık',     en: 'Weaving',            de: 'Weberei',              fr: 'Tissage',                ar: 'النسيج'             },
  metallurgy_copper:  { tr: 'Bakır İşleme',    en: 'Copper Metallurgy',  de: 'Kupfermetallurgie',    fr: 'Métallurgie du cuivre',  ar: 'معادن النحاس'       },
  writing_system:     { tr: 'Yazı Sistemi',    en: 'Writing System',     de: 'Schriftsystem',        fr: 'Système d\'écriture',    ar: 'نظام الكتابة'       },
  calendar:           { tr: 'Takvim',          en: 'Calendar',           de: 'Kalender',             fr: 'Calendrier',             ar: 'التقويم'            },
  mathematics_basic:  { tr: 'Temel Matematik', en: 'Basic Mathematics',  de: 'Grundmathematik',      fr: 'Mathématiques de base',  ar: 'الرياضيات الأساسية' },
  architecture_stone: { tr: 'Taş Mimari',      en: 'Stone Architecture', de: 'Steinarchitektur',     fr: 'Architecture en pierre', ar: 'العمارة الحجرية'    },
  wheel:              { tr: 'Tekerlek',         en: 'The Wheel',          de: 'Das Rad',              fr: 'La roue',                ar: 'العجلة'             },
  irrigation:         { tr: 'Sulama',          en: 'Irrigation',         de: 'Bewässerung',          fr: 'Irrigation',             ar: 'الري'               },
  sailing:            { tr: 'Denizcilik',      en: 'Sailing',            de: 'Segeln',               fr: 'Navigation',             ar: 'الإبحار'            },
  metallurgy_iron:    { tr: 'Demir İşleme',    en: 'Iron Metallurgy',    de: 'Eisenmetallurgie',     fr: 'Métallurgie du fer',     ar: 'معادن الحديد'       },
};

const HOW_STORIES: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  fire_making:        { tr: 'Taşları birbirine sürtünce kıvılcım çıktığını fark etti.',                      en: 'Noticed sparks when striking stones together.',                                de: 'Bemerkte Funken beim Aufeinanderreiben von Steinen.',                       fr: 'Remarqua des étincelles en frottant des pierres ensemble.',                    ar: 'لاحظ شرارات عند احتكاك الحجارة ببعضها.' },
  stone_tools:        { tr: 'Sert taşları kırarak kesici kenarlar oluşturdu.',                               en: 'Broke hard stones to create sharp cutting edges.',                            de: 'Zerschlug harte Steine, um scharfe Schneidekanten zu erzeugen.',             fr: 'Cassa des pierres dures pour créer des bords tranchants.',                    ar: 'كسر الحجارة الصلبة لصنع حواف حادة.' },
  foraging:           { tr: 'Bitki ve meyveleri gözlemleyerek yenilebilirliğini öğrendi.',                   en: 'Learned edibility by careful observation of plants and fruits.',               de: 'Erlernte Essbarkeit durch sorgfältige Beobachtung von Pflanzen und Früchten.',fr: 'Apprit la comestibilité par observation attentive des plantes et fruits.',     ar: 'تعلم صلاحية الأكل من خلال مراقبة النباتات والثمار.' },
  hunting_spear:      { tr: 'Uzun bir dala keskin bir taş bağlayarak av menzilini artırdı.',                 en: 'Tied a sharp stone to a long branch to extend hunting reach.',                de: 'Band einen scharfen Stein an einen langen Ast, um die Jagdreichweite zu erhöhen.',fr: 'Attacha une pierre pointue à une longue branche pour chasser de plus loin.',  ar: 'ربط حجرًا حادًا بغصن طويل لتوسيع مدى الصيد.' },
  shelter_basic:      { tr: 'Dal ve yapraklarla soğuk ve yağmurdan korundu.',                                en: 'Used branches and leaves to shelter from cold and rain.',                      de: 'Nutzte Äste und Blätter als Schutz vor Kälte und Regen.',                  fr: 'Utilisa des branches et des feuilles pour se protéger du froid et de la pluie.',ar: 'استخدم الأغصان والأوراق للحماية من البرد والمطر.' },
  water_container:    { tr: 'Su taşımak için hayvan derisi veya büyük yaprağı kullandı.',                    en: 'Used animal hide or large leaves to carry water.',                            de: 'Nutzte Tierhaut oder große Blätter zum Wassertransport.',                   fr: 'Utilisa une peau animale ou de grandes feuilles pour transporter de l\'eau.',  ar: 'استخدم جلد الحيوان أو الأوراق الكبيرة لنقل الماء.' },
  animal_trap:        { tr: 'Hayvan izlerini takip ederek geçiş noktalarına tuzak kurdu.',                   en: 'Tracked animals and set traps along their paths.',                            de: 'Verfolgte Tiere und stellte Fallen entlang ihrer Pfade auf.',               fr: 'Suivit les animaux et posa des pièges sur leurs chemins.',                    ar: 'تتبع الحيوانات ونصب فخاخًا على مساراتها.' },
  clothing_basic:     { tr: 'Avladığı hayvanların derisini vücuduna sararak sıcak kaldı.',                   en: 'Wrapped animal hides around the body to stay warm.',                          de: 'Wickelte Tierhäute um den Körper, um warm zu bleiben.',                     fr: 'S\'enveloppa de peaux animales pour rester au chaud.',                        ar: 'لف جلود الحيوانات حول جسده للبقاء دافئًا.' },
  fishing:            { tr: 'Sığ sularda balıkların hareketini izleyerek yakalamayı keşfetti.',              en: 'Observed fish movements in shallow water and caught them.',                   de: 'Beobachtete Fischbewegungen in flachem Wasser und fing sie.',               fr: 'Observa les mouvements des poissons en eau peu profonde et les attrapa.',     ar: 'راقب حركة الأسماك في المياه الضحلة واصطادها.' },
  plant_cultivation:  { tr: 'Düşen tohumların bahar ayında filizlendiğini fark ederek tarımı buldu.',        en: 'Noticed fallen seeds sprouting each spring and began cultivating.',           de: 'Bemerkte, dass gefallene Samen jeden Frühling keimten, und begann zu kultivieren.',fr: 'Remarqua que les graines germaient chaque printemps et commença à cultiver.', ar: 'لاحظ أن البذور تنبت كل ربيع فبدأ الزراعة.' },
  animal_herding:     { tr: 'Yavrularını yetiştirerek kontrollü sürü oluşturmayı öğrendi.',                  en: 'Raised young animals to build a managed herd.',                               de: 'Zog junge Tiere auf, um eine kontrollierte Herde aufzubauen.',              fr: 'Éleva de jeunes animaux pour constituer un troupeau géré.',                   ar: 'ربّى صغار الحيوانات لبناء قطيع منظم.' },
  food_preservation:  { tr: 'Kurutma ve tuzlamanın yiyecekleri uzun süre koruduğunu keşfetti.',              en: 'Discovered that drying and salting extends food life significantly.',         de: 'Entdeckte, dass Trocknen und Salzen die Haltbarkeit von Lebensmitteln verlängert.',fr: 'Découvrit que sécher et saler prolonge la durée de conservation des aliments.',ar: 'اكتشف أن التجفيف والتمليح يطيل صلاحية الطعام.' },
  bow_arrow:          { tr: 'Esnek bir dalı iple gererek ok fırlatabildğini keşfetti.',                       en: 'Found that bending a flexible branch with sinew could launch projectiles.',   de: 'Entdeckte, dass ein gebogener Ast mit Sehne Pfeile abschießen kann.',       fr: 'Découvrit qu\'une branche flexible tendue avec un tendon pouvait lancer des projectiles.',ar: 'اكتشف أن الغصن المرن مع وتر يمكنه إطلاق مقذوفات.' },
  pottery:            { tr: 'Islatılan kilin ateşte sertleştiğini gözlemleyerek kaplar yaptı.',              en: 'Observed that wet clay hardens in fire and formed vessels.',                  de: 'Beobachtete, dass nasser Ton im Feuer hart wird, und formte Gefäße.',       fr: 'Observa que l\'argile humide durcit au feu et fabriqua des récipients.',     ar: 'لاحظ أن الطين الرطب يتصلب بالنار فصنع أوانٍ.' },
  weaving:            { tr: 'Bitki liflerini birbirine geçirerek dayanıklı bez üretti.',                      en: 'Interlaced plant fibers to produce durable cloth.',                           de: 'Verflocht Pflanzenfasern zu haltbarem Stoff.',                              fr: 'Entrecroisa des fibres végétales pour produire un tissu solide.',             ar: 'نسج ألياف النباتات لإنتاج قماش متين.' },
  metallurgy_copper:  { tr: 'Bakır cevherinin ateşte eriyip kalıba dökülebildğini keşfetti.',                en: 'Discovered copper ore melts and can be cast into shapes.',                    de: 'Entdeckte, dass Kupfererz schmilzt und in Formen gegossen werden kann.',    fr: 'Découvrit que le minerai de cuivre fond et peut être moulé.',                ar: 'اكتشف أن خام النحاس يذوب ويمكن صبه في قوالب.' },
  writing_system:     { tr: 'Kil üzerine işaretler çizerek bilgiyi gelecek nesillere aktardı.',              en: 'Drew marks on clay to pass knowledge to future generations.',                  de: 'Ritzte Zeichen in Ton, um Wissen an kommende Generationen weiterzugeben.',  fr: 'Grava des signes sur l\'argile pour transmettre le savoir aux générations futures.',ar: 'رسم علامات على الطين لنقل المعرفة للأجيال القادمة.' },
  calendar:           { tr: 'Gökyüzündeki yıldız ve Güneş hareketlerini izleyerek mevsimleri öngördü.',      en: 'Predicted seasons by studying star and sun movements.',                       de: 'Sagte Jahreszeiten vorher, indem er Stern- und Sonnenbewegungen studierte.',  fr: 'Prédit les saisons en étudiant les mouvements des étoiles et du soleil.',    ar: 'تنبأ بالمواسم بدراسة حركة النجوم والشمس.' },
  mathematics_basic:  { tr: 'Nesneleri sayarak ve gruplandırarak sayı kavramını geliştirdi.',                 en: 'Developed number concepts by counting and grouping objects.',                  de: 'Entwickelte Zahlenbegriffe durch Zählen und Gruppieren von Objekten.',      fr: 'Développa des concepts numériques en comptant et regroupant des objets.',     ar: 'طور مفاهيم الأعداد بعد وتجميع الأشياء.' },
  architecture_stone: { tr: 'Taşları özenle üst üste dizerek kalıcı yapılar inşa etti.',                    en: 'Stacked stones carefully to construct permanent structures.',                  de: 'Schichtete Steine sorgfältig, um dauerhafte Strukturen zu bauen.',          fr: 'Empila des pierres soigneusement pour construire des structures permanentes.', ar: 'رص الحجارة بعناية لبناء هياكل دائمة.' },
  wheel:              { tr: 'Yuvarlak taşların kaymasından ilham alarak tekerleği geliştirdi.',              en: 'Inspired by rolling stones, developed the wheel.',                            de: 'Durch rollende Steine inspiriert, entwickelte er das Rad.',                 fr: 'Inspiré par des pierres qui roulent, développa la roue.',                    ar: 'مستلهمًا من الحجارة المتدحرجة، طور العجلة.' },
  irrigation:         { tr: 'Su kanalları açarak uzak tarım arazilerini suladı.',                            en: 'Dug channels to bring water to distant farmland.',                            de: 'Grub Kanäle, um Wasser auf entfernte Felder zu bringen.',                   fr: 'Creusa des canaux pour amener l\'eau aux champs éloignés.',                  ar: 'حفر قنوات لإيصال الماء إلى الأراضي الزراعية البعيدة.' },
  sailing:            { tr: 'Rüzgarın gücünü ahşap bir tekneye aktarmayı öğrendi.',                          en: 'Learned to harness wind power on a wooden vessel.',                           de: 'Lernte, Windkraft auf einem Holzschiff zu nutzen.',                         fr: 'Apprit à exploiter la force du vent sur un bateau en bois.',                 ar: 'تعلم تسخير قوة الرياح على سفينة خشبية.' },
  metallurgy_iron:    { tr: 'Çok yüksek ısıda demir cevherini işlemenin yolunu buldu.',                     en: 'Found a way to work iron ore at extreme temperatures.',                       de: 'Fand einen Weg, Eisenerz bei extremen Temperaturen zu verarbeiten.',        fr: 'Trouva le moyen de travailler le minerai de fer à des températures extrêmes.', ar: 'وجد طريقة لمعالجة خام الحديد في درجات حرارة قصوى.' },
};

const TECH_ICONS: Record<string, string> = {
  fire_making: '🔥', stone_tools: '🪨', foraging: '🌿',
  hunting_spear: '🏹', shelter_basic: '⛺', water_container: '🫙',
  animal_trap: '🕸', clothing_basic: '🧥', fishing: '🐟',
  plant_cultivation: '🌾', animal_herding: '🐂', food_preservation: '🥩',
  bow_arrow: '🏹', pottery: '🏺', weaving: '🧵',
  metallurgy_copper: '⚒', writing_system: '📜', calendar: '📅',
  mathematics_basic: '🔢', architecture_stone: '🏛', wheel: '⚙',
  irrigation: '💧', sailing: '⛵', metallurgy_iron: '⚔',
};

const TIER_COLORS = ['text-sim-muted', 'text-green-400', 'text-yellow-400', 'text-orange-400', 'text-red-400'];

function nameFromIndividual(ind: any): string {
  return ind?.phenotype?.name ?? ind?.name ?? (ind?.id ? `ID:${ind.id.slice(-5)}` : '—');
}

export default function TechnologyPanel() {
  const { stats, events, lang, currentSim, accessToken } = useSimStore();

  const totalTechs = stats?.technologies ?? 0;
  const techProgress = stats?.tech_progress ?? {};
  const discoveredList = events
    .filter(e => e.event_type === 'technology')
    .map(e => e.data?.tech_id ?? e.description?.replace('Technology discovered: ', '') ?? '');
  const discoveredSet = new Set(discoveredList);

  const techEvents = events
    .filter(e => e.event_type === 'technology')
    .sort((a, b) => (a.sim_day ?? 0) - (b.sim_day ?? 0));

  const [discoverers, setDiscoverers] = useState<Record<string, any>>({});

  useEffect(() => {
    if (!currentSim || !accessToken) return;
    const ids = [...new Set(techEvents.map(e => e.data?.discoverer_id).filter(Boolean) as string[])];
    for (const id of ids) {
      if (discoverers[id]) continue;
      axios.get(`/api/simulations/${currentSim.id}/population/${id}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      }).then(res => setDiscoverers(prev => ({ ...prev, [id]: res.data }))).catch(() => {});
    }
  }, [techEvents.length, currentSim?.id]);

  return (
    <DetailPanel panelId="technology" title="Technology" titleTr="Teknoloji">

      {/* ── Summary bar ── */}
      <div className="flex justify-between items-center bg-sim-surface rounded-lg p-3 mb-2">
        <span className="text-sim-muted">{text(lang as LangCode, { tr: 'Keşfedilen', en: 'Discovered', de: 'Entdeckt', fr: 'Découvert', ar: 'مكتشف' })}</span>
        <span className="text-sim-gold font-bold text-lg">{totalTechs} / {stats?.total_techs ?? 25}</span>
      </div>

      {/* ── Discovery log ── */}
      {techEvents.length > 0 && (
        <div className="mb-4">
          <div className="font-share-tech tracking-widest mb-2" style={{ fontSize: 11, color: '#6a8878', letterSpacing: '0.12em', borderBottom: '1px solid rgba(0,232,135,0.1)', paddingBottom: 2 }}>
            {text(lang as LangCode, { tr: 'KEŞİF GÜNLÜĞÜ', en: 'DISCOVERY LOG', de: 'ENTDECKUNGSPROTOKOLL', fr: 'JOURNAL DES DÉCOUVERTES', ar: 'سجل الاكتشافات' })}
          </div>
          <div className="space-y-2">
            {techEvents.map((ev, i) => {
              const techId = ev.data?.tech_id ?? ev.description?.replace('Technology discovered: ', '') ?? '';
              const discId = ev.data?.discoverer_id;
              const disc = discId ? discoverers[discId] : null;
              const discName = disc ? nameFromIndividual(disc) : null;
              const age = disc ? Math.floor(parseFloat(disc.age_years ?? 0)) : null;
              const iq = disc ? Math.round((disc.phenotype?.fluid_intelligence ?? 0) * 100) : null;
              const curiosity = disc ? Math.round((disc.phenotype?.curiosity ?? 0) * 100) : null;
              const story = HOW_STORIES[techId];
              const techName = TECH_NAMES[techId] ? text(lang as LangCode, TECH_NAMES[techId]) : translateTech(techId, lang as LangCode);
              return (
                <div key={i} style={{
                  background: 'rgba(4,4,18,0.7)',
                  border: '1px solid rgba(78,203,113,0.18)',
                  borderLeft: '3px solid #4ecb71',
                  padding: '8px 10px',
                }}>
                  <div className="flex items-start gap-2">
                    <span style={{ fontSize: 16, lineHeight: 1.1, flexShrink: 0 }}>{TECH_ICONS[techId] ?? '⚙'}</span>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-share-tech" style={{ fontSize: 12, color: '#4ecb71', letterSpacing: '0.05em' }}>
                          {techName}
                        </span>
                        <span className="font-share-tech flex-shrink-0" style={{ fontSize: 10, color: '#6a8878' }}>
                          Y{ev.sim_year}·G{ev.sim_day}
                        </span>
                      </div>

                      {/* Discoverer */}
                      {(discName || discId) && (
                        <div className="font-share-tech" style={{ fontSize: 11, color: '#a0b4ff', marginTop: 3, lineHeight: 1.3 }}>
                          {text(lang as LangCode, { tr: 'Keşfeden:', en: 'Discoverer:', de: 'Entdecker:', fr: 'Découvreur:', ar: 'المكتشف:' })} <span style={{ color: disc?.sex === 'female' ? '#ff8ab0' : '#6090ff' }}>{discName ?? `…`}</span>
                          {age !== null && <span style={{ color: '#6a8878' }}> · {age} {text(lang as LangCode, { tr: 'yaş', en: 'yr', de: 'J.', fr: 'ans', ar: 'سنة' })}</span>}
                          {iq !== null && <span style={{ color: '#6a8878' }}> · IQ {iq}% · {text(lang as LangCode, { tr: 'merak', en: 'curio', de: 'Neugier', fr: 'curiosité', ar: 'فضول' })} {curiosity}%</span>}
                        </div>
                      )}

                      {/* How story */}
                      {story && (
                        <div className="font-share-tech" style={{ fontSize: 11, color: '#8898c8', marginTop: 4, lineHeight: 1.4, fontStyle: 'italic' }}>
                          "{text(lang as LangCode, story)}"
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* ── Tier tree ── */}
      {TECH_TIERS.map(tier => (
        <div key={tier.tier} className="mb-3">
          <h4 className={`text-sm font-semibold uppercase tracking-widest mb-2 ${TIER_COLORS[tier.tier]}`}>
            {text(lang as LangCode, { en: `Tier ${tier.tier}`, tr: `Seviye ${tier.tier}`, de: `Stufe ${tier.tier}`, fr: `Niveau ${tier.tier}`, ar: `المستوى ${tier.tier}` })} — {text(lang as LangCode, tier.label)}
          </h4>
          <div className="space-y-1">
            {tier.techs.map(techId => {
              const isDiscovered = discoveredSet.has(techId);
              const progress = techProgress[techId] ?? 0;
              return (
                <div
                  key={techId}
                  className={`p-1.5 rounded ${isDiscovered ? 'bg-sim-accent/10' : 'bg-sim-surface/30'}`}
                >
                  <div className="flex items-center gap-2">
                    {isDiscovered
                      ? <Check size={10} className="text-sim-accent flex-shrink-0" />
                      : <Lock size={10} className="text-sim-muted flex-shrink-0" />
                    }
                    <span className={isDiscovered ? 'text-sim-text' : 'text-sim-muted'}>
                      {TECH_NAMES[techId] ? text(lang as LangCode, TECH_NAMES[techId]) : translateTech(techId, lang as LangCode)}
                    </span>
                    {!isDiscovered && progress > 0 && (
                      <span className="ml-auto text-sim-muted" style={{ fontSize: 10 }}>
                        {(progress * 100).toFixed(0)}%
                      </span>
                    )}
                  </div>
                  {!isDiscovered && progress > 0 && (
                    <div className="mt-1 h-0.5 bg-sim-border rounded-full overflow-hidden">
                      <div
                        className="h-full bg-sim-accent/60 rounded-full transition-all duration-300"
                        style={{ width: `${Math.min(100, progress * 100)}%` }}
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </DetailPanel>
  );
}
