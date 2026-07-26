import { useState, useEffect, useRef } from 'react';
import axios from 'axios';
import { authUrl } from '../utils/cloud';

/* ── helpers ─────────────────────────────────────────────────────────────── */
const toCm   = (v: number) => Math.round(150 + Math.max(0, Math.min(1, v)) * 45);
const fromCm = (cm: number) => Math.max(0, Math.min(1, (cm - 150) / 45));
const toKg   = (hv: number, mv: number) => { const h = toCm(hv)/100; return Math.round(h*h*(19+Math.max(0,Math.min(1,mv))*8)); };
const fromKg = (kg: number, hv: number) => { const h = toCm(hv)/100; return Math.max(0, Math.min(1, (kg/(h*h)-19)/8)); };

/* ── option lists ───────────────────────────────────────────────────────── */
const EYE_OPTS  = [
  {v:'brown', tr:'Kahverengi', en:'Brown', de:'Braun',      fr:'Marron',   ar:'بني',     c:'#6b3a1f'},
  {v:'hazel',  tr:'Ela',       en:'Hazel', de:'Haselnuss',  fr:'Noisette', ar:'بندقي',   c:'#8b6914'},
  {v:'green',  tr:'Yeşil',     en:'Green', de:'Grün',       fr:'Vert',     ar:'أخضر',    c:'#2d6a2d'},
  {v:'blue',   tr:'Mavi',      en:'Blue',  de:'Blau',       fr:'Bleu',     ar:'أزرق',    c:'#1a5276'},
];
const HAIR_OPTS = [
  {v:'black', tr:'Siyah', en:'Black', de:'Schwarz', fr:'Noir',   ar:'أسود',   c:'#111'},
  {v:'dark',  tr:'Koyu',  en:'Dark',  de:'Dunkel',  fr:'Foncé',  ar:'داكن',   c:'#2c1810'},
  {v:'brown', tr:'Kahve', en:'Brown', de:'Braun',   fr:'Marron', ar:'بني',    c:'#5c3317'},
  {v:'light', tr:'Açık',  en:'Light', de:'Hell',    fr:'Clair',  ar:'فاتح',   c:'#c68642'},
  {v:'blond', tr:'Sarı',  en:'Blond', de:'Blond',   fr:'Blond',  ar:'أشقر',   c:'#d4a017'},
  {v:'red',   tr:'Kızıl', en:'Red',   de:'Rot',     fr:'Roux',   ar:'أحمر',   c:'#8b2500'},
];
const SKIN_OPTS = [
  {v:'fair',  tr:'Açık',   en:'Fair',  de:'Hell',      fr:'Clair',    ar:'فاتح',    c:'#fde8d0'},
  {v:'light', tr:'Bej',    en:'Light', de:'Blass',     fr:'Pâle',     ar:'شاحب',    c:'#f5c9a0'},
  {v:'olive', tr:'Buğday', en:'Olive', de:'Oliv',      fr:'Olivâtre', ar:'زيتوني',  c:'#c68642'},
  {v:'tan',   tr:'Bronz',  en:'Tan',   de:'Gebräunt',  fr:'Bronzé',   ar:'برونزي',  c:'#a0614a'},
  {v:'brown', tr:'Esmer',  en:'Brown', de:'Braun',     fr:'Brun',     ar:'بني',     c:'#7b4a2d'},
  {v:'dark',  tr:'Koyu',   en:'Dark',  de:'Dunkel',    fr:'Foncé',    ar:'داكن',    c:'#3d1f0d'},
];

/* ── all genetics traits (20) ────────────────────────────────────────────── */
const ALL_TRAITS = [
  {id:'fluid_intelligence', tr:'Zeka',           en:'Intelligence',       de:'Intelligenz',         fr:'Intelligence',           ar:'الذكاء',             c:'#7c3aed', gTr:'ZİHİN — BİLİŞSEL', gEn:'MIND — COGNITIVE', gDe:'GEIST — KOGNITIV',    gFr:'ESPRIT — COGNITIF',    gAr:'العقل — المعرفي'},
  {id:'curiosity',          tr:'Merak',           en:'Curiosity',          de:'Neugier',             fr:'Curiosité',              ar:'الفضول',             c:'#f59e0b', gTr:'ZİHİN — BİLİŞSEL', gEn:'MIND — COGNITIVE', gDe:'GEIST — KOGNITIV',    gFr:'ESPRIT — COGNITIF',    gAr:'العقل — المعرفي'},
  {id:'language_capacity',  tr:'Dil Yeteneği',    en:'Language',           de:'Sprachtalent',        fr:'Aptitude Linguistique',  ar:'القدرة اللغوية',     c:'#14b8a6', gTr:'ZİHİN — BİLİŞSEL', gEn:'MIND — COGNITIVE', gDe:'GEIST — KOGNITIV',    gFr:'ESPRIT — COGNITIF',    gAr:'العقل — المعرفي'},
  {id:'learning_rate',      tr:'Öğrenme Hızı',    en:'Learning Rate',      de:'Lerngeschwindigkeit', fr:'Vitesse d\'Apprentissage',ar:'سرعة التعلم',        c:'#818cf8', gTr:'ZİHİN — BİLİŞSEL', gEn:'MIND — COGNITIVE', gDe:'GEIST — KOGNITIV',    gFr:'ESPRIT — COGNITIF',    gAr:'العقل — المعرفي'},
  {id:'conscientiousness',  tr:'Disiplin',        en:'Discipline',         de:'Gewissenhaftigkeit',  fr:'Conscience',             ar:'الانضباط',           c:'#3b82f6', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'self_awareness',     tr:'Öz Farkındalık',  en:'Self Awareness',     de:'Selbstwahrnehmung',   fr:'Conscience de Soi',      ar:'الوعي الذاتي',       c:'#8b5cf6', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'stress_resilience',  tr:'Stres Direnci',   en:'Stress Resil.',      de:'Stressresistenz',     fr:'Résilience au Stress',   ar:'مقاومة التوتر',      c:'#10b981', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'risk_tolerance',     tr:'Risk Toleransı',  en:'Risk Tolerance',     de:'Risikobereitschaft',  fr:'Tolérance au Risque',    ar:'تحمل المخاطر',       c:'#fb7185', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'innovation',         tr:'İnovasyon',       en:'Innovation',         de:'Innovation',          fr:'Innovation',             ar:'الابتكار',           c:'#e879f9', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'artistic_sense',     tr:'Sanat Duygusu',   en:'Art Sense',          de:'Kunstsinn',           fr:'Sens Artistique',        ar:'الحس الفني',         c:'#f97316', gTr:'ZİHİN — KİŞİLİK',  gEn:'MIND — CHARACTER', gDe:'GEIST — CHARAKTER',   gFr:'ESPRIT — CARACTÈRE',   gAr:'العقل — الشخصية'},
  {id:'empathy',            tr:'Empati',          en:'Empathy',            de:'Empathie',            fr:'Empathie',               ar:'التعاطف',            c:'#ec4899', gTr:'SOSYAL',            gEn:'SOCIAL',           gDe:'SOZIAL',               gFr:'SOCIAL',                gAr:'الاجتماعي'},
  {id:'social_bonding',     tr:'Sosyal Bağ',      en:'Social Bonding',     de:'Soziale Bindung',     fr:'Lien Social',            ar:'الترابط الاجتماعي',  c:'#f472b6', gTr:'SOSYAL',            gEn:'SOCIAL',           gDe:'SOZIAL',               gFr:'SOCIAL',                gAr:'الاجتماعي'},
  {id:'aggression',         tr:'Saldırganlık',    en:'Aggression',         de:'Aggression',          fr:'Agressivité',            ar:'العدوانية',          c:'#ef4444', gTr:'SOSYAL',            gEn:'SOCIAL',           gDe:'SOZIAL',               gFr:'SOCIAL',                gAr:'الاجتماعي'},
  {id:'cooperation',        tr:'İşbirliği',       en:'Cooperation',        de:'Kooperation',         fr:'Coopération',            ar:'التعاون',            c:'#34d399', gTr:'SOSYAL',            gEn:'SOCIAL',           gDe:'SOZIAL',               gFr:'SOCIAL',                gAr:'الاجتماعي'},
  {id:'dominance',          tr:'Liderlik Eğilimi',en:'Leadership',         de:'Führungsdrang',       fr:'Leadership',             ar:'الميل للقيادة',      c:'#fb923c', gTr:'SOSYAL',            gEn:'SOCIAL',           gDe:'SOZIAL',               gFr:'SOCIAL',                gAr:'الاجتماعي'},
  {id:'physical_strength',  tr:'Fiziksel Güç',    en:'Phys. Strength',     de:'Körperkraft',         fr:'Force Physique',         ar:'القوة البدنية',      c:'#fb923c', gTr:'BEDEN',             gEn:'BODY',             gDe:'KÖRPER',               gFr:'CORPS',                 gAr:'الجسد'},
  {id:'endurance',          tr:'Dayanıklılık',    en:'Endurance',          de:'Ausdauer',            fr:'Endurance',              ar:'التحمل',             c:'#fbbf24', gTr:'BEDEN',             gEn:'BODY',             gDe:'KÖRPER',               gFr:'CORPS',                 gAr:'الجسد'},
  {id:'immune_strength',    tr:'Bağışıklık',      en:'Immunity',           de:'Immunstärke',         fr:'Immunité',               ar:'المناعة',            c:'#22c55e', gTr:'BEDEN',             gEn:'BODY',             gDe:'KÖRPER',               gFr:'CORPS',                 gAr:'الجسد'},
  {id:'fertility',          tr:'Üreme Dürtüsü',   en:'Fertility',          de:'Fruchtbarkeit',       fr:'Fertilité',              ar:'الخصوبة',            c:'#f43f5e', gTr:'BEDEN',             gEn:'BODY',             gDe:'KÖRPER',               gFr:'CORPS',                 gAr:'الجسد'},
  {id:'longevity',          tr:'Uzun Ömür',       en:'Longevity',          de:'Langlebigkeit',       fr:'Longévité',              ar:'طول العمر',          c:'#84cc16', gTr:'BEDEN',             gEn:'BODY',             gDe:'KÖRPER',               gFr:'CORPS',                 gAr:'الجسد'},
];

