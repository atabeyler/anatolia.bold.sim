import { useEffect, useMemo, useState, useRef } from 'react';
import axios from 'axios';
import { useSimStore } from '../../store/simStore';
import DetailPanel from './DetailPanel';
import { Users, MapPin, ChevronRight, X, ChevronDown } from 'lucide-react';
import { text, type LangCode, translateEventDescription, translateStageName, translateMentalState } from '../../utils/i18n';
import { HORMONE_GROUPS } from '../../utils/hormoneGroups';

const CAUSE_I18N: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  starvation:                  { tr: 'Açlık',                    en: 'Starvation',             de: 'Verhungern',              fr: 'Famine',                      ar: 'مجاعة'             },
  dehydration:                 { tr: 'Susuzluk',                 en: 'Dehydration',            de: 'Austrocknung',            fr: 'Déshydratation',              ar: 'جفاف'              },
  old_age:                     { tr: 'Yaşlılık',                 en: 'Old age',                de: 'Alter',                   fr: 'Vieillesse',                  ar: 'الشيخوخة'          },
  predator:                    { tr: 'Yırtıcı hayvan',           en: 'Predator',               de: 'Raubtier',                fr: 'Prédateur',                   ar: 'حيوان مفترس'       },
  genetic_disease:             { tr: 'Genetik hastalık',         en: 'Genetic disease',        de: 'Erbkrankheit',            fr: 'Maladie génétique',           ar: 'مرض وراثي'         },
  infection:                   { tr: 'Enfeksiyon',               en: 'Infection',              de: 'Infektion',               fr: 'Infection',                   ar: 'عدوى'              },
  exposure:                    { tr: 'Soğuk/sıcak çarpması',     en: 'Exposure',               de: 'Unterkühlung/Hitzschlag', fr: 'Exposition',                  ar: 'التعرض للعوامل الجوية' },
  wildlife_encounter:          { tr: 'Yaban hayvanı saldırısı',  en: 'Wildlife encounter',     de: 'Wildtierbegegnung',       fr: 'Rencontre animale',           ar: 'مواجهة حيوان بري'   },
  injury:                      { tr: 'Yaralanma',                en: 'Injury',                 de: 'Verletzung',              fr: 'Blessure',                    ar: 'إصابة'              },
  birth_complications:         { tr: 'Doğum komplikasyonu',      en: 'Birth complications',    de: 'Geburtskomplikation',     fr: 'Complications accouchement',  ar: 'مضاعفات الولادة'   },
  conflict:                    { tr: 'Çatışma',                  en: 'Conflict',               de: 'Konflikt',                fr: 'Conflit',                     ar: 'صراع'              },
  drowning:                    { tr: 'Boğulma',                  en: 'Drowning',               de: 'Ertrinken',               fr: 'Noyade',                      ar: 'غرق'               },
  meteor_tsunami:              { tr: 'Meteor çarpması ve tsunami', en: 'Meteor impact and tsunami', de: 'Meteoriteneinschlag und Tsunami', fr: 'Impact de météore et tsunami', ar: 'اصطدام نيزك وتسونامي' },
  unknown:                     { tr: 'Bilinmeyen',               en: 'Unknown',                de: 'Unbekannt',               fr: 'Inconnu',                     ar: 'مجهول'             },
  disease_intestinal_parasite: { tr: 'Bağırsak paraziti',        en: 'Intestinal parasite',    de: 'Darmparasit',             fr: 'Parasite intestinal',         ar: 'طفيليات معوية'     },
  disease_cholera_like:        { tr: 'Kolera benzeri hastalık',  en: 'Cholera-like disease',   de: 'Cholera-ähnl. Krkh.',     fr: 'Maladie cholériforme',        ar: 'مرض شبيه بالكوليرا' },
  disease_respiratory_common:  { tr: 'Solunum yolu hastalığı',   en: 'Respiratory illness',    de: 'Atemwegserkrankung',      fr: 'Maladie respiratoire',        ar: 'مرض تنفسي'         },
  disease_pneumonia_like:      { tr: 'Zatürre benzeri hastalık', en: 'Pneumonia-like illness', de: 'Pneumonie-ähnl. Krkh.',   fr: 'Maladie pneumonique',         ar: 'مرض شبيه بالالتهاب الرئوي' },
  disease_plague_like:         { tr: 'Veba benzeri salgın',      en: 'Plague-like epidemic',   de: 'Pestartige Seuche',       fr: 'Épidémie pestilentielle',     ar: 'وباء شبيه بالطاعون' },
  disease_malaria_like:        { tr: 'Sıtma benzeri hastalık',   en: 'Malaria-like disease',   de: 'Malaria-ähnl. Krkh.',     fr: 'Maladie palustre',            ar: 'مرض شبيه بالملاريا' },
  disease_fever_tick:          { tr: 'Kene ateşi',               en: 'Tick fever',             de: 'Zeckenfieber',            fr: 'Fièvre à tiques',             ar: 'حمى القراد'        },
  disease_wound_infection:     { tr: 'Yara enfeksiyonu',         en: 'Wound infection',        de: 'Wundinfektion',           fr: 'Infection de plaie',          ar: 'عدوى الجرح'        },
  disease_fungal_skin:         { tr: 'Mantar enfeksiyonu',       en: 'Fungal skin infection',  de: 'Pilzinfektion',           fr: 'Infection fongique cutanée',  ar: 'عدوى فطرية جلدية'  },
};

// Full de/fr/ar coverage for the short Turkish labels used throughout the
// individual detail / family tree / journal / compare modals below. Keyed by
// the Turkish string since that's the stable, unique key at every call site
// (English abbreviations vary per call site, e.g. "Imm." vs "Immunity").
const POP_PANEL_I18N: Record<string, { de: string; fr: string; ar: string }> = {
  'ARŞİV': { de: 'ARCHIV', fr: 'ARCHIVE', ar: 'الأرشيف' },
  'Akraba Evliliği': { de: 'Inzucht', fr: 'Consanguinité', ar: 'زواج الأقارب' },
  'Anne': { de: 'Mutter', fr: 'Mère', ar: 'الأم' },
  'Arşivi Temizle': { de: 'Archiv leeren', fr: 'Vider les archives', ar: 'مسح الأرشيف' },
  'Azami Ömür': { de: 'Max. Lebensdauer', fr: 'Longévité max.', ar: 'أقصى عمر' },
  'Aşama': { de: 'Stufe', fr: 'Étape', ar: 'مرحلة' },
  'Baba': { de: 'Vater', fr: 'Père', ar: 'الأب' },
  'Baskınlık': { de: 'Dominanz', fr: 'Dominance', ar: 'السيطرة' },
  'Bağımsızlık': { de: 'Unabhängigkeit', fr: 'Indépendance', ar: 'الاستقلالية' },
  'Bağışıklık': { de: 'Immunität', fr: 'Immunité', ar: 'المناعة' },
  'Bilinç': { de: 'Bewusstsein', fr: 'Conscience', ar: 'الوعي' },
  'Bilinç Pot.': { de: 'Bewusst.-Pot.', fr: 'Pot. conscience', ar: 'إمكانية الوعي' },
  'Boy': { de: 'Größe', fr: 'Taille', ar: 'الطول' },
  'BÜYÜKANNE / BÜYÜKBABA': { de: 'GROSSELTERN', fr: 'GRANDS-PARENTS', ar: 'الأجداد' },
  'BÜYÜKANNE/BABA': { de: 'GROSSELTERN', fr: 'GRANDS-PARENTS', ar: 'الأجداد' },
  'BİLİNÇ': { de: 'BEWUSSTSEIN', fr: 'CONSCIENCE', ar: 'الوعي' },
  'BİLİNÇ & RUH HALİ': { de: 'BEWUSSTSEIN & STIMMUNG', fr: 'CONSCIENCE & HUMEUR', ar: 'الوعي والمزاج' },
  'BİLİŞSEL': { de: 'KOGNITIV', fr: 'COGNITIF', ar: 'معرفي' },
  'BİREY': { de: 'INDIVIDUUM', fr: 'INDIVIDU', ar: 'الفرد' },
  'BİREY KARŞILAŞTIRMA': { de: 'INDIVIDUENVERGLEICH', fr: "COMPARAISON D'INDIVIDUS", ar: 'مقارنة الأفراد' },
  'Can': { de: 'LP', fr: 'PV', ar: 'الصحة' },
  'Dayanıklılık': { de: 'Ausdauer', fr: 'Endurance', ar: 'التحمل' },
  'Dil Kap.': { de: 'Sprachf.', fr: 'Capac.lang.', ar: 'قدرة لغوية' },
  'Dil Kapasitesi': { de: 'Sprachfähigkeit', fr: 'Capacité linguistique', ar: 'القدرة اللغوية' },
  'Dindarlık': { de: 'Frömmigkeit', fr: 'Religiosité', ar: 'التدين' },
  'Doğurganlık': { de: 'Fruchtbarkeit', fr: 'Fécondité', ar: 'الخصوبة' },
  'DİL': { de: 'SPRACHE', fr: 'LANGUE', ar: 'اللغة' },
  'EBEVEYNLER': { de: 'ELTERN', fr: 'PARENTS', ar: 'الوالدان' },
  'Ebeveyn': { de: 'Elternteil', fr: 'Parent', ar: 'أحد الوالدين' },
  'Ebeveyn Bakımı': { de: 'Elternfürsorge', fr: 'Soins parentaux', ar: 'الرعاية الوالدية' },
  'Emin misin?': { de: 'Bist du sicher?', fr: 'Êtes-vous sûr ?', ar: 'هل أنت متأكد؟' },
  'Empati': { de: 'Empathie', fr: 'Empathie', ar: 'التعاطف' },
  'Erkek': { de: 'Männlich', fr: 'Homme', ar: 'ذكر' },
  'Fiziksel Güç': { de: 'Körperkraft', fr: 'Force physique', ar: 'القوة الجسدية' },
  'FİZİKSEL': { de: 'KÖRPERLICH', fr: 'PHYSIQUE', ar: 'جسدي' },
  'Grup': { de: 'Gruppe', fr: 'Groupe', ar: 'مجموعة' },
  'GÖRÜNÜM': { de: 'ERSCHEINUNG', fr: 'APPARENCE', ar: 'المظهر' },
  'Göz Rengi': { de: 'Augenfarbe', fr: 'Couleur des yeux', ar: 'لون العين' },
  'Güç': { de: 'Kraft', fr: 'Force', ar: 'القوة' },
  'HAYAT HİKÂYESİ': { de: 'LEBENSGESCHICHTE', fr: 'HISTOIRE DE VIE', ar: 'قصة الحياة' },
  'HAYAT HİKÂYESİ ARŞİVİ': { de: 'LEBENSGESCHICHTE-ARCHIV', fr: 'ARCHIVES DE VIE', ar: 'أرشيف قصة الحياة' },
  'HAYATINI KAYBETTİ': { de: 'VERSTORBEN', fr: 'DÉCÉDÉ', ar: 'متوفى' },
  'Hamilelik Günü': { de: 'Schwangerschaftstag', fr: 'Jour de grossesse', ar: 'يوم الحمل' },
  'Hastalık Direnci': { de: 'Krankheitsresist.', fr: 'Résist. maladies', ar: 'مقاومة الأمراض' },
  'Henüz arşivlenmiş olay yok': { de: 'Noch keine archivierten Ereignisse', fr: 'Aucun événement archivé', ar: 'لا توجد أحداث مؤرشفة بعد' },
  'Henüz çocuk yok.': { de: 'Noch keine Kinder.', fr: "Pas encore d'enfants.", ar: 'لا يوجد أطفال بعد.' },
  'Henüz çocuğu yok': { de: 'Noch keine Kinder', fr: "Pas encore d'enfants", ar: 'لا يوجد أطفال بعد' },
  'KURUCU': { de: 'GRÜNDER', fr: 'FONDATEUR', ar: 'مؤسس' },
  'KARDEŞLER': { de: 'GESCHWISTER', fr: 'FRÈRES ET SŒURS', ar: 'الإخوة' },
  'Kadın': { de: 'Weiblich', fr: 'Femme', ar: 'أنثى' },
  'Kalori': { de: 'Kalorien', fr: 'Calories', ar: 'السعرات' },
  'Kaygı': { de: 'Angst', fr: 'Anxiété', ar: 'القلق' },
  'Kelime': { de: 'Wörter', fr: 'Mots', ar: 'كلمات' },
  'Kelime Sayısı': { de: 'Wortschatz', fr: 'Vocabulaire', ar: 'عدد الكلمات' },
  'Kilo': { de: 'Gewicht', fr: 'Poids', ar: 'الوزن' },
  'Kurucu': { de: 'Gründer', fr: 'Fondateur', ar: 'مؤسس' },
  'Kurucu Birey — Medeniyetin Atası': { de: 'Gründungsindividuum — Vorfahre der Zivilisation', fr: 'Individu fondateur — Ancêtre de la civilisation', ar: 'فرد مؤسس — سلف الحضارة' },
  'Kurucu — bilinen atası yok': { de: 'Gründer — keine bekannten Vorfahren', fr: "Fondateur — pas d'ancêtres connus", ar: 'مؤسس — لا يوجد أسلاف معروفون' },
  'KİŞİLİK': { de: 'PERSÖNLICHKEIT', fr: 'PERSONNALITÉ', ar: 'الشخصية' },
  'Merak': { de: 'Neugier', fr: 'Curiosité', ar: 'الفضول' },
  'Metabolizma': { de: 'Stoffwechsel', fr: 'Métabolisme', ar: 'الأيض' },
  'Potansiyel': { de: 'Potenzial', fr: 'Potentiel', ar: 'الإمكانية' },
  'Risk Toleransı': { de: 'Risikotoler.', fr: 'Tolér. risque', ar: 'تحمل المخاطر' },
  'SAĞLIK': { de: 'GESUNDHEIT', fr: 'SANTÉ', ar: 'الصحة' },
  'SOSYAL': { de: 'SOZIAL', fr: 'SOCIAL', ar: 'اجتماعي' },
  'SOY AĞACI': { de: 'STAMMBAUM', fr: 'ARBRE GÉNÉALOGIQUE', ar: 'شجرة العائلة' },
  'SOYAĞACI': { de: 'STAMMBAUM', fr: 'ARBRE GÉNÉALOGIQUE', ar: 'شجرة العائلة' },
  'Saldırganlık': { de: 'Aggression', fr: 'Agressivité', ar: 'العدوانية' },
  'Sanatsal Duygu': { de: 'Kunstsinn', fr: 'Sens artistique', ar: 'الحس الفني' },
  'Saç Rengi': { de: 'Haarfarbe', fr: 'Couleur des cheveux', ar: 'لون الشعر' },
  'Serotonin': { de: 'Serotonin', fr: 'Sérotonine', ar: 'السيروتونين' },
  'Stres': { de: 'Stress', fr: 'Stress', ar: 'الإجهاد' },
  'Stres Direnci': { de: 'Stressresist.', fr: 'Résist. stress', ar: 'مقاومة الإجهاد' },
  'Su': { de: 'Wasser', fr: 'Eau', ar: 'الماء' },
  'TORUNLAR': { de: 'ENKEL', fr: 'PETITS-ENFANTS', ar: 'الأحفاد' },
  'Temizle': { de: 'Löschen', fr: 'Effacer', ar: 'مسح' },
  'Ten Tonu': { de: 'Hautton', fr: 'Teint', ar: 'لون البشرة' },
  'Yalnız': { de: 'Allein', fr: 'Seul', ar: 'وحيد' },
  'Yaralanma': { de: 'Verletzung', fr: 'Blessure', ar: 'إصابة' },
  'Zekâ': { de: 'Intelligenz', fr: 'Intelligence', ar: 'الذكاء' },
  'anne tarafı': { de: 'mütterlicherseits', fr: 'maternel', ar: 'من جهة الأم' },
  'baba tarafı': { de: 'väterlicherseits', fr: 'paternel', ar: 'من جهة الأب' },
  'daha': { de: 'mehr', fr: 'de plus', ar: 'أكثر' },
  'ebeveyn': { de: 'Elternteil(e)', fr: 'parent(s)', ar: 'أحد الوالدين' },
  'kelime': { de: 'Wörter', fr: 'mots', ar: 'كلمات' },
  'olay': { de: 'Ereignisse', fr: 'événements', ar: 'أحداث' },
  'torun': { de: 'Enkel', fr: 'petits-enfants', ar: 'أحفاد' },
  'yaş': { de: 'J.', fr: 'an', ar: 'سنة' },
  'yıl': { de: 'J.', fr: 'an', ar: 'سنة' },
  'ÇOCUKLAR': { de: 'KINDER', fr: 'ENFANTS', ar: 'الأطفال' },
  'Çalışma Belleği': { de: 'Arbeitsged.', fr: 'Mém. travail', ar: 'الذاكرة العاملة' },
  'Çift': { de: 'Verpaart', fr: 'En couple', ar: 'مرتبط' },
  'Çocuk': { de: 'Kind', fr: 'Enfant', ar: 'طفل' },
  'Öz Disiplin': { de: 'Gewissenhaft.', fr: 'Consciencieux', ar: 'الانضباط الذاتي' },
  'Öz Farkındalık': { de: 'Selbstwahrn.', fr: 'Conscience de soi', ar: 'الوعي الذاتي' },
  'Özgecilik': { de: 'Altruismus', fr: 'Altruisme', ar: 'الإيثار' },
  'Öğrenme': { de: 'Lernen', fr: 'Apprentissage', ar: 'التعلم' },
  'Öğrenme Hızı': { de: 'Lernrate', fr: "Taux d'appr.", ar: 'معدل التعلم' },
  'çocuk': { de: 'Kinder', fr: 'enfants', ar: 'أطفال' },
  'ölü': { de: 'verst.', fr: 'déc.', ar: 'متوفى' },
  'İnanç Kapasitesi': { de: 'Glaubenskap.', fr: 'Capac. croyance', ar: 'القدرة على الإيمان' },
  'İnovasyon': { de: 'Innovation', fr: 'Innovation', ar: 'الابتكار' },
  'İptal': { de: 'Abbrechen', fr: 'Annuler', ar: 'إلغاء' },
  'İşbirliği': { de: 'Kooperation', fr: 'Coopération', ar: 'التعاون' },
};

