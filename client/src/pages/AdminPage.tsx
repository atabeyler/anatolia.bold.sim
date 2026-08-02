import { useState, useEffect } from 'react';
import FooterBar from '../components/layout/FooterBar';
import { useNavigate } from 'react-router-dom';
import axios from 'axios';
import { useSimStore } from '../store/simStore';
import { LogOut, CheckCircle, XCircle, Ban, Trash2, ShieldOff, Clock, Eye, EyeOff } from 'lucide-react';
import { text, type LangCode } from '../utils/i18n';
import { cloudUrl } from '../utils/cloud';

const LOCALE_MAP: Record<string, string> = { tr: 'tr-TR', en: 'en-US', de: 'de-DE', fr: 'fr-FR', ar: 'ar-SA' };

type UserRow = {
  id: string; user_code: string; first_name: string; last_name: string;
  tc_no: string; email: string; role: string;
  is_approved: boolean; is_banned: boolean; ban_reason: string | null;
  created_at: string;
};

function Badge({ label, color }: { label: string; color: string }) {
  return (
    <span className="font-share-tech tracking-widest px-2 py-0.5" style={{ fontSize: 11, color, border: `1px solid ${color}55`, background: `${color}11` }}>
      {label}
    </span>
  );
}

// The full national ID sat in plaintext in the DOM for every row on page
// load -- trivially readable over-the-shoulder or in a careless screenshot.
// Masking by default (revealing only on explicit per-row click) keeps the
// real value available to an admin who actually needs it, without leaving
// it exposed by default.
function maskTc(tc: string): string {
  if (tc.length <= 4) return '•'.repeat(tc.length);
  return '•'.repeat(tc.length - 4) + tc.slice(-4);
}