/* ── trait metadata ─────────────────────────────────────────────────────── */
// r tuple: [trName,enName,deName,frName,arName, trDetail,enDetail,deDetail,frDetail,arDetail]
type L5  = [string,string,string,string,string];
type R10 = [string,string,string,string,string,string,string,string,string,string];
const LI: Record<string,number> = {tr:0,en:1,de:2,fr:3,ar:4};
type TM = {
  lo: L5; hi: L5;
  fx: L5[];
  d: [L5, L5, L5];
  r: [R10, R10, R10];
};
const TRAIT_META: TM[] = [
  /* 0 fluid_intelligence */ {
    lo:['Sıradan','Ordinary','Gewöhnlich','Ordinaire','عادي'],
    hi:['Dahi','Genius','Genie','Génie','عبقري'],
    fx:[['Teknoloji Hızı','Tech Speed','Technologietempo','Vitesse Tech.','سرعة التقنية'],['Bilimsel Keşif','Research','Forschung','Recherche','البحث العلمي'],['Stratejik Düşünce','Strategy','Strategie','Stratégie','التفكير الاستراتيجي']],
    d:[
      ['Temel ihtiyaçlara odaklanır; yenilik yavaş ve ağrılı gelişir.','Focused on basic needs; innovation is slow and painful.','Fokus auf Grundbedürfnisse; Innovation verläuft langsam.','Centré sur les besoins de base ; l\'innovation est lente.','التركيز على الاحتياجات الأساسية؛ الابتكار بطيء.'],
      ['Pratik zeka öne çıkar; dengeli ve ölçülü bir ilerleme sağlanır.','Practical intelligence dominates; steady, measured progress.','Praktische Intelligenz dominiert; stetiger Fortschritt.','L\'intelligence pratique domine ; progrès régulier.','الذكاء العملي يهيمن؛ تقدم ثابت ومتزن.'],
      ['Çağını aşan icatlar doğar; teknoloji sıçramaları yaşanır.','Era-defining inventions emerge; technological leaps occur.','Bahnbrechende Erfindungen entstehen; technologische Sprünge.','Des inventions marquantes émergent ; des bonds technologiques.','تظهر اختراعات تعريفية؛ تحدث قفزات تكنولوجية.'],
    ],
    r:[
      ['Neanderthal Toplulukları','Neanderthal Communities','Neandertaler-Gemeinschaften','Communautés néandertaliennes','مجتمعات نياندرتال','Temel araçlar ve ateş yeterli görülür; sembolik düşünce sınırlıdır.','Basic tools and fire suffice; symbolic thinking is limited.','Grundwerkzeuge und Feuer genügen; symbolisches Denken ist begrenzt.','Les outils de base et le feu suffisent ; la pensée symbolique est limitée.','الأدوات والنار كافية؛ التفكير الرمزي محدود.'],
      ['Orta Çağ Anadolu','Medieval Anatolia','Mittelalterliches Anatolien','Anatolie médiévale','الأناضول في العصور الوسطى','Pratik bilgi birikimi güçlü; teorik ilerleme yavaştır.','Practical knowledge is strong; theoretical advancement is slow.','Praktisches Wissen ist stark; theoretischer Fortschritt ist langsam.','La connaissance pratique est forte ; l\'avancement théorique est lent.','المعرفة العملية قوية؛ التقدم النظري بطيء.'],
      ['Atina · İskenderiye','Athens · Alexandria','Athen · Alexandria','Athènes · Alexandrie','أثينا · الإسكندرية','Felsefe, matematik ve doğa bilimi medeniyetin özünü oluşturur.','Philosophy, math and natural science form the core of civilization.','Philosophie, Mathematik und Naturwissenschaft bilden den Kern der Zivilisation.','La philosophie, les maths et les sciences forment le cœur de la civilisation.','الفلسفة والرياضيات والعلوم تشكل جوهر الحضارة.'],
    ],
  },
  /* 1 curiosity */ {
    lo:['Durağan','Stagnant','Stagnierend','Stagnant','راكد'],
    hi:['Kaşif','Explorer','Entdecker','Explorateur','مستكشف'],
    fx:[['Keşif Hızı','Exploration Rate','Entdeckungsrate','Taux d\'Exploration','معدل الاستكشاف'],['Ticaret Ağı','Trade Network','Handelsnetz','Réseau Commercial','شبكة التجارة'],['Kültürel Çeşitlilik','Cultural Diversity','Kulturelle Vielfalt','Diversité Culturelle','التنوع الثقافي']],
    d:[
      ['Topluluk kendi sınırlarına kapanır; değişime karşı direniş güçlüdür.','The community retreats inward; resistance to change is strong.','Die Gemeinschaft zieht sich zurück; Widerstand gegen Veränderung ist stark.','La communauté se replie ; la résistance au changement est forte.','المجتمع ينكفئ على نفسه؛ مقاومة التغيير قوية.'],
      ['Seçici keşif — pratik ve faydalı yeniliklere yönelir.','Selective exploration — focuses on practical innovations.','Selektive Erkundung — fokussiert auf praktische Innovationen.','Exploration sélective — axée sur les innovations pratiques.','استكشاف انتقائي — مركّز على الابتكارات العملية.'],
      ['Sınırsız merak; kıtalararası ticaret ve büyük keşifler hızlanır.','Boundless curiosity; intercontinental trade and great discoveries accelerate.','Grenzenlose Neugier; interkontinentaler Handel und Entdeckungen beschleunigen sich.','Curiosité sans bornes ; le commerce intercontinental et les grandes découvertes s\'accélèrent.','فضول لا حدود له؛ التجارة القارية والاكتشافات الكبرى تتسارع.'],
    ],
    r:[
      ['İzole Ada Toplulukları','Isolated Island Communities','Isolierte Inselgemeinschaften','Communautés insulaires isolées','مجتمعات الجزر المنعزلة','Dışarıdaki dünyadan habersiz; içe dönük döngüsel bir yaşam.','Unaware of the outside world; an inward, cyclical existence.','Unwissend über die Außenwelt; ein nach innen gerichtetes Dasein.','Ignorant du monde extérieur ; une existence intérieure cyclique.','غير مدركة للعالم الخارجي؛ وجود دوري منغلق.'],
      ['Anadolu Kervan Yolları','Anatolian Caravan Routes','Anatolische Karawanenrouten','Routes caravanières d\'Anatolie','طرق القوافل الأناضولية','Doğu ile Batı arasında bilgi ve mal akışı.','Flow of knowledge and goods between East and West.','Wissens- und Warenfluss zwischen Ost und West.','Flux de connaissances et de marchandises entre l\'Est et l\'Ouest.','تدفق المعرفة والبضائع بين الشرق والغرب.'],
      ['Fenikeli Tüccarlar','Phoenician Merchants','Phönizische Händler','Marchands phéniciens','التجار الفينيقيون','Akdeniz\'i aşan ticaret ağı; alfabe ve kültür taşıyıcısı.','A trade network spanning the Mediterranean; carrier of the alphabet and culture.','Ein Handelsnetz über das Mittelmeer; Träger des Alphabets und der Kultur.','Un réseau commercial couvrant la Méditerranée ; porteur de l\'alphabet et de la culture.','شبكة تجارية عبر المتوسط؛ ناقلة الأبجدية والثقافة.'],
    ],
  },
  /* 2 language_capacity */ {
    lo:['Kısıtlı','Limited','Eingeschränkt','Limité','محدود'],
    hi:['Hatip','Orator','Redner','Orateur','خطيب'],
    fx:[['Kültür Yayılımı','Culture Spread','Kulturverbreitung','Diffusion Culturelle','انتشار الثقافة'],['Diplomasi','Diplomacy','Diplomatie','Diplomatie','الدبلوماسية'],['Bilgi Aktarımı','Knowledge Transfer','Wissenstransfer','Transfert de Savoir','نقل المعرفة']],
    d:[
      ['Bilgi nesiller arası aktarılamaz; kurumlar zayıf ve kısa ömürlü kalır.','Knowledge cannot be passed between generations; institutions remain weak.','Wissen kann nicht zwischen Generationen weitergegeben werden; Institutionen bleiben schwach.','Le savoir ne peut être transmis entre générations ; les institutions restent faibles.','لا يمكن نقل المعرفة بين الأجيال؛ المؤسسات تبقى ضعيفة.'],
      ['Yazılı kültür gelişir; yasalar, destanlar ve tarih kayda alınır.','Written culture develops; laws, epics and history are recorded.','Schriftliche Kultur entwickelt sich; Gesetze, Epen und Geschichte werden aufgezeichnet.','La culture écrite se développe ; lois, épopées et histoire sont consignées.','تتطور الثقافة المكتوبة؛ تُسجَّل القوانين والملاحم والتاريخ.'],
      ['Felsefe, hukuk ve edebiyat medeniyetin çekirdeğine yerleşir.','Philosophy, law and literature settle at the core of civilization.','Philosophie, Recht und Literatur setzen sich als Kern der Zivilisation durch.','La philosophie, le droit et la littérature s\'installent au cœur de la civilisation.','الفلسفة والقانون والأدب تستقر في قلب الحضارة.'],
    ],
    r:[
      ['Tarih Öncesi Topluluklar','Pre-Historic Communities','Vorgeschichtliche Gemeinschaften','Communautés préhistoriques','المجتمعات ما قبل التاريخية','Yalnızca sözlü gelenek; hafıza nesil sınırlıdır.','Oral tradition only; memory is limited to one generation.','Nur mündliche Überlieferung; Gedächtnis auf eine Generation begrenzt.','Tradition orale uniquement ; la mémoire se limite à une génération.','التقليد الشفهي فقط؛ الذاكرة محدودة بجيل واحد.'],
      ['Sümer Yazıcıları','Sumerian Scribes','Sumerische Schreiber','Scribes sumériens','الكتاب السومريون','Çivi yazısı ile ticaret kayıtları ve yasalar kalıcı hale gelir.','Cuneiform makes trade records and laws permanent.','Keilschrift macht Handelsaufzeichnungen und Gesetze dauerhaft.','Le cunéiforme rend permanents les registres commerciaux et les lois.','الكتابة المسمارية تجعل السجلات والقوانين دائمة.'],
      ['Antik Atina · Osmanlı Divanı','Ancient Athens · Ottoman Divan','Antikes Athen · Osmanischer Divan','Athènes antique · Divan ottoman','أثينا القديمة · الديوان العثماني','Hatiplik, şiir ve retorik devlet yönetiminin ayrılmaz parçasıdır.','Oratory, poetry and rhetoric are inseparable from governance.','Beredsamkeit, Dichtung und Rhetorik sind untrennbar von der Staatsführung.','Éloquence, poésie et rhétorique sont indissociables de la gouvernance.','الخطابة والشعر والبلاغة لا تنفصل عن الحكم.'],
    ],
  },
  /* 3 learning_rate */ {
    lo:['Gelenekçi','Traditionalist','Traditionalist','Traditionaliste','تقليدي'],
    hi:['Adaptif','Adaptive','Adaptiv','Adaptable','تكيفي'],
    fx:[['Teknoloji Adaptasyonu','Tech Adoption','Technologieübernahme','Adoption Tech.','اعتماد التكنولوجيا'],['Kriz Yönetimi','Crisis Management','Krisenmanagement','Gestion de Crise','إدارة الأزمات'],['Nesil Dönüşümü','Generational Change','Generationenwandel','Changement Générationnel','التحول الجيلي']],
    d:[
      ['Değişime yavaş uyum; gelenekler baskın, yenilik toplumsal sürtüşmeye yol açar.','Slow adaptation; traditions dominate, innovation causes social friction.','Langsame Anpassung; Traditionen dominieren, Innovation verursacht soziale Reibung.','Adaptation lente ; les traditions dominent, l\'innovation crée des frictions sociales.','تكيف بطيء؛ التقاليد تهيمن، الابتكار يسبب احتكاكاً اجتماعياً.'],
      ['Dengeli öğrenme; istikrarlı ve sürdürülebilir bir ilerleme temposu.','Balanced learning; stable and sustainable pace of progress.','Ausgewogenes Lernen; stabiles und nachhaltiges Fortschrittstempo.','Apprentissage équilibré ; rythme de progrès stable et durable.','تعلم متوازن؛ وتيرة تقدم مستقرة ومستدامة.'],
      ['Kriz anında hızlı uyum; yabancı teknolojiler süratle benimsenir.','Rapid adaptation in crisis; foreign technologies adopted swiftly.','Schnelle Anpassung in der Krise; fremde Technologien werden schnell übernommen.','Adaptation rapide en crise ; les technologies étrangères sont adoptées rapidement.','تكيف سريع في الأزمات؛ التقنيات الأجنبية تُعتمد بسرعة.'],
    ],
    r:[
      ['Geç Bizans','Late Byzantium','Spätes Byzanz','Byzance tardive','بيزنطة المتأخرة','Yeni gelişmelere kapalı yapı; değişim çoğunlukla dışarıdan zorla gelir.','Closed to new developments; change usually comes forced from outside.','Für neue Entwicklungen geschlossen; Veränderung kommt meist von außen erzwungen.','Fermé aux nouveaux développements ; le changement vient généralement de l\'extérieur.','مغلق أمام المستجدات؛ التغيير يأتي مفروضاً من الخارج.'],
      ['Roma İmparatorluğu','Roman Empire','Römisches Reich','Empire romain','الإمبراطورية الرومانية','Fethedilen halklardan teknoloji ve kültür özümsenir.','Technology and culture absorbed from conquered peoples.','Technologie und Kultur werden von eroberten Völkern übernommen.','Technologie et culture absorbées des peuples conquis.','التكنولوجيا والثقافة مستوعبة من الشعوب المفتوحة.'],
      ['Meiji Japonya\'sı','Meiji Japan','Meiji-Japan','Japon Meiji','اليابان في عصر ميجي','Batı teknolojisi onlarca yılda özümsenerek yerel kimlikle harmanlanır.','Western technology absorbed in decades and blended with local identity.','Westliche Technologie in Jahrzehnten aufgenommen und mit lokaler Identität vermischt.','Technologie occidentale absorbée en décennies et mêlée à l\'identité locale.','التقنية الغربية مستوعبة في عقود وممزوجة بالهوية المحلية.'],
    ],
  },
  /* 4 conscientiousness */ {
    lo:['Özgür Ruh','Free Spirit','Freier Geist','Esprit Libre','روح حرة'],
    hi:['Disiplinli','Disciplined','Diszipliniert','Discipliné','منضبط'],
    fx:[['Üretim Verimliliği','Production Eff.','Produktionseffizienz','Efficacité Prod.','كفاءة الإنتاج'],['Ordu Gücü','Military Power','Militärstärke','Puissance Militaire','القوة العسكرية'],['Altyapı','Infrastructure','Infrastruktur','Infrastructure','البنية التحتية']],
    d:[
      ['Yaratıcı ama dağınık; uzun vadeli planlama ve örgütlenme güçleşir.','Creative but scattered; long-term planning and organization become difficult.','Kreativ aber zerstreut; langfristige Planung und Organisation werden schwierig.','Créatif mais dispersé ; la planification à long terme devient difficile.','مبدع لكن مشتت؛ التخطيط والتنظيم طويل الأمد يصبح صعباً.'],
      ['Hem üretim hem özgürlük bir arada — dengeli ve işlevsel bir yapı.','Production and freedom coexist — a balanced, functional structure.','Produktion und Freiheit koexistieren — eine ausgewogene, funktionale Struktur.','Production et liberté coexistent — une structure équilibrée et fonctionnelle.','الإنتاج والحرية يتعايشان — هيكل متوازن ووظيفي.'],
      ['Güçlü ordu ve altyapı; disiplin büyük medeniyetlerin çimentosudur.','Strong military and infrastructure; discipline is the cement of great civilizations.','Starkes Militär und Infrastruktur; Disziplin ist der Zement großer Zivilisationen.','Armée et infrastructure solides ; la discipline est le ciment des grandes civilisations.','جيش وبنية تحتية قوية؛ الانضباط إسمنت الحضارات العظيمة.'],
    ],
    r:[
      ['Göçebe Türk Boyları','Nomadic Turkish Tribes','Nomadische türkische Stämme','Tribus turques nomades','القبائل التركية الرحالة','Hızlı hareket ve özgürlük; kalıcı yapı ve kurumlar ikincil planda.','Swift movement and freedom; permanent structures and institutions are secondary.','Schnelle Bewegung und Freiheit; dauerhafte Strukturen sind sekundär.','Mouvement rapide et liberté ; les structures permanentes sont secondaires.','حركة سريعة وحرية؛ الهياكل الدائمة ثانوية.'],
      ['Abbasi Halifeliği','Abbasid Caliphate','Abbasidisches Kalifat','Califat abbasside','الخلافة العباسية','Bilim kurumları ile askeri disiplin dengeli biçimde var olur.','Scientific institutions and military discipline coexist in balance.','Wissenschaftliche Institutionen und militärische Disziplin koexistieren ausgewogen.','Institutions scientifiques et discipline militaire coexistent en équilibre.','المؤسسات العلمية والانضباط العسكري يتعايشان بتوازن.'],
      ['Roma Lejionu · Prusya','Roman Legion · Prussia','Römische Legion · Preußen','Légion romaine · Prusse','الفيلق الروماني · بروسيا','Disiplin, devlet örgütlenmesinin ve fetihlerinin temel taşıdır.','Discipline is the cornerstone of state organization and conquests.','Disziplin ist der Eckpfeiler der Staatsorganisation und der Eroberungen.','La discipline est la pierre angulaire de l\'organisation étatique et des conquêtes.','الانضباط حجر الزاوية في التنظيم الحكومي والفتوحات.'],
    ],
  },
  /* 5 self_awareness */ {
    lo:['İçgüdüsel','Instinctive','Instinktiv','Instinctif','غريزي'],
    hi:['Bilge','Wise','Weise','Sage','حكيم'],
    fx:[['Lider Kalitesi','Leader Quality','Führungsqualität','Qualité du Leader','جودة القيادة'],['Kriz Kararları','Crisis Decisions','Krisenentscheidungen','Décisions de Crise','قرارات الأزمات'],['Toplum Uyumu','Social Cohesion','Sozialer Zusammenhalt','Cohésion Sociale','تماسك المجتمع']],
    d:[
      ['Anlık tepkiler hakimdir; hatalar tekrarlanır, uzun vadeli sonuçlar öngörülmez.','Immediate reactions dominate; mistakes repeat, long-term outcomes are unseen.','Unmittelbare Reaktionen dominieren; Fehler wiederholen sich, langfristige Folgen werden nicht gesehen.','Les réactions immédiates dominent ; les erreurs se répètent, les résultats à long terme sont invisibles.','ردود الفعل الفورية تهيمن؛ الأخطاء تتكرر، العواقب بعيدة المدى لا تُرى.'],
      ['Tecrübeyle öğrenen, hatalardan ders çıkaran olgun bir liderlik kültürü.','A mature leadership culture that learns from experience and mistakes.','Eine reife Führungskultur, die aus Erfahrungen und Fehlern lernt.','Une culture de leadership mature qui apprend de l\'expérience et des erreurs.','ثقافة قيادية ناضجة تتعلم من التجربة والأخطاء.'],
      ['Derin felsefi gelenek; devlet kurumları kuşaklar boyu işlevini korur.','Deep philosophical tradition; state institutions function across generations.','Tiefe philosophische Tradition; staatliche Institutionen funktionieren über Generationen.','Tradition philosophique profonde ; les institutions étatiques fonctionnent à travers les générations.','تقليد فلسفي عميق؛ المؤسسات الحكومية تعمل عبر الأجيال.'],
    ],
    r:[
      ['Erken Orta Asya Göçebeleri','Early Central Asian Nomads','Frühe zentralasiatische Nomaden','Nomades d\'Asie centrale précoces','البدو الرحالة في آسيا الوسطى المبكرة','Kişisel kahramanlık ön planda; kurumsal süreklilik zayıf.','Personal heroism comes first; institutional continuity is weak.','Persönlicher Heroismus steht im Vordergrund; institutionelle Kontinuität ist schwach.','L\'héroïsme personnel est au premier plan ; la continuité institutionnelle est faible.','البطولة الشخصية في المقدمة؛ الاستمرارية المؤسسية ضعيفة.'],
      ['Selçuklu Devlet Geleneği','Seljuk State Tradition','Seldschukische Staatstradition','Tradition étatique seldjoukide','التقليد الحكومي السلجوقي','Tecrübeli vezirler ve danışmanlar devlet hafızasını korur.','Experienced viziers and advisors preserve institutional memory.','Erfahrene Wesire und Berater bewahren das institutionelle Gedächtnis.','Des vizirs et conseillers expérimentés préservent la mémoire institutionnelle.','الوزراء والمستشارون يحفظون الذاكرة المؤسسية.'],
      ['Konfüçyüs Çin\'i · Stoacı Roma','Confucian China · Stoic Rome','Konfuzianisches China · Stoisches Rom','Chine confucéenne · Rome stoïque','الصين الكونفوشيوسية · روما الرواقية','Öz disiplin ve bilgelik, iyi yönetimin temeli sayılır.','Self-discipline and wisdom are considered the basis of good governance.','Selbstdisziplin und Weisheit gelten als Grundlage guter Regierungsführung.','L\'autodiscipline et la sagesse sont considérées comme la base d\'une bonne gouvernance.','الانضباط الذاتي والحكمة أساس الحكم الرشيد.'],
    ],
  },
  /* 6 stress_resilience */ {
    lo:['Kırılgan','Fragile','Zerbrechlich','Fragile','هش'],
    hi:['Çelik','Steel','Stahl','Acier','صلب'],
    fx:[['Kriz Hayatta Kalma','Crisis Survival','Krisenüberleben','Survie en Crise','البقاء في الأزمات'],['Savaş Morali','Battle Morale','Kampfmoral','Moral au Combat','روح المعنويات القتالية'],['Çevre Adaptasyonu','Env. Adaptation','Umweltanpassung','Adaptation Env.','التكيف البيئي']],
    d:[
      ['Kıtlık ve savaşta toplum çabuk çöker; göç ya da çözülme kaçınılmaz.','Society collapses quickly under famine and war; migration or dissolution is inevitable.','Die Gesellschaft kollabiert schnell unter Hunger und Krieg; Migration oder Auflösung ist unvermeidlich.','La société s\'effondre rapidement sous la famine et la guerre ; la migration est inévitable.','المجتمع ينهار بسرعة تحت المجاعة والحرب؛ الهجرة أو التفكك حتمي.'],
      ['Ortalama krizleri atlatır; büyük yıkımların izi yüzyıl sürer.','Survives average crises; the mark of great catastrophes lasts a century.','Übersteht Durchschnittskrisen; das Mal großer Katastrophen dauert ein Jahrhundert.','Survit aux crises moyennes ; la marque des grandes catastrophes dure un siècle.','يتجاوز الأزمات العادية؛ أثر الكوارث الكبرى يدوم قرناً.'],
      ['Uzun savaşlara, salgınlara ve yıkımlara rağmen toplum ayakta kalır.','Society stands despite long wars, epidemics, and devastation.','Die Gesellschaft steht trotz langer Kriege, Epidemien und Verwüstungen.','La société résiste malgré de longues guerres, épidémies et dévastations.','المجتمع يصمد رغم الحروب الطويلة والأوبئة والدمار.'],
    ],
    r:[
      ['Küçük Ada Medeniyetleri','Small Island Civilizations','Kleine Inselzivilisationen','Petites civilisations insulaires','الحضارات الجزرية الصغيرة','İklim değişikliği ya da kuraklık karşısında savunmasız; çöküş ani.','Vulnerable to climate change or drought; collapse is sudden.','Anfällig für Klimawandel oder Dürre; der Zusammenbruch ist plötzlich.','Vulnérable aux changements climatiques ; l\'effondrement est soudain.','هشة أمام تغير المناخ؛ الانهيار مفاجئ.'],
      ['Bizans İmparatorluğu','Byzantine Empire','Byzantinisches Reich','Empire byzantin','الإمبراطورية البيزنطية','Yüzyıllarca kriz ve kuşatmalara rağmen direnç ve süreklilik.','Resilience and continuity despite centuries of crises and sieges.','Widerstandsfähigkeit und Kontinuität trotz jahrhundertelanger Krisen.','Résilience et continuité malgré des siècles de crises et de sièges.','صمود واستمرارية رغم قرون من الأزمات والحصارات.'],
      ['Osmanlı · Roma','Ottoman · Rome','Osmanisches Reich · Rom','Ottoman · Rome','العثمانيون · روما','Yıkıcı savaşlardan sonra dahi toparlanma ve yeniden inşa kapasitesi.','Capacity for recovery and reconstruction even after devastating wars.','Kapazität zur Erholung und zum Wiederaufbau auch nach verheerenden Kriegen.','Capacité de récupération et de reconstruction même après des guerres dévastatrices.','قدرة على الانتعاش وإعادة البناء حتى بعد الحروب المدمرة.'],
    ],
  },
  /* 7 risk_tolerance */ {
    lo:['Temkinli','Cautious','Vorsichtig','Prudent','حذر'],
    hi:['Cüretkar','Bold','Kühn','Audacieux','جريء'],
    fx:[['Fetih Seferleri','Conquest Campaigns','Feldzüge','Campagnes de Conquête','حملات الفتح'],['Ekonomik Yatırım','Economic Investment','Wirtschaftsinvestition','Investissement Écon.','الاستثمار الاقتصادي'],['Deniz Keşfi','Naval Exploration','Seeexploration','Exploration Navale','الاستكشاف البحري']],
    d:[
      ['Savunmacı strateji; büyüme yavaş ama toprak ve kaynak kaybı minimal.','Defensive strategy; growth is slow but territory and resource loss is minimal.','Defensive Strategie; Wachstum ist langsam, aber Gebiets- und Ressourcenverluste sind minimal.','Stratégie défensive ; la croissance est lente mais la perte de territoire est minimale.','استراتيجية دفاعية؛ النمو بطيء لكن الخسائر ضئيلة.'],
      ['Hesaplı riskler alınır; dengeli ve uzak görüşlü bir politika izlenir.','Calculated risks are taken; a balanced and far-sighted policy is pursued.','Kalkulierte Risiken werden eingegangen; eine ausgewogene und weitblickende Politik wird verfolgt.','Des risques calculés sont pris ; une politique équilibrée et prévoyante est poursuivie.','مخاطر محسوبة تُتخذ؛ سياسة متوازنة وبعيدة النظر.'],
      ['Hızlı genişleme; yüksek kazanım ya da büyük çöküş — orta yol azdır.','Rapid expansion; high gain or great collapse — there is little middle ground.','Schnelle Expansion; hoher Gewinn oder großer Zusammenbruch — es gibt wenig Mittelweg.','Expansion rapide ; gains élevés ou grand effondrement — peu de juste milieu.','توسع سريع؛ مكسب كبير أو انهيار كبير — القليل من الوسطية.'],
    ],
    r:[
      ['Savunmacı Çin','Defensive China','Defensives China','Chine défensive','الصين الدفاعية','Çin Seddi politikası: dışa kapalılık ve içsel istikrar önceliği.','Great Wall policy: closed to outside, prioritizing internal stability.','Chinesische Mauer-Politik: nach außen geschlossen, innere Stabilität hat Vorrang.','Politique de la Grande Muraille : fermé à l\'extérieur, priorité à la stabilité interne.','سياسة السور الصيني: منغلقة على الخارج، الأولوية للاستقرار الداخلي.'],
      ['Venedik Cumhuriyeti','Republic of Venice','Republik Venedig','République de Venise','جمهورية البندقية','Ticaret ve ittifakla büyüme; riskler titizlikle hesaplanır.','Growth through trade and alliances; risks are carefully calculated.','Wachstum durch Handel und Bündnisse; Risiken werden sorgfältig kalkuliert.','Croissance par le commerce et les alliances ; les risques sont soigneusement calculés.','النمو عبر التجارة والتحالفات؛ المخاطر تُحسب بعناية.'],
      ['İskender · Vikinglar','Alexander · Vikings','Alexander · Wikinger','Alexandre · Vikings','الإسكندر · الفايكنج','Bilinmeyene atılan cesur adım; ya imparatorluk ya yok oluş.','A bold step into the unknown; either empire or extinction.','Ein kühner Schritt ins Unbekannte; entweder Imperium oder Auslöschung.','Un pas audacieux vers l\'inconnu ; soit l\'empire, soit l\'extinction.','خطوة جريئة نحو المجهول؛ إما إمبراطورية أو فناء.'],
    ],
  },
  /* 8 innovation */ {
    lo:['Gelenekçi','Conservative','Konservativ','Conservateur','محافظ'],
    hi:['Öncü','Pioneer','Pionier','Pionnier','رائد'],
    fx:[['Teknoloji Ağacı','Technology Tree','Technologiebaum','Arbre Techno.','شجرة التكنولوجيا'],['Üretim Araçları','Production Tools','Produktionswerkzeuge','Outils de Prod.','أدوات الإنتاج'],['Mimari','Architecture','Architektur','Architecture','الهندسة المعمارية']],
    d:[
      ['Köklü gelenekler egemendir; yavaş ama sağlam, kademeli bir ilerleme var.','Deep traditions dominate; slow but solid, incremental progress.','Tiefe Traditionen dominieren; langsamer, aber solider, inkrementeller Fortschritt.','Les traditions profondes dominent ; progrès lent mais solide et progressif.','التقاليد العميقة تهيمن؛ تقدم بطيء لكن صلب وتدريجي.'],
      ['Pratik yenilikler ön planda; mevcut olanı sürekli iyileştirme zihniyeti.','Practical innovations take the lead; a mindset of continuous improvement.','Praktische Innovationen führen; eine Mentalität der kontinuierlichen Verbesserung.','Les innovations pratiques prennent le devant ; une mentalité d\'amélioration continue.','الابتكارات العملية في المقدمة؛ عقلية التحسين المستمر.'],
      ['Çağını aşan buluşlar; teknoloji ve bilimdeki atılımlar nesilleri şekillendirir.','Era-defining inventions; breakthroughs in technology and science shape generations.','Bahnbrechende Erfindungen; Durchbrüche in Technologie und Wissenschaft prägen Generationen.','Des inventions marquantes ; des percées en technologie et en science façonnent des générations.','اختراعات تعريفية؛ اختراقات في التكنولوجيا والعلوم تشكل الأجيال.'],
    ],
    r:[
      ['Geç Bizans','Late Byzantium','Spätes Byzanz','Byzance tardive','بيزنطة المتأخرة','Kurumsal muhafazakarlık; yenilik tehditlere karşı dondurulmuş.','Institutional conservatism; innovation frozen against perceived threats.','Institutioneller Konservatismus; Innovation gegen wahrgenommene Bedrohungen eingefroren.','Conservatisme institutionnel ; l\'innovation est figée face aux menaces perçues.','المحافظة المؤسسية؛ الابتكار مجمد في مواجهة التهديدات.'],
      ['İslam Altın Çağı','Islamic Golden Age','Islamisches Goldenes Zeitalter','Âge d\'or islamique','العصر الذهبي الإسلامي','Matematik, astronomi ve tıp alanında sistematik, birikimli ilerleme.','Systematic, cumulative progress in mathematics, astronomy and medicine.','Systematischer, kumulativer Fortschritt in Mathematik, Astronomie und Medizin.','Progrès systématique et cumulatif en mathématiques, astronomie et médecine.','تقدم منهجي في الرياضيات والفلك والطب.'],
      ['Rönesans İtalya\'sı · Bağdat','Renaissance Italy · Baghdad','Renaissanceitalien · Bagdad','Italie Renaissance · Bagdad','إيطاليا في عصر النهضة · بغداد','Sanat ve bilimin iç içe geçtiği yaratıcı patlama dönemi.','A creative explosion where art and science are intertwined.','Eine kreative Explosion, in der Kunst und Wissenschaft verflochten sind.','Une explosion créative où l\'art et la science sont entrelacés.','انفجار إبداعي تتشابك فيه الفنون والعلوم.'],
    ],
  },
  /* 9 artistic_sense */ {
    lo:['Sade','Austere','Schlicht','Austère','بسيط'],
    hi:['Estetik Usta','Aesthetic Master','Ästhetikmeister','Maître Esthétique','سيد الجمال'],
    fx:[['Kültür Puanı','Culture Score','Kulturpunkte','Score Culturel','نقاط الثقافة'],['Diplomatik Cazibe','Diplomatic Pull','Diplomatische Anziehung','Attrait Diplomatique','الجاذبية الدبلوماسية'],['Mimari Miras','Architectural Legacy','Architektonisches Erbe','Héritage Architectural','الإرث المعماري']],
    d:[
      ['İşlevsellik ön planda; estetik ikincil, süsleme yok denecek kadar az.','Functionality first; aesthetics are secondary, ornamentation nearly absent.','Funktionalität steht im Vordergrund; Ästhetik ist sekundär, Verzierung kaum vorhanden.','La fonctionnalité est prioritaire ; l\'esthétique est secondaire, l\'ornementation presque absente.','الوظيفية في المقدمة؛ الجماليات ثانوية، الزينة شبه غائبة.'],
      ['Kültürel kimlik oluşur; mimari, müzik ve el sanatları filiz verir.','Cultural identity forms; architecture, music and crafts begin to flourish.','Kulturelle Identität bildet sich; Architektur, Musik und Handwerk beginnen zu gedeihen.','L\'identité culturelle se forme ; l\'architecture, la musique et les arts commencent à s\'épanouir.','الهوية الثقافية تتشكل؛ العمارة والموسيقى والحرف تبدأ بالازدهار.'],
      ['Evrensel sanat mirası bırakılır; güzellik bizzat bir güç unsuru olur.','A universal artistic legacy is left; beauty itself becomes a source of power.','Ein universelles künstlerisches Erbe wird hinterlassen; Schönheit wird selbst zu einer Machtquelle.','Un héritage artistique universel est laissé ; la beauté elle-même devient source de pouvoir.','إرث فني عالمي يُخلَّف؛ الجمال نفسه يصبح مصدر قوة.'],
    ],
    r:[
      ['Erken Neolitik Topluluklar','Early Neolithic Communities','Frühe neolithische Gemeinschaften','Communautés néolithiques précoces','المجتمعات النيوليثية المبكرة','Araç ve barınak işlevsel; süslemeler henüz rudimenter.','Tools and shelters are functional; decorations are still rudimentary.','Werkzeuge und Unterkünfte sind funktional; Dekorationen sind noch rudimentär.','Les outils et abris sont fonctionnels ; les décorations sont encore rudimentaires.','الأدوات والملاجئ وظيفية؛ الزينة لا تزال ابتدائية.'],
      ['Anadolu Selçuklu Mimarisi','Anatolian Seljuk Architecture','Anatolische Seldschukenarchitektur','Architecture seldjoukide d\'Anatolie','العمارة السلجوقية الأناضولية','Geometrik bezeme ve çini sanatı zirvede; cami ve medrese ustalığı.','Geometric ornamentation and tile art at their peak; mastery of mosque and madrassa.','Geometrische Verzierung und Fliesenkunst auf ihrem Höhepunkt; Meisterschaft bei Moscheen.','L\'ornementation géométrique et l\'art de la faïence à leur apogée ; maîtrise de la mosquée.','الزخرفة الهندسية وفن القاشاني في ذروتهما؛ إتقان المساجد والمدارس.'],
      ['Osmanlı · Bizans','Ottoman · Byzantium','Osmanisches Reich · Byzanz','Ottoman · Byzance','العثمانيون · بيزنطة','Saray kültürü, mozaik ve minyatür — güzelliğin devlet politikası olduğu çağ.','Palace culture, mosaic and miniature — an era where beauty is state policy.','Palastkultur, Mosaik und Miniatur — eine Ära, in der Schönheit Staatspolitik ist.','Culture palatiale, mosaïque et miniature — une ère où la beauté est politique d\'État.','ثقافة القصر والفسيفساء والمنمنمات — حقبة تكون فيها الجمال سياسة دولة.'],
    ],
  },
  /* 10 empathy */ {
    lo:['Soğuk','Cold','Kalt','Froid','بارد'],
    hi:['Şefkatli','Compassionate','Mitfühlend','Compatissant','رحيم'],
    fx:[['Sosyal Uyum','Social Cohesion','Sozialer Zusammenhalt','Cohésion Sociale','التماسك الاجتماعي'],['Sınıf Çatışması','Class Conflict','Klassenkonflikt','Conflit de Classes','الصراع الطبقي'],['Hukuki Eşitlik','Legal Equality','Rechtliche Gleichheit','Égalité Juridique','المساواة القانونية']],
    d:[
      ['Güçlü hiyerarşi; sosyal eşitsizlik yaygın, zayıfların korunması sınırlı.','Strong hierarchy; social inequality is widespread, protection of the weak is limited.','Starke Hierarchie; soziale Ungleichheit ist weit verbreitet, Schutz der Schwachen ist begrenzt.','Forte hiérarchie ; l\'inégalité sociale est répandue, la protection des faibles est limitée.','هرمية قوية؛ التفاوت الاجتماعي واسع الانتشار، حماية الضعفاء محدودة.'],
      ['Sınırlı sosyal güvenlik ağı; orta sınıf kademeli biçimde gelişir.','A limited social safety net; the middle class develops gradually.','Ein begrenztes soziales Sicherheitsnetz; die Mittelschicht entwickelt sich allmählich.','Un filet de sécurité sociale limité ; la classe moyenne se développe progressivement.','شبكة أمان اجتماعي محدودة؛ الطبقة الوسطى تتطور تدريجياً.'],
      ['Güçlü yardımlaşma kültürü; iç çatışmalar azalır, uyum güçlenir.','A strong culture of mutual aid; internal conflicts decrease, cohesion strengthens.','Eine starke Kultur der gegenseitigen Hilfe; interne Konflikte nehmen ab, Zusammenhalt stärkt sich.','Une forte culture d\'entraide ; les conflits internes diminuent, la cohésion se renforce.','ثقافة قوية للتعاون المتبادل؛ الصراعات تتراجع، التماسك يتعزز.'],
    ],
    r:[
      ['Sparta Kast Sistemi','Spartan Caste System','Spartanisches Kastensystem','Système de castes spartiate','نظام الطبقات الإسبارطي','Helotlar sistematik baskı altında; güçlü olan hayatta kalır.','Helots under systematic oppression; the strong survive.','Heloten unter systematischer Unterdrückung; die Starken überleben.','Les hilotes sous oppression systématique ; les forts survivent.','الهيلوت تحت القمع المنهجي؛ الأقوياء يبقون.'],
      ['Orta Çağ Feodal Düzeni','Medieval Feudal Order','Mittelalterliche Feudalordnung','Ordre féodal médiéval','النظام الإقطاعي في العصور الوسطى','Kilise aracılığıyla sınırlı koruma; yardım erdem sayılır ama zorunlu değildir.','Limited protection through the Church; charity is a virtue but not obligatory.','Begrenzte Schutz durch die Kirche; Wohltätigkeit ist eine Tugend, aber nicht verpflichtend.','Protection limitée par l\'Église ; la charité est une vertu mais pas une obligation.','حماية محدودة عبر الكنيسة؛ الخير فضيلة لكن ليس إلزامياً.'],
      ['Endülüs İslam Medeniyeti','Andalusian Islamic Civilization','Andalusische islamische Zivilisation','Civilisation islamique andalouse','الحضارة الإسلامية الأندلسية','Üç dinin bir arada yaşadığı tolerans modeli; hoşgörü devlet politikası.','A model of tolerance where three faiths coexist; tolerance is state policy.','Ein Toleranzmodell, in dem drei Glaubensrichtungen koexistieren; Toleranz ist Staatspolitik.','Un modèle de tolérance où trois foi coexistent ; la tolérance est politique d\'État.','نموذج تعايش ثلاث ديانات؛ التسامح سياسة دولة.'],
    ],
  },
  /* 11 social_bonding */ {
    lo:['Bireyci','Individualist','Individualist','Individualiste','فردي'],
    hi:['Toplulukçu','Collectivist','Kollektivist','Collectiviste','جماعي'],
    fx:[['Koalisyon Gücü','Coalition Power','Koalitionsstärke','Puissance de Coalition','قوة التحالف'],['İç Savaş Riski','Civil War Risk','Bürgerkriegsrisiko','Risque de Guerre Civile','خطر الحرب الأهلية'],['Büyük Projeler','Grand Projects','Großprojekte','Grands Projets','المشاريع الكبرى']],
    d:[
      ['Güçlü bireyler; zayıf devlet kurumları ve parçalı, dağınık yapılar.','Strong individuals; weak state institutions and fragmented structures.','Starke Individuen; schwache staatliche Institutionen und fragmentierte Strukturen.','Individus forts ; institutions étatiques faibles et structures fragmentées.','أفراد أقوياء؛ مؤسسات حكومية ضعيفة وهياكل مشتتة.'],
      ['Aile ve klan bağları sağlam; devlet orta ölçekte çalışır.','Family and clan ties are solid; the state operates at a medium scale.','Familien- und Klanbindungen sind solide; der Staat arbeitet in mittlerem Maßstab.','Les liens familiaux et claniques sont solides ; l\'État opère à une échelle moyenne.','الروابط العائلية والقبلية متينة؛ الدولة تعمل على نطاق متوسط.'],
      ['Güçlü kolektif eylem; dev altyapı ve inşaat projeleri hayata geçer.','Strong collective action; grand infrastructure and construction projects come to life.','Starkes kollektives Handeln; große Infrastruktur- und Bauprojekte werden realisiert.','Action collective forte ; de grands projets d\'infrastructure et de construction voient le jour.','عمل جماعي قوي؛ مشاريع بنية تحتية وبناء ضخمة تُنفَّذ.'],
    ],
    r:[
      ['Bozkır Göçebe Toplulukları','Steppe Nomadic Communities','Steppen-Nomaden-Gemeinschaften','Communautés nomades des steppes','مجتمعات رحالة السهوب','Bireysel özgürlük yüksek; kalıcı kurumlar kurmak güç.','Individual freedom is high; establishing permanent institutions is difficult.','Individuelle Freiheit ist hoch; dauerhafte Institutionen zu gründen ist schwierig.','La liberté individuelle est élevée ; établir des institutions permanentes est difficile.','الحرية الفردية عالية؛ إنشاء مؤسسات دائمة صعب.'],
      ['Anadolu Köy Kültürü','Anatolian Village Culture','Anatolische Dorfkultur','Culture villageoise anatolienne','ثقافة القرية الأناضولية','İmece geleneği; klan ve komşuluk dayanışması toplumu ayakta tutar.','İmece tradition; clan and neighborhood solidarity keeps society standing.','İmece-Tradition; Stammes- und Nachbarschaftssolidarität hält die Gesellschaft aufrecht.','Tradition de l\'İmece ; la solidarité clanique et de voisinage maintient la société debout.','تقليد الإيميجه؛ تضامن القبيلة والجيران يحافظ على المجتمع.'],
      ['Eski Mısır · Çin İmparatorluğu','Ancient Egypt · Chinese Empire','Altes Ägypten · Chinesisches Reich','Égypte antique · Empire chinois','مصر القديمة · الإمبراطورية الصينية','Dev inşaat projeleri ve merkezi devlet — kolektif gücün doruk noktası.','Massive construction projects and central state — the pinnacle of collective power.','Riesige Bauprojekte und Zentralstaat — der Höhepunkt kollektiver Macht.','Projets de construction massifs et État central — le sommet du pouvoir collectif.','مشاريع بناء ضخمة ودولة مركزية — قمة القوة الجماعية.'],
    ],
  },
  /* 12 aggression */ {
    lo:['Barışçıl','Peaceful','Friedlich','Pacifique','سلمي'],
    hi:['Savaşçı','Warlike','Kriegerisch','Belliqueux','محارب'],
    fx:[['Askeri Güç','Military Power','Militärstärke','Puissance Militaire','القوة العسكرية'],['Toprak Genişlemesi','Territorial Expansion','Territoriale Expansion','Expansion Territoriale','التوسع الإقليمي'],['Diplomatik İlişkiler','Diplomatic Relations','Diplomatische Beziehungen','Relations Diplomatiques','العلاقات الدبلوماسية']],
    d:[
      ['Barışçıl ilişkiler; kültür, ticaret ve diplomasi ön plana geçer.','Peaceful relations; culture, trade and diplomacy take the foreground.','Friedliche Beziehungen; Kultur, Handel und Diplomatie treten in den Vordergrund.','Relations pacifiques ; la culture, le commerce et la diplomatie prennent le devant.','علاقات سلمية؛ الثقافة والتجارة والدبلوماسية تتصدر المشهد.'],
      ['Savunma güçlü; saldırı hesaplı ve stratejik olarak kullanılır.','Defense is strong; aggression is used in a calculated, strategic manner.','Verteidigung ist stark; Aggression wird kalkuliert und strategisch eingesetzt.','La défense est forte ; l\'agression est utilisée de manière calculée et stratégique.','الدفاع قوي؛ العدوانية تُستخدم بشكل محسوب واستراتيجي.'],
      ['Hızlı toprak genişlemesi; sürekli savaş hali toplumu ve kurumları şekillendirir.','Rapid territorial expansion; the permanent state of war shapes society and institutions.','Schnelle territoriale Expansion; der permanente Kriegszustand prägt Gesellschaft und Institutionen.','Expansion territoriale rapide ; l\'état de guerre façonne la société et les institutions.','توسع إقليمي سريع؛ حالة الحرب الدائمة تشكل المجتمع والمؤسسات.'],
    ],
    r:[
      ['Hitit Diplomasisi','Hittite Diplomacy','Hethitische Diplomatie','Diplomatie hittite','الدبلوماسية الحيثية','Tarihte bilinen ilk barış antlaşması; savaş yerine müzakere.','The first known peace treaty in history; negotiation instead of war.','Der erste bekannte Friedensvertrag in der Geschichte; Verhandlung statt Krieg.','Le premier traité de paix connu dans l\'histoire ; négociation au lieu de guerre.','أول معاهدة سلام في التاريخ؛ التفاوض عوضاً عن الحرب.'],
      ['Osmanlı Denge Politikası','Ottoman Balance Policy','Osmanische Ausgewogenheitspolitik','Politique d\'équilibre ottoman','سياسة التوازن العثمانية','Güç yoluyla barış; stratejik savaş ve diplomatik ittifakların dengesi.','Peace through power; a balance of strategic war and diplomatic alliances.','Frieden durch Macht; eine Balance von strategischem Krieg und diplomatischen Bündnissen.','Paix par la puissance ; un équilibre entre guerre stratégique et alliances diplomatiques.','السلام بالقوة؛ توازن بين الحرب والتحالفات الدبلوماسية.'],
      ['Moğol İmparatorluğu · Vikinglar','Mongol Empire · Vikings','Mongolisches Reich · Wikinger','Empire mongol · Vikings','الإمبراطورية المغولية · الفايكنج','Fetih kültürü; korku ve hız yoluyla geniş coğrafyalar denetim altına alınır.','A culture of conquest; vast geographies controlled through fear and speed.','Eine Kultur der Eroberung; weite Geographien durch Angst und Schnelligkeit kontrolliert.','Une culture de conquête ; de vastes géographies contrôlées par la peur et la vitesse.','ثقافة الفتح؛ رقعة شاسعة تُسيطر عليها بالخوف والسرعة.'],
    ],
  },
  /* 13 cooperation */ {
    lo:['Rekabetçi','Competitive','Wettbewerbsorientiert','Compétitif','تنافسي'],
    hi:['Dayanışmacı','Cooperative','Kooperativ','Coopératif','تعاوني'],
    fx:[['İttifak Ağı','Alliance Network','Allianznetzwerk','Réseau d\'Alliance','شبكة التحالفات'],['Kaynak Paylaşımı','Resource Sharing','Ressourcenteilung','Partage de Ressources','مشاركة الموارد'],['Ortak Projeler','Joint Projects','Gemeinsame Projekte','Projets Communs','المشاريع المشتركة']],
    d:[
      ['İç rekabet yenilik doğurur; ama kaynaklar dağınık ve verimsiz kullanılır.','Internal competition breeds innovation; but resources are used scattered and inefficiently.','Interner Wettbewerb erzeugt Innovation; aber Ressourcen werden verstreut und ineffizient genutzt.','La compétition interne engendre l\'innovation ; mais les ressources sont utilisées de façon dispersée.','المنافسة الداخلية تولد الابتكار؛ لكن الموارد تُستخدم بشكل مشتت وغير كفء.'],
      ['Seçici iş birliği; stratejik ittifaklar dikkatle kurulur ve korunur.','Selective cooperation; strategic alliances are carefully built and maintained.','Selektive Zusammenarbeit; strategische Allianzen werden sorgfältig aufgebaut und gepflegt.','Coopération sélective ; les alliances stratégiques sont soigneusement construites et maintenues.','تعاون انتقائي؛ التحالفات الاستراتيجية تُبنى بعناية وتُحافظ عليها.'],
      ['Güçlü ittifak ağları; ortak projeler medeniyetin ilerleyiş hızını artırır.','Strong alliance networks; joint projects accelerate the pace of civilizational progress.','Starke Allianznetzwerke; gemeinsame Projekte beschleunigen das Tempo des zivilisatorischen Fortschritts.','Réseaux d\'alliance forts ; les projets communs accélèrent le rythme du progrès civilisationnel.','شبكات تحالف قوية؛ المشاريع المشتركة تسرع وتيرة التقدم الحضاري.'],
    ],
    r:[
      ['İtalyan Şehir Devletleri','Italian City-States','Italienische Stadtstaaten','Cités-États italiennes','المدن-الدول الإيطالية','Floransa, Venedik, Cenova rekabeti: yenilik ateşleyici, güç dağıtık.','Florence, Venice, Genoa rivalry: competition ignites innovation, power remains dispersed.','Florenz, Venedig, Genua-Rivalität: Wettbewerb entfacht Innovation, Macht bleibt verteilt.','Rivalité Florence, Venise, Gênes : la compétition enflamme l\'innovation, le pouvoir reste dispersé.','تنافس فلورنسا والبندقية وجنوة: المنافسة تشعل الابتكار، القوة مشتتة.'],
      ['Hansa Birliği','Hanseatic League','Hansebund','Ligue hanséatique','رابطة هانسا','Kuzey Avrupa ticaret şehirlerinin ortak savunma ve ticaret ağı.','A joint defense and trade network of northern European merchant cities.','Ein gemeinsames Verteidigungs- und Handelsnetzwerk nordeuropäischer Handelsstädte.','Un réseau commun de défense et de commerce des villes marchandes d\'Europe du Nord.','شبكة دفاع وتجارة مشتركة لمدن التجارة في شمال أوروبا.'],
      ['Roma Müttefik Sistemi · Osmanlı Millet Düzeni','Roman Alliance System · Ottoman Millet Order','Römisches Bündnissystem · Osmanisches Millettsystem','Système d\'alliance romain · Ordre Millet ottoman','النظام التحالفي الروماني · نظام الملة العثماني','Farklı halkların ortak çatı altında örgütlenmesi — çoklukta birlik.','Organization of diverse peoples under a common roof — unity within diversity.','Organisation verschiedener Völker unter einem gemeinsamen Dach — Einheit in der Vielfalt.','Organisation de peuples divers sous un toit commun — unité dans la diversité.','تنظيم شعوب متنوعة تحت سقف مشترك — وحدة في التنوع.'],
    ],
  },
  /* 14 dominance */ {
    lo:['Eşitlikçi','Egalitarian','Egalitär','Égalitaire','مساواتي'],
    hi:['Otoriter','Authoritarian','Autoritär','Autoritaire','استبدادي'],
    fx:[['Devlet Yapısı','State Structure','Staatsstruktur','Structure de l\'État','هيكل الدولة'],['Karar Hızı','Decision Speed','Entscheidungsgeschwindigkeit','Vitesse de Décision','سرعة القرار'],['Ordu Komutası','Military Command','Militärkommando','Commandement Militaire','القيادة العسكرية']],
    d:[
      ['Demokratik eğilim; geniş katılımlı yavaş kararlar, çatışmalar uzlaşıyla çözülür.','Democratic tendency; wide-participation slow decisions, conflicts resolved by consensus.','Demokratische Tendenz; breit beteiligte langsame Entscheidungen, Konflikte durch Konsens gelöst.','Tendance démocratique ; décisions lentes à large participation, conflits résolus par consensus.','ميل ديمقراطي؛ قرارات بطيئة بمشاركة واسعة، الصراعات تُحلّ بالتوافق.'],
      ['Oligarşik yapı; liderler seçilir, yetki dengeli biçimde dağıtılır.','Oligarchic structure; leaders are chosen, power is balanced and distributed.','Oligarchische Struktur; Führer werden gewählt, Macht ist ausgewogen verteilt.','Structure oligarchique ; les dirigeants sont choisis, le pouvoir est équilibré et distribué.','هيكل أوليغارشي؛ القادة يُختارون، السلطة تُوزَّع بشكل متوازن.'],
      ['Güçlü merkezi otorite; hızlı kararlar ve sert yönetim büyük toplulukları yönetir.','Strong central authority; swift decisions and firm governance manage large communities.','Starke Zentralautorität; schnelle Entscheidungen und strenge Regierung führen große Gemeinschaften.','Autorité centrale forte ; des décisions rapides et une gouvernance ferme gèrent de grandes communautés.','سلطة مركزية قوية؛ قرارات سريعة وحوكمة صارمة تدير مجتمعات كبيرة.'],
    ],
    r:[
      ['Atina Demokrasisi','Athenian Democracy','Athenische Demokratie','Démocratie athénienne','الديمقراطية الأثينية','Agora tartışmaları; her yurttaşın söz hakkı, kararlar ağır alınır.','Agora debates; every citizen has a voice, decisions are taken slowly.','Agora-Debatten; jeder Bürger hat eine Stimme, Entscheidungen werden langsam getroffen.','Débats à l\'Agora ; chaque citoyen a voix au chapitre, les décisions sont prises lentement.','نقاشات الأغورا؛ لكل مواطن رأي، القرارات تُتَّخذ ببطء.'],
      ['Venedik Doge Konseyi','Venetian Doge Council','Venezianischer Dogenrat','Conseil du Doge de Venise','مجلس دوج البندقية','Aristokrat oy birliği; liderlik tekeli önlenir, denge korunur.','Aristocratic consensus; monopoly of leadership is prevented, balance is maintained.','Aristokratischer Konsens; Führungsmonopol wird verhindert, Balance wird aufrechterhalten.','Consensus aristocratique ; le monopole du leadership est évité, l\'équilibre est maintenu.','توافق أرستقراطي؛ احتكار القيادة يُمنع، التوازن يُصان.'],
      ['Osmanlı Sultanlığı · Moğol Han Sistemi','Ottoman Sultanate · Mongol Khan System','Osmanisches Sultanat · Mongolisches Khansystem','Sultanat ottoman · Système Khan mongol','السلطنة العثمانية · نظام خان المغول','Mutlak otorite; süratli seferber olma ve merkezi kaynak yönetimi.','Absolute authority; rapid mobilization and centralized resource management.','Absolute Autorität; schnelle Mobilisierung und zentralisiertes Ressourcenmanagement.','Autorité absolue ; mobilisation rapide et gestion centralisée des ressources.','سلطة مطلقة؛ تعبئة سريعة وإدارة مركزية للموارد.'],
    ],
  },
  /* 15 physical_strength */ {
    lo:['Narin','Slender','Zierlich','Élancé','رشيق'],
    hi:['Güçlü','Powerful','Kraftvoll','Puissant','قوي'],
    fx:[['Savaş Gücü','Combat Power','Kampfkraft','Puissance de Combat','قوة القتال'],['Tarım Üretimi','Agricultural Output','Landwirtschaftliche Prod.','Production Agricole','الإنتاج الزراعي'],['İnşaat Kapasitesi','Construction Cap.','Baukapazität','Capacité de Construction','طاقة البناء']],
    d:[
      ['Strateji, zeka ve teknoloji öne çıkar; kaba kuvvetten ziyade yetkinlik belirleyici.','Strategy, intelligence and technology come to the fore; skill rather than brute force is decisive.','Strategie, Intelligenz und Technologie treten in den Vordergrund; Können statt roher Gewalt ist entscheidend.','Stratégie, intelligence et technologie émergent ; la compétence plutôt que la force brute est décisive.','الاستراتيجية والذكاء والتكنولوجيا تتصدر؛ المهارة وليس القوة الغاشمة هي الحاسمة.'],
      ['Dengeli fizik; hem tarımda hem muharebe alanında yeterli kapasite.','Balanced physique; sufficient capacity in both agriculture and combat.','Ausgewogener Körperbau; ausreichende Kapazität sowohl in der Landwirtschaft als auch im Kampf.','Physique équilibrée ; capacité suffisante tant dans l\'agriculture que dans le combat.','بنية جسدية متوازنة؛ طاقة كافية في الزراعة والقتال.'],
      ['Yüksek üretim ve savaş kapasitesi; büyük yapılar ve uzun seferler mümkün.','High production and combat capacity; great structures and long campaigns become possible.','Hohe Produktions- und Kampfkapazität; große Strukturen und lange Feldzüge werden möglich.','Haute capacité de production et de combat ; de grandes structures et de longues campagnes deviennent possibles.','طاقة عالية للإنتاج والقتال؛ الهياكل الكبرى والحملات الطويلة تصبح ممكنة.'],
    ],
    r:[
      ['Bizans Strateji Odaklı Ordu','Byzantine Strategic Army','Byzantinische Strategiearmee','Armée stratégique byzantine','الجيش البيزنطي الاستراتيجي','Sayısal üstünlük yerine taktik üstünlük; az kuvvetle çok başarı.','Tactical superiority over numerical strength; great success with few forces.','Taktische Überlegenheit über numerische Stärke; großer Erfolg mit wenigen Kräften.','Supériorité tactique sur la force numérique ; grand succès avec peu de forces.','التفوق التكتيكي على القوة العددية؛ نجاح كبير بقوات قليلة.'],
      ['Osmanlı Yeniçeri Ocağı','Ottoman Janissary Corps','Osmanisches Janitscharenkorps','Corps des janissaires ottomans','الإنكشارية العثمانية','Eğitim ve fiziksel kapasite bir arada; disiplinli ağır piyade.','Training and physical capacity together; disciplined heavy infantry.','Training und physische Kapazität zusammen; disziplinierte schwere Infanterie.','Formation et capacité physique ensemble ; infanterie lourde disciplinée.','التدريب والقدرة البدنية معاً؛ مشاة ثقيلة منضبطة.'],
      ['Spartalı Savaşçılar · Moğol Atlıları','Spartan Warriors · Mongol Cavalry','Spartanische Krieger · Mongolische Kavallerie','Guerriers spartiates · Cavalerie mongole','المقاتلون الإسبارطيون · الفرسان المغول','Fizik üstünlük medeniyetin temeli; savaşa adanmış toplum modeli.','Physical superiority is the foundation of civilization; a society dedicated to war.','Physische Überlegenheit ist das Fundament der Zivilisation; eine dem Krieg gewidmete Gesellschaft.','La supériorité physique est le fondement de la civilisation ; une société dédiée à la guerre.','التفوق الجسدي أساس الحضارة؛ نموذج مجتمع مكرَّس للحرب.'],
    ],
  },
  /* 16 endurance */ {
    lo:['Kırılgan','Fragile','Zerbrechlich','Fragile','هش'],
    hi:['Yılmaz','Relentless','Unermüdlich','Infatigable','لا يكل'],
    fx:[['Uzun Sefer Kapasitesi','Long Campaign Cap.','Kapazität langer Feldzüge','Cap. de Longue Campagne','طاقة الحملات الطويلة'],['Kıtlık Direnci','Famine Resistance','Hungerresistenz','Résistance à la Famine','مقاومة المجاعة'],['Nüfus Büyümesi','Population Growth','Bevölkerungswachstum','Croissance Démographique','النمو السكاني']],
    d:[
      ['Kısa seferler ve şehir savunması tercih edilir; yorgunluk ve hastalık hızlı girer.','Short campaigns and city defense are preferred; fatigue and disease set in quickly.','Kurze Feldzüge und Stadtverteidigung werden bevorzugt; Erschöpfung und Krankheit setzen schnell ein.','Les campagnes courtes et la défense des villes sont préférées ; la fatigue et la maladie s\'installent rapidement.','الحملات القصيرة والدفاع عن المدن هي المفضلة؛ التعب والمرض يتسربان بسرعة.'],
      ['Orta mesafe seferler; kıtlık kısmen atlatılır, toplum büyük kayıplar verir.','Medium-range campaigns; famine is partially overcome, society suffers major losses.','Mittelstreckenfeldzüge; Hunger wird teilweise überwunden, die Gesellschaft erleidet große Verluste.','Campagnes à moyenne portée ; la famine est partiellement surmontée, la société subit de lourdes pertes.','حملات متوسطة المدى؛ المجاعة تُتجاوز جزئياً، المجتمع يعاني خسائر كبيرة.'],
      ['Kıtalararası seferler ve aşırı koşullarda hayatta kalış; toplum zorlukla güçlenir.','Intercontinental campaigns and survival in extreme conditions; society grows stronger through hardship.','Interkontinentale Feldzüge und Überleben unter extremen Bedingungen; die Gesellschaft wird durch Härten stärker.','Campagnes intercontinentales et survie dans des conditions extrêmes ; la société se renforce à travers les épreuves.','حملات بين القارات والبقاء في ظروف قاسية؛ المجتمع يشتد من خلال الصعاب.'],
    ],
    r:[
      ['Küçük Akdeniz Şehir Devletleri','Small Mediterranean City-States','Kleine Mittelmeerstadtstaaten','Petites cités-États méditerranéennes','المدن-الدول المتوسطية الصغيرة','Kısa mesafeli ticaret ve savunma; uzun mesafeli güç projeksiyonu sınırlı.','Short-range trade and defense; long-range power projection is limited.','Kurzstreckiger Handel und Verteidigung; Machtprojektion über lange Distanz ist begrenzt.','Commerce et défense à courte portée ; la projection de puissance à longue portée est limitée.','تجارة ودفاع قصيرا المدى؛ إسقاط القوة على مدى بعيد محدود.'],
      ['Selçuklu Sefer Ordusu','Seljuk Campaign Army','Seldschukische Feldzugsarmee','Armée de campagne seldjoukide','جيش الحملات السلجوقية','At ve deve ile uzun bozkır seferleri; lojistik zorlukları aşma kapasitesi.','Long steppe campaigns with horse and camel; capacity to overcome logistical challenges.','Lange Steppenfeldzüge mit Pferd und Kamel; Kapazität zur Überwindung logistischer Herausforderungen.','Longues campagnes de steppe avec cheval et chameau ; capacité à surmonter les défis logistiques.','حملات سهوب طويلة بالخيل والإبل؛ القدرة على تجاوز التحديات اللوجستية.'],
      ['Moğol · Türk Büyük Göç Dalgaları','Mongol · Turkish Great Migrations','Mongolische · Türkische Große Migrationen','Grandes migrations mongoles · turques','هجرات المغول والأتراك الكبرى','Binlerce kilometre at sırtında; iklim ve coğrafya engel tanınmaz.','Thousands of kilometers on horseback; climate and geography pose no barriers.','Tausende Kilometer zu Pferd; Klima und Geographie stellen keine Hindernisse dar.','Des milliers de kilomètres à cheval ; le climat et la géographie ne posent aucun obstacle.','آلاف الكيلومترات على الخيل؛ المناخ والجغرافيا لا تشكلان عائقاً.'],
    ],
  },
  /* 17 immune_strength */ {
    lo:['Hassas','Delicate','Empfindlich','Délicat','حساس'],
    hi:['Gürbüz','Robust','Robust','Robuste','متين'],
    fx:[['Salgın Direnci','Epidemic Resistance','Seuchenresistenz','Résistance Épidémique','مقاومة الأوبئة'],['Nüfus İstikrarı','Population Stability','Bevölkerungsstabilität','Stabilité Démographique','استقرار السكان'],['Coğrafi Yayılım','Geographic Spread','Geographische Ausbreitung','Extension Géographique','الانتشار الجغرافي']],
    d:[
      ['Salgın hastalıklar toplumda yıkıcı etki yaratır; nüfus çabuk erir.','Epidemic diseases have a devastating effect on society; population erodes quickly.','Seuchenkrankheiten haben verheerende Auswirkungen auf die Gesellschaft; die Bevölkerung erodiert schnell.','Les maladies épidémiques ont un effet dévastateur sur la société ; la population s\'érode rapidement.','الأمراض الوبائية لها أثر مدمر على المجتمع؛ السكان يتآكلون بسرعة.'],
      ['Endemik hastalıklarla denge kurulur; kayıplar acı ama yönetilebilir.','Balance is struck with endemic diseases; losses are painful but manageable.','Balance wird mit endemischen Krankheiten hergestellt; Verluste sind schmerzhaft, aber handhabbar.','L\'équilibre est trouvé avec les maladies endémiques ; les pertes sont douloureuses mais gérables.','التوازن يُضرب مع الأمراض المستوطنة؛ الخسائر مؤلمة لكن قابلة للإدارة.'],
      ['Nüfus istikrarı güçlü; salgınlar görece hızlı atlatılır, toplum toparlanır.','Population stability is strong; epidemics are overcome relatively quickly, society recovers.','Bevölkerungsstabilität ist stark; Epidemien werden relativ schnell überwunden, die Gesellschaft erholt sich.','La stabilité démographique est forte ; les épidémies sont surmontées relativement rapidement.','استقرار السكان قوي؛ الأوبئة تُتجاوز بسرعة نسبية، المجتمع يتعافى.'],
    ],
    r:[
      ['Kara Ölüm Sonrası Avrupa','Post-Black Death Europe','Post-Schwarzer-Tod-Europa','Europe post-Mort noire','أوروبا ما بعد الموت الأسود','Nüfusun üçte birini silen veba; kültürel ve ekonomik çöküş dönemi.','A plague that wiped out a third of the population; a period of cultural and economic collapse.','Eine Pest, die ein Drittel der Bevölkerung auslöschte; eine Zeit des kulturellen und wirtschaftlichen Zusammenbruchs.','Une peste qui anéantit un tiers de la population ; une période d\'effondrement culturel et économique.','طاعون أبادت ثلث السكان؛ فترة انهيار ثقافي واقتصادي.'],
      ['Orta Çağ Anadolu Kentleri','Medieval Anatolian Cities','Mittelalterliche anatolische Städte','Villes anatoliennes médiévales','المدن الأناضولية في العصور الوسطى','Doğu-Batı ticaretinin buluşma noktası; salgınlara maruz ama dirençli.','The meeting point of East-West trade; exposed to epidemics but resilient.','Der Treffpunkt des Ost-West-Handels; Epidemien ausgesetzt, aber widerstandsfähig.','Le point de rencontre du commerce Est-Ouest ; exposé aux épidémies mais résilient.','نقطة تلاقي تجارة الشرق والغرب؛ عرضة للأوبئة لكنها صامدة.'],
      ['Osmanlı Şehirleri (erken dönem)','Ottoman Cities (Early Period)','Osmanische Städte (Frühperiode)','Villes ottomanes (première période)','المدن العثمانية (الفترة المبكرة)','Karantina uygulamaları erken benimsendi; görece düşük salgın ölüm oranı.','Quarantine practices adopted early; relatively low epidemic mortality rates.','Quarantänepraktiken früh übernommen; relativ niedrige Seuchenmortalitätsraten.','Pratiques de quarantaine adoptées tôt ; taux de mortalité épidémique relativement faible.','ممارسات الحجر الصحي اعتُمدت مبكراً؛ معدلات وفيات وبائية منخفضة نسبياً.'],
    ],
  },
  /* 18 fertility */ {
    lo:['Kontrollü','Controlled','Kontrolliert','Contrôlé','مضبوط'],
    hi:['Verimli','Prolific','Fruchtbar','Prolifique','خصب'],
    fx:[['Nüfus Artış Hızı','Population Growth','Bevölkerungswachstumsrate','Taux de Croissance Démog.','معدل النمو السكاني'],['Yayılma Kapasitesi','Expansion Capacity','Expansionskapazität','Capacité d\'Expansion','طاقة التوسع'],['Kaynak Baskısı','Resource Pressure','Ressourcendruck','Pression sur les Ressources','ضغط الموارد']],
    d:[
      ['Yavaş nüfus artışı; kaynaklara baskı az, yaşam kalitesi yüksek tutulabilir.','Slow population growth; low pressure on resources, high quality of life is maintainable.','Langsames Bevölkerungswachstum; geringer Ressourcendruck, hohe Lebensqualität ist aufrechtzuerhalten.','Croissance démographique lente ; faible pression sur les ressources, qualité de vie élevée est maintenable.','نمو سكاني بطيء؛ ضغط منخفض على الموارد، جودة الحياة العالية قابلة للصون.'],
      ['Dengeli büyüme; istikrarlı ve sürdürülebilir coğrafi genişleme.','Balanced growth; stable and sustainable geographic expansion.','Ausgewogenes Wachstum; stabile und nachhaltige geografische Expansion.','Croissance équilibrée ; expansion géographique stable et durable.','نمو متوازن؛ توسع جغرافي مستقر ومستدام.'],
      ['Hızlı nüfus patlaması; toprak ve kaynak ihtiyacı baskı yaratır, göç artar.','Rapid population explosion; need for land and resources creates pressure, migration increases.','Schnelle Bevölkerungsexplosion; Bedarf an Land und Ressourcen erzeugt Druck, Migration nimmt zu.','Explosion démographique rapide ; le besoin de terre et de ressources crée une pression, la migration augmente.','انفجار سكاني سريع؛ الحاجة للأرض والموارد تخلق ضغطاً، الهجرة تزداد.'],
    ],
    r:[
      ['Antik Yunan Şehir Devletleri','Ancient Greek City-States','Antike griechische Stadtstaaten','Cités-États grecques antiques','المدن-الدول اليونانية القديمة','Nüfus kontrolü bilinçli uygulandı; toprak sınırlı, kalite ön planda.','Population control was consciously practiced; land is limited, quality comes first.','Bevölkerungskontrolle wurde bewusst praktiziert; Land ist begrenzt, Qualität hat Vorrang.','Le contrôle démographique était consciemment pratiqué ; la terre est limitée, la qualité prime.','ضبط النسل مورس بوعي؛ الأرض محدودة، الجودة في المقدمة.'],
      ['Osmanlı Klasik Dönemi','Ottoman Classical Period','Osmanische Klassikperiode','Période classique ottomane','العصر العثماني الكلاسيكي','Dengeli demografik büyüme; sürdürülebilir toprak ve kaynak yönetimi.','Balanced demographic growth; sustainable land and resource management.','Ausgewogenes demografisches Wachstum; nachhaltiges Land- und Ressourcenmanagement.','Croissance démographique équilibrée ; gestion durable des terres et des ressources.','نمو ديموغرافي متوازن؛ إدارة مستدامة للأراضي والموارد.'],
      ['Erken İslam Fetihleri','Early Islamic Conquests','Frühe islamische Eroberungen','Premières conquêtes islamiques','الفتوحات الإسلامية المبكرة','Demografik dinamizm ve genç nüfus; fetihlerin arkasındaki insan gücü.','Demographic dynamism and a young population; the human power behind the conquests.','Demografischer Dynamismus und junge Bevölkerung; die menschliche Kraft hinter den Eroberungen.','Dynamisme démographique et population jeune ; la puissance humaine derrière les conquêtes.','الديناميكية الديموغرافية والسكان الشباب؛ القوة البشرية وراء الفتوحات.'],
    ],
  },
  /* 19 longevity */ {
    lo:['Kısa Ömürlü','Short-Lived','Kurzlebig','Éphémère','قصير العمر'],
    hi:['Uzun Ömürlü','Long-Lived','Langlebig','Longévif','طويل العمر'],
    fx:[['Kurumsal Hafıza','Institutional Memory','Institutionelles Gedächtnis','Mémoire Institutionnelle','الذاكرة المؤسسية'],['Nesil Aktarımı','Gen. Transfer','Generationsübertragung','Transfert Génér.','نقل الأجيال'],['Bilge Lider Süresi','Elder Leadership','Ältere Führung','Direction des Anciens','قيادة الكبار']],
    d:[
      ['Tecrübe birikmeden nesil değişir; kurumlar zayıf ve kısa ömürlü kalır.','Generations change before experience accumulates; institutions remain weak and short-lived.','Generationen wechseln, bevor Erfahrung aufgebaut wird; Institutionen bleiben schwach und kurzlebig.','Les générations changent avant que l\'expérience ne s\'accumule ; les institutions restent faibles et éphémères.','الأجيال تتغير قبل أن تتراكم التجربة؛ المؤسسات تبقى ضعيفة وقصيرة الأجل.'],
      ['Orta yaşam süresi; bilgi ve tecrübe kuşaklar arası yavaşça aktarılır.','Medium lifespan; knowledge and experience are slowly transferred between generations.','Mittlere Lebensdauer; Wissen und Erfahrung werden langsam zwischen Generationen übertragen.','Durée de vie moyenne ; les connaissances et l\'expérience sont lentement transmises entre générations.','متوسط عمر؛ المعرفة والتجربة تُنقل ببطء بين الأجيال.'],
      ['Uzun ömürlü liderler ve güçlü kurumsal hafıza; medeniyetin sürekliliği güvence altında.','Long-lived leaders and strong institutional memory; the continuity of civilization is secured.','Langlebige Führer und starkes institutionelles Gedächtnis; die Kontinuität der Zivilisation ist gesichert.','Des dirigeants longévifs et une forte mémoire institutionnelle ; la continuité de la civilisation est assurée.','قادة طويلو العمر وذاكرة مؤسسية قوية؛ استمرارية الحضارة مضمونة.'],
    ],
    r:[
      ['Erken Göçebe Topluluklar','Early Nomadic Communities','Frühe Nomadengemeinschaften','Communautés nomades précoces','المجتمعات الرحالة المبكرة','Kısa ortalama yaşam; bilgelik aktarımı sözlü ve sınırlı.','Short average lifespan; wisdom transfer is oral and limited.','Kurze durchschnittliche Lebensdauer; Weisheitsübertragung ist mündlich und begrenzt.','Durée de vie moyenne courte ; le transfert de sagesse est oral et limité.','متوسط عمر قصير؛ نقل الحكمة شفهي ومحدود.'],
      ['Osmanlı Vezir Sistemi','Ottoman Vizier System','Osmanisches Wesirsystem','Système du vizir ottoman','نظام الوزراء العثماني','Tecrübeli bürokrasi; devlet hafızası kurumsal kanallarla korunur.','Experienced bureaucracy; state memory is preserved through institutional channels.','Erfahrene Bürokratie; Staatsgedächtnis wird durch institutionelle Kanäle bewahrt.','Bureaucratie expérimentée ; la mémoire de l\'État est préservée par des canaux institutionnels.','بيروقراطية ذات خبرة؛ ذاكرة الدولة تُحفظ عبر القنوات المؤسسية.'],
      ['Çin İmparatorluk Bürokrasisi · Roma Senatosu','Chinese Imperial Bureaucracy · Roman Senate','Chinesische Kaiserliche Bürokratie · Römischer Senat','Bureaucratie impériale chinoise · Sénat romain','البيروقراطية الإمبراطورية الصينية · مجلس الشيوخ الروماني','Nesiller boyu süren kurumsal derinlik — medeniyetin güvencesi.','Institutional depth spanning generations — the guarantee of civilization.','Institutionelle Tiefe über Generationen — die Garantie der Zivilisation.','Profondeur institutionnelle s\'étendant sur des générations — la garantie de la civilisation.','عمق مؤسسي يمتد عبر الأجيال — ضمانة الحضارة.'],
    ],
  },
];