function makeTr(lang: LangCode) {
  return (trStr: string, enStr: string) => {
    if (lang === 'tr') return trStr;
    if (lang === 'en') return enStr;
    const entry = POP_PANEL_I18N[trStr];
    return entry ? entry[lang] : enStr;
  };
}

function causeLabel(cause: string | null | undefined, lang: string): string {
  if (!cause) return text(lang as LangCode, { tr: 'Bilinmeyen', en: 'Unknown', de: 'Unbekannt', fr: 'Inconnu', ar: 'مجهول' });
  const entry = CAUSE_I18N[cause];
  if (entry) return text(lang as LangCode, entry);
  return cause.replace(/_/g, ' ');
}

// Cardinal rule: an individual with no phenotype.name yet (their own lived
// language hasn't reached the point of originating one -- see sim-core's
// naming.rs) must not be given one here either. This used to hash the id
// into a fixed 60-name pool and return one unconditionally, which meant
// every single individual displayed as named regardless of what the
// simulation actually produced. Falls back to a bare, anonymous id tag,
// same convention used elsewhere in the app for an unnamed individual.
function nameFromId(id: string, sex: string, storedName?: string | null): string {
  if (storedName) return storedName;
  return `${sex === 'male' ? '♂' : '♀'}-${id.slice(-4).toUpperCase()}`;
}

function lifeStage(age: number, lang: string): { label: string; color: string } {
  if (age < 2)  return { label: text(lang as LangCode, { en: 'Infant',  tr: 'Bebek'    }), color: '#00d4ff' };
  if (age < 12) return { label: text(lang as LangCode, { en: 'Child',   tr: 'Çocuk'    }), color: '#4ecb71' };
  if (age < 18) return { label: text(lang as LangCode, { en: 'Youth',   tr: 'Genç'     }), color: '#a0b4ff' };
  if (age < 45) return { label: text(lang as LangCode, { en: 'Adult',   tr: 'Yetişkin' }), color: '#d4a838' };
  return            { label: text(lang as LangCode, { en: 'Elder',   tr: 'Yaşlı'    }), color: '#e05a5a' };
}

function isAlive(obj: any) { return obj && obj.alive !== false && !obj.is_dead; }

function PersonRow({ obj, fallbackId, tag, lang }: { obj?: any; fallbackId?: string; tag?: string; lang: string }) {
  if (!obj && !fallbackId) return null;
  const alive = isAlive(obj);
  const isMale = obj?.sex === 'male';
  const nameColor = alive ? (isMale ? '#8ab0ff' : '#ffaac8') : '#7a4a4a';
  const dotColor  = alive ? (isMale ? '#6090ff' : '#ff8ab0') : '#a05050';
  const displayName = obj
    ? nameFromId(obj.id, obj.sex, obj.phenotype?.name ?? obj.name)
    : `ID:${fallbackId?.slice(-6)}`;
  return (
    <div className="flex items-center gap-1.5 mb-0.5" style={{ paddingLeft: 10 }}>
      <div className="w-1.5 h-1.5 rounded-full flex-shrink-0" style={{ background: dotColor }} />
      <span className="font-share-tech" style={{ fontSize: 12, color: nameColor }}>{displayName}</span>
      {tag && <span className="font-share-tech" style={{ fontSize: 12, color: '#8abda0', marginLeft: 2 }}>{tag}</span>}
      {obj && (
        alive
          ? <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>{parseFloat(obj.age_years ?? 0).toFixed(0)}{text(lang as LangCode, { en: ' yr', tr: ' yaş', de: ' yr', fr: ' yr', ar: ' yr' })}</span>
          : <span className="font-share-tech" style={{ fontSize: 12, color: '#a05050' }}>
              † {obj.death_cause ? causeLabel(obj.death_cause, lang) : text(lang as LangCode, { en: 'dec.', tr: 'ölü', de: 'dec.', fr: 'dec.', ar: 'dec.' })}
            </span>
      )}
      {!obj && <span className="font-share-tech" style={{ fontSize: 12, color: '#a05050' }}>† {text(lang as LangCode, { en: 'dec.', tr: 'ölü', de: 'dec.', fr: 'dec.', ar: 'dec.' })}</span>}
    </div>
  );
}

function FamilySection({ label, indent, children }: { label: string; indent: number; children: React.ReactNode }) {
  return (
    <div className="mb-2" style={{ marginLeft: indent * 8 }}>
      <div style={{ fontSize: 12, color: '#8abda0', letterSpacing: '0.08em', marginBottom: 3, borderLeft: '1px solid rgba(160,200,176,0.3)', paddingLeft: 4 }}>
        {label}
      </div>
      {children}
    </div>
  );
}

const EYE_COLORS: Record<string, { dot: string; labelTr: string; labelEn: string; labelDe: string; labelFr: string; labelAr: string }> = {
  brown:  { dot: '#7a4a1e', labelTr: 'Kahverengi', labelEn: 'Brown', labelDe: 'Braun',      labelFr: 'Marron',   labelAr: 'بني'    },
  blue:   { dot: '#3a8ad4', labelTr: 'Mavi',        labelEn: 'Blue',  labelDe: 'Blau',       labelFr: 'Bleu',     labelAr: 'أزرق'   },
  green:  { dot: '#3a9a4a', labelTr: 'Yeşil',       labelEn: 'Green', labelDe: 'Grün',       labelFr: 'Vert',     labelAr: 'أخضر'   },
  hazel:  { dot: '#8a6a2e', labelTr: 'Ela',         labelEn: 'Hazel', labelDe: 'Haselnuss',  labelFr: 'Noisette', labelAr: 'بندقي'  },
};
const HAIR_COLORS: Record<string, { dot: string; labelTr: string; labelEn: string; labelDe: string; labelFr: string; labelAr: string }> = {
  dark:   { dot: '#2a1a08', labelTr: 'Koyu', labelEn: 'Dark',   labelDe: 'Dunkel', labelFr: 'Foncé', labelAr: 'داكن'   },
  medium: { dot: '#6a3a18', labelTr: 'Orta', labelEn: 'Medium', labelDe: 'Mittel', labelFr: 'Moyen', labelAr: 'متوسط'  },
  light:  { dot: '#c8a060', labelTr: 'Açık', labelEn: 'Light',  labelDe: 'Hell',   labelFr: 'Clair', labelAr: 'فاتح'   },
};
const ROLE_LABELS: Record<string, { tr: string; en: string; de: string; fr: string; ar: string }> = {
  leader:   { tr: 'Lider',          en: 'Leader',   de: 'Anführer',  fr: 'Chef',       ar: 'قائد'   },
  elder:    { tr: 'Yaşlı',          en: 'Elder',    de: 'Ältester',  fr: 'Aîné',       ar: 'كبير'   },
  warrior:  { tr: 'Savaşçı',        en: 'Warrior',  de: 'Krieger',   fr: 'Guerrier',   ar: 'محارب'  },
  gatherer: { tr: 'Toplayıcı',      en: 'Gatherer', de: 'Sammler',   fr: 'Cueilleur',  ar: 'جامع'   },
  healer:   { tr: 'İyileştirici',   en: 'Healer',   de: 'Heiler',    fr: 'Guérisseur', ar: 'معالج'  },
  member:   { tr: 'Üye',            en: 'Member',   de: 'Mitglied',  fr: 'Membre',     ar: 'عضو'    },
};

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="font-share-tech tracking-widest mb-2" style={{ fontSize: 12, color: '#6a8878', letterSpacing: '0.12em', borderBottom: '1px solid rgba(0,232,135,0.1)', paddingBottom: 2 }}>
      {label}
    </div>
  );
}

function TraitRow({ label, value, color, max = 1 }: { label: string; value: number; color: string; max?: number }) {
  const pct = Math.min(100, Math.round((value / max) * 100));
  return (
    <div>
      <div className="flex justify-between mb-0.5">
        <span className="font-share-tech" style={{ fontSize: 12, color: '#8898c8' }}>{label}</span>
        <span className="font-share-tech" style={{ fontSize: 12, color }}>{pct}%</span>
      </div>
      <div style={{ height: 3, background: 'rgba(79,110,247,0.1)', borderRadius: 2, overflow: 'hidden' }}>
        <div style={{ width: `${pct}%`, height: '100%', background: color, borderRadius: 2 }} />
      </div>
    </div>
  );
}

function StatRow({ label, value, color = '#a0b4ff' }: { label: string; value: React.ReactNode; color?: string }) {
  return (
    <div className="flex justify-between">
      <span className="font-share-tech" style={{ fontSize: 12, color: '#8898c8' }}>{label}</span>
      <span className="font-share-tech" style={{ fontSize: 12, color }}>{value}</span>
    </div>
  );
}


