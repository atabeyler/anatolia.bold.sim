import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { translateEventDescription, text, type LangCode, type TranslationMap } from '../../utils/i18n';

const SEASON_NAMES: Record<string, { label: TranslationMap; color: string; icon: string }> = {
  spring: { label: { tr: 'İlkbahar', en: 'Spring', de: 'Frühling',  fr: 'Printemps', ar: 'الربيع'  }, color: '#4ecb71', icon: '🌿' },
  summer: { label: { tr: 'Yaz',      en: 'Summer', de: 'Sommer',    fr: 'Été',        ar: 'الصيف'   }, color: '#f97316', icon: '☀️' },
  autumn: { label: { tr: 'Sonbahar', en: 'Autumn', de: 'Herbst',    fr: 'Automne',    ar: 'الخريف'  }, color: '#d4a838', icon: '🍂' },
  winter: { label: { tr: 'Kış',      en: 'Winter', de: 'Winter',    fr: 'Hiver',      ar: 'الشتاء'  }, color: '#00d4ff', icon: '❄️' },
};

const BIOME_NAMES: Record<string, { label: TranslationMap; color: string }> = {
  tropical_rainforest: { label: { tr: 'Tropikal Yağmur Ormanı', en: 'Tropical Rainforest', de: 'Tropischer Regenwald',   fr: 'Forêt tropicale humide', ar: 'غابة استوائية مطيرة' }, color: '#22c55e' },
  tropical_savanna:    { label: { tr: 'Tropikal Savan',          en: 'Tropical Savanna',    de: 'Tropische Savanne',      fr: 'Savane tropicale',        ar: 'سافانا استوائية'      }, color: '#84cc16' },
  desert:              { label: { tr: 'Çöl',                     en: 'Desert',              de: 'Wüste',                  fr: 'Désert',                  ar: 'صحراء'                }, color: '#f59e0b' },
  mediterranean:       { label: { tr: 'Akdeniz',                 en: 'Mediterranean',       de: 'Mediterran',             fr: 'Méditerranéen',           ar: 'متوسطي'               }, color: '#06b6d4' },
  temperate_forest:    { label: { tr: 'Ilıman Orman',            en: 'Temperate Forest',    de: 'Gemäßigter Wald',        fr: 'Forêt tempérée',          ar: 'غابة معتدلة'          }, color: '#4ade80' },
  grassland:           { label: { tr: 'Step/Çayırlık',           en: 'Grassland',           de: 'Grasland',               fr: 'Prairie',                 ar: 'مرج'                  }, color: '#a3e635' },
  boreal_forest:       { label: { tr: 'Boreal Orman',            en: 'Boreal Forest',       de: 'Borealer Wald',          fr: 'Forêt boréale',           ar: 'غابة شمالية'          }, color: '#34d399' },
  tundra:              { label: { tr: 'Tundra',                  en: 'Tundra',              de: 'Tundra',                 fr: 'Toundra',                 ar: 'التندرا'              }, color: '#7dd3fc' },
  mountain:            { label: { tr: 'Dağlık',                  en: 'Mountain',            de: 'Gebirge',                fr: 'Montagne',                ar: 'جبلي'                 }, color: '#c0c0d0' },
  coastal:             { label: { tr: 'Kıyı',                    en: 'Coastal',             de: 'Küste',                  fr: 'Côtier',                  ar: 'ساحلي'                }, color: '#38bdf8' },
};

const RAINFALL_BY_BIOME: Record<string, number> = {
  tropical_rainforest: 0.95, coastal: 0.75, temperate_forest: 0.65,
  boreal_forest: 0.55, tropical_savanna: 0.5, mountain: 0.5,
  mediterranean: 0.4, grassland: 0.3, tundra: 0.2, desert: 0.05,
};

const SEASON_RAIN_MULT: Record<string, number> = {
  spring: 1.3, summer: 0.7, autumn: 1.1, winter: 0.9,
};

function Bar({ value, color, max = 1 }: { value: number; color: string; max?: number }) {
  const pct = Math.round((value / max) * 100);
  return (
    <div className="h-1.5 rounded-full overflow-hidden" style={{ background: 'rgba(79,110,247,0.1)' }}>
      <div className="h-full rounded-full transition-all duration-700"
        style={{ width: `${Math.min(100, pct)}%`, background: color, boxShadow: `0 0 6px ${color}60` }} />
    </div>
  );
}