const TRAIT_DEFAULTS = Object.fromEntries(ALL_TRAITS.map(t => [t.id, 0.5]));

export const founderDefaults = (sex: 'male'|'female') => ({
  name: sex==='male' ? 'Alp Anatol' : 'Ayla Anatol',
  ageYears: sex==='male' ? 22 : 20,
  sex,
  eye_color:'brown', hair_color: sex==='male' ? 'dark' : 'brown', skin_tone:'olive',
  height: sex==='male' ? 0.56 : 0.44, metabolism: 0.45,
  ...TRAIT_DEFAULTS,
  fluid_intelligence:0.68, curiosity:0.60, conscientiousness:0.72,
  language_capacity:0.55,  artistic_sense:0.50, self_awareness:0.55,
  stress_resilience:0.65,  empathy:0.60,  social_bonding:0.75,
  aggression:0.35, cooperation:0.72, dominance:0.50,
  physical_strength:0.72,  endurance:0.70, immune_strength:0.74,
  fertility:0.80, longevity:0.68, learning_rate:0.65,
  risk_tolerance:0.45, innovation:0.55,
});

/* ── step definitions ────────────────────────────────────────────────────── */
type StepDef =
  | { type: 'sim-info' }
  | { type: 'identity';   f: 1|2 }
  | { type: 'physical';   f: 1|2 }
  | { type: 'appearance'; f: 1|2 }
  | { type: 'trait';      f: 1|2; idx: number }
  | { type: 'summary' };

