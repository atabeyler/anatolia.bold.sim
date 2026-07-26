import { describe, it, expect } from 'vitest';
import { text, translateSeason, translateEventDescription, translateEventType, describeBeliefCode, beliefCodeNumber } from './i18n';

// ── text() ──────────────────────────────────────────────────────────────────

describe('text()', () => {
  it('istenen dil varsa onu döndürür', () => {
    expect(text('tr', { tr: 'Merhaba', en: 'Hello' })).toBe('Merhaba');
  });

  it('istenen dil yoksa "en" döndürür', () => {
    expect(text('de', { en: 'Hello', tr: 'Merhaba' })).toBe('Hello');
  });

  it('ne "en" ne istenen dil yoksa "tr" döndürür', () => {
    expect(text('de', { tr: 'Merhaba' })).toBe('Merhaba');
  });

  it('boş fallback değerinde boş string döndürür', () => {
    expect(text('fr', { en: '' })).toBe('');
  });
});

// ── translateSeason() ───────────────────────────────────────────────────────

describe('translateSeason()', () => {
  it('"spring" → "İlkbahar" (Türkçe)', () => {
    expect(translateSeason('spring', 'tr')).toBe('İlkbahar');
  });

  it('"summer" → "Yaz" (Türkçe)', () => {
    expect(translateSeason('summer', 'tr')).toBe('Yaz');
  });

  it('"autumn" → "Sonbahar" (Türkçe)', () => {
    expect(translateSeason('autumn', 'tr')).toBe('Sonbahar');
  });

  it('"winter" → "Winter" (İngilizce — çeviri yok)', () => {
    expect(translateSeason('winter', 'en')).toBe('Winter');
  });

  it('boş string → "—" döndürür', () => {
    expect(translateSeason('', 'tr')).toBe('—');
  });

  it('büyük/küçük harf farkı gözetmez', () => {
    expect(translateSeason('SPRING', 'tr')).toBe('İlkbahar');
  });
});

// ── translateEventDescription() ─────────────────────────────────────────────