function StatRow({ label, value, unit = '', color = '#a0b4ff' }: { label: string; value: string | number; unit?: string; color?: string }) {
  return (
    <div className="flex justify-between items-center py-0.5">
      <span className="font-share-tech" style={{ fontSize: 12, color: '#a0b4ff' }}>{label}</span>
      <span className="font-orbitron font-bold" style={{ fontSize: 12, color }}>{value}{unit}</span>
    </div>
  );
}

export default function EnvironmentPanel() {
  const { stats, events, lang } = useSimStore();

  const season = stats?.season ?? 'spring';
  const temp = stats?.temperature ?? 20;
  const food = stats?.food_abundance ?? 0.5;
  const water = stats?.water_abundance ?? 0.7;
  const biome = stats?.biome ?? 'mediterranean';
  const biomeInfo = BIOME_NAMES[biome] ?? { label: { tr: biome, en: biome }, color: '#a0b4ff' };
  const seasonInfo = SEASON_NAMES[season] ?? { label: { tr: season, en: season }, color: '#a0b4ff', icon: '🌍' };

  const baseRain = RAINFALL_BY_BIOME[biome] ?? 0.5;
  const rainMult = SEASON_RAIN_MULT[season] ?? 1;
  const precipitation = Math.min(1, baseRain * rainMult);

  const windBase = season === 'winter' ? 0.7 : season === 'autumn' ? 0.55 : season === 'spring' ? 0.45 : 0.3;
  const windSpeed = Math.round(windBase * 40 + 5);

  const humidity = Math.min(1, (precipitation * 0.7 + water * 0.3));
  const uvIndex = temp > 30 ? 9 : temp > 20 ? 6 : temp > 10 ? 3 : 1;

  const disasterEvents = events.filter(e => e.event_type === 'disaster');

  return (
    <DetailPanel panelId="environment" title="Environment" titleTr="Çevre">

      {/* Biome badge */}
      <div className="mb-3 px-2 py-1.5" style={{ background: `${biomeInfo.color}12`, border: `1px solid ${biomeInfo.color}40` }}>
        <div className="font-share-tech tracking-widest" style={{ fontSize: 12, color: '#a0b4ff' }}>{text(lang as LangCode, { tr: 'BİYOM', en: 'BIOME', de: 'BIOM', fr: 'BIOME', ar: 'المناخ الحيوي' })}</div>
        <div className="font-orbitron font-bold mt-0.5" style={{ fontSize: 12, color: biomeInfo.color }}>
          {text(lang as LangCode, biomeInfo.label).toUpperCase()}
        </div>
      </div>

      {/* Season + Temp side by side */}
      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="p-2 text-center" style={{ background: `${seasonInfo.color}10`, border: `1px solid ${seasonInfo.color}35` }}>
          <div className="font-orbitron font-bold" style={{ color: seasonInfo.color, fontSize: 13 }}>
            {text(lang as LangCode, seasonInfo.label)}
          </div>
          <div className="font-share-tech text-sim-muted tracking-widest mt-0.5" style={{ fontSize: 12 }}>
            {text(lang as LangCode, { tr: 'MEVSİM', en: 'SEASON', de: 'JAHRESZEIT', fr: 'SAISON', ar: 'الموسم' })}
          </div>
        </div>
        <div className="p-2 text-center" style={{ background: temp > 35 ? 'rgba(224,90,90,0.1)' : temp < 0 ? 'rgba(0,212,255,0.1)' : 'rgba(79,110,247,0.08)', border: `1px solid ${temp > 35 ? '#e05a5a' : temp < 0 ? '#00d4ff' : '#4f6ef7'}40` }}>
          <div className="font-orbitron font-bold" style={{ color: temp > 35 ? '#e05a5a' : temp < 0 ? '#00d4ff' : '#a0b4ff', fontSize: 13 }}>
            {temp}°C
          </div>
          <div className="font-share-tech text-sim-muted tracking-widest mt-0.5" style={{ fontSize: 12 }}>
            {text(lang as LangCode, { tr: 'SICAKLIK', en: 'TEMP', de: 'TEMP', fr: 'TEMP', ar: 'الحرارة' })}
          </div>
        </div>
      </div>

      {/* Weather metrics */}
      <div className="mb-3">
        <div className="font-share-tech text-sim-muted tracking-widest mb-2" style={{ fontSize: 12 }}>
          {text(lang as LangCode, { tr: 'HAVA KOŞULLARI', en: 'WEATHER', de: 'WETTER', fr: 'MÉTÉO', ar: 'الطقس' })}
        </div>
        <div className="space-y-2">
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'Yağış İndeksi', en: 'Precipitation', de: 'Niederschlag', fr: 'Précipitation', ar: 'هطول الأمطار' })} value={Math.round(precipitation * 100)} unit="%" color="#00d4ff" />
            <Bar value={precipitation} color="#00d4ff" />
          </div>
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'Nem', en: 'Humidity', de: 'Luftfeuchtigkeit', fr: 'Humidité', ar: 'الرطوبة' })} value={Math.round(humidity * 100)} unit="%" color="#7dd3fc" />
            <Bar value={humidity} color="#7dd3fc" />
          </div>
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'Rüzgar', en: 'Wind', de: 'Wind', fr: 'Vent', ar: 'الرياح' })} value={windSpeed} unit=" km/h" color="#a0b4ff" />
            <Bar value={windSpeed} max={60} color="#a0b4ff" />
          </div>
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'UV İndeksi', en: 'UV Index', de: 'UV-Index', fr: 'Indice UV', ar: 'مؤشر الأشعة فوق البنفسجية' })} value={uvIndex} color={uvIndex > 7 ? '#f97316' : uvIndex > 4 ? '#d4a838' : '#4ecb71'} />
            <Bar value={uvIndex} max={11} color={uvIndex > 7 ? '#f97316' : uvIndex > 4 ? '#d4a838' : '#4ecb71'} />
          </div>
        </div>
      </div>

      {/* Resources */}
      <div className="mb-3">
        <div className="font-share-tech text-sim-muted tracking-widest mb-2" style={{ fontSize: 12 }}>
          {text(lang as LangCode, { tr: 'KAYNAKLAR', en: 'RESOURCES', de: 'RESSOURCEN', fr: 'RESSOURCES', ar: 'الموارد' })}
        </div>
        <div className="space-y-2">
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'Yiyecek Bolluğu', en: 'Food Abundance', de: 'Nahrungsüberfluss', fr: 'Abondance alimentaire', ar: 'وفرة الغذاء' })} value={Math.round(food * 100)} unit="%" color={food > 0.6 ? '#4ecb71' : food > 0.3 ? '#d4a838' : '#e05a5a'} />
            <Bar value={food} color={food > 0.6 ? '#4ecb71' : food > 0.3 ? '#d4a838' : '#e05a5a'} />
          </div>
          <div>
            <StatRow label={text(lang as LangCode, { tr: 'Su Kaynakları', en: 'Water', de: 'Wasserquellen', fr: 'Eau', ar: 'الماء' })} value={Math.round(water * 100)} unit="%" color="#00d4ff" />
            <Bar value={water} color="#00d4ff" />
          </div>
        </div>
      </div>

      {/* Disaster log */}
      <div>
        <div className="font-share-tech text-sim-muted tracking-widest mb-2" style={{ fontSize: 12 }}>
          {text(lang as LangCode, { tr: 'AFET GEÇMİŞİ', en: 'DISASTER LOG', de: 'KATASTROPHENPROTOKOLL', fr: 'JOURNAL DES CATASTROPHES', ar: 'سجل الكوارث' })}
        </div>
        {disasterEvents.length === 0 ? (
          <div className="font-share-tech text-sim-muted/40 italic" style={{ fontSize: 12 }}>
            {text(lang as LangCode, { tr: 'Kayıtlı afet yok.', en: 'No disasters recorded.', de: 'Keine Katastrophen aufgezeichnet.', fr: 'Aucune catastrophe enregistrée.', ar: 'لا توجد كوارث مسجلة.' })}
          </div>
        ) : (
          <div className="space-y-1 max-h-32 overflow-y-auto">
            {disasterEvents.slice(0, 10).map((ev, i) => (
              <div key={i} className="flex gap-2 py-0.5" style={{ borderBottom: '1px solid rgba(224,90,90,0.1)' }}>
                <span className="font-share-tech" style={{ fontSize: 12, color: '#e05a5a' }}>Y{ev.sim_year}</span>
                <span className="font-share-tech text-sim-muted" style={{ fontSize: 12 }}>{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}