export default function AdminPage() {
  const navigate = useNavigate();
  const { user, accessToken, logout, lang } = useSimStore();
  const l = lang as LangCode;
  const [users, setUsers] = useState<UserRow[]>([]);
  const [loadError, setLoadError] = useState(false);
  const [tab, setTab] = useState<'pending' | 'approved' | 'all'>('pending');
  const [banReason, setBanReason] = useState('');
  const [banTarget, setBanTarget] = useState<string | null>(null);
  const [revealedTc, setRevealedTc] = useState<Set<string>>(new Set());
  const toggleTc = (id: string) => setRevealedTc(prev => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
  const headers = { Authorization: `Bearer ${accessToken}` };

  // The role check already happened in <AdminRoute> before this component
  // was even mounted -- re-checking `user` here too was redundant, and
  // vulnerable to bouncing back to "/" if `user` was momentarily unset on
  // this component's very first render for any reason (e.g. a hard reload
  // landing directly on /admin). Leaving admission entirely to the route
  // guard removes that extra failure mode.
  useEffect(() => {
    load();
  }, []);

  // Unlike SimulationPage's loadSimulation(), this had no error handling at
  // all -- a single transient network/cold-start blip left `users` empty
  // forever with no visible feedback, so the admin panel just looked
  // broken until a manual page reload happened to land on a working
  // request. That's what made it look like "admin login needs several
  // tries": login itself succeeded immediately every time, but this
  // subsequent fetch was the flaky, silently-failing part. Retries with
  // backoff now, same pattern as the simulation page.
  async function load() {
    const retryDelaysMs = [300, 800, 1500, 3000];
    for (let attempt = 0; attempt <= retryDelaysMs.length; attempt++) {
      try {
        const { data } = await axios.get(cloudUrl('/api/admin/users'), { headers });
        setUsers(data);
        setLoadError(false);
        return;
      } catch {
        if (attempt === retryDelaysMs.length) { setLoadError(true); return; }
        await new Promise(r => setTimeout(r, retryDelaysMs[attempt]));
      }
    }
  }

  async function approve(id: string) {
    await axios.post(cloudUrl(`/api/admin/users/${id}/approve`), {}, { headers });
    load();
  }

  async function reject(id: string) {
    if (!confirm(text(l, { tr: 'Kayıt talebi reddedilsin mi?', en: 'Reject this registration request?', de: 'Diese Registrierungsanfrage ablehnen?', fr: "Rejeter cette demande d'inscription ?", ar: 'رفض طلب التسجيل هذا؟' }))) return;
    await axios.post(cloudUrl(`/api/admin/users/${id}/reject`), {}, { headers });
    load();
  }

  async function ban(id: string) {
    await axios.post(cloudUrl(`/api/admin/users/${id}/ban`), { reason: banReason }, { headers });
    setBanTarget(null); setBanReason('');
    load();
  }

  async function unban(id: string) {
    await axios.post(cloudUrl(`/api/admin/users/${id}/unban`), {}, { headers });
    load();
  }

  async function deleteUser(id: string) {
    if (!confirm(text(l, { tr: 'Kullanıcı kalıcı olarak silinsin mi?', en: 'Permanently delete this user?', de: 'Diesen Benutzer dauerhaft löschen?', fr: 'Supprimer définitivement cet utilisateur ?', ar: 'حذف هذا المستخدم نهائياً؟' }))) return;
    await axios.delete(cloudUrl(`/api/admin/users/${id}`), { headers });
    load();
  }

  const pending = users.filter(u => !u.is_approved && u.role === 'pending');
  const approved = users.filter(u => u.is_approved);
  const displayed = tab === 'pending' ? pending : tab === 'approved' ? approved : users;

  return (
    <div className="min-h-screen text-sim-text flex flex-col" style={{ background: '#030310' }}>
      <div className="pointer-events-none fixed inset-0"
        style={{ background: 'repeating-linear-gradient(to bottom, transparent 0, transparent 2px, rgba(0,0,0,0.06) 2px, rgba(0,0,0,0.06) 4px)' }} />

      {/* Header — dashboard ile aynı stil */}
      <div className="sticky top-0 z-10"
        style={{
          background: 'rgba(3,3,16,0.97)',
          borderBottom: '1px solid rgba(200,34,34,0.7)',
          backdropFilter: 'blur(20px)',
          boxShadow: '0 2px 20px rgba(200,34,34,0.5), 0 0 8px rgba(200,34,34,0.3)',
        }}>
        <div className="w-full px-3 sm:px-6 h-14 sm:h-16 flex items-center justify-between gap-2">
          {/* Brand */}
          <div className="flex items-center gap-2 flex-shrink-0">
            <div className="relative w-7 h-7 flex items-center justify-center">
              <div className="absolute inset-0 neon-breathe"
                style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '1.5px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }} />
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
                <circle cx="12" cy="12" r="10" />
                <ellipse cx="12" cy="12" rx="6" ry="14.29" />
                <path d="M2 12h20" />
                <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
              </svg>
            </div>
            <div className="flex flex-col leading-none gap-0.5 items-center">
              <span className="font-orbitron font-bold tracking-[0.2em]" style={{ fontSize: 'clamp(12px, 3.8vw, 18px)', color: '#e0e0f0' }}>{l === 'tr' ? 'ANATOLİA-SİM' : 'ANATOLIA-SIM'}</span>
              <span className="font-share-tech tracking-[0.25em]" style={{ fontSize: 'clamp(10px, 3vw, 16px)', color: '#cc2222' }}>{text(l, { tr: 'YÖNETİM PANELİ', en: 'ADMIN PANEL', de: 'ADMIN-PANEL', fr: "PANNEAU D'ADMINISTRATION", ar: 'لوحة الإدارة' })}</span>
            </div>
          </div>

          {/* Right actions */}
          <div className="flex items-center gap-2 flex-shrink-0">
            {pending.length > 0 && (
              <div className="flex items-center gap-2 px-3 py-1"
                style={{ background: 'rgba(212,168,56,0.1)', border: '1px solid rgba(212,168,56,0.3)' }}>
                <Clock size={13} className="text-sim-gold" />
                <span className="font-share-tech text-sim-gold tracking-widest" style={{ fontSize: 13 }}>{pending.length} {text(l, { tr: 'BEKLEYEN', en: 'PENDING', de: 'AUSSTEHEND', fr: 'EN ATTENTE', ar: 'قيد الانتظار' })}</span>
              </div>
            )}
            <span className="hidden sm:block font-share-tech tracking-widest font-bold" style={{ fontSize: 14, color: '#ffffff' }}>{user?.username?.toUpperCase()}</span>
            <button onClick={() => { logout(); navigate('/login'); }}
              className="flex items-center gap-1.5 transition-colors"
              style={{ fontFamily: 'Share Tech Mono,monospace', fontSize: 14, fontWeight: 700, letterSpacing: '0.1em', color: '#ffffff', border: 'none', background: 'transparent', padding: '4px 10px' }}>
              <LogOut size={13} />
              <span className="hidden sm:inline">{text(l, { tr: 'ÇIKIŞ', en: 'EXIT', de: 'AUSGANG', fr: 'QUITTER', ar: 'خروج' })}</span>
            </button>
            <button onClick={() => navigate('/')}
              style={{ display: 'flex', alignItems: 'center', gap: 3, padding: '4px 10px', border: 'none', color: '#ffffff', background: 'transparent', fontSize: 14, letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace', cursor: 'pointer', flexShrink: 0 }}>
              ☰ {text(l, { tr: 'MENÜ', en: 'MENU', de: 'MENÜ', fr: 'MENU', ar: 'القائمة' })}
            </button>
          </div>
        </div>
      </div>

      <div className="max-w-6xl mx-auto px-6 py-8 relative flex-1 w-full pb-16">
        {/* Tabs */}
        <div className="hud-panel mb-6 relative flex flex-wrap items-stretch">
          <span className="hud-corner-tr" /><span className="hud-corner-bl" />
          {([
            ['pending',  text(l, { tr: 'BEKLEYEN', en: 'PENDING', de: 'AUSSTEHEND', fr: 'EN ATTENTE', ar: 'قيد الانتظار' }),     pending.length],
            ['approved', text(l, { tr: 'ONAYLANANLAR', en: 'APPROVED', de: 'GENEHMIGT', fr: 'APPROUVÉS', ar: 'موافَق عليهم' }), approved.length],
            ['all',      text(l, { tr: 'TÜMÜ', en: 'ALL', de: 'ALLE', fr: 'TOUS', ar: 'الكل' }),         users.length],
          ] as const).map(([key, label, count]) => (
            <button key={key} onClick={() => setTab(key)}
              className="font-share-tech tracking-widest px-4 py-2 transition-all"
              style={{
                fontSize: 13,
                background: tab === key ? 'rgba(200,34,34,0.18)' : 'transparent',
                border: 'none',
                borderRight: '1px solid rgba(200,34,34,0.25)',
                boxShadow: tab === key ? 'inset 0 -2px 0 rgba(200,34,34,0.8)' : 'none',
                color: tab === key ? '#ffffff' : 'rgba(255,255,255,0.45)',
              }}>
              {label} ({count})
            </button>
          ))}
          <div className="flex-1" />
          <button onClick={() => navigate('/')}
            className="font-share-tech tracking-widest px-4 py-2 transition-all"
            style={{
              fontSize: 13,
              background: 'transparent',
              border: 'none',
              borderLeft: '1px solid rgba(200,34,34,0.25)',
              color: 'rgba(255,255,255,0.45)',
            }}>
            {text(l, { tr: '← SİMÜLASYONLAR', en: '← SIMULATIONS', de: '← SIMULATIONEN', fr: '← SIMULATIONS', ar: '← المحاكاة' })}
          </button>
        </div>

        {/* Ban modal */}
        {banTarget && (
          <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: 'rgba(0,0,0,0.7)' }}>
            <div className="w-96 p-6" style={{ background: 'rgba(4,4,15,0.98)', border: '1px solid rgba(224,90,90,0.4)' }}>
              <div className="font-orbitron text-sim-red font-bold tracking-widest mb-4" style={{ fontSize: 14 }}>{text(l, { tr: 'KULLANICI ENGELLE', en: 'BAN USER', de: 'BENUTZER SPERREN', fr: "BANNIR L'UTILISATEUR", ar: 'حظر المستخدم' })}</div>
              <input
                className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-red mb-4"
                style={{ fontSize: 14 }}
                placeholder={text(l, { tr: 'Engelleme sebebi (opsiyonel)', en: 'Ban reason (optional)', de: 'Sperrgrund (optional)', fr: 'Motif du bannissement (facultatif)', ar: 'سبب الحظر (اختياري)' })}
                value={banReason}
                onChange={e => setBanReason(e.target.value)}
              />
              <div className="flex gap-2">
                <button onClick={() => ban(banTarget)}
                  className="flex-1 py-2 font-share-tech tracking-widest text-sim-red"
                  style={{ fontSize: 13, background: 'rgba(224,90,90,0.15)', border: '1px solid rgba(224,90,90,0.4)' }}>
                  {text(l, { tr: 'ENGELLE', en: 'BAN', de: 'SPERREN', fr: 'BANNIR', ar: 'حظر' })}
                </button>
                <button onClick={() => { setBanTarget(null); setBanReason(''); }}
                  className="flex-1 py-2 font-share-tech tracking-widest text-sim-muted"
                  style={{ fontSize: 13, background: 'rgba(22,22,58,0.5)', border: '1px solid rgba(79,110,247,0.15)' }}>
                  {text(l, { tr: 'İPTAL', en: 'CANCEL', de: 'ABBRECHEN', fr: 'ANNULER', ar: 'إلغاء' })}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* User table */}
        {displayed.length === 0 ? (
          <div className="hud-panel flex flex-col items-center justify-center py-20">
            <span className="hud-corner-tr" /><span className="hud-corner-bl" />
            <div className="relative w-16 h-16 flex items-center justify-center mb-5">
              <div className="absolute inset-0 rounded-full" style={{
                border: '1.5px solid rgba(200,34,34,0.7)',
                boxShadow: '0 0 10px rgba(200,34,34,0.5), inset 0 0 10px rgba(200,34,34,0.1)',
                animation: 'ring-expand 2.4s ease-out infinite',
              }} />
              <div className="absolute inset-0 rounded-full" style={{
                border: '1px solid rgba(200,34,34,0.45)',
                boxShadow: '0 0 14px rgba(200,34,34,0.35)',
                animation: 'ring-expand 2.4s ease-out 0.8s infinite',
              }} />
              <div className="absolute inset-0 rounded-full" style={{
                border: '1px solid rgba(200,34,34,0.25)',
                animation: 'ring-expand 2.4s ease-out 1.6s infinite',
              }} />
              <div className="relative w-10 h-10 flex items-center justify-center neon-breathe"
                style={{ background: 'linear-gradient(135deg, rgba(79,110,247,0.35), rgba(79,110,247,0.08))', border: '2px solid #6f8bff', clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)' }}>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#ffffff" strokeWidth="1.6" strokeLinecap="round" style={{ overflow: 'visible' }}>
                  <circle cx="12" cy="12" r="10" />
                  <ellipse cx="12" cy="12" rx="6" ry="14.29" />
                  <path d="M2 12h20" />
                  <circle cx="12" cy="12" r="1.8" fill="#00e887" stroke="none" />
                </svg>
              </div>
            </div>
            {loadError ? (
              <>
                <p className="font-share-tech tracking-widest" style={{ fontSize: 14, color: '#e05a5a' }}>{text(l, { tr: 'KULLANICI LİSTESİ YÜKLENEMEDİ', en: 'FAILED TO LOAD USER LIST', de: 'BENUTZERLISTE KONNTE NICHT GELADEN WERDEN', fr: "ÉCHEC DU CHARGEMENT DE LA LISTE", ar: 'فشل تحميل قائمة المستخدمين' })}</p>
                <button onClick={load} className="mt-3 font-share-tech tracking-widest"
                  style={{ fontSize: 12, color: '#4f9ef7', border: '1px solid rgba(79,158,247,0.4)', background: 'rgba(79,158,247,0.1)', padding: '6px 14px', cursor: 'pointer' }}>
                  {text(l, { tr: 'TEKRAR DENE', en: 'RETRY', de: 'ERNEUT VERSUCHEN', fr: 'RÉESSAYER', ar: 'إعادة المحاولة' })}
                </button>
              </>
            ) : (
              <p className="font-share-tech tracking-widest" style={{ fontSize: 14, color: '#ffffff' }}>{text(l, { tr: 'KAYIT BULUNAMADI', en: 'NO RECORDS FOUND', de: 'KEINE EINTRÄGE GEFUNDEN', fr: 'AUCUN ENREGISTREMENT TROUVÉ', ar: 'لا توجد سجلات' })}</p>
            )}
          </div>
        ) : (
          <div style={{ border: '1px solid rgba(79,110,247,0.18)', background: 'rgba(4,4,15,0.9)' }}>
            <table className="w-full">
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(79,110,247,0.2)' }}>
                  {[
                    text(l, { tr: 'KOD', en: 'CODE', de: 'CODE', fr: 'CODE', ar: 'الرمز' }),
                    text(l, { tr: 'AD SOYAD', en: 'FULL NAME', de: 'NAME', fr: 'NOM COMPLET', ar: 'الاسم الكامل' }),
                    text(l, { tr: 'TC NO', en: 'ID NO', de: 'AUSWEIS-NR.', fr: "N° D'IDENTITÉ", ar: 'رقم الهوية' }),
                    text(l, { tr: 'E-POSTA', en: 'EMAIL', de: 'E-MAIL', fr: 'E-MAIL', ar: 'البريد الإلكتروني' }),
                    text(l, { tr: 'DURUM', en: 'STATUS', de: 'STATUS', fr: 'STATUT', ar: 'الحالة' }),
                    text(l, { tr: 'TARİH', en: 'DATE', de: 'DATUM', fr: 'DATE', ar: 'التاريخ' }),
                    text(l, { tr: 'İŞLEMLER', en: 'ACTIONS', de: 'AKTIONEN', fr: 'ACTIONS', ar: 'الإجراءات' }),
                  ].map(h => (
                    <th key={h} className="text-left px-4 py-3">
                      <span className="font-share-tech tracking-widest" style={{ fontSize: 11, color: '#4f6ef7' }}>{h}</span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {displayed.map(u => (
                  <tr key={u.id} style={{ borderBottom: '1px solid rgba(79,110,247,0.06)' }}
                    className="hover:bg-sim-border/10 transition-colors">
                    <td className="px-4 py-3">
                      <span className="font-orbitron font-bold text-sim-accent" style={{ fontSize: 13 }}>{u.user_code}</span>
                    </td>
                    <td className="px-4 py-3">
                      <span className="font-share-tech text-sim-text" style={{ fontSize: 14 }}>{u.first_name} {u.last_name}</span>
                    </td>
                    <td className="px-4 py-3">
                      <button
                        onClick={() => toggleTc(u.id)}
                        title={text(l, { tr: 'Göster/Gizle', en: 'Show/Hide', de: 'Anzeigen/Verbergen', fr: 'Afficher/Masquer', ar: 'إظهار/إخفاء' })}
                        className="flex items-center gap-1.5 font-share-tech hover:text-sim-accent transition-colors"
                        style={{ fontSize: 13, color: '#8abda0' }}
                      >
                        {revealedTc.has(u.id) ? <Eye size={12} /> : <EyeOff size={12} />}
                        {revealedTc.has(u.id) ? u.tc_no : maskTc(u.tc_no)}
                      </button>
                    </td>
                    <td className="px-4 py-3">
                      <span className="font-share-tech" style={{ fontSize: 13, color: '#8abda0' }}>{u.email}</span>
                    </td>
                    <td className="px-4 py-3">
                      {u.is_banned
                        ? <Badge label={text(l, { tr: 'BANLANDI', en: 'BANNED', de: 'GESPERRT', fr: 'BANNI', ar: 'محظور' })} color="#e05a5a" />
                        : u.is_approved
                          ? <Badge label={text(l, { tr: 'ONAYLANDI', en: 'APPROVED', de: 'GENEHMIGT', fr: 'APPROUVÉ', ar: 'موافَق عليه' })} color="#4ecb71" />
                          : <Badge label={text(l, { tr: 'BEKLIYOR', en: 'PENDING', de: 'AUSSTEHEND', fr: 'EN ATTENTE', ar: 'قيد الانتظار' })} color="#d4a838" />}
                      {u.role === 'admin' && <Badge label={text(l, { tr: 'ADMİN', en: 'ADMIN', de: 'ADMIN', fr: 'ADMIN', ar: 'مسؤول' })} color="#00d4ff" />}
                    </td>
                    <td className="px-4 py-3">
                      <span className="font-share-tech" style={{ fontSize: 13, color: '#6090a0' }}>
                        {new Date(u.created_at).toLocaleDateString(LOCALE_MAP[l] ?? 'en-US')}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-1">
                        {!u.is_approved && u.role !== 'admin' && (<>
                          <button onClick={() => approve(u.id)} title={text(l, { tr: 'Onayla', en: 'Approve', de: 'Genehmigen', fr: 'Approuver', ar: 'موافقة' })}
                            className="p-1.5 text-sim-green hover:bg-sim-green/10 transition-colors rounded">
                            <CheckCircle size={16} />
                          </button>
                          <button onClick={() => reject(u.id)} title={text(l, { tr: 'Reddet', en: 'Reject', de: 'Ablehnen', fr: 'Rejeter', ar: 'رفض' })}
                            className="p-1.5 text-sim-red hover:bg-sim-red/10 transition-colors rounded">
                            <XCircle size={16} />
                          </button>
                        </>)}
                        {u.is_approved && u.role !== 'admin' && (
                          u.is_banned
                            ? <button onClick={() => unban(u.id)} title={text(l, { tr: 'Engeli Kaldır', en: 'Unban', de: 'Entsperren', fr: 'Débannir', ar: 'إلغاء الحظر' })}
                                className="p-1.5 text-sim-gold hover:bg-sim-gold/10 transition-colors rounded">
                                <ShieldOff size={16} />
                              </button>
                            : <button onClick={() => setBanTarget(u.id)} title={text(l, { tr: 'Engelle', en: 'Ban', de: 'Sperren', fr: 'Bannir', ar: 'حظر' })}
                                className="p-1.5 text-sim-red hover:bg-sim-red/10 transition-colors rounded">
                                <Ban size={16} />
                              </button>
                        )}
                        {u.role !== 'admin' && (
                          <button onClick={() => deleteUser(u.id)} title={text(l, { tr: 'Sil', en: 'Delete', de: 'Löschen', fr: 'Supprimer', ar: 'حذف' })}
                            className="p-1.5 text-sim-muted hover:text-sim-red hover:bg-sim-red/10 transition-colors rounded">
                            <Trash2 size={16} />
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <FooterBar mode="fixed" />
    </div>
  );
}