describe('translateEventDescription() — Türkçe', () => {
  it('ölüm olayı: "X died: starvation" → Türkçe', () => {
    const result = translateEventDescription('Karo died: starvation', 'tr');
    expect(result).toContain('öldü');
    expect(result).toContain('açlık');
  });

  it('doğum olayı: "Born: Name (Anne & Baba)" → Türkçe', () => {
    const result = translateEventDescription('Born: Metu (Karo & Ro)', 'tr');
    expect(result).toContain('Doğdu');
  });

  it('inanç oluşumu: gerçek din adı hiç geçmeden çevrilir', () => {
    // Cardinal rule: belief_formed never carries a real-world religion name
    // (see sim-core's belief.rs) -- the backend sends a neutral phrase.
    const result = translateEventDescription('A new belief takes hold', 'tr');
    expect(result).toBe('Yeni bir inanç filizleniyor');
  });

  it('inanç oluşumu: kurucu adı bilinir ama etiket henüz yoksa', () => {
    const result = translateEventDescription('Mete gave rise to a new belief', 'tr');
    expect(result).toBe('Mete yeni bir inanç başlattı');
  });

  it('inanç oluşumu: kurucu adı ve türetilmiş etiket ikisi de bilinirse', () => {
    const result = translateEventDescription('Mete gave rise to Karvun', 'tr');
    expect(result).toBe('Mete, Karvun inancını başlattı');
  });

  it('inanç oluşumu: kurucu bilinmiyor ama etiket bilinirse', () => {
    const result = translateEventDescription('A new belief, Karvun, takes hold', 'tr');
    expect(result).toBe('Yeni bir inanç, Karvun, filizleniyor');
  });

  it('inanç oluşumu: henüz etiket yokken opak kod numarası gösterilir', () => {
    // Per the cardinal rule (see sim-core's belief.rs), the raw archetype
    // string is never shown -- only its opaque numeric code, until the
    // population's own language names it.
    const result = translateEventDescription('A new belief (#5) takes hold', 'tr');
    expect(result).toBe('Yeni bir inanç (#5) filizleniyor');
  });

  it('inanç oluşumu: kurucu adı bilinir, kod numarası da belirtilir', () => {
    const result = translateEventDescription('Mete gave rise to belief #5', 'tr');
    expect(result).toBe('Mete, #5 numaralı inanca öncülük etti');
  });

  it('ritüel oluşumu: henüz isimlenmemiş inanç için nötr ifade', () => {
    const result = translateEventDescription('A ritual emerges in the group', 'tr');
    expect(result).toBe('Grupta bir ritüel ortaya çıktı');
  });

  it('ritüel oluşumu: türetilmiş etiket olduğu gibi geçer, çevrilmez', () => {
    const result = translateEventDescription('A Sekibo ritual emerges in the group', 'tr');
    expect(result).toBe('Grupta Sekibo ritüeli ortaya çıktı');
  });

  it('ritüel oluşumu: henüz etiket yokken opak kod numarası gösterilir', () => {
    const result = translateEventDescription('A ritual (belief #3) emerges in the group', 'tr');
    expect(result).toBe('Grupta #3 numaralı inanç ritüeli ortaya çıktı');
  });

  it('teknoloji keşfi: "Technology discovered: pottery" → Türkçe', () => {
    const result = translateEventDescription('Technology discovered: pottery', 'tr');
    expect(result).toContain('Çömlekçilik');
  });

  it('salgın: "A malaria like outbreak begins" → Türkçe', () => {
    const result = translateEventDescription('A malaria like outbreak begins', 'tr');
    expect(result).toContain('salgını');
  });

  it('"en" dilinde açıklama değişmeden döner', () => {
    const desc = 'A new belief takes hold';
    expect(translateEventDescription(desc, 'en')).toBe(desc);
  });

  it('boş string boş string döndürür', () => {
    expect(translateEventDescription('', 'tr')).toBe('');
  });

  it('norm ihlali: gerçek norm metni çevrilir, alt çizgiyle değiştirilmez', () => {
    const result = translateEventDescription("Norm violated: Taking others' possessions is prohibited", 'tr');
    expect(result).toBe('Norm ihlal edildi: Başkalarının eşyalarını almak yasaktır');
  });

  it('norm oluşumu: gerçek norm metni çevrilir', () => {
    const result = translateEventDescription('Norm emerged: The leader resolves disputes', 'tr');
    expect(result).toBe('Norm oluştu: Lider anlaşmazlıkları çözer');
  });

  it('"Unnamed" her yerde yerelleştirilir', () => {
    const result = translateEventDescription('Born: Unnamed (Damla & Unnamed)', 'tr');
    expect(result).toBe('Doğdu: İsimsiz (Damla & İsimsiz)');
  });

  it('inanç yayılması: henüz isimlenmemişse nötr ifade kullanılır', () => {
    const result = translateEventDescription('Elif embraced a belief', 'tr');
    expect(result).toBe('Elif bir inanca bağlandı');
  });

  it('inanç yayılması: türetilmiş etiket olduğu gibi geçer, gerçek bir din adı değil', () => {
    const result = translateEventDescription('Elif embraced Sekibo', 'tr');
    expect(result).toBe('Elif, Sekibo inancını benimsedi');
  });

  it('inanç yayılması: henüz etiket yokken opak kod numarası gösterilir', () => {
    const result = translateEventDescription('Elif embraced belief #2', 'tr');
    expect(result).toBe('Elif, #2 numaralı inancı benimsedi');
  });

  it('grup ayrılığı: sabit cümle çevrilir', () => {
    expect(translateEventDescription('A group split into two bands', 'tr')).toBe('Bir grup ikiye bölündü');
  });

  it('inanç isimlenmesi: türetilmiş etiket duyurulur', () => {
    expect(translateEventDescription('Their belief becomes known as Kelu', 'tr')).toBe('İnançları artık "Kelu" olarak biliniyor');
  });

  it('grup isimlenmesi: türetilmiş isim duyurulur', () => {
    expect(translateEventDescription('The group becomes known as Baru', 'tr')).toBe('Grup artık "Baru" olarak biliniyor');
  });

  it('medeniyet isimlenmesi: türetilmiş isim duyurulur', () => {
    expect(translateEventDescription('Their civilization becomes known as Anoteva', 'tr')).toBe('Medeniyetleri artık "Anoteva" olarak biliniyor');
  });

  it('liderlik değişimi: "X became the new leader" → Türkçe', () => {
    expect(translateEventDescription('Kaan became the new leader', 'tr')).toBe('Kaan yeni lider oldu');
  });

  it('ticaret: "X traded with Y" → Türkçe', () => {
    expect(translateEventDescription('Ayla traded with Kaan', 'tr')).toBe('Ayla, Kaan ile takas yaptı');
  });
});

// ── translateEventType() ───────────────────────────────────────────────────