function IndividualDetail({ ind, allIndividuals, onClose }: { ind: any; allIndividuals: any[]; onClose: () => void }) {
  const { lang, events, watchedIndividualId, setWatchedIndividual } = useSimStore();
  const name = nameFromId(ind.id, ind.sex, ind.phenotype?.name ?? ind.name);
  const age = parseFloat(ind.age_years ?? 0);
  const stage = lifeStage(age, lang);
  const ph = ind.phenotype ?? {};
  const soc = ind.social ?? {};
  const ps = ind.psychology ?? {};
  const health = ind.health ?? {};
  const mind = ind.mind ?? {};
  const lang_ = ind.language ?? {};
  const horm = ind.hormones ?? {};
  const isDead = ind.alive === false || ind.is_dead;
  const isFounder = ind.is_founder || (!ind.parent_1_id && !ind.parent_2_id);
  const tr = makeTr(lang as LangCode);

  const [archivedJournal, setArchivedJournal] = useState<any[]>([]);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [treeOpen, setTreeOpen] = useState(false);
  const [voiceOpen, setVoiceOpen] = useState(false);
  const [hormonesExpanded, setHormonesExpanded] = useState(false);

  const TYPE_ICON: Record<string, string> = {
    birth: '✦', death: '†', language: '◆', technology: '⚙',
    communication: '◈', disaster: '⚠', belief: '☽', art: '🎨',
  };

  const liveJournalEvents = events.filter(ev => {
    const d = ev.data ?? {};
    return d.individual_id === ind.id || d.discoverer_id === ind.id
        || d.sender_id === ind.id || d.receiver_id === ind.id;
  }).slice(0, 25);

  // Load archive from localStorage on mount
  useEffect(() => {
    try {
      const raw = localStorage.getItem(`journal_${ind.id}`);
      if (raw) setArchivedJournal(JSON.parse(raw));
    } catch {}
  }, [ind.id]);

  // Auto-save whenever live events change
  useEffect(() => {
    if (!liveJournalEvents.length) return;
    try {
      const stored = localStorage.getItem(`journal_${ind.id}`);
      const existing: any[] = stored ? JSON.parse(stored) : [];
      const seen = new Set<string>();
      const merged: any[] = [];
      for (const ev of [...existing, ...liveJournalEvents]) {
        const sig = `${ev.sim_day}_${ev.event_type}_${ev.description}`;
        if (!seen.has(sig)) { seen.add(sig); merged.push(ev); }
      }
      merged.sort((a, b) => a.sim_day - b.sim_day);
      localStorage.setItem(`journal_${ind.id}`, JSON.stringify(merged));
      setArchivedJournal(merged);
    } catch {}
  }, [liveJournalEvents.length, ind.id]);

  const parent1 = allIndividuals.find(i => i.id === ind.parent_1_id);
  const parent2 = allIndividuals.find(i => i.id === ind.parent_2_id);
  const gp_p1a = parent1 ? allIndividuals.find(i => i.id === parent1.parent_1_id) : null;
  const gp_p1b = parent1 ? allIndividuals.find(i => i.id === parent1.parent_2_id) : null;
  const gp_p2a = parent2 ? allIndividuals.find(i => i.id === parent2.parent_1_id) : null;
  const gp_p2b = parent2 ? allIndividuals.find(i => i.id === parent2.parent_2_id) : null;
  const grandparents = [
    gp_p1a ? { obj: gp_p1a, side: tr('baba tarafı', 'paternal') } : null,
    gp_p1b ? { obj: gp_p1b, side: tr('baba tarafı', 'paternal') } : null,
    gp_p2a ? { obj: gp_p2a, side: tr('anne tarafı', 'maternal') } : null,
    gp_p2b ? { obj: gp_p2b, side: tr('anne tarafı', 'maternal') } : null,
  ].filter(Boolean) as { obj: any; side: string }[];

  const children = allIndividuals.filter(i => i.parent_1_id === ind.id || i.parent_2_id === ind.id);
  const grandchildren = children.flatMap(c =>
    allIndividuals.filter(i => i.parent_1_id === c.id || i.parent_2_id === c.id)
      .map(gc => ({ obj: gc, parentName: nameFromId(c.id, c.sex, c.phenotype?.name ?? c.name) }))
  );
  const siblings = allIndividuals.filter(i =>
    i.id !== ind.id &&
    ((ind.parent_1_id && i.parent_1_id === ind.parent_1_id) ||
     (ind.parent_2_id && i.parent_2_id === ind.parent_2_id))
  );

  const eyeInfo   = EYE_COLORS[ph.eye_color ?? 'brown']   ?? EYE_COLORS.brown;
  const hairInfo  = HAIR_COLORS[ph.hair_color ?? 'dark']   ?? HAIR_COLORS.dark;
  const skinPct   = Math.round((ph.skin_tone ?? 0.5) * 100);
  const wordCount = Object.keys(lang_.vocabulary ?? {}).length;
  const inbreedingPct = Math.round((ind.inbreeding_coeff ?? 0) * 100);

  // Bond strengths this individual has actually accumulated with specific
  // others (psychology::process_bonding, rust/sim-core) -- tracked in the DB
  // since the Rust engine writes it, but never previously surfaced anywhere
  // in the client.
  const topRelationships = (Object.entries(ps.relationships ?? {}) as [string, number][])
    .sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]))
    .slice(0, 5)
    .map(([otherId, bond]) => {
      const other = allIndividuals.find(i => i.id === otherId);
      const otherName = other ? nameFromId(other.id, other.sex, other.phenotype?.name ?? other.name) : tr('Bilinmeyen', 'Unknown');
      return { otherId, otherName, bond };
    });

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: 'rgba(0,0,0,0.6)', backdropFilter: 'blur(4px)' }}>
      <div className="relative flex flex-col" style={{ width: 420, maxHeight: '88vh', background: 'rgba(4,4,18,0.98)', border: '1px solid rgba(79,110,247,0.4)', backdropFilter: 'blur(20px)', boxShadow: '0 16px 60px rgba(0,0,0,0.8)' }}>

        {/* Header */}
        <div className="flex items-center gap-2 px-4 py-3 flex-shrink-0" style={{ borderBottom: '1px solid rgba(79,110,247,0.2)' }}>
          <div className="flex flex-col flex-1">
            <div className="flex items-center gap-2">
              <span className="font-orbitron font-bold tracking-wider" style={{ color: isDead ? '#a05050' : (ind.sex === 'male' ? '#6090ff' : '#ff8ab0'), fontSize: 14 }}>{name}</span>
              {isFounder && <span className="font-share-tech" style={{ fontSize: 12, color: '#d4a838', border: '1px solid rgba(212,168,56,0.4)', padding: '1px 5px' }}>★ {tr('KURUCU', 'FOUNDER')}</span>}
              {isDead && <span className="font-share-tech" style={{ fontSize: 12, color: '#a05050' }}>† {tr('HAYATINI KAYBETTİ', 'DECEASED')}</span>}
            </div>
            <div className="flex items-center flex-wrap gap-1.5 mt-0.5">
              {!isDead && <><span className="font-share-tech" style={{ fontSize: 12, color: stage.color }}>{tr(stage.label, stage.label)}</span><span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span></>}
              <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>{age.toFixed(1)} {tr('yaş', 'yr')}</span>
              <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span>
              <span className="font-share-tech" style={{ fontSize: 12, color: ind.sex === 'male' ? '#6090ff' : '#ff8ab0' }}>{ind.sex === 'male' ? tr('Erkek', 'Male') : tr('Kadın', 'Female')}</span>
              {ind.group_role && <><span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span><span className="font-share-tech" style={{ fontSize: 12, color: '#d4a838' }}>{ind.group_role && ROLE_LABELS[ind.group_role] ? text(lang as LangCode, ROLE_LABELS[ind.group_role]) : ind.group_role}</span></>}
            </div>
          </div>
          <div className="flex items-center gap-2 flex-shrink-0">
            <button
              onClick={() => setVoiceOpen(true)}
              title={text(lang as LangCode, { en: 'Inner voice', tr: 'İç ses', de: 'Innere Stimme', fr: 'Voix intérieure', ar: 'الصوت الداخلي' })}
              style={{
                background: 'transparent',
                border: '1px solid rgba(200,180,255,0.3)',
                color: '#c8b4ff', cursor: 'pointer', padding: '2px 6px',
                fontSize: 12, lineHeight: 1, fontFamily: 'Share Tech Mono, monospace', borderRadius: 2,
              }}>
              {text(lang as LangCode, { en: '💭 VOICE', tr: '💭 İÇ SES', de: '💭 STIMME', fr: '💭 VOIX', ar: '💭 الصوت' })}
            </button>
            <button
              onClick={() => setTreeOpen(true)}
              title={text(lang as LangCode, { en: 'View family tree', tr: 'Soy ağacını görüntüle', de: 'Stammbaum anzeigen', fr: 'Voir l\'arbre généalogique', ar: 'عرض شجرة العائلة' })}
              style={{
                background: 'transparent',
                border: '1px solid rgba(160,200,176,0.3)',
                color: '#a0c8b0', cursor: 'pointer', padding: '2px 6px',
                fontSize: 12, lineHeight: 1, fontFamily: 'Share Tech Mono, monospace', borderRadius: 2,
              }}>
              {text(lang as LangCode, { en: '🌿 TREE', tr: '🌿 SOY', de: '🌿 BAUM', fr: '🌿 ARBRE', ar: '🌿 الشجرة' })}
            </button>
            <button
              onClick={() => { setWatchedIndividual(watchedIndividualId === ind.id ? null : ind.id); onClose(); }}
              title={watchedIndividualId === ind.id ? text(lang as LangCode, { en: 'Stop watching', tr: 'Takibi bırak', de: 'Beobachtung stoppen', fr: 'Arrêter le suivi', ar: 'إيقاف المتابعة' }) : text(lang as LangCode, { en: 'Watch in witness mode', tr: 'Tanık modunda takip et', de: 'Im Zeugmodus verfolgen', fr: 'Suivre en mode témoin', ar: 'المتابعة في وضع الشاهد' })}
              style={{
                background: watchedIndividualId === ind.id ? 'rgba(0,212,255,0.15)' : 'transparent',
                border: `1px solid ${watchedIndividualId === ind.id ? 'rgba(0,212,255,0.6)' : 'rgba(160,200,176,0.3)'}`,
                color: watchedIndividualId === ind.id ? '#00d4ff' : '#a0c8b0',
                cursor: 'pointer', padding: '2px 6px', fontSize: 12, lineHeight: 1,
                fontFamily: 'Share Tech Mono, monospace', borderRadius: 2,
              }}>
              {watchedIndividualId === ind.id ? text(lang as LangCode, { en: '👁 WATCHING', tr: '👁 TAKİPTE', de: '👁 VERFOLGT', fr: '👁 EN SURVEILLANCE', ar: '👁 قيد المتابعة' }) : text(lang as LangCode, { en: 'WATCH', tr: 'TAKİP ET', de: 'VERFOLGEN', fr: 'SUIVRE', ar: 'تابع' })}
            </button>
            <button onClick={onClose} className="text-sim-muted hover:text-sim-accent transition-colors"><X size={14} /></button>
          </div>
        </div>

        {/* -- Scrollable content -- */}
        <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">

          {/* -- Görünüm / Appearance -- */}
          <div>
            <SectionHeader label={tr('GÖRÜNÜM', 'APPEARANCE')} />
            <div className="space-y-2">
              <div className="flex justify-between items-center">
                <span className="font-share-tech" style={{ fontSize: 12, color: '#8898c8' }}>{tr('Göz Rengi', 'Eye Color')}</span>
                <div className="flex items-center gap-1.5">
                  <div style={{ width: 12, height: 12, borderRadius: '50%', background: eyeInfo.dot, border: '1px solid rgba(255,255,255,0.2)' }} />
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#c8d8e8' }}>{text(lang as LangCode, { tr: eyeInfo.labelTr, en: eyeInfo.labelEn, de: eyeInfo.labelDe, fr: eyeInfo.labelFr, ar: eyeInfo.labelAr })}</span>
                </div>
              </div>
              <div className="flex justify-between items-center">
                <span className="font-share-tech" style={{ fontSize: 12, color: '#8898c8' }}>{tr('Saç Rengi', 'Hair Color')}</span>
                <div className="flex items-center gap-1.5">
                  <div style={{ width: 12, height: 12, borderRadius: '50%', background: hairInfo.dot, border: '1px solid rgba(255,255,255,0.2)' }} />
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#c8d8e8' }}>{text(lang as LangCode, { tr: hairInfo.labelTr, en: hairInfo.labelEn, de: hairInfo.labelDe, fr: hairInfo.labelFr, ar: hairInfo.labelAr })}</span>
                </div>
              </div>
              <div>
                <div className="flex justify-between mb-0.5">
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#8898c8' }}>{tr('Ten Tonu', 'Skin Tone')}</span>
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#c8a880' }}>{skinPct}%</span>
                </div>
                <div style={{ height: 6, borderRadius: 3, overflow: 'hidden', background: 'linear-gradient(to right, #f5deb3, #8b4513)' }}>
                  <div style={{ width: `${skinPct}%`, height: '100%', background: 'transparent', borderRight: '2px solid rgba(255,255,255,0.8)' }} />
                </div>
              </div>
            </div>
          </div>

          {/* -- Fiziksel / Physical -- */}
          <div>
            <SectionHeader label={tr('FİZİKSEL', 'PHYSICAL')} />
            <div className="space-y-1.5">
              <StatRow label={tr('Boy', 'Height')} value={`${ind.height_cm ?? '—'} cm`} />
              <StatRow label={tr('Kilo', 'Weight')} value={`${ind.weight_kg ?? '—'} kg`} />
              <TraitRow label={tr('Fiziksel Güç', 'Physical Strength')} value={ph.physical_strength ?? 0} color="#e05a5a" />
              <TraitRow label={tr('Dayanıklılık', 'Endurance')} value={ph.physical_endurance ?? ph.endurance ?? 0} color="#f97316" />
              <TraitRow label={tr('Metabolizma', 'Metabolism')} value={ph.metabolism ?? 0} color="#d4a838" />
              <TraitRow label={tr('Doğurganlık', 'Fertility')} value={ph.fertility ?? 0} color="#ff8ab0" />
              <TraitRow label={tr('Bağışıklık', 'Immunity')} value={ph.immune_strength ?? 0} color="#4f6ef7" />
              <StatRow label={tr('Azami Ömür', 'Max Lifespan')} value={`~${Math.round(ph.max_lifespan ?? 90)} ${tr('yıl','yr')}`} color="#a0b4ff" />
            </div>
          </div>

          {/* -- Bilişsel / Cognitive -- */}
          <div>
            <SectionHeader label={tr('BİLİŞSEL', 'COGNITIVE')} />
            <div className="space-y-1.5">
              <TraitRow label={tr('Zekâ', 'Intelligence')}          value={ph.fluid_intelligence ?? 0}    color="#d4a838" />
              <TraitRow label={tr('Çalışma Belleği', 'Working Mem.')} value={ph.working_memory ?? 0}     color="#e8c840" />
              <TraitRow label={tr('Öğrenme Hızı', 'Learning Rate')} value={ph.learning_rate ?? 0}         color="#4ecb71" />
              <TraitRow label={tr('Dil Kapasitesi', 'Lang. Capacity')} value={ph.language_capacity ?? 0} color="#00d4ff" />
              <TraitRow label={tr('Merak', 'Curiosity')}             value={ph.curiosity ?? 0}             color="#4ecb71" />
              <TraitRow label={tr('İnovasyon', 'Innovation')}        value={ph.innovation ?? 0}            color="#7dd3fc" />
              <TraitRow label={tr('Öz Disiplin', 'Conscientiousness')} value={ph.conscientiousness ?? 0} color="#a0b4ff" />
            </div>
          </div>

          {/* -- Kişilik / Personality -- */}
          <div>
            <SectionHeader label={tr('KİŞİLİK', 'PERSONALITY')} />
            <div className="space-y-1.5">
              <TraitRow label={tr('Empati', 'Empathy')}            value={ph.empathy ?? 0}          color="#00d4ff" />
              <TraitRow label={tr('İşbirliği', 'Cooperation')}     value={ph.cooperation ?? 0}      color="#4ecb71" />
              <TraitRow label={tr('Özgecilik', 'Altruism')}        value={ph.altruism ?? 0}         color="#7dd3fc" />
              <TraitRow label={tr('Ebeveyn Bakımı', 'Parental Care')} value={ph.parental_care ?? 0} color="#ff8ab0" />
              <TraitRow label={tr('Saldırganlık', 'Aggression')}   value={ph.aggression ?? 0}       color="#f97316" />
              <TraitRow label={tr('Baskınlık', 'Dominance')}       value={ph.dominance ?? 0}        color="#e05a5a" />
              <TraitRow label={tr('Risk Toleransı', 'Risk Toler.')} value={ph.risk_tolerance ?? 0} color="#d4a838" />
              <TraitRow label={tr('Bağımsızlık', 'Independence')}  value={ph.independence ?? 0}    color="#a855f7" />
              <TraitRow label={tr('Kaygı', 'Anxiety')}             value={ph.anxiety ?? 0}          color="#e05a5a" />
              <TraitRow label={tr('Sanatsal Duygu', 'Artistic Sense')} value={ph.artistic_sense ?? 0} color="#a855f7" />
            </div>
          </div>

          {/* -- Bilinç & Ruh Hali / Consciousness -- */}
          <div>
            <SectionHeader label={tr('BİLİNÇ & RUH HALİ', 'CONSCIOUSNESS')} />
            <div className="space-y-1.5">
              <TraitRow label={tr('Bilinç', 'Consciousness')}         value={mind.consciousness ?? 0}       color="#c8b4ff" />
              <TraitRow label={tr('Bilinç Pot.', 'Consc. Potential')} value={ph.consciousness_potential ?? 0} color="#a855f7" />
              <TraitRow label={tr('Öz Farkındalık', 'Self Awareness')} value={ph.self_awareness ?? 0}      color="#d4a838" />
              <TraitRow label={tr('İnanç Kapasitesi', 'Belief Cap.')} value={ph.belief_capacity ?? 0}      color="#a0b4ff" />
              <TraitRow label={tr('Dindarlık', 'Religiosity')}        value={ph.religiosity ?? 0}           color="#c8a0e0" />
              <TraitRow label={tr('Stres', 'Stress')}                 value={mind.stress ?? 0}              color="#e05a5a" />
              <TraitRow label={tr('Serotonin', 'Serotonin')}          value={ph.serotonin ?? 0}             color="#4ecb71" />
              <TraitRow label={tr('Stres Direnci', 'Stress Resil.')}  value={ph.stress_resilience ?? 0}    color="#7dd3fc" />
              {!isDead && (
                <>
                  <StatRow label={tr('Ruh Hali', 'Mood')} value={ps.mental_state ? translateMentalState(ps.mental_state, lang as LangCode) : '—'} color="#c8b4ff" />
                  <TraitRow label={tr('Empati Kurma (ToM)', 'Theory of Mind')} value={(ps.theory_of_mind ?? 0) / 3} color="#00d4ff" />
                </>
              )}
            </div>
          </div>

          {/* -- Dil / Language -- */}
          <div>
            <SectionHeader label={tr('DİL', 'LANGUAGE')} />
            <div className="space-y-1.5">
              <StatRow label={tr('Aşama', 'Stage')} value={translateStageName(lang_.stage_name, lang)} color="#00d4ff" />
              <StatRow label={tr('Kelime Sayısı', 'Vocabulary')} value={`${wordCount} ${tr('kelime', 'words')}`} color="#7dd3fc" />
              <TraitRow label="FOXP2" value={lang_.foxp2_expression ?? (ph.language_capacity ?? 0) * 0.1} color="#00e887" />
            </div>
          </div>

          {/* -- Sağlık / Health -- */}
          {!isDead && (
            <div>
              <SectionHeader label={tr('SAĞLIK', 'HEALTH')} />
              <div className="space-y-1.5">
                <TraitRow label={tr('Can', 'HP')}           value={health.hp ?? 0}          color="#4ecb71" />
                <TraitRow label={tr('Kalori', 'Calories')}  value={health.calories ?? 0}    color="#d4a838" />
                <TraitRow label={tr('Su', 'Hydration')}     value={health.hydration ?? 0}   color="#7dd3fc" />
                <TraitRow label={tr('Hastalık Direnci', 'Disease Resist.')} value={health.disease_resistance ?? 0} color="#4f6ef7" />
                {(health.injuries?.length > 0) && (
                  <StatRow label={tr('Yaralanma', 'Injuries')} value={health.injuries.length} color="#e05a5a" />
                )}
                {(health.pregnancy) && (
                  <StatRow label={tr('Hamilelik Günü', 'Pregnancy Day')} value={health.pregnancy_day ?? '—'} color="#ff8ab0" />
                )}
              </div>
            </div>
          )}

          {/* -- Hormonlar / Hormones -- this individual's own live 49-hormone
               state (rust/sim-core/src/hormones.rs), not a population average --
               see AGENTS.md's Hormones section. Collapsed by default: dense at
               49 items, expand to verify a specific mechanism (e.g. pregnancy
               raising estrogen/progesterone, low HP spiking adrenaline). */}
          {!isDead && (
            <div>
              <button
                onClick={() => setHormonesExpanded(v => !v)}
                className="flex items-center justify-between w-full"
                style={{ marginBottom: hormonesExpanded ? 6 : 0 }}
              >
                <SectionHeader label={tr('HORMONLAR', 'HORMONES')} />
                <ChevronDown size={12} style={{ color: '#6a8878', transform: hormonesExpanded ? 'rotate(180deg)' : 'none', transition: 'transform 0.2s' }} />
              </button>
              {hormonesExpanded && (
                <div className="space-y-3">
                  {HORMONE_GROUPS.map(group => (
                    <div key={group.title.en}>
                      <div className="text-sim-muted uppercase tracking-wide opacity-70" style={{ fontSize: 10, marginBottom: 3 }}>
                        {text(lang as LangCode, group.title)}
                      </div>
                      <div className="space-y-1.5">
                        {group.items.map(({ key, label }) => (
                          <TraitRow key={key} label={text(lang as LangCode, label)} value={horm[key] ?? 0} color="#c8b4ff" />
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* -- Sosyal / Social -- */}
          {!isDead && (
            <div>
              <SectionHeader label={tr('SOSYAL', 'SOCIAL')} />
              <div className="flex flex-wrap gap-1.5 mb-1.5">
                {soc.has_mate && <span className="font-share-tech px-1.5 py-0.5" style={{ fontSize: 12, color: '#ff8ab0', border: '1px solid rgba(255,138,176,0.3)', background: 'rgba(255,138,176,0.08)' }}>{tr('Çift', 'Paired')}</span>}
                {(soc.children_ids?.length > 0) && <span className="font-share-tech px-1.5 py-0.5" style={{ fontSize: 12, color: '#4ecb71', border: '1px solid rgba(78,203,113,0.3)', background: 'rgba(78,203,113,0.08)' }}>{soc.children_ids.length} {tr('Çocuk', 'Child.')}</span>}
                {soc.group_id && <span className="font-share-tech px-1.5 py-0.5" style={{ fontSize: 12, color: '#4f6ef7', border: '1px solid rgba(79,110,247,0.3)', background: 'rgba(79,110,247,0.08)' }}>{tr('Grup', 'Group')}</span>}
                {!soc.has_mate && !soc.group_id && <span className="font-share-tech" style={{ fontSize: 12, color: '#6a8878' }}>{tr('Yalnız', 'Alone')}</span>}
              </div>
              <StatRow label={tr('İtibar', 'Reputation')} value={`${Math.round((soc.reputation ?? 0) * 100)}%`} color="#d4a838" />
              {inbreedingPct > 0 && (
                <StatRow label={tr('Akraba Evliliği', 'Inbreeding')} value={`${inbreedingPct}%`} color={inbreedingPct > 25 ? '#e05a5a' : '#d4a838'} />
              )}
              {topRelationships.length > 0 && (
                <div className="mt-2">
                  <div className="text-sim-muted" style={{ fontSize: 11, marginBottom: 3 }}>{tr('En Güçlü Bağlar', 'Strongest Bonds')}</div>
                  <div className="space-y-1">
                    {topRelationships.map(({ otherId, otherName, bond }) => (
                      <div key={otherId} className="flex items-center justify-between">
                        <span className="font-share-tech" style={{ fontSize: 12, color: '#a0b4ff' }}>{otherName}</span>
                        <span className="font-share-tech" style={{ fontSize: 12, color: bond >= 0 ? '#4ecb71' : '#e05a5a' }}>
                          {bond >= 0 ? '+' : ''}{bond.toFixed(2)}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* -- Soyağacı / Family Tree -- */}
          <div>
            <SectionHeader label={tr('SOYAĞACI', 'FAMILY TREE')} />

            {isFounder && (
              <div className="font-share-tech px-2 py-1 mb-3" style={{ fontSize: 12, color: '#d4a838', border: '1px solid rgba(212,168,56,0.3)', background: 'rgba(212,168,56,0.06)' }}>
                ★ {tr('Kurucu Birey — Medeniyetin Atası', 'Founding Individual — Ancestor of Civilization')}
              </div>
            )}

            {grandparents.length > 0 && (
              <FamilySection label={`${tr('BÜYÜKANNE/BABA', 'GRANDPARENTS')} (${grandparents.length})`} indent={0}>
                {grandparents.map(({ obj, side }, idx) => <PersonRow key={idx} obj={obj} tag={side} lang={lang} />)}
              </FamilySection>
            )}

            {(parent1 || parent2 || ind.parent_1_id || ind.parent_2_id) && (
              <FamilySection label={tr('EBEVEYNLER', 'PARENTS')} indent={grandparents.length > 0 ? 1 : 0}>
                {[{ obj: parent1, id: ind.parent_1_id }, { obj: parent2, id: ind.parent_2_id }]
                  .filter(p => p.obj || p.id)
                  .map(({ obj, id }, idx) => {
                    const roleLabel = obj?.sex === 'male' ? tr('Baba','Father') : obj?.sex === 'female' ? tr('Anne','Mother') : tr('Ebeveyn','Parent');
                    return <PersonRow key={idx} obj={obj} fallbackId={id} tag={roleLabel} lang={lang} />;
                  })}
              </FamilySection>
            )}

            {siblings.length > 0 && (
              <FamilySection label={`${tr('KARDEŞLER', 'SIBLINGS')} (${siblings.length})`} indent={2}>
                {siblings.slice(0, 6).map(s => <PersonRow key={s.id} obj={s} lang={lang} />)}
                {siblings.length > 6 && <div className="font-share-tech text-sim-muted" style={{ fontSize: 12, paddingLeft: 10 }}>+{siblings.length - 6} {tr('daha','more')}</div>}
              </FamilySection>
            )}

            <div className="flex items-center gap-2 my-1 px-1" style={{ borderLeft: '2px solid rgba(212,168,56,0.6)', marginLeft: 2 }}>
              <span style={{ fontSize: 12, color: '#d4a838' }}>★</span>
              <span className="font-orbitron font-bold" style={{ fontSize: 12, color: ind.sex === 'male' ? '#8ab0ff' : '#ffaac8' }}>{name}</span>
              <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>
                {isDead ? `† ${tr('ölü','dec.')}` : `${age.toFixed(0)} ${tr('yaş','yr')}`}
              </span>
            </div>

            {children.length > 0 && (
              <FamilySection label={`${tr('ÇOCUKLAR', 'CHILDREN')} (${children.length})`} indent={1}>
                {children.slice(0, 8).map(c => <PersonRow key={c.id} obj={c} lang={lang} />)}
                {children.length > 8 && <div className="font-share-tech text-sim-muted" style={{ fontSize: 12, paddingLeft: 10 }}>+{children.length - 8} {tr('daha','more')}</div>}
              </FamilySection>
            )}

            {grandchildren.length > 0 && (
              <FamilySection label={`${tr('TORUNLAR', 'GRANDCHILDREN')} (${grandchildren.length})`} indent={2}>
                {grandchildren.slice(0, 6).map(({ obj: gc, parentName }, idx) => <PersonRow key={idx} obj={gc} tag={parentName} lang={lang} />)}
                {grandchildren.length > 6 && <div className="font-share-tech text-sim-muted" style={{ fontSize: 12, paddingLeft: 10 }}>+{grandchildren.length - 6} {tr('daha','more')}</div>}
              </FamilySection>
            )}

            {isFounder && children.length === 0 && (
              <div className="font-share-tech text-sim-muted mt-2" style={{ fontSize: 12 }}>
                {tr('Henüz çocuk yok.', 'No children yet.')}
              </div>
            )}
          </div>

          {/* -- Hayat Hikâyesi / Life Journal -- */}
          {(liveJournalEvents.length > 0 || archivedJournal.length > 0) && (
            <div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
                <span className="font-share-tech tracking-widest" style={{ fontSize: 12, color: '#6a8878', letterSpacing: '0.12em' }}>
                  {tr('HAYAT HİKÂYESİ', 'LIFE STORY')}
                </span>
                {archivedJournal.length > 0 && (
                  <button
                    onClick={() => setArchiveOpen(true)}
                    style={{
                      background: 'transparent',
                      border: '1px solid rgba(0,212,255,0.3)',
                      color: '#00d4ff',
                      cursor: 'pointer', padding: '1px 7px', fontSize: 12,
                      fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.05em',
                    }}>
                    {tr('ARŞİV', 'ARCHIVE')} ({archivedJournal.length})
                  </button>
                )}
              </div>
              <div style={{ height: 1, background: 'rgba(0,232,135,0.1)', marginBottom: 6 }} />
              <div className="space-y-1">
                {liveJournalEvents.map((ev, i) => (
                  <div key={i} style={{ display: 'flex', gap: 6, alignItems: 'baseline' }}>
                    <span style={{ fontSize: 12, color: '#6a8878', flexShrink: 0, fontFamily: 'Share Tech Mono, monospace' }}>
                      Y{ev.sim_year}G{ev.sim_day}
                    </span>
                    <span style={{ fontSize: 12, flexShrink: 0 }}>{TYPE_ICON[ev.event_type] ?? '·'}</span>
                    <span style={{ fontSize: 12, color: '#a0b4ff', lineHeight: 1.4, fontFamily: 'Share Tech Mono, monospace' }}>
                      {(() => { const d = translateEventDescription(ev.description ?? '', lang as LangCode, ev); return d.length > 70 ? d.slice(0, 70) + '…' : d; })()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

        </div>{/* end scrollable content */}
      </div>{/* end modal box */}

      {voiceOpen && (
        <InnerVoiceModal
          ind={ind}
          lang={lang}
          onClose={() => setVoiceOpen(false)}
        />
      )}

      {treeOpen && (
        <FamilyTreeModal
          ind={ind}
          allIndividuals={allIndividuals}
          lang={lang}
          onClose={() => setTreeOpen(false)}
        />
      )}

      {archiveOpen && (
        <JournalArchiveModal
          name={name}
          entries={archivedJournal}
          typeIcon={TYPE_ICON}
          lang={lang}
          onClear={() => {
            try { localStorage.removeItem(`journal_${ind.id}`); } catch {}
            setArchivedJournal([]);
            setArchiveOpen(false);
          }}
          onClose={() => setArchiveOpen(false)}
        />
      )}

    </div>
  );
}

const INNER_VOICE_ARCHIVE_KEY = (id: string) => `inner_voice_${id}`;

const CONCEPT_I18N: Record<string, Record<string, string>> = {
  food:    { tr: 'yiyecek', en: 'food',    de: 'Essen',    fr: 'nourriture', ar: 'طعام'   },
  water:   { tr: 'su',      en: 'water',   de: 'Wasser',   fr: 'eau',        ar: 'ماء'    },
  danger:  { tr: 'tehlike', en: 'danger',  de: 'Gefahr',   fr: 'danger',     ar: 'خطر'   },
  pain:    { tr: 'acı',     en: 'pain',    de: 'Schmerz',  fr: 'douleur',    ar: 'ألم'    },
  fire:    { tr: 'ateş',    en: 'fire',    de: 'Feuer',    fr: 'feu',        ar: 'نار'    },
  sleep:   { tr: 'uyku',    en: 'sleep',   de: 'Schlaf',   fr: 'sommeil',    ar: 'نوم'    },
  hunt:    { tr: 'av',      en: 'hunt',    de: 'Jagd',     fr: 'chasse',     ar: 'صيد'    },
  run:     { tr: 'koş',     en: 'run',     de: 'laufen',   fr: 'courir',     ar: 'ركض'    },
  me:      { tr: 'ben',     en: 'me',      de: 'ich',      fr: 'moi',        ar: 'أنا'    },
  you:     { tr: 'sen',     en: 'you',     de: 'du',       fr: 'toi',        ar: 'أنت'    },
  us:      { tr: 'biz',     en: 'us',      de: 'wir',      fr: 'nous',       ar: 'نحن'    },
  die:     { tr: 'ölüm',    en: 'die',     de: 'Tod',      fr: 'mort',       ar: 'موت'    },
  born:    { tr: 'doğum',   en: 'born',    de: 'Geburt',   fr: 'naissance',  ar: 'ولادة'  },
  good:    { tr: 'iyi',     en: 'good',    de: 'gut',      fr: 'bon',        ar: 'جيد'    },
  bad:     { tr: 'kötü',    en: 'bad',     de: 'schlecht', fr: 'mauvais',    ar: 'سيئ'    },
  here:    { tr: 'burada',  en: 'here',    de: 'hier',     fr: 'ici',        ar: 'هنا'    },
  there:   { tr: 'orada',   en: 'there',   de: 'dort',     fr: 'là-bas',     ar: 'هناك'   },
  them:    { tr: 'onlar',   en: 'them',    de: 'sie',      fr: 'eux',        ar: 'هم'     },
  sun:     { tr: 'güneş',   en: 'sun',     de: 'Sonne',    fr: 'soleil',     ar: 'شمس'    },
  moon:    { tr: 'ay',      en: 'moon',    de: 'Mond',     fr: 'lune',       ar: 'قمر'    },
  rain:    { tr: 'yağmur',  en: 'rain',    de: 'Regen',    fr: 'pluie',      ar: 'مطر'    },
  dark:    { tr: 'karanlık',en: 'dark',    de: 'dunkel',   fr: 'sombre',     ar: 'ظلام'   },
  light:   { tr: 'ışık',    en: 'light',   de: 'Licht',    fr: 'lumière',    ar: 'ضوء'    },
  earth:   { tr: 'toprak',  en: 'earth',   de: 'Erde',     fr: 'terre',      ar: 'أرض'    },
  eat:     { tr: 'ye',      en: 'eat',     de: 'essen',    fr: 'manger',     ar: 'أكل'    },
  time:    { tr: 'zaman',   en: 'time',    de: 'Zeit',     fr: 'temps',      ar: 'وقت'    },
  god:     { tr: 'tanrı',   en: 'god',     de: 'Gott',     fr: 'dieu',       ar: 'إله'    },
  spirit:  { tr: 'ruh',     en: 'spirit',  de: 'Geist',    fr: 'esprit',     ar: 'روح'    },
  sky:     { tr: 'gökyüzü', en: 'sky',     de: 'Himmel',   fr: 'ciel',       ar: 'سماء'   },
};

function conceptLabel(concept: string, lang: string): string {
  const row = CONCEPT_I18N[concept];
  if (!row) return concept;
  return row[lang] ?? row.en ?? concept;
}


// offset param rotates through known words so thoughts vary over time
function clientThoughtFromVocab(ind: any, simDay: number, offset: number, lang = 'en'): { proto: string; annotated: string } | null {
  const vocab: Record<string, string> = ind.language?.vocabulary ?? {};
  const stage = ind.language?.stage ?? 0;
  if (stage < 2 || Object.keys(vocab).length === 0) return null;

  const ps  = ind.psychology ?? {};
  const ph  = ind.phenotype ?? {};
  const c   = ind.mind?.consciousness ?? 0;
  const hunger  = 1 - (ind.satiation ?? 0.5);
  const thirst  = 1 - (ind.hydration ?? 0.5);
  const hp      = ind.health?.hp ?? 1;
  const mental  = ps.mental_state ?? 'calm';
  const wellbeing = ps.wellbeing ?? 0.5;
  const hasGroup  = !!ind.group_id;
  const hasMate   = !!(ps.mate_id || ind.social?.mate_id);
  const recentDeath = (ps.trauma_events ?? []).some((e: any) => e.type === 'kin_death' && (simDay - e.day) < 20);
  const recentDisaster = (ps.trauma_events ?? []).some((e: any) => e.type !== 'kin_death' && (simDay - e.day) < 15);

  const priority: string[] = [];
  if (hunger > 0.7)           priority.push('food', 'hunt', 'eat');
  if (thirst > 0.7)           priority.push('water');
  if (hp < 0.3)               priority.push('pain', 'die');
  if (recentDisaster)         priority.push('danger', 'run', 'fire');
  if (mental === 'grieving' || recentDeath) priority.push('die', 'you', 'bad');
  if (mental === 'anxious')   priority.push('danger', 'bad');
  if (mental === 'depressed') priority.push('bad', 'sleep');
  if (hunger > 0.45)          priority.push('food');
  if (thirst > 0.45)          priority.push('water');
  if (!hasGroup)              priority.push('us', 'here', 'you');
  if (hasMate)                priority.push('you', 'good');
  if (mental === 'excited')   priority.push('good', 'us');
  if (c > 0.3 && (ph.curiosity ?? 0.5) > 0.6) priority.push('sky', 'sun', 'moon', 'time');
  if (c > 0.5)                priority.push('god', 'spirit', 'time');
  if (wellbeing > 0.7)        priority.push('good', 'here');
  priority.push('me', 'here', 'good', 'bad', 'sleep', 'earth', 'light', 'dark', 'rain', 'sun');

  const seen = new Set<string>();
  const ordered = priority.filter(concept => { if (seen.has(concept)) return false; seen.add(concept); return true; });
  const known = ordered.filter(concept => vocab[concept]);
  if (known.length === 0) return null;

  const maxWords = stage <= 2 ? 1 : stage === 3 ? Math.min(2, known.length) : Math.min(3 + Math.floor(c * 3), known.length);
  // rotate start index by offset so each tick picks a different slice
  const startIdx = offset % Math.max(1, known.length - maxWords + 1);
  const selected = known.slice(startIdx, startIdx + maxWords);
  if (selected.length === 0) return null;
  return {
    proto: selected.map(concept => vocab[concept]).join(stage >= 4 ? '  ' : '... '),
    annotated: selected.map(concept => `${vocab[concept]} [${conceptLabel(concept, lang)}]`).join(stage >= 4 ? '  ' : '... '),
  };
}

function InnerVoiceModal({ ind, lang, onClose }: { ind: any; lang: string; onClose: () => void }) {
  const { stats } = useSimStore();
  const L = (trStr: string, enStr: string, deStr = enStr, frStr = enStr, arStr = enStr) =>
    text(lang as LangCode, { tr: trStr, en: enStr, de: deStr, fr: frStr, ar: arStr });

  const storageKey = INNER_VOICE_ARCHIVE_KEY(ind.id);
  const [archive, setArchive] = useState<{ proto: string; annotated: string; year: number }[]>(() => {
    try { return JSON.parse(localStorage.getItem(storageKey) ?? '[]'); } catch { return []; }
  });
  const [tick, setTick] = useState(0);
  const [activeTab, setActiveTab] = useState<'log' | 'archive'>('log');

  const simYear = (stats as any)?.year ?? 0;
  const simDay  = (stats as any)?.day ?? 0;
  const c       = ind.mind?.consciousness ?? 0;
  const stage   = ind.language?.stage ?? 0;
  const vocab   = ind.language?.vocabulary ?? {};
  const wordCount = Object.keys(vocab).length;
  const accentColor = c > 0.5 ? '#c8b4ff' : c > 0.15 ? '#7dd3fc' : '#6a8878';
  const name    = nameFromId(ind.id, ind.sex, ind.phenotype?.name ?? ind.name);

  // Refresh interval: faster for higher consciousness (min 5s, max 20s)
  const refreshMs = Math.round(20000 - c * 15000);

  // Auto-refresh ticker
  useEffect(() => {
    const id = setInterval(() => setTick(t => t + 1), refreshMs);
    return () => clearInterval(id);
  }, [refreshMs]);

  // Compute current thought from tick-based offset
  const serverThought = ind.mind?.inner_thought;
  const thought: { proto: string; annotated: string } | null =
    serverThought?.annotated
      ? { proto: serverThought.proto ?? '', annotated: serverThought.annotated }
      : clientThoughtFromVocab(ind, simDay, tick, lang);

  // Archive every new thought automatically
  useEffect(() => {
    if (!thought?.annotated) return;
    setArchive(prev => {
      if (prev[0]?.annotated === thought.annotated) return prev;
      const updated = [{ proto: thought.proto, annotated: thought.annotated, year: simYear }, ...prev].slice(0, 100);
      try { localStorage.setItem(storageKey, JSON.stringify(updated)); } catch {}
      return updated;
    });
  }, [thought?.annotated]);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center" style={{ background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(6px)' }}>
      <div className="flex flex-col" style={{ width: 420, maxHeight: '84vh', background: 'rgba(4,4,18,0.98)', border: `1px solid ${accentColor}40`, boxShadow: '0 16px 60px rgba(0,0,0,0.8)', borderRadius: 2 }}>

        {/* Header */}
        <div className="flex items-center gap-2 px-4 py-3 flex-shrink-0" style={{ borderBottom: `1px solid ${accentColor}20` }}>
          <span style={{ fontSize: 14, color: accentColor, fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.12em', flex: 1 }}>
            💭 {L('İÇ SES', 'INNER VOICE', 'INNERE STIMME', 'VOIX INTÉRIEURE', 'الصوت الداخلي')} — {name}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: '#6a8878', cursor: 'pointer', lineHeight: 0, padding: 2 }}>
            <X size={16} />
          </button>
        </div>

        {/* Current thought */}
        <div className="px-4 pt-4 pb-3 flex-shrink-0">
          <div style={{ background: `${accentColor}06`, border: `1px solid ${accentColor}20`, borderLeft: `3px solid ${accentColor}`, padding: '12px 14px', borderRadius: 2, minHeight: 72 }}>
            {thought ? (
              <>
                <p className="font-share-tech" style={{ fontSize: 17, color: '#e8e0ff', lineHeight: 1.8, letterSpacing: '0.1em' }}>
                  {thought.proto}
                </p>
                <p className="font-share-tech" style={{ fontSize: 12, color: '#6a7888', marginTop: 6, lineHeight: 1.5 }}>
                  {thought.annotated}
                </p>
              </>
            ) : stage < 2 ? (
              <p className="font-share-tech" style={{ fontSize: 14, color: '#4a5568', fontStyle: 'italic' }}>
                {L('Dil yok — henüz düşünce oluşmuyor.', 'No language — no thought yet.',
                   'Keine Sprache — noch kein Gedanke.', 'Pas de langage — pas encore de pensée.', 'لا لغة — لا فكر بعد.')}
              </p>
            ) : (
              <p className="font-share-tech" style={{ fontSize: 14, color: '#4a5568', fontStyle: 'italic' }}>
                {L('Kelime bilmiyor — iç ses sessiz.', 'No words known — inner voice silent.',
                   'Keine Wörter — innere Stimme still.', 'Aucun mot connu — voix intérieure silencieuse.', 'لا كلمات — الصوت الداخلي صامت.')}
              </p>
            )}
          </div>

          {/* Stats row */}
          <div className="flex items-center gap-3 mt-2">
            <div style={{ flex: 1, height: 2, background: 'rgba(255,255,255,0.06)', borderRadius: 1 }}>
              <div style={{ width: `${Math.round(c * 100)}%`, height: '100%', background: accentColor, borderRadius: 1 }} />
            </div>
            <span className="font-share-tech" style={{ fontSize: 12, color: '#6a8878' }}>
              {L('Bilinç', 'Consc.', 'Bewusstsein', 'Conscience', 'وعي')} {Math.round(c * 100)}%
            </span>
            <span className="font-share-tech" style={{ fontSize: 12, color: '#6a8878' }}>
              {wordCount} {L('kelime', 'words', 'Wörter', 'mots', 'كلمة')}
            </span>
          </div>

          {/* Vocabulary peek */}
          {wordCount > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {Object.entries(vocab).slice(0, 14).map(([concept, word]) => (
                <span key={concept} className="font-share-tech" style={{ fontSize: 12, color: '#8898a8', background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.06)', padding: '2px 5px', borderRadius: 2 }}>
                  {String(word)} <span style={{ color: '#4a5568' }}>[{conceptLabel(concept, lang)}]</span>
                </span>
              ))}
              {wordCount > 14 && <span className="font-share-tech" style={{ fontSize: 12, color: '#4a5568' }}>+{wordCount - 14}</span>}
            </div>
          )}
        </div>

        {/* Tab bar */}
        <div className="flex flex-shrink-0" style={{ borderTop: `1px solid ${accentColor}15` }}>
          {(['log', 'archive'] as const).map(tab => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              style={{
                flex: 1, padding: '7px 0', background: 'transparent', border: 'none',
                borderBottom: activeTab === tab ? `2px solid ${accentColor}` : '2px solid transparent',
                color: activeTab === tab ? accentColor : '#4a5568',
                fontFamily: 'Share Tech Mono, monospace', fontSize: 12, letterSpacing: '0.1em', cursor: 'pointer',
              }}>
              {tab === 'log'
                ? L('YAŞAM GÜNLÜĞÜ', 'LIFE LOG', 'LEBENSLOG', 'JOURNAL DE VIE', 'سجل الحياة')
                : L('ANLIK ARŞİV', 'SESSION ARCHIVE', 'SITZUNGSARCHIV', 'ARCHIVE DE SESSION', 'أرشيف الجلسة')}
            </button>
          ))}
        </div>

        {/* Tab content */}
        <div className="flex-1 overflow-y-auto px-4 pb-4 pt-3 space-y-2">
          {activeTab === 'log' ? (() => {
            const lifelog: Array<{ day: number; kind: string; thought: { proto?: string; annotated?: string } }> =
              (ind.mind as any)?.inner_thought_log ?? [];
            const kindLabel = (kind: string) => {
              const map: Record<string, string[]> = {
                first_word:        [L('İlk Kelime', 'First Word', 'Erstes Wort', 'Premier mot', 'أول كلمة')],
                first_thought:     [L('İlk Düşünce', 'First Thought', 'Erster Gedanke', 'Première pensée', 'أول فكرة')],
                first_abstract:    [L('İlk Soyut Kavram', 'First Abstract', 'Erstes Abstraktum', 'Première abstraction', 'أول مفهوم مجرد')],
                consciousness_10:  ['10% ↑'],
                consciousness_25:  ['25% ↑'],
                consciousness_50:  ['50% ↑'],
                consciousness_75:  ['75% ↑'],
                death_proximity:   [L('Ölüm Yakın', 'Near Death', 'Nahe dem Tod', 'Près de la mort', 'قرب الموت')],
                grief:             [L('Yas', 'Grief', 'Trauer', 'Deuil', 'الحزن')],
              };
              return (map[kind] ?? [kind])[0];
            };
            const kindColor = (kind: string) => {
              if (kind === 'first_word') return '#a0c080';
              if (kind === 'first_thought') return '#80a0c0';
              if (kind === 'first_abstract') return '#c0a0e0';
              if (kind.startsWith('consciousness')) return accentColor;
              if (kind === 'death_proximity') return '#c06060';
              if (kind === 'grief') return '#8070a0';
              return '#6a8878';
            };
            if (lifelog.length === 0) return (
              <p style={{ fontSize: 13, color: '#4a5568', fontStyle: 'italic', fontFamily: 'Share Tech Mono, monospace' }}>
                {L('Henüz önemli an kaydedilmedi.', 'No milestone moments recorded yet.',
                   'Noch keine Meilensteine aufgezeichnet.', 'Pas encore de moments importants enregistrés.', 'لم تُسجَّل لحظات بارزة بعد.')}
              </p>
            );
            return (
              <>
                {lifelog.map((entry, i) => (
                  <div key={i} style={{ borderLeft: `2px solid ${kindColor(entry.kind)}50`, paddingLeft: 10 }}>
                    <div className="flex items-center gap-2">
                      <span className="font-share-tech" style={{ fontSize: 12, color: kindColor(entry.kind), letterSpacing: '0.1em' }}>
                        {kindLabel(entry.kind)}
                      </span>
                      <span className="font-share-tech" style={{ fontSize: 12, color: '#4a5568' }}>
                        · {L('Gün', 'Day', 'Tag', 'Jour', 'يوم')} {entry.day}
                      </span>
                    </div>
                    {entry.thought?.proto && (
                      <p className="font-share-tech" style={{ fontSize: 14, color: '#8898a8', letterSpacing: '0.06em', marginTop: 2 }}>
                        {entry.thought.proto}
                      </p>
                    )}
                    {entry.thought?.annotated && (
                      <p className="font-share-tech" style={{ fontSize: 12, color: '#4a5568', marginTop: 1 }}>
                        {entry.thought.annotated}
                      </p>
                    )}
                  </div>
                ))}
              </>
            );
          })() : (
            <>
              {archive.length > 0 && (
                <div className="flex justify-end mb-1">
                  <button
                    onClick={() => { setArchive([]); try { localStorage.removeItem(storageKey); } catch {} }}
                    style={{ background: 'transparent', border: '1px solid rgba(160,80,80,0.3)', color: '#a05050', cursor: 'pointer', padding: '2px 8px', fontSize: 12, fontFamily: 'Share Tech Mono, monospace', borderRadius: 2 }}>
                    {L('TEMİZLE', 'CLEAR', 'LÖSCHEN', 'EFFACER', 'مسح')}
                  </button>
                </div>
              )}
              {archive.length === 0 ? (
                <p style={{ fontSize: 13, color: '#4a5568', fontStyle: 'italic', fontFamily: 'Share Tech Mono, monospace' }}>
                  {L('Henüz arşivlenmiş düşünce yok.', 'No archived thoughts yet.',
                     'Noch keine archivierten Gedanken.', 'Pas encore de pensées archivées.', 'لا توجد أفكار مؤرشفة بعد.')}
                </p>
              ) : archive.map((entry, i) => (
                <div key={i} style={{ borderLeft: `2px solid ${accentColor}20`, paddingLeft: 10 }}>
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#4a6068' }}>
                    {L('Yıl', 'Year', 'Jahr', 'An', 'سنة')} {entry.year}
                  </span>
                  <p className="font-share-tech" style={{ fontSize: 14, color: '#8898a8', letterSpacing: '0.06em', marginTop: 2 }}>
                    {entry.proto}
                  </p>
                  <p className="font-share-tech" style={{ fontSize: 12, color: '#4a5568', marginTop: 1 }}>
                    {entry.annotated}
                  </p>
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function FamilyTreeModal({ ind, allIndividuals, lang, onClose }: {
  ind: any; allIndividuals: any[]; lang: string; onClose: () => void;
}) {
  const tr = makeTr(lang as LangCode);

  const parent1 = allIndividuals.find(i => i.id === ind.parent_1_id) ?? null;
  const parent2 = allIndividuals.find(i => i.id === ind.parent_2_id) ?? null;
  const gp = [
    parent1 ? allIndividuals.find(i => i.id === parent1.parent_1_id) : null,
    parent1 ? allIndividuals.find(i => i.id === parent1.parent_2_id) : null,
    parent2 ? allIndividuals.find(i => i.id === parent2.parent_1_id) : null,
    parent2 ? allIndividuals.find(i => i.id === parent2.parent_2_id) : null,
  ].filter(Boolean) as any[];
  const children = allIndividuals.filter(i => i.parent_1_id === ind.id || i.parent_2_id === ind.id);
  const grandchildren = [...new Map(
    children.flatMap(c => allIndividuals.filter(i => i.parent_1_id === c.id || i.parent_2_id === c.id))
      .map(x => [x.id, x])
  ).values()];
  const parents = [parent1, parent2].filter(Boolean) as any[];
  const isFounder = !parent1 && !parent2;

  function Chip({ person, star = false }: { person: any; star?: boolean }) {
    const n = nameFromId(person.id, person.sex, person.phenotype?.name ?? person.name);
    const dead = person.alive === false || person.is_dead;
    const age = Math.floor(parseFloat(person.age_years ?? 0));
    const col = person.sex === 'male' ? '#6090ff' : '#ff8ab0';
    return (
      <div style={{
        padding: '5px 9px', textAlign: 'center', minWidth: 58, maxWidth: 88,
        background: star ? 'rgba(79,110,247,0.13)' : 'rgba(4,4,18,0.9)',
        border: `1px solid ${star ? 'rgba(79,110,247,0.5)' : 'rgba(160,200,176,0.12)'}`,
        boxShadow: star ? '0 0 10px rgba(79,110,247,0.18)' : 'none',
      }}>
        <div style={{ fontSize: 12, fontFamily: 'Orbitron, monospace', color: col, fontWeight: 700, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {star && '★ '}{n}{dead ? ' †' : ''}
        </div>
        <div style={{ fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace', marginTop: 1 }}>
          {age} {tr('yaş', 'yr')}
        </div>
      </div>
    );
  }

  function RowLabel({ label }: { label: string }) {
    return (
      <div style={{ textAlign: 'center', fontSize: 12, color: '#4a5848', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.12em', marginBottom: 5 }}>
        {label}
      </div>
    );
  }

  function VLine() {
    return <div style={{ width: 1, height: 18, background: '#2a3e30', margin: '3px auto' }} />;
  }

  function ChipRow({ items }: { items: any[] }) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', flexWrap: 'wrap', gap: 6 }}>
        {items.map((p, i) => <Chip key={i} person={p} />)}
      </div>
    );
  }

  return (
    <div
      className="fixed inset-0 flex items-center justify-center"
      style={{ zIndex: 60, background: 'rgba(0,0,0,0.58)', backdropFilter: 'blur(4px)' }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="flex flex-col" style={{
        width: 420, maxHeight: '82vh',
        background: 'rgba(4,4,18,0.98)', border: '1px solid rgba(79,110,247,0.32)',
        boxShadow: '0 16px 60px rgba(0,0,0,0.85)',
      }}>
        {/* Header */}
        <div className="flex items-center gap-2 px-4 py-3 flex-shrink-0" style={{ borderBottom: '1px solid rgba(79,110,247,0.15)' }}>
          <span style={{ fontSize: 12, color: '#4f6ef7', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.15em', flex: 1 }}>
            🌿 {tr('SOY AĞACI', 'FAMILY TREE')}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: '#6a8878', cursor: 'pointer', lineHeight: 0, padding: 2 }}>
            <X size={13} />
          </button>
        </div>

        {/* Tree */}
        <div className="flex-1 overflow-y-auto" style={{ padding: '20px 16px' }}>

          {gp.length > 0 && (
            <>
              <RowLabel label={tr('BÜYÜKANNE / BÜYÜKBABA', 'GRANDPARENTS')} />
              <ChipRow items={gp} />
              <VLine />
            </>
          )}

          {parents.length > 0 && (
            <>
              <RowLabel label={tr('EBEVEYNLER', 'PARENTS')} />
              <ChipRow items={parents} />
              <VLine />
            </>
          )}

          <RowLabel label={tr('BİREY', 'INDIVIDUAL')} />
          <div style={{ display: 'flex', justifyContent: 'center' }}>
            <Chip person={ind} star />
          </div>

          {isFounder && (
            <div style={{ textAlign: 'center', marginTop: 6, fontSize: 12, color: '#d4a838', fontFamily: 'Share Tech Mono, monospace' }}>
              ★ {tr('Kurucu — bilinen atası yok', 'Founder — no known ancestors')}
            </div>
          )}

          {children.length > 0 && (
            <>
              <VLine />
              <RowLabel label={`${tr('ÇOCUKLAR', 'CHILDREN')} (${children.length})`} />
              <ChipRow items={children.slice(0, 10)} />
              {children.length > 10 && (
                <div style={{ textAlign: 'center', fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace', marginTop: 4 }}>
                  +{children.length - 10} {tr('daha', 'more')}
                </div>
              )}
            </>
          )}

          {grandchildren.length > 0 && (
            <>
              <VLine />
              <RowLabel label={`${tr('TORUNLAR', 'GRANDCHILDREN')} (${grandchildren.length})`} />
              <ChipRow items={grandchildren.slice(0, 8)} />
              {grandchildren.length > 8 && (
                <div style={{ textAlign: 'center', fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace', marginTop: 4 }}>
                  +{grandchildren.length - 8} {tr('daha', 'more')}
                </div>
              )}
            </>
          )}

          {!isFounder && !children.length && !grandchildren.length && (
            <div style={{ textAlign: 'center', marginTop: 12, fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace' }}>
              {tr('Henüz çocuğu yok', 'No children yet')}
            </div>
          )}

          {/* Stats footer */}
          <div style={{ marginTop: 14, paddingTop: 10, borderTop: '1px solid rgba(79,110,247,0.1)', textAlign: 'center', fontSize: 12, color: '#4a5848', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.08em' }}>
            {isFounder ? tr('Kurucu', 'Founder') : `${parents.length} ${tr('ebeveyn', 'parent(s)')}`}
            {children.length > 0 && ` · ${children.length} ${tr('çocuk', 'children')}`}
            {grandchildren.length > 0 && ` · ${grandchildren.length} ${tr('torun', 'grandchildren')}`}
          </div>
        </div>
      </div>
    </div>
  );
}

function JournalArchiveModal({ name, entries, typeIcon, lang, onClear, onClose }: {
  name: string; entries: any[]; typeIcon: Record<string, string>;
  lang: string; onClear: () => void; onClose: () => void;
}) {
  const tr = makeTr(lang as LangCode);
  const [confirmClear, setConfirmClear] = useState(false);

  return (
    <div
      className="fixed inset-0 flex items-center justify-center"
      style={{ zIndex: 60, background: 'rgba(0,0,0,0.55)', backdropFilter: 'blur(4px)' }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="flex flex-col" style={{
        width: 400, maxHeight: '80vh',
        background: 'rgba(4,4,18,0.98)', border: '1px solid rgba(0,212,255,0.3)',
        backdropFilter: 'blur(20px)', boxShadow: '0 16px 60px rgba(0,0,0,0.85)',
      }}>
        {/* Header */}
        <div className="flex items-center gap-2 px-4 py-3 flex-shrink-0" style={{ borderBottom: '1px solid rgba(0,212,255,0.15)' }}>
          <span style={{ fontSize: 12, color: '#00d4ff', fontFamily: 'Share Tech Mono, monospace', letterSpacing: '0.15em', flex: 1 }}>
            ◈ {tr('HAYAT HİKÂYESİ ARŞİVİ', 'LIFE STORY ARCHIVE')}
          </span>
          <span style={{ fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace', marginRight: 8 }}>
            {name} · {entries.length} {tr('olay', 'events')}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: '#6a8878', cursor: 'pointer', lineHeight: 0, padding: 2 }}>
            <X size={13} />
          </button>
        </div>

        {/* Entries */}
        <div className="flex-1 overflow-y-auto" style={{ padding: '10px 14px' }}>
          {entries.length === 0 ? (
            <div style={{ fontSize: 12, color: '#4a6858', fontFamily: 'Share Tech Mono, monospace', textAlign: 'center', paddingTop: 24 }}>
              {tr('Henüz arşivlenmiş olay yok', 'No archived events yet')}
            </div>
          ) : (
            <div className="space-y-1">
              {entries.map((ev, i) => (
                <div key={i} style={{ display: 'flex', gap: 7, alignItems: 'baseline', paddingBottom: 4, borderBottom: i < entries.length - 1 ? '1px solid rgba(0,212,255,0.04)' : 'none' }}>
                  <span style={{ fontSize: 12, color: '#4a6858', flexShrink: 0, fontFamily: 'Share Tech Mono, monospace', minWidth: 64 }}>
                    Y{ev.sim_year}G{ev.sim_day}
                  </span>
                  <span style={{ fontSize: 12, flexShrink: 0 }}>{typeIcon[ev.event_type] ?? '·'}</span>
                  <span style={{ fontSize: 12, color: '#8898c8', lineHeight: 1.45, fontFamily: 'Share Tech Mono, monospace' }}>
                    {translateEventDescription(ev.description ?? ev.event_type ?? '', lang as LangCode, ev)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex-shrink-0 flex justify-end gap-2 px-4 py-2" style={{ borderTop: '1px solid rgba(0,212,255,0.1)' }}>
          {confirmClear ? (
            <>
              <span style={{ fontSize: 12, color: '#e05a5a', fontFamily: 'Share Tech Mono, monospace', alignSelf: 'center' }}>
                {tr('Emin misin?', 'Are you sure?')}
              </span>
              <button onClick={() => setConfirmClear(false)}
                style={{ fontSize: 12, fontFamily: 'Share Tech Mono, monospace', background: 'transparent', border: '1px solid rgba(160,200,176,0.25)', color: '#6a8878', cursor: 'pointer', padding: '2px 8px' }}>
                {tr('İptal', 'Cancel')}
              </button>
              <button onClick={onClear}
                style={{ fontSize: 12, fontFamily: 'Share Tech Mono, monospace', background: 'rgba(160,80,80,0.15)', border: '1px solid rgba(224,90,90,0.4)', color: '#e05a5a', cursor: 'pointer', padding: '2px 8px' }}>
                {tr('Temizle', 'Clear')}
              </button>
            </>
          ) : (
            <button onClick={() => setConfirmClear(true)}
              style={{ fontSize: 12, fontFamily: 'Share Tech Mono, monospace', background: 'transparent', border: '1px solid rgba(160,200,176,0.2)', color: '#4a6858', cursor: 'pointer', padding: '2px 8px' }}>
              {tr('Arşivi Temizle', 'Clear Archive')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function CompareModal({ indA, indB, onClose }: { indA: any; indB: any; onClose: () => void }) {
  const { lang } = useSimStore();
  const tr = makeTr(lang as LangCode);

  function Col({ ind }: { ind: any }) {
    const name = nameFromId(ind.id, ind.sex, ind.phenotype?.name ?? ind.name);
    const age = parseFloat(ind.age_years ?? 0);
    const ph = ind.phenotype ?? {};
    const mind = ind.mind ?? {};
    const lang_ = ind.language ?? {};
    const health = ind.health ?? {};
    const isDead = ind.alive === false || ind.is_dead;
    const isFounder = !ind.parent_1_id && !ind.parent_2_id;
    const wordCount = Object.keys(lang_.vocabulary ?? {}).length;

    return (
      <div style={{ flex: 1, padding: '0 14px', overflowY: 'auto' }}>
        <div style={{ marginBottom: 10, paddingBottom: 8, borderBottom: '1px solid rgba(79,110,247,0.15)' }}>
          <div style={{ fontFamily: 'Orbitron, monospace', fontWeight: 700, fontSize: 13, color: ind.sex === 'male' ? '#6090ff' : '#ff8ab0' }}>{name}</div>
          <div style={{ fontSize: 12, color: '#a0b4ff', fontFamily: 'Share Tech Mono, monospace', marginTop: 2 }}>
            {age.toFixed(1)} {tr('yaş', 'yr')} · {ind.sex === 'male' ? tr('Erkek', 'Male') : tr('Kadın', 'Female')}
            {isFounder && <span style={{ color: '#d4a838', marginLeft: 6 }}>★ {tr('Kurucu', 'Founder')}</span>}
            {isDead && <span style={{ color: '#e05a5a', marginLeft: 6 }}>†</span>}
          </div>
        </div>

        <SectionHeader label={tr('FİZİKSEL', 'PHYSICAL')} />
        <div style={{ marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <TraitRow label={tr('Güç', 'Strength')}     value={ph.physical_strength ?? 0}                    color="#e05a5a" />
          <TraitRow label={tr('Dayanıklılık', 'End.')} value={ph.physical_endurance ?? ph.endurance ?? 0} color="#f97316" />
          <TraitRow label={tr('Bağışıklık', 'Imm.')}  value={ph.immune_strength ?? 0}                     color="#4f6ef7" />
          <TraitRow label={tr('Doğurganlık', 'Fert.')} value={ph.fertility ?? 0}                          color="#ff8ab0" />
        </div>

        <SectionHeader label={tr('BİLİŞSEL', 'COGNITIVE')} />
        <div style={{ marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <TraitRow label={tr('Zekâ', 'Intelligence')} value={ph.fluid_intelligence ?? 0}  color="#d4a838" />
          <TraitRow label={tr('Öğrenme', 'Learning')}  value={ph.learning_rate ?? 0}        color="#4ecb71" />
          <TraitRow label={tr('Dil Kap.', 'Lang.Cap.')} value={ph.language_capacity ?? 0}  color="#00d4ff" />
          <TraitRow label={tr('İnovasyon', 'Innov.')}  value={ph.innovation ?? 0}           color="#7dd3fc" />
        </div>

        <SectionHeader label={tr('BİLİNÇ', 'CONSCIOUSNESS')} />
        <div style={{ marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <TraitRow label={tr('Bilinç', 'Conscious.')} value={mind.consciousness ?? 0}            color="#c8b4ff" />
          <TraitRow label={tr('Potansiyel', 'Potential')} value={ph.consciousness_potential ?? 0} color="#a855f7" />
        </div>

        <SectionHeader label={tr('DİL', 'LANGUAGE')} />
        <div style={{ marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 3 }}>
          <StatRow label={tr('Aşama', 'Stage')} value={translateStageName(lang_.stage_name, lang)} color="#00d4ff" />
          <StatRow label={tr('Kelime', 'Words')} value={wordCount} color="#7dd3fc" />
        </div>

        {!isDead && (
          <>
            <SectionHeader label={tr('SAĞLIK', 'HEALTH')} />
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <TraitRow label="HP"                        value={health.hp ?? 0}        color="#4ecb71" />
              <TraitRow label={tr('Kalori', 'Calories')}  value={health.calories ?? 0}  color="#d4a838" />
              <TraitRow label={tr('Su', 'Hydration')}     value={health.hydration ?? 0} color="#7dd3fc" />
            </div>
          </>
        )}
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: 'rgba(0,0,0,0.65)', backdropFilter: 'blur(4px)' }}>
      <div style={{ width: Math.min(640, window.innerWidth - 24), maxHeight: '88vh', background: 'rgba(4,4,18,0.98)', border: '1px solid rgba(79,110,247,0.4)', display: 'flex', flexDirection: 'column', boxShadow: '0 16px 60px rgba(0,0,0,0.8)' }}>
        <div style={{ padding: '10px 14px', borderBottom: '1px solid rgba(79,110,247,0.2)', display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexShrink: 0 }}>
          <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color: '#4f6ef7', letterSpacing: '0.15em' }}>
            {tr('BİREY KARŞILAŞTIRMA', 'INDIVIDUAL COMPARISON')}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: '#a0c8b0', cursor: 'pointer', lineHeight: 0 }}>
            <X size={14} />
          </button>
        </div>
        <div style={{ flex: 1, overflowY: 'hidden', display: 'flex', paddingTop: 12 }}>
          <Col ind={indA} />
          <div style={{ width: 1, background: 'rgba(79,110,247,0.2)', flexShrink: 0 }} />
          <Col ind={indB} />
        </div>
      </div>
    </div>
  );
}

export default function PopulationPanel() {
  const { currentSim, accessToken, lang, stats } = useSimStore();
  const [individuals, setIndividuals] = useState<any[]>([]);
  const [deadIndividuals, setDeadIndividuals] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<any>(null);
  const [compareSet, setCompareSet] = useState<any[]>([]);
  const [showCompare, setShowCompare] = useState(false);
  const [filter, setFilter] = useState<'all' | 'male' | 'female'>('all');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');
  const [deadExpanded, setDeadExpanded] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval>>();
  const loadingRef = useRef(false);

  function toggleCompare(e: React.MouseEvent, ind: any) {
    e.stopPropagation();
    setCompareSet(prev => {
      const exists = prev.find(i => i.id === ind.id);
      if (exists) return prev.filter(i => i.id !== ind.id);
      if (prev.length >= 2) return [prev[1], ind];
      return [...prev, ind];
    });
  }

  async function load() {
    if (!currentSim || !accessToken) return;
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    try {
      const headers = { Authorization: `Bearer ${accessToken}` };
      const [aliveRes, deadRes] = await Promise.all([
        axios.get(`/api/simulations/${currentSim.id}/population?alive=true&limit=200`, { headers }),
        axios.get(`/api/simulations/${currentSim.id}/population?alive=false&limit=100`, { headers }),
      ]);
      setIndividuals(aliveRes.data);
      setDeadIndividuals(deadRes.data);
    } catch {}
    setLoading(false);
    loadingRef.current = false;
  }

  useEffect(() => {
    load();
    // List refreshes more slowly than stats; counts come from WebSocket in real time.
    intervalRef.current = setInterval(load, 10000);
    return () => clearInterval(intervalRef.current);
  }, [currentSim?.id, accessToken]);

  const allForLookup = useMemo(() => [...individuals, ...deadIndividuals], [individuals, deadIndividuals]);
  const filtered = useMemo(() => {
    return [...individuals]
      .filter(i => filter === 'all' || i.sex === filter)
      .sort((a, b) => {
        const diff = parseFloat(a.age_years ?? 0) - parseFloat(b.age_years ?? 0);
        return sortDir === 'asc' ? diff : -diff;
      });
  }, [individuals, filter, sortDir]);

  return (
    <DetailPanel panelId="population" title="Population" titleTr="Nüfus">
      {selected && <IndividualDetail ind={selected} allIndividuals={allForLookup} onClose={() => setSelected(null)} />}
      {showCompare && compareSet.length === 2 && (
        <CompareModal indA={compareSet[0]} indB={compareSet[1]} onClose={() => setShowCompare(false)} />
      )}

      {/* Compare action bar */}
      {compareSet.length > 0 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8, padding: '6px 8px', background: 'rgba(79,110,247,0.08)', border: '1px solid rgba(79,110,247,0.3)' }}>
          <span style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color: '#a0b4ff', flex: 1 }}>
            {compareSet.map(i => nameFromId(i.id, i.sex, i.phenotype?.name ?? i.name)).join(' vs ')}
          </span>
          {compareSet.length === 2 && (
            <button onClick={() => setShowCompare(true)}
              style={{ fontFamily: 'Share Tech Mono, monospace', fontSize: 12, color: '#4f6ef7', background: 'rgba(79,110,247,0.15)', border: '1px solid rgba(79,110,247,0.5)', padding: '2px 8px', cursor: 'pointer' }}>
              {text(lang as LangCode, { en: 'COMPARE', tr: 'KARŞILAŞTIR', de: 'VERGLEICHEN', fr: 'COMPARER', ar: 'مقارنة' })}
            </button>
          )}
          <button onClick={() => setCompareSet([])}
            style={{ background: 'transparent', border: 'none', color: '#a0c8b0', cursor: 'pointer', lineHeight: 0 }}>
            <X size={12} />
          </button>
        </div>
      )}

      {/* Summary bar — always use WebSocket stats for consistency with TopBar */}
      <div className="flex gap-2 mb-3">
        <div className="flex-1 p-2 text-center" style={{ background: 'rgba(79,110,247,0.08)', border: '1px solid rgba(79,110,247,0.2)' }}>
          <div className="font-orbitron font-bold" style={{ color: '#4f6ef7', fontSize: 14 }}>{stats?.population ?? individuals.length}</div>
          <div className="font-share-tech text-sim-muted tracking-widest" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'TOTAL', tr: 'TOPLAM', de: 'GESAMT', fr: 'TOTAL', ar: 'الإجمالي' })}</div>
        </div>
        <div className="flex-1 p-2 text-center" style={{ background: 'rgba(96,144,255,0.08)', border: '1px solid rgba(96,144,255,0.2)' }}>
          <div className="font-orbitron font-bold" style={{ color: '#6090ff', fontSize: 14 }}>
            {stats != null
              ? Math.round(stats.population * (stats.sex_ratio ?? 0.5))
              : individuals.filter(i => i.sex === 'male').length}
          </div>
          <div className="font-share-tech text-sim-muted tracking-widest" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'MALE', tr: 'ERKEK', de: 'MÄNNLICH', fr: 'MÂLE', ar: 'ذكر' })}</div>
        </div>
        <div className="flex-1 p-2 text-center" style={{ background: 'rgba(255,138,176,0.08)', border: '1px solid rgba(255,138,176,0.2)' }}>
          <div className="font-orbitron font-bold" style={{ color: '#ff8ab0', fontSize: 14 }}>
            {stats != null
              ? stats.population - Math.round(stats.population * (stats.sex_ratio ?? 0.5))
              : individuals.filter(i => i.sex === 'female').length}
          </div>
          <div className="font-share-tech text-sim-muted tracking-widest" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'FEMALE', tr: 'KADIN', de: 'WEIBLICH', fr: 'FEMELLE', ar: 'أنثى' })}</div>
        </div>
      </div>

      {/* Filter tabs + sort */}
      <div className="flex gap-1 mb-3">
        {(['all', 'male', 'female'] as const).map(f => (
          <button key={f} onClick={() => setFilter(f)}
            className="flex-1 font-share-tech tracking-widest transition-all"
            style={{
              padding: '3px 0', fontSize: 12,
              background: filter === f ? 'rgba(79,110,247,0.2)' : 'transparent',
              border: `1px solid ${filter === f ? 'rgba(79,110,247,0.5)' : 'rgba(79,110,247,0.15)'}`,
              color: filter === f ? '#c0ccff' : '#8898c8',
            }}>
            {f === 'all' ? text(lang as LangCode, { en: 'ALL', tr: 'TÜMÜ', de: 'ALLE', fr: 'TOUT', ar: 'الكل' }) : f === 'male' ? text(lang as LangCode, { en: 'MALE', tr: 'ERKEK', de: 'MÄNNLICH', fr: 'MÂLE', ar: 'ذكر' }) : text(lang as LangCode, { en: 'FEMALE', tr: 'KADIN', de: 'WEIBLICH', fr: 'FEMELLE', ar: 'أنثى' })}
          </button>
        ))}
        <button
          onClick={() => setSortDir(d => d === 'asc' ? 'desc' : 'asc')}
          title={sortDir === 'asc' ? text(lang as LangCode, { en: 'Youngest first', tr: 'En genç önce', de: 'Jüngste zuerst', fr: 'Le plus jeune d’abord', ar: 'الأصغر أولاً' }) : text(lang as LangCode, { en: 'Oldest first', tr: 'En yaşlı önce', de: 'Älteste zuerst', fr: 'Le plus âgé d’abord', ar: 'الأكبر أولاً' })}
          style={{
            padding: '3px 7px', fontSize: 12, flexShrink: 0,
            background: 'transparent',
            border: '1px solid rgba(79,110,247,0.15)',
            color: '#8898c8', cursor: 'pointer',
            fontFamily: 'Share Tech Mono, monospace',
          }}>
          {text(lang as LangCode, { en: 'AGE', tr: 'YAŞ', de: 'ALTER', fr: 'ÂGE', ar: 'العمر' })} {sortDir === 'asc' ? '↑' : '↓'}
        </button>
      </div>

      {loading && individuals.length === 0 && (
        <div className="text-center py-4">
          <span className="font-share-tech text-sim-muted/50 animate-pulse tracking-widest" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'LOADING DATA...', tr: 'VERİ YÜKLENİYOR...', de: 'DATEN WERDEN GELADEN...', fr: 'CHARGEMENT DES DONNÉES...', ar: 'جارٍ تحميل البيانات...' })}</span>
        </div>
      )}

      {/* Alive individual list */}
      <div className="space-y-0.5">
        {filtered.slice(0, 100).map((ind) => {
          const name = nameFromId(ind.id, ind.sex, ind.name);
          const age = parseFloat(ind.age_years ?? 0);
          const stage = lifeStage(age, lang);
          const isMale = ind.sex === "male";
          const isFounder = !ind.parent_1_id && !ind.parent_2_id;

          return (
            <button key={ind.id} onClick={() => setSelected(ind)}
              className="w-full flex items-center gap-2 px-2 py-1.5 transition-all text-left hover:bg-sim-border/20"
              style={{ border: "1px solid transparent", borderBottom: "1px solid rgba(79,110,247,0.06)" }}>
              <div className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                style={{ background: isMale ? "#6090ff" : "#ff8ab0", boxShadow: `0 0 4px ${isMale ? "#6090ff" : "#ff8ab0"}` }} />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="font-share-tech font-bold tracking-wider truncate"
                    style={{ fontSize: 12, color: isMale ? '#8ab0ff' : '#ffaac8' }}>
                    {name}
                  </span>
                  {isFounder && (
                    <span className="font-share-tech px-1 py-0" style={{ fontSize: 12, color: '#d4a838', border: '1px solid rgba(212,168,56,0.4)' }}>{text(lang as LangCode, { en: 'FOUNDER', tr: 'KURUCU', de: 'GRÜNDER', fr: 'FONDATEUR', ar: 'مؤسس' })}</span>
                  )}
                  {!isMale && ind.health?.pregnancy && (
                    <span title={text(lang as LangCode, { en: 'Pregnant', tr: 'Hamile', de: 'Schwanger', fr: 'Enceinte', ar: 'حامل' })} style={{ fontSize: 13, lineHeight: 1 }}>◆</span>
                  )}
                </div>
                <div className="flex items-center gap-1 mt-0.5">
                  <span className="font-share-tech" style={{ fontSize: 12, color: stage.color }}>{stage.label}</span>
                  <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span>
                  <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>{age.toFixed(0)} {text(lang as LangCode, { en: 'yr', tr: 'yaş', de: 'J.', fr: 'an', ar: 'سنة' })}</span>
                  <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span>
                  <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>
                    {(ind.y ?? 0).toFixed(1)}° {(ind.x ?? 0).toFixed(1)}°
                  </span>
                </div>
              </div>
              <button
                onClick={e => toggleCompare(e, ind)}
                title={text(lang as LangCode, { en: 'Add to comparison', tr: 'Karşılaştırmaya ekle', de: 'Zum Vergleich hinzufügen', fr: 'Ajouter à la comparaison', ar: 'أضف للمقارنة' })}
                style={{
                  background: compareSet.find(i => i.id === ind.id) ? 'rgba(79,110,247,0.25)' : 'transparent',
                  border: `1px solid ${compareSet.find(i => i.id === ind.id) ? 'rgba(79,110,247,0.7)' : 'rgba(79,110,247,0.2)'}`,
                  color: compareSet.find(i => i.id === ind.id) ? '#4f6ef7' : '#6a8878',
                  width: 18, height: 18, borderRadius: 2, cursor: 'pointer',
                  fontSize: 12, lineHeight: '16px', flexShrink: 0, padding: 0,
                  fontFamily: 'Share Tech Mono, monospace',
                }}>
                ⊕
              </button>
              <ChevronRight size={10} className="text-sim-muted flex-shrink-0" />
            </button>
          );
        })}
      </div>

      {filtered.length > 100 && (
        <div className="text-center py-2">
          <span className="font-share-tech text-sim-muted/40 tracking-widest" style={{ fontSize: 12 }}>
            +{filtered.length - 100} {text(lang as LangCode, { en: 'more individuals', tr: 'birey daha', de: 'weitere Individuen', fr: 'individus de plus', ar: 'فرد إضافي' })}
          </span>
        </div>
      )}

      {filtered.length === 0 && !loading && (
        <div className="flex flex-col items-center py-6 gap-2">
          <Users size={24} className="text-sim-muted/20" />
          <span className="font-share-tech text-sim-muted/40 tracking-widest" style={{ fontSize: 12 }}>{text(lang as LangCode, { en: 'NO POPULATION', tr: 'NÜFUS YOK', de: 'KEINE BEVÖLKERUNG', fr: 'AUCUNE POPULATION', ar: 'لا يوجد سكان' })}</span>
        </div>
      )}

      {/* Dead individuals section */}
      {deadIndividuals.length > 0 && (
        <div className="mt-4">
          <button
            onClick={() => setDeadExpanded(v => !v)}
            className="w-full flex items-center gap-2 px-2 py-1.5"
            style={{ background: 'rgba(160,80,80,0.08)', border: '1px solid rgba(160,80,80,0.25)' }}>
            <span className="font-share-tech tracking-widest flex-1 text-left" style={{ fontSize: 12, color: '#a05050' }}>
              † {text(lang as LangCode, { en: 'DECEASED', tr: 'HAYATINI KAYBETTİLER', de: 'VERSTORBEN', fr: 'DÉCÉDÉS', ar: 'المتوفون' })} ({stats?.deaths ?? deadIndividuals.length ?? 0})
            </span>
            <ChevronDown size={10} style={{ color: '#a05050', transform: deadExpanded ? 'rotate(180deg)' : 'none', transition: 'transform 0.2s' }} />
          </button>

          {deadExpanded && (
            <div className="space-y-0.5 mt-0.5">
              {deadIndividuals.slice(0, 100).map((ind) => {
                const name = nameFromId(ind.id, ind.sex, ind.phenotype?.name ?? ind.name);
                const age = parseFloat(ind.age_years ?? 0);
                const isMale = ind.sex === 'male';
                return (
                  <button key={ind.id} onClick={() => setSelected(ind)}
                    className="w-full flex items-center gap-2 px-2 py-1.5 transition-all text-left"
                    style={{ background: 'rgba(160,80,80,0.04)', border: '1px solid rgba(160,80,80,0.12)', borderTop: 'none' }}>
                    <span style={{ fontSize: 12, color: '#a05050', flexShrink: 0, lineHeight: 1 }}>†</span>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="font-share-tech font-bold tracking-wider truncate"
                          style={{ fontSize: 12, color: isMale ? '#a090b8' : '#b090a0' }}>
                          {name}
                        </span>
                      </div>
                      <div className="flex items-center gap-1 mt-0.5">
                        <span className="font-share-tech" style={{ fontSize: 12, color: '#c07070' }}>
                          {causeLabel(ind.death_cause, lang)}
                        </span>
                        <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>·</span>
                        <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>{age.toFixed(0)} {text(lang as LangCode, { en: 'yr', tr: 'yaş', de: 'J.', fr: 'an', ar: 'سنة' })}</span>
                      </div>
                    </div>
                    <ChevronRight size={10} style={{ color: '#a05050', flexShrink: 0 }} />
                  </button>
                );
              })}
              {deadIndividuals.length > 100 && (
                <div className="text-center py-1">
                  <span className="font-share-tech" style={{ fontSize: 12, color: '#703030' }}>
                    +{deadIndividuals.length - 100} {text(lang as LangCode, { en: 'more', tr: 'daha', de: 'mehr', fr: 'de plus', ar: 'المزيد' })}
                  </span>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </DetailPanel>
  );
}
