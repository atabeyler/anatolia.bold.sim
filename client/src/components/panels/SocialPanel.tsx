import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import { Users, Crown, Swords, ArrowLeftRight } from 'lucide-react';
import { text, translateEventDescription, type LangCode } from '../../utils/i18n';

const ROLES: { tr: string; en: string; de: string; fr: string; ar: string }[] = [
  { tr: 'Lider',         en: 'Leader',  de: 'Anführer',    fr: 'Chef',       ar: 'الزعيم'   },
  { tr: 'Yaşlı',        en: 'Elder',   de: 'Ältester',    fr: 'Aîné',       ar: 'مسن'      },
  { tr: 'Savaşçı',      en: 'Warrior', de: 'Krieger',     fr: 'Guerrier',   ar: 'المحارب'  },
  { tr: 'İyileştirici', en: 'Healer',  de: 'Heiler',      fr: 'Guérisseur', ar: 'المعالج'  },
  { tr: 'Üye',          en: 'Member',  de: 'Mitglied',    fr: 'Membre',     ar: 'العضو'    },
];

export default function SocialPanel() {
  const { stats, events, lang } = useSimStore();

  const groupCount = (stats as any)?.groups ?? 0;
  const socialEvents = events.filter(e =>
    ['group_formed', 'group_split', 'leadership_change', 'intergroup_conflict', 'group_join'].includes(e.event_type)
  );
  const conflictEvents = events.filter(e => e.event_type === 'intergroup_conflict');
  const leadershipEvents = events.filter(e => e.event_type === 'leadership_change');

  return (
    <DetailPanel panelId="social" title="Social" titleTr="Sosyal">
      <div className="grid grid-cols-2 gap-2 mb-3">
        <StatCard icon={<Users size={14} />} label={text(lang as LangCode, { tr: 'Gruplar', en: 'Groups', de: 'Gruppen', fr: 'Groupes', ar: 'المجموعات' })} value={groupCount} color="text-blue-400" />
        <StatCard icon={<Swords size={14} />} label={text(lang as LangCode, { tr: 'Çatışmalar', en: 'Conflicts', de: 'Konflikte', fr: 'Conflits', ar: 'النزاعات' })} value={conflictEvents.length} color="text-red-400" />
        <StatCard icon={<Crown size={14} />} label={text(lang as LangCode, { tr: 'Liderlik Değ.', en: 'Leadership Changes', de: 'Führungswechsel', fr: 'Changements de chef', ar: 'تغييرات القيادة' })} value={leadershipEvents.length} color="text-yellow-400" />
        <StatCard icon={<ArrowLeftRight size={14} />} label={text(lang as LangCode, { tr: 'Bölünmeler', en: 'Splits', de: 'Spaltungen', fr: 'Scissions', ar: 'الانقسامات' })} value={events.filter(e => e.event_type === 'group_split').length} color="text-orange-400" />
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Sosyal Hiyerarşi Modeli', en: 'Social Hierarchy Model', de: 'Soziales Hierarchiemodell', fr: 'Modèle de hiérarchie sociale', ar: 'نموذج التسلسل الاجتماعي' })}
        </h4>
        <p className="text-sim-muted text-sm italic mb-2">
          {text(lang as LangCode, {
            // MHC/immune-locus similarity only ever factors into mate
            // selection (reproduction.rs's conception-odds bonus) -- real
            // intergroup conflict power is avg aggression × group size ×
            // defense bonus (social.rs's process_intergroup_conflict), with
            // no MHC term at all. The previous copy claimed a mechanic the
            // engine doesn't actually implement.
            tr: 'Baskınlık × güç → liderlik. Yüksek bağımsızlık → bölünme. Ortalama saldırganlık × grup büyüklüğü → çatışma gücü.',
            en: 'Dominance × strength → leadership. High independence → fission. Average aggression × group size → conflict power.',
            de: 'Dominanz × Stärke → Führung. Hohe Unabhängigkeit → Spaltung. Durchschnittliche Aggression × Gruppengröße → Konfliktstärke.',
            fr: 'Dominance × force → leadership. Haute indépendance → fission. Agressivité moyenne × taille du groupe → puissance de conflit.',
            ar: 'الهيمنة × القوة → القيادة. الاستقلالية العالية → الانقسام. متوسط العدوانية × حجم المجموعة → قوة الصراع.',
          })}
        </p>
        <div className="space-y-1">
          {ROLES.map(role => (
            <div key={role.en} className="flex justify-between py-0.5 text-sm border-b border-sim-border/30">
              <span className="text-sim-text">{text(lang as LangCode, role)}</span>
              <span className="text-sim-muted">{text(lang as LangCode, { tr: 'Özellik tabanlı', en: 'Trait-driven assignment', de: 'Eigenschaftsbasiert', fr: 'Basé sur les traits', ar: 'تعيين قائم على السمات' })}</span>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h4 className="text-sim-gold text-sm font-semibold uppercase tracking-widest mb-2">
          {text(lang as LangCode, { tr: 'Son Sosyal Olaylar', en: 'Recent Social Events', de: 'Aktuelle Sozialmeldungen', fr: 'Événements sociaux récents', ar: 'الأحداث الاجتماعية الأخيرة' })}
        </h4>
        {socialEvents.length === 0 ? (
          <p className="text-sim-muted italic text-sm">{text(lang as LangCode, { tr: 'Henüz sosyal olay yok.', en: 'No social events yet.', de: 'Noch keine Sozialmeldungen.', fr: 'Pas encore d\'événements sociaux.', ar: 'لا توجد أحداث اجتماعية بعد.' })}</p>
        ) : (
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {socialEvents.slice(0, 12).map((ev, i) => (
              <div key={i} className="flex items-start gap-2 py-1 border-b border-sim-border/30">
                <span className="text-sim-accent font-mono text-sm flex-shrink-0">Y{ev.sim_year}</span>
                <span className="text-sim-muted text-sm">{translateEventDescription(ev.description ?? '', lang as LangCode, ev)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </DetailPanel>
  );
}

function StatCard({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: number; color: string }) {
  return (
    <div className="bg-sim-surface rounded-lg p-2 flex flex-col items-center gap-1">
      <div className={color}>{icon}</div>
      <div className={`font-bold text-lg ${color}`}>{value}</div>
      <div className="text-sim-muted text-sm text-center">{label}</div>
    </div>
  );
}