describe('translateEventType()', () => {
  it('"birth" → "doğum" (Türkçe)', () => {
    expect(translateEventType('birth', 'tr')).toBe('doğum');
  });

  it('"technology" → "teknoloji" (Türkçe)', () => {
    expect(translateEventType('technology', 'tr')).toBe('teknoloji');
  });

  it('"belief" → "belief" (İngilizce — çeviri aynı)', () => {
    expect(translateEventType('belief', 'en')).toBe('belief');
  });

  it('bilinmeyen tip olduğu gibi döner', () => {
    expect(translateEventType('unknown_custom_type', 'tr')).toBe('unknown_custom_type');
  });

  it('boş string boş string döndürür', () => {
    expect(translateEventType('', 'tr')).toBe('');
  });

  it('"cultural_diffusion" "culture" alt dizesini içermez, ayrı bir girdi gerekir', () => {
    expect(translateEventType('cultural_diffusion', 'tr')).toBe('kültürel yayılma');
  });

  it('"cultural_meme_emerged" ayrı bir girdi gerekir', () => {
    expect(translateEventType('cultural_meme_emerged', 'tr')).toBe('kültürel motif');
  });

  it('"group_split" hiçbir anahtar kelimeyle eşleşmiyordu, ayrı girdi gerekir', () => {
    expect(translateEventType('group_split', 'tr')).toBe('grup ayrılığı');
  });

  it('"leadership_change" ayrı bir girdi gerekir', () => {
    expect(translateEventType('leadership_change', 'tr')).toBe('liderlik değişimi');
  });
});

// ── isValidLangCode() ───────────────────────────────────────────────────────

import { isValidLangCode } from './i18n';

describe('isValidLangCode()', () => {
  it('geçerli kodları kabul eder', () => {
    expect(isValidLangCode('tr')).toBe(true);
    expect(isValidLangCode('en')).toBe(true);
    expect(isValidLangCode('de')).toBe(true);
    expect(isValidLangCode('fr')).toBe(true);
    expect(isValidLangCode('ar')).toBe(true);
  });

  it('geçersiz kodları reddeder', () => {
    expect(isValidLangCode('es')).toBe(false);
    expect(isValidLangCode('')).toBe(false);
    expect(isValidLangCode(null)).toBe(false);
    expect(isValidLangCode(undefined)).toBe(false);
    expect(isValidLangCode(42)).toBe(false);
  });
});

// ── translateEventDescription() de/fr/ar ───────────────────────────────────

describe('translateEventDescription() — de/fr/ar', () => {
  it('Almanca: ölüm olayını çevirir', () => {
    expect(translateEventDescription('John died: starvation', 'de')).toBe('John starb: Verhungern');
  });

  it('Almanca: doğum olayını çevirir', () => {
    expect(translateEventDescription('Born: Alice (Bob & Carol)', 'de')).toBe('Geboren: Alice (Bob & Carol)');
  });

  it('Fransızca: ölüm olayını çevirir', () => {
    expect(translateEventDescription('Jane died: old_age', 'fr')).toBe('Jane est décédé: Vieillesse');
  });

  it('Arapça: ölüm olayını çevirir', () => {
    expect(translateEventDescription('Ali died: infection', 'ar')).toBe('مات Ali: عدوى');
  });

  it('İngilizce: olduğu gibi döner', () => {
    expect(translateEventDescription('John died: starvation', 'en')).toBe('John died: starvation');
  });

  it('Almanca: afet olayını çevirir', () => {
    expect(translateEventDescription('Earthquake killed 5 individuals', 'de')).toBe('Erdbeben tötete 5 Personen');
  });

  it('Fransızca: afet olayını çevirir', () => {
    expect(translateEventDescription('Flood killed 3 individuals', 'fr')).toBe('Inondation a tué 3 personnes');
  });

  it('Almanca: norm ihlali gerçek metni çevirir', () => {
    const result = translateEventDescription("Norm violated: Taking others' possessions is prohibited", 'de');
    expect(result).toBe('Norm verletzt: Das Nehmen fremden Besitzes ist verboten');
  });

  it('Fransızca: norm ihlali gerçek metni çevirir', () => {
    const result = translateEventDescription("Norm violated: Taking others' possessions is prohibited", 'fr');
    expect(result).toBe("Norme violée : Il est interdit de prendre les biens d'autrui");
  });

  it('Arapça: norm ihlali gerçek metni çevirir', () => {
    const result = translateEventDescription("Norm violated: Taking others' possessions is prohibited", 'ar');
    expect(result).toBe('انتُهك عرف: يُمنع أخذ ممتلكات الآخرين');
  });

  it('Almanca: "Unnamed" yerelleştirilir', () => {
    expect(translateEventDescription('Born: Unnamed (Bob & Carol)', 'de')).toBe('Geboren: Unbenannt (Bob & Carol)');
  });

  it('Fransızca: "Unnamed" yerelleştirilir', () => {
    expect(translateEventDescription('Born: Unnamed (Bob & Carol)', 'fr')).toBe('Né: Sans nom (Bob & Carol)');
  });

  it('Arapça: "Unnamed" yerelleştirilir', () => {
    expect(translateEventDescription('Born: Unnamed (Bob & Carol)', 'ar')).toBe('وُلد: بدون اسم (Bob & Carol)');
  });

  it('Almanca: inanç yayılması, henüz isimlenmemişse nötr ifade', () => {
    expect(translateEventDescription('Elif embraced a belief', 'de')).toBe('Elif nahm einen Glauben an');
  });

  it('Almanca: inanç yayılması, türetilmiş etiket olduğu gibi geçer', () => {
    expect(translateEventDescription('Elif embraced Sekibo', 'de')).toBe('Elif nahm den Glauben Sekibo an');
  });

  it('Almanca: inanç oluşumu, kurucu adı ve etiket ikisi de bilinirse', () => {
    expect(translateEventDescription('Mete gave rise to Karvun', 'de')).toBe('Mete begründete den Glauben Karvun');
  });

  it('Fransızca: inanç oluşumu, kurucu adı bilinir ama etiket henüz yoksa', () => {
    expect(translateEventDescription('Mete gave rise to a new belief', 'fr')).toBe('Mete a donné naissance à une nouvelle croyance');
  });

  it('Arapça: inanç oluşumu, kurucu bilinmiyor ama etiket bilinirse', () => {
    expect(translateEventDescription('A new belief, Karvun, takes hold', 'ar')).toBe('ينشأ معتقد جديد، Karvun');
  });

  it('Fransızca: grup ayrılığı sabit cümlesi çevrilir', () => {
    expect(translateEventDescription('A group split into two bands', 'fr')).toBe("Un groupe s'est divisé en deux bandes");
  });

  it('Arapça: liderlik değişimi çevrilir', () => {
    expect(translateEventDescription('Kaan became the new leader', 'ar')).toBe('أصبح Kaan الزعيم الجديد');
  });

  it('Almanca: inanç yayılması, henüz etiket yokken opak kod numarası gösterilir', () => {
    expect(translateEventDescription('Elif embraced belief #2', 'de')).toBe('Elif nahm den Glauben #2 an');
  });

  it('Fransızca: inanç oluşumu, henüz etiket yokken opak kod numarası gösterilir', () => {
    expect(translateEventDescription('Mete gave rise to belief #5', 'fr')).toBe('Mete a donné naissance à la croyance #5');
  });

  it('Arapça: ritüel oluşumu, henüz etiket yokken opak kod numarası gösterilir', () => {
    expect(translateEventDescription('A ritual (belief #3) emerges in the group', 'ar')).toBe('ظهرت طقوس (معتقد رقم 3) في المجموعة');
  });
});