const STEPS: StepDef[] = [
  { type: 'sim-info' },
  { type: 'identity',   f: 1 },
  { type: 'physical',   f: 1 },
  { type: 'appearance', f: 1 },
  ...ALL_TRAITS.map((_, i) => ({ type: 'trait' as const, f: 1 as const, idx: i })),
  { type: 'identity',   f: 2 },
  { type: 'physical',   f: 2 },
  { type: 'appearance', f: 2 },
  ...ALL_TRAITS.map((_, i) => ({ type: 'trait' as const, f: 2 as const, idx: i })),
  { type: 'summary' },
];
const TOTAL = STEPS.length;

/* ── shared styles ───────────────────────────────────────────────────────── */
const CLIP = 'polygon(0 0,calc(100% - 6px) 0,100% 6px,100% 100%,6px 100%,0 calc(100% - 6px))';
const inputBase: React.CSSProperties = {
  outline:'none', fontFamily:'Share Tech Mono,monospace',
  background:'rgba(7,7,26,0.9)', border:'1px solid rgba(79,110,247,0.25)',
  padding:'9px 12px', fontSize:14, color:'#e0e0f0', clipPath:CLIP, width:'100%',
};
const btnBase: React.CSSProperties = {
  fontFamily:'Share Tech Mono,monospace', letterSpacing:'0.12em', fontSize:16,
  clipPath:CLIP, cursor:'pointer', padding:'9px 20px',
};
const btnNext  = { ...btnBase, background:'rgba(79,110,247,0.25)', border:'1px solid rgba(79,110,247,0.5)',  color:'#e0e0f0' };
const btnBack  = { ...btnBase, background:'rgba(22,22,58,0.5)',    border:'1px solid rgba(79,110,247,0.2)',  color:'#e0e0f0' };
const btnExit  = { ...btnBase, background:'rgba(150,30,30,0.15)',  border:'1px solid rgba(200,34,34,0.3)',   color:'#e0e0f0', padding:'9px 14px' };
const btnStart = { ...btnBase, fontSize:14, background:'rgba(78,203,113,0.2)', border:'1px solid rgba(78,203,113,0.5)', color:'#4ecb71', padding:'9px 28px' };