// ── translateSeason() de/fr/ar ──────────────────────────────────────────────

describe('translateSeason() — de/fr/ar', () => {
  it('"spring" → "Frühling" (Almanca)', () => {
    expect(translateSeason('spring', 'de')).toBe('Frühling');
  });

  it('"summer" → "Été" (Fransızca)', () => {
    expect(translateSeason('summer', 'fr')).toBe('Été');
  });

  it('"winter" → "الشتاء" (Arapça)', () => {
    expect(translateSeason('winter', 'ar')).toBe('الشتاء');
  });
});

// ── beliefCodeNumber() / describeBeliefCode() ───────────────────────────────
// Cardinal rule (see sim-core's belief.rs): belief archetype ids are opaque
// codes, never a real-world religion name -- these two helpers must never
// invent one either, only expose the numeric suffix and a description built
// purely from mechanical thresholds (stage/IQ/foxp2/tech).

describe('beliefCodeNumber()', () => {
  it('opaque id\'den sayısal soneki çıkarır', () => {
    expect(beliefCodeNumber('belief_5')).toBe('5');
  });

  it('beklenmeyen bir biçimde ise ham değeri olduğu gibi döndürür', () => {
    expect(beliefCodeNumber('mystery')).toBe('mystery');
  });
});

describe('describeBeliefCode()', () => {
  it('her kod için gerçek bir din adı geçmeyen kısa bir açıklama döndürür', () => {
    for (const code of ['belief_1', 'belief_2', 'belief_3', 'belief_4', 'belief_5', 'belief_6']) {
      for (const lang of ['tr', 'en', 'de', 'fr', 'ar'] as const) {
        const desc = describeBeliefCode(code, lang);
        expect(desc.length).toBeGreaterThan(0);
        for (const realWorldName of ['animism', 'ancestor cult', 'shamanism', 'polytheism', 'monotheism', 'philosophical']) {
          expect(desc.toLowerCase()).not.toContain(realWorldName);
        }
      }
    }
  });

  it('bilinmeyen bir kod için boş string döndürür', () => {
    expect(describeBeliefCode('unknown_code', 'en')).toBe('');
  });
});