/* ── sub-components ──────────────────────────────────────────────────────── */
function Lbl({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ fontSize:14, color:'#4f9ef7', fontFamily:'Share Tech Mono,monospace',
      letterSpacing:'0.12em', marginBottom:6 }}>
      {children}
    </div>
  );
}

function HudInput({ label, type='text', value, onChange, min, max, step }: any) {
  return (
    <div style={{ marginBottom:16 }}>
      <Lbl>{label}</Lbl>
      <input type={type} value={value} onChange={onChange} min={min} max={max} step={step}
        style={inputBase}
        onFocus={e => (e.currentTarget.style.borderColor = 'rgba(79,110,247,0.7)')}
        onBlur={e  => (e.currentTarget.style.borderColor = 'rgba(79,110,247,0.25)')}
      />
    </div>
  );
}

function NumInput({ value, unit, min, max, color, onChange }: {
  value: number; unit: string; min: number; max: number; color: string;
  onChange: (v: number) => void;
}) {
  const [raw, setRaw] = useState(String(value));
  useEffect(() => setRaw(String(value)), [value]);
  function commit(s: string) {
    const v = parseInt(s, 10);
    if (!isNaN(v)) { const c = Math.max(min, Math.min(max, v)); onChange(c); setRaw(String(c)); }
    else setRaw(String(value));
  }
  return (
    <div style={{ display:'flex', alignItems:'center', gap:8 }}>
      <input type="number" min={min} max={max} value={raw}
        onChange={e => setRaw(e.target.value)}
        onBlur={e => { commit(raw); e.currentTarget.style.borderColor = 'rgba(79,110,247,0.25)'; }}
        onKeyDown={e => e.key==='Enter' && commit((e.target as HTMLInputElement).value)}
        style={{ ...inputBase, width:100, fontSize:20, color, textAlign:'center', padding:'8px' }}
        onFocus={e => (e.currentTarget.style.borderColor = 'rgba(79,110,247,0.7)')}
      />
      <span style={{ fontSize:16, color:'#4f9ef7', fontFamily:'Share Tech Mono,monospace' }}>{unit}</span>
    </div>
  );
}

function SliderBar({ value, color, onChange }: { value: number; color: string; onChange: (v: number) => void }) {
  return (
    <div style={{ position:'relative', height:8, background:'rgba(10,10,30,0.9)',
      border:`1px solid ${color}40`, marginTop:8 }}>
      <div style={{ position:'absolute', inset:'0 auto 0 0', width:`${value*100}%`,
        background:`linear-gradient(90deg,${color}50,${color})`, transition:'width 0.1s' }} />
      <input type="range" min={0} max={1} step={0.01} value={value}
        onChange={e => onChange(+e.target.value)}
        style={{ position:'absolute', inset:0, width:'100%', height:'100%', opacity:0, cursor:'pointer', margin:0 }} />
    </div>
  );
}

function ColorPicker({ label, opts, value, onChange, lang }: any) {
  return (
    <div style={{ marginBottom:20 }}>
      <Lbl>{label}</Lbl>
      <div style={{ display:'flex', gap:12, flexWrap:'wrap' }}>
        {opts.map((o: any) => (
          <div key={o.v} style={{ display:'flex', flexDirection:'column', alignItems:'center', gap:5, cursor:'pointer' }}
            onClick={() => onChange(o.v)}>
            <div style={{
              width:38, height:38, background:o.c, clipPath:CLIP,
              border: value===o.v ? '2px solid #4f9ef7' : '2px solid rgba(79,158,247,0.15)',
              boxShadow: value===o.v ? '0 0 10px #4f9ef780' : 'none', transition:'all 0.15s',
            }} />
            <span style={{ fontSize:14, color: value===o.v ? '#4f9ef7' : '#e0e0f0',
              fontFamily:'Share Tech Mono,monospace', letterSpacing:'0.04em', whiteSpace:'nowrap' }}>
              {(o as any)[lang] ?? o.en}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function SumRow({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display:'flex', justifyContent:'space-between', padding:'4px 0',
      borderBottom:'1px solid rgba(79,110,247,0.1)' }}>
      <span style={{ fontSize:14, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace' }}>{label}</span>
      <span style={{ fontSize:14, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace' }}>{value}</span>
    </div>
  );
}

/* ── main wizard ─────────────────────────────────────────────────────────── */
interface Props {
  lang: 'en'|'tr'|'de'|'fr'|'ar';
  loading: boolean;
  accessToken: string | null;
  onSubmit: (form: any, f1: any, f2: any) => void;
  onExit: () => void;
}

export default function SimCreationWizard({ lang, loading, accessToken, onSubmit, onExit }: Props) {
  const [step, setStep] = useState(0);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [simForm, setSimForm] = useState<{ name: string; latitude: string; longitude: string }>({ name:'', latitude:'', longitude:'' });
  const [f1, setF1] = useState<any>(founderDefaults('male'));
  const [f2, setF2] = useState<any>(founderDefaults('female'));

  // Wizard defaults are stored server-side (account-scoped) rather than in
  // localStorage: iOS Safari's ITP purges script-writable storage after ~7
  // days without a top-level visit, silently losing the old localStorage
  // copy. Server storage also means the same defaults follow the user to
  // any device/browser they log into.
  useEffect(() => {
    if (!accessToken) return;
    axios.get(authUrl('/api/auth/wizard-defaults'), { headers: { Authorization: `Bearer ${accessToken}` } })
      .then(({ data }) => {
        const d = data?.defaults;
        if (!d) return;
        if (d.simForm) setSimForm(d.simForm);
        if (d.f1) setF1(d.f1);
        if (d.f2) setF2(d.f2);
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accessToken]);

  const meta  = STEPS[step];
  const isF2  = meta.type !== 'sim-info' && meta.type !== 'summary' && (meta as any).f === 2;
  const fd    = isF2 ? f2 : f1;
  const setFd = isF2 ? setF2 : setF1;
  const setT  = (id: string, val: number) => setFd((p: any) => ({ ...p, [id]: val }));

  const next = () => setStep(s => Math.min(s + 1, TOTAL - 1));
  const back = () => setStep(s => Math.max(s - 1, 0));
  const t    = (tr: string, en: string, de = en, fr = en, ar = en) => ({ tr, en, de, fr, ar } as any)[lang] ?? en;

  const stepRef = useRef(step);
  stepRef.current = step;

  const scrubRef = useRef<HTMLDivElement>(null);
  function scrubToX(clientX: number) {
    const el = scrubRef.current;
    if (!el) return;
    const { left, width } = el.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientX - left) / width));
    setStep(Math.min(TOTAL - 1, Math.max(0, Math.floor(ratio * TOTAL))));
  }

  // Keep window globals in sync so AriaButton can send context to the API
  useEffect(() => {
    const cur = STEPS[step];
    (window as any).__ariaWizardStep = step;
    (window as any).__ariaWizardStepType = cur.type;
    (window as any).__ariaWizardFounder = (cur as any).f ?? null;
    (window as any).__ariaWizardTraitName = cur.type === 'trait'
      ? `${ALL_TRAITS[(cur as any).idx]?.tr} (${ALL_TRAITS[(cur as any).idx]?.id})`
      : null;
  }, [step]);

  useEffect(() => {
    (window as any).__ariaWizardReady = true;
    function onAriaWizard(e: Event) {
      const detail = (e as CustomEvent).detail;
      const { action, field, value, founder } = detail;

      // Determine target founder setter for 'set' action
      const curStep = STEPS[stepRef.current];
      const isF2now = curStep.type !== 'sim-info' && curStep.type !== 'summary' && (curStep as any).f === 2;
      const targetSet = founder === 1 ? setF1 : founder === 2 ? setF2 : (isF2now ? setF2 : setF1);

      switch (action) {
        case 'next':   setStep(s => Math.min(s + 1, TOTAL - 1)); break;
        case 'back':   setStep(s => Math.max(s - 1, 0)); break;
        case 'submit':
          if (STEPS[stepRef.current]?.type === 'summary') setConfirmOpen(true);
          else setStep(TOTAL - 1);
          break;
        case 'exit': onExit(); break;

        case 'set': {
          // Sim info fields
          if (field === 'sim_name')      { setSimForm((p: any) => ({...p, name: String(value)})); break; }
          if (field === 'sim_latitude')  { setSimForm((p: any) => ({...p, latitude: String(value)})); break; }
          if (field === 'sim_longitude') { setSimForm((p: any) => ({...p, longitude: String(value)})); break; }
          // Founder identity fields
          if (field === 'founder_name')  { targetSet((p: any) => ({...p, name: String(value)})); break; }
          if (field === 'founder_age')   { targetSet((p: any) => ({...p, ageYears: Math.max(16, Math.min(60, Math.round(Number(value))))})); break; }
          if (field === 'founder_sex')   { targetSet((p: any) => ({...p, sex: String(value)})); break; }
          // Physical fields — height: fromCm=(cm-150)/45, weight: fromKg uses current height
          if (field === 'founder_height') {
            const cm = Number(value);
            targetSet((p: any) => ({...p, height: fromCm(cm)}));
            break;
          }
          if (field === 'founder_weight' || field === 'founder_kilo') {
            const kg = Number(value);
            targetSet((p: any) => ({...p, metabolism: fromKg(kg, p.height)}));
            break;
          }
          if (field === 'founder_metabolism') {
            const num = Number(value);
            const norm = num > 1 ? num / 100 : num;
            targetSet((p: any) => ({...p, metabolism: Math.max(0, Math.min(1, norm))}));
            break;
          }
          // Appearance fields
          if (field === 'founder_eye')  { targetSet((p: any) => ({...p, eye_color: String(value)})); break; }
          if (field === 'founder_hair') { targetSet((p: any) => ({...p, hair_color: String(value)})); break; }
          if (field === 'founder_skin') { targetSet((p: any) => ({...p, skin_tone: String(value)})); break; }
          // Set whichever trait is on the CURRENT step
          if (field === 'current_trait') {
            const def = STEPS[stepRef.current];
            if (def?.type === 'trait') {
              const tid = ALL_TRAITS[(def as any).idx].id;
              const num = Number(value);
              const norm = num > 1 ? num / 100 : num;
              targetSet((p: any) => ({...p, [tid]: Math.max(0, Math.min(1, norm))}));
            }
            break;
          }
          // Fallback: treat field as direct trait/property key
          {
            const num = Number(value);
            if (!isNaN(num)) {
              const norm = num > 1 ? num / 100 : num;
              targetSet((p: any) => ({...p, [field]: Math.max(0, Math.min(1, norm))}));
            } else {
              targetSet((p: any) => ({...p, [field]: value}));
            }
          }
          break;
        }
      }
    }
    window.addEventListener('aria-wizard', onAriaWizard);
    return () => {
      (window as any).__ariaWizardReady = false;
      window.removeEventListener('aria-wizard', onAriaWizard);
    };
  }, [onExit]);

  const founderLabel = (meta.type !== 'sim-info' && meta.type !== 'summary')
    ? ((meta as any).f === 1 ? t('KURUCU 1 — ERKEK', 'FOUNDER 1 — MALE', 'GRÜNDER 1 — MÄNNLICH', 'FONDATEUR 1 — MÂLE', 'المؤسس 1 — ذكر')
                              : t('KURUCU 2 — KADIN', 'FOUNDER 2 — FEMALE', 'GRÜNDERIN 2 — WEIBLICH', 'FONDATRICE 2 — FEMELLE', 'المؤسسة 2 — أنثى'))
    : null;

  const isSummary  = meta.type === 'summary';
  const canNext    = meta.type === 'sim-info' ? simForm.name.trim() !== '' : true;
  const traitColor = meta.type === 'trait' ? ALL_TRAITS[meta.idx].c : null;

  // Global Enter key handler — works regardless of which element has focus
  const canNextRef   = useRef(canNext);
  const isSummaryRef = useRef(isSummary);
  const confirmOpenRef = useRef(confirmOpen);
  canNextRef.current   = canNext;
  isSummaryRef.current = isSummary;
  confirmOpenRef.current = confirmOpen;

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (confirmOpenRef.current) { setConfirmOpen(false); return; }
        onExit();
        return;
      }
      if (e.key !== 'Enter') return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'TEXTAREA') return;
      if (tag === 'BUTTON') return; // let button's own click handle it
      if (confirmOpenRef.current) return; // dialog open — don't double-fire
      e.preventDefault();
      if (isSummaryRef.current) { setConfirmOpen(true); return; }
      if (canNextRef.current) setStep(s => Math.min(s + 1, TOTAL - 1));
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, []);

  /* step title */
  function stepTitle(): string {
    switch (meta.type) {
      case 'sim-info':   return t('SİMÜLASYON BİLGİLERİ', 'SIMULATION INFO', 'SIMULATIONSINFOS', 'INFOS SIMULATION', 'معلومات المحاكاة');
      case 'identity':   return t('KİMLİK BİLGİLERİ', 'IDENTITY', 'IDENTITÄT', 'IDENTITÉ', 'الهوية');
      case 'physical':   return t('FİZİKSEL ÖLÇÜLER', 'PHYSICAL', 'KÖRPERMASSE', 'PHYSIQUE', 'الجسد');
      case 'appearance': return t('DIŞ GÖRÜNÜŞ', 'APPEARANCE', 'AUSSEHEN', 'APPARENCE', 'المظهر');
      case 'trait': {
        const tr = ALL_TRAITS[meta.idx];
        return t(tr.tr, tr.en, tr.de, tr.fr, tr.ar).toUpperCase();
      }
      case 'summary': return t('ÖZET', 'SUMMARY', 'ZUSAMMENFASSUNG', 'RÉSUMÉ', 'ملخص');
    }
  }

  function stepSubtitle(): string | null {
    if (meta.type === 'trait') {
      const tr = ALL_TRAITS[meta.idx];
      return t(tr.gTr, tr.gEn, tr.gDe, tr.gFr, tr.gAr);
    }
    return null;
  }

  /* ── step content ────────────────────────────────────────────────────── */
  function renderContent() {

    /* Sim info */
    if (meta.type === 'sim-info') return (
      <>
        <HudInput label={t('SİMÜLASYON ADI', 'SIMULATION NAME', 'SIMULATIONSNAME', 'NOM DE SIMULATION', 'اسم المحاكاة')} value={simForm.name}
          onChange={(e: any) => setSimForm(p => ({ ...p, name: e.target.value }))} />
        <HudInput label={t('ENLEM (°N)', 'LATITUDE (°N)', 'BREITENGRAD (°N)', 'LATITUDE (°N)', 'خط العرض (°N)')} type="number" step="0.0001" value={simForm.latitude}
          onChange={(e: any) => setSimForm(p => ({ ...p, latitude: e.target.value }))} />
        <HudInput label={t('BOYLAM (°E)', 'LONGITUDE (°E)', 'LÄNGENGRAD (°E)', 'LONGITUDE (°E)', 'خط الطول (°E)')} type="number" step="0.0001" value={simForm.longitude}
          onChange={(e: any) => setSimForm(p => ({ ...p, longitude: e.target.value }))} />
      </>
    );

    /* Identity */
    if (meta.type === 'identity') return (
      <>
        <HudInput label={t('İSİM', 'NAME', 'NAME', 'NOM', 'الاسم')} value={fd.name}
          onChange={(e: any) => setFd((p: any) => ({ ...p, name: e.target.value }))} />
        <HudInput label={t('YAŞ', 'AGE', 'ALTER', 'ÂGE', 'العمر')} type="number" min={16} max={60} value={fd.ageYears}
          onChange={(e: any) => setFd((p: any) => ({ ...p, ageYears: +e.target.value }))} />
        <div style={{ marginBottom:16 }}>
          <Lbl>{t('CİNSİYET', 'SEX', 'GESCHLECHT', 'SEXE', 'الجنس')}</Lbl>
          <div style={{ display:'flex', gap:8 }}>
            {[{ v:'male', tr:'ERKEK', en:'MALE', de:'MÄNNLICH', fr:'MÂLE', ar:'ذكر' }, { v:'female', tr:'KADIN', en:'FEMALE', de:'WEIBLICH', fr:'FEMELLE', ar:'أنثى' }].map(opt => (
              <button key={opt.v} onClick={() => setFd((p: any) => ({ ...p, sex: opt.v }))}
                style={{ fontFamily:'Share Tech Mono,monospace', letterSpacing:'0.12em', padding:'8px 22px', fontSize:14,
                  background: fd.sex===opt.v ? 'rgba(79,110,247,0.25)' : 'rgba(22,22,58,0.5)',
                  border: `1px solid ${fd.sex===opt.v ? 'rgba(79,110,247,0.6)' : 'rgba(79,110,247,0.15)'}`,
                  color:'#e0e0f0', clipPath:CLIP, cursor:'pointer' }}>
                {t(opt.tr, opt.en, (opt as any).de, (opt as any).fr, (opt as any).ar)}
              </button>
            ))}
          </div>
        </div>
      </>
    );

    /* Physical */
    if (meta.type === 'physical') return (
      <>
        <div style={{ marginBottom:24 }}>
          <Lbl>{t('BOY', 'HEIGHT', 'GRÖSSE', 'TAILLE', 'الطول')}</Lbl>
          <div style={{ display:'flex', alignItems:'center', gap:16 }}>
            <NumInput value={toCm(fd.height)} unit="cm" min={145} max={200} color="#06b6d4"
              onChange={cm => setT('height', fromCm(cm))} />
          </div>
          <SliderBar value={fd.height} color="#06b6d4" onChange={v => setT('height', v)} />
        </div>
        <div style={{ marginBottom:12 }}>
          <Lbl>{t('KİLO', 'WEIGHT', 'GEWICHT', 'POIDS', 'الوزن')}</Lbl>
          <div style={{ display:'flex', alignItems:'center', gap:16 }}>
            <NumInput value={toKg(fd.height, fd.metabolism)} unit="kg" min={40} max={130} color="#a855f7"
              onChange={kg => setT('metabolism', fromKg(kg, fd.height))} />
          </div>
          <SliderBar value={fd.metabolism} color="#a855f7" onChange={v => setT('metabolism', v)} />
          <div style={{ display:'flex', justifyContent:'space-between', marginTop:5,
            fontSize:14, color:'#4f9ef7', fontFamily:'Share Tech Mono,monospace' }}>
            <span>{t('İnce', 'Lean', 'Schlank', 'Mince', 'نحيف')}</span><span>{t('Orta', 'Average', 'Mittel', 'Moyen', 'متوسط')}</span><span>{t('Kaslı', 'Heavy', 'Muskulös', 'Musclé', 'مفتول')}</span>
          </div>
        </div>
      </>
    );

    /* Appearance */
    if (meta.type === 'appearance') return (
      <>
        <ColorPicker label={t('GÖZ RENGİ', 'EYE COLOR', 'AUGENFARBE', 'COULEUR DES YEUX', 'لون العيون')}  opts={EYE_OPTS}  value={fd.eye_color}
          onChange={(v: string) => setFd((p: any) => ({ ...p, eye_color: v }))}  lang={lang} />
        <ColorPicker label={t('SAÇ RENGİ', 'HAIR COLOR', 'HAARFARBE', 'COULEUR DES CHEVEUX', 'لون الشعر')} opts={HAIR_OPTS} value={fd.hair_color}
          onChange={(v: string) => setFd((p: any) => ({ ...p, hair_color: v }))} lang={lang} />
        <ColorPicker label={t('TEN RENGİ', 'SKIN TONE', 'HAUTTON', 'TEINT', 'لون البشرة')}  opts={SKIN_OPTS} value={fd.skin_tone}
          onChange={(v: string) => setFd((p: any) => ({ ...p, skin_tone: v }))}  lang={lang} />
      </>
    );

    /* Trait — enriched display */
    if (meta.type === 'trait') {
      const trait = ALL_TRAITS[meta.idx];
      const tm    = TRAIT_META[meta.idx];
      const val   = fd[trait.id] ?? 0.5;
      const pct   = `${(val * 100).toFixed(0)}%`;
      const tier  = val < 0.34 ? 0 : val < 0.67 ? 1 : 2;
      const li    = LI[lang] ?? 1;
      return (
        <div>
          {/* 1 — big percentage */}
          <div style={{ textAlign:'center', marginBottom:14 }}>
            <div style={{ fontSize:52, color: trait.c, fontFamily:'Orbitron,monospace',
              fontWeight:900, lineHeight:1, textShadow:`0 0 24px ${trait.c}70` }}>
              {pct}
            </div>
          </div>

          {/* 2 — slider + spectrum labels */}
          <SliderBar value={val} color={trait.c} onChange={v => setT(trait.id, v)} />
          <div style={{ display:'flex', justifyContent:'space-between', marginTop:5,
            fontSize:14, fontFamily:'Share Tech Mono,monospace' }}>
            <span style={{ color:'#e0e0f0' }}>{tm.lo[li]}</span>
            <span style={{ color:'#e0e0f0' }}>{tm.hi[li]}</span>
          </div>

          {/* 3 — simulation impact chips */}
          <div style={{ display:'flex', gap:7, flexWrap:'wrap', marginTop:18 }}>
            {tm.fx.map(fx => (
              <div key={fx[0]} style={{
                padding:'3px 10px', fontSize:14, fontFamily:'Share Tech Mono,monospace',
                color: trait.c, border:`1px solid ${trait.c}45`,
                background:`${trait.c}12`, clipPath:CLIP, letterSpacing:'0.07em',
              }}>
                {fx[li]}
              </div>
            ))}
          </div>

          {/* 4a — dynamic description */}
          <div style={{ marginTop:14, fontSize:14, color:'#e0e0f0',
            fontFamily:'Share Tech Mono,monospace', lineHeight:1.65, letterSpacing:'0.03em' }}>
            {tm.d[tier][li]}
          </div>

          {/* 4b — historical reference card */}
          <div style={{ marginTop:12, padding:'10px 14px',
            background:'rgba(8,8,26,0.85)', border:`1px solid ${trait.c}28`, clipPath:CLIP }}>
            <div style={{ fontSize:14, color: trait.c, fontFamily:'Share Tech Mono,monospace',
              letterSpacing:'0.12em', marginBottom:6 }}>
              ◈ {tm.r[tier][li]}
            </div>
            <div style={{ fontSize:14, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace',
              lineHeight:1.55, letterSpacing:'0.03em' }}>
              {tm.r[tier][li + 5]}
            </div>
          </div>
        </div>
      );
    }

    /* Summary */
    if (meta.type === 'summary') return (
      <div style={{ display:'grid', gridTemplateColumns:'1fr 1fr', gap:20 }}>
        <div style={{ gridColumn:'1/-1' }}>
          <div style={{ fontSize:16, color:'#4f9ef7', fontFamily:'Share Tech Mono,monospace',
            letterSpacing:'0.15em', marginBottom:8 }}>
            {t('SİMÜLASYON', 'SIMULATION', 'SIMULATION', 'SIMULATION', 'المحاكاة')}
          </div>
          <SumRow label={t('AD', 'NAME', 'NAME', 'NOM', 'الاسم')}    value={simForm.name || '—'} />
          <SumRow label={t('ENLEM', 'LAT', 'BREITE', 'LAT', 'خط العرض')}  value={simForm.latitude || '—'} />
          <SumRow label={t('BOYLAM', 'LNG', 'LÄNGE', 'LNG', 'خط الطول')} value={simForm.longitude || '—'} />
        </div>
        {([{ fd: f1, sex: 'male' }, { fd: f2, sex: 'female' }] as { fd: any; sex: string }[]).map(({ fd: founder, sex }) => (
          <div key={sex}>
            <div style={{ fontSize:16, color: sex==='male' ? '#4f9ef7' : '#ec4899',
              fontFamily:'Share Tech Mono,monospace', letterSpacing:'0.12em', marginBottom:8 }}>
              {sex==='male' ? t('KURUCU 1 — ERKEK', 'FOUNDER 1 — MALE', 'GRÜNDER 1 — MÄNNLICH', 'FONDATEUR 1 — MÂLE', 'المؤسس 1 — ذكر') : t('KURUCU 2 — KADIN', 'FOUNDER 2 — FEMALE', 'GRÜNDERIN 2 — WEIBLICH', 'FONDATRICE 2 — FEMELLE', 'المؤسسة 2 — أنثى')}
            </div>
            <SumRow label={t('İSİM', 'NAME', 'NAME', 'NOM', 'الاسم')}   value={founder.name} />
            <SumRow label={t('YAŞ', 'AGE', 'ALTER', 'ÂGE', 'العمر')}     value={String(founder.ageYears)} />
            <SumRow label={t('BOY', 'HEIGHT', 'GRÖSSE', 'TAILLE', 'الطول')}  value={`${toCm(founder.height)} cm`} />
            <SumRow label={t('KİLO', 'WEIGHT', 'GEWICHT', 'POIDS', 'الوزن')} value={`${toKg(founder.height, founder.metabolism)} kg`} />
            <SumRow label={t('ZEKA', 'IQ', 'IQ', 'QI', 'معدل الذكاء')}     value={`${(founder.fluid_intelligence * 100).toFixed(0)}%`} />
            <SumRow label={t('SOSYAL BAĞ', 'SOC.BOND', 'SOZ.BIND.', 'LIEN SOC.', 'الترابط الاجتماعي')} value={`${(founder.social_bonding * 100).toFixed(0)}%`} />
            <SumRow label={t('BAĞIŞIKLIK', 'IMMUNITY', 'IMMUNITÄT', 'IMMUNITÉ', 'المناعة')} value={`${(founder.immune_strength * 100).toFixed(0)}%`} />
          </div>
        ))}
      </div>
    );

    return null;
  }

  /* ── render ──────────────────────────────────────────────────────────── */
  const subtitle = stepSubtitle();
  return (
    <div
      style={{ width:'min(580px, 92vw)', height:'min(80vh, 720px)', margin:'0 auto', background:'rgba(4,4,15,0.97)',
      border:'1px solid rgba(79,110,247,0.4)', animation:'boot-in 0.3s ease-out both', display:'flex', flexDirection:'column', overflow:'hidden' }}>

      {/* Progress — interactive scrubber */}
      <div
        ref={scrubRef}
        title={`${step + 1} / ${TOTAL}`}
        style={{ height:32, position:'relative', cursor:'pointer', flexShrink:0, display:'flex', alignItems:'center', touchAction:'none' }}
        onMouseDown={e => {
          scrubToX(e.clientX);
          const onMove = (ev: MouseEvent) => scrubToX(ev.clientX);
          const onUp   = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
          window.addEventListener('mousemove', onMove);
          window.addEventListener('mouseup', onUp);
        }}
        onTouchStart={e => {
          e.preventDefault();
          scrubToX(e.touches[0].clientX);
          const onMove = (ev: TouchEvent) => scrubToX(ev.touches[0].clientX);
          const onEnd  = () => { window.removeEventListener('touchmove', onMove); window.removeEventListener('touchend', onEnd); };
          window.addEventListener('touchmove', onMove, { passive: false });
          window.addEventListener('touchend', onEnd);
        }}
      >
        {/* Track */}
        <div style={{ position:'absolute', inset:0, top:'50%', transform:'translateY(-50%)', height:4,
          background:'rgba(79,110,247,0.15)' }} />
        {/* Fill */}
        <div style={{ position:'absolute', left:0, top:'50%', transform:'translateY(-50%)', height:4,
          width:`${((step+1)/TOTAL)*100}%`,
          background:'linear-gradient(90deg,#4f6ef7,#4f9ef7)', transition:'width 0.12s ease-out' }} />
        {/* Thumb */}
        <div style={{
          position:'absolute', top:'50%',
          left:`${((step+1)/TOTAL)*100}%`,
          transform:'translate(-50%,-50%)',
          width:28, height:28,
          background:'linear-gradient(135deg,#6f9ef7,#4f6ef7)',
          border:'2.5px solid #c8dcff',
          borderRadius:'50%',
          boxShadow:'0 0 12px #4f9ef7aa, 0 2px 6px rgba(0,0,0,0.5)',
          pointerEvents:'none', transition:'left 0.12s ease-out',
          zIndex:1,
        }} />
      </div>

      {/* Header */}
      <div style={{ padding:'12px 20px', borderBottom:'1px solid rgba(79,110,247,0.2)',
        display:'flex', alignItems:'center', justifyContent:'space-between' }}>
        <div>
          {founderLabel && (
            <div style={{ fontSize:14, color:'#4f9ef7', fontFamily:'Share Tech Mono,monospace',
              letterSpacing:'0.2em', marginBottom:3 }}>
              {founderLabel}
            </div>
          )}
          {subtitle && (
            <div style={{ fontSize:14, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace',
              letterSpacing:'0.15em', marginBottom:2 }}>
              {subtitle}
            </div>
          )}
          <div style={{
            fontSize: traitColor ? 24 : 14,
            color: traitColor ?? '#e0e0f0',
            fontFamily: traitColor ? 'Orbitron,monospace' : 'Share Tech Mono,monospace',
            letterSpacing:'0.15em', fontWeight:700,
            textShadow: traitColor ? `0 0 12px ${traitColor}60` : 'none',
          }}>
            {stepTitle()}
          </div>
        </div>
        <div style={{ fontSize:14, color:'#e0e0f0', fontFamily:'Orbitron,monospace', letterSpacing:'0.1em' }}>
          {step+1} / {TOTAL}
        </div>
      </div>

      {/* Content */}
      <div style={{ padding:'22px 24px', flex:1, overflowY:'auto' }}>
        {renderContent()}
      </div>

      {/* Confirmation modal */}
      {confirmOpen && (
        <div style={{ position:'fixed', inset:0, zIndex:999,
          background:'rgba(0,0,10,0.82)', display:'flex', alignItems:'center', justifyContent:'center' }}>
          <div style={{ maxWidth:400, width:'90%', background:'rgba(4,4,18,0.98)',
            border:'1px solid rgba(204,34,34,0.6)', clipPath:CLIP, padding:'28px 28px 24px' }}>
            <div style={{ fontSize:20, color:'#cc2222', fontFamily:'Orbitron,monospace',
              letterSpacing:'0.2em', marginBottom:16, display:'flex', alignItems:'center', gap:10 }}>
              <span style={{
                display:'inline-block',
                fontSize: 52,
                lineHeight: 1,
                filter:'drop-shadow(0 0 8px #cc2222) drop-shadow(0 0 18px #cc222299)',
                animation:'warn-pulse 1.4s ease-in-out infinite',
              }}>⚠</span>
              <span style={{ textShadow:'0 0 12px #cc222288' }}>{t('UYARI', 'WARNING', 'WARNUNG', 'AVERTISSEMENT', 'تحذير')}</span>
            </div>
            <div style={{ fontSize:16, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace',
              lineHeight:1.7, letterSpacing:'0.04em', marginBottom:24 }}>
              {t(
                'Bu adım geri döndürülemez. Simülasyon başlatıldıktan sonra kurucu ayarları değiştirilemez.',
                'This step is irreversible. Founder settings cannot be changed once the simulation is launched.',
                'Dieser Schritt ist unwiderruflich. Gründereinstellungen können nach dem Start nicht mehr geändert werden.',
                'Cette étape est irréversible. Les paramètres du fondateur ne peuvent pas être modifiés après le lancement.',
                'هذه الخطوة لا يمكن التراجع عنها. لا يمكن تغيير إعدادات المؤسس بعد إطلاق المحاكاة.'
              )}
            </div>
            <div style={{ fontSize:16, color:'#e0e0f0', fontFamily:'Share Tech Mono,monospace',
              letterSpacing:'0.12em', marginBottom:20, borderTop:'1px solid rgba(204,34,34,0.25)',
              paddingTop:16 }}>
              {t('Onaylıyor musunuz?', 'Do you confirm?', 'Bestätigen Sie?', 'Confirmez-vous?', 'هل تؤكد؟')}
            </div>
            <div style={{ display:'flex', gap:12 }}>
              <button
                onClick={() => {
                  setConfirmOpen(false);
                  if (accessToken) {
                    axios.post(authUrl('/api/auth/wizard-defaults'), { simForm, f1, f2 }, { headers: { Authorization: `Bearer ${accessToken}` } }).catch(() => {});
                  }
                  onSubmit(simForm, f1, f2);
                }}
                style={{ ...btnBase, fontSize:16, flex:1,
                  background:'rgba(78,203,113,0.18)', border:'1px solid rgba(78,203,113,0.55)',
                  color:'#4ecb71' }}>
                {t('ONAYLA', 'CONFIRM', 'BESTÄTIGEN', 'CONFIRMER', 'تأكيد')}
              </button>
              <button
                onClick={() => setConfirmOpen(false)}
                style={{ ...btnBase, fontSize:16, flex:1,
                  background:'rgba(204,34,34,0.15)', border:'1px solid rgba(204,34,34,0.45)',
                  color:'#e05555' }}>
                {t('VAZGEÇ', 'CANCEL', 'ABBRECHEN', 'ANNULER', 'إلغاء')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Navigation */}
      <div style={{ padding:'8px 12px 6px', marginTop:-32, borderTop:'1px solid rgba(79,110,247,0.15)',
        display:'flex', gap:6 }}>
        <button onClick={onExit}
          style={{ ...btnExit, flex:1, textAlign:'center', padding:'9px 4px', minWidth:0 }}>
          {t('ÇIK', 'EXIT', 'BEENDEN', 'QUITTER', 'خروج')}
        </button>
        {step > 0 && (
          <button onClick={back}
            style={{ ...btnBack, flex:1, textAlign:'center', padding:'9px 4px', minWidth:0 }}>
            ← {t('GERİ', 'BACK', 'ZURÜCK', 'RETOUR', 'رجوع')}
          </button>
        )}
        {isSummary ? (
          <button onClick={() => setConfirmOpen(true)} disabled={loading}
            style={{ ...btnStart, flex:2, textAlign:'center', padding:'9px 4px', minWidth:0,
              opacity: loading ? 0.5 : 1, cursor: loading ? 'not-allowed' : 'pointer' }}>
            {loading ? t('BAŞLATILIYOR…', 'INITIALIZING…', 'WIRD GESTARTET…', 'DÉMARRAGE…', 'جارٍ التشغيل…') : t('BAŞLAT', 'LAUNCH', 'STARTEN', 'LANCER', 'تشغيل')}
          </button>
        ) : (
          <button onClick={next} disabled={!canNext}
            style={{ ...btnNext, flex:2, textAlign:'center', padding:'9px 4px', minWidth:0,
              opacity: !canNext ? 0.4 : 1, cursor: !canNext ? 'not-allowed' : 'pointer' }}>
            {t('DEVAM ET', 'CONTINUE', 'WEITER', 'CONTINUER', 'متابعة')} →
          </button>
        )}
      </div>
    </div>
  );
}
