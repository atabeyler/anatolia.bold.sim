import { useState, useEffect } from 'react';
import FooterBar from '../components/layout/FooterBar';
import { useNavigate } from 'react-router-dom';
import axios from 'axios';
import { useSimStore } from '../store/simStore';
import { LogOut, CheckCircle, XCircle, Ban, Trash2, ShieldOff, Clock, Eye, EyeOff, Pencil } from 'lucide-react';
import { text, type LangCode } from '../utils/i18n';
import { cloudUrl } from '../utils/cloud';

const LOCALE_MAP: Record<string, string> = { tr: 'tr-TR', en: 'en-US', de: 'de-DE', fr: 'fr-FR', ar: 'ar-SA' };

type UserRow = {
  id: string; user_code: string; first_name: string; last_name: string;
  tc_no: string | null; email: string; role: string; username: string | null;
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
function maskTc(tc: string | null): string {
  if (!tc) return '—';
  if (tc.length <= 4) return '•'.repeat(tc.length);
  return '•'.repeat(tc.length - 4) + tc.slice(-4);
}

export default function AdminPage() {
  const navigate = useNavigate();
  const { user, accessToken, logout, lang } = useSimStore();
  const l = lang as LangCode;
  const [users, setUsers] = useState<UserRow[]>([]);
  const [loadError, setLoadError] = useState(false);
  const [banReason, setBanReason] = useState('');
  const [banTarget, setBanTarget] = useState<string | null>(null);
  const [revealedTc, setRevealedTc] = useState<Set<string>>(new Set());
  const [newCode, setNewCode] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [newFirstName, setNewFirstName] = useState('');
  const [newLastName, setNewLastName] = useState('');
  const [newTcNo, setNewTcNo] = useState('');
  const [newUsername, setNewUsername] = useState('');
  const [newEmail, setNewEmail] = useState('');
  const [newIsAdmin, setNewIsAdmin] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editTarget, setEditTarget] = useState<UserRow | null>(null);
  const [editCode, setEditCode] = useState('');
  const [editFirstName, setEditFirstName] = useState('');
  const [editLastName, setEditLastName] = useState('');
  const [editTcNo, setEditTcNo] = useState('');
  const [editUsername, setEditUsername] = useState('');
  const [editEmail, setEditEmail] = useState('');
  const [editPassword, setEditPassword] = useState('');
  const [editIsAdmin, setEditIsAdmin] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const [editSaving, setEditSaving] = useState(false);
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

  async function createUser() {
    setCreateError(null);
    setCreating(true);
    try {
      await axios.post(cloudUrl('/api/admin/users'), {
        user_code: newCode,
        password: newPassword,
        first_name: newFirstName,
        last_name: newLastName,
        tc_no: newTcNo,
        username: newUsername.trim() || null,
        email: newEmail.trim() || null,
        is_admin: newIsAdmin,
      }, { headers });
      setNewCode(''); setNewPassword(''); setNewFirstName(''); setNewLastName(''); setNewTcNo('');
      setNewUsername(''); setNewEmail(''); setNewIsAdmin(false);
      load();
    } catch (err: any) {
      setCreateError(err?.response?.data?.error ?? text(l, { tr: 'Kullanıcı oluşturulamadı.', en: 'Failed to create user.', de: 'Benutzer konnte nicht erstellt werden.', fr: "Échec de la création de l'utilisateur.", ar: 'فشل إنشاء المستخدم.' }));
    } finally {
      setCreating(false);
    }
  }

  function openEdit(u: UserRow) {
    setEditTarget(u);
    setEditCode(u.user_code ?? '');
    setEditFirstName(u.first_name ?? '');
    setEditLastName(u.last_name ?? '');
    setEditTcNo(u.tc_no ?? '');
    setEditUsername(u.username ?? '');
    setEditEmail(u.email.endsWith('@no-email.internal') ? '' : u.email);
    setEditPassword('');
    setEditIsAdmin(u.role === 'admin');
    setEditError(null);
  }

  async function saveEdit() {
    if (!editTarget) return;
    setEditError(null);
    setEditSaving(true);
    try {
      await axios.put(cloudUrl(`/api/admin/users/${editTarget.id}`), {
        user_code: editCode,
        first_name: editFirstName,
        last_name: editLastName,
        tc_no: editTcNo,
        username: editUsername.trim() || null,
        email: editEmail.trim() || null,
        password: editPassword.trim() || null,
        is_admin: editIsAdmin,
      }, { headers });
      setEditTarget(null);
      load();
    } catch (err: any) {
      setEditError(err?.response?.data?.error ?? text(l, { tr: 'Kullanıcı güncellenemedi.', en: 'Failed to update user.', de: 'Benutzer konnte nicht aktualisiert werden.', fr: "Échec de la mise à jour de l'utilisateur.", ar: 'فشل تحديث المستخدم.' }));
    } finally {
      setEditSaving(false);
    }
  }

  async function deleteUser(id: string) {
    if (!confirm(text(l, { tr: 'Kullanıcı kalıcı olarak silinsin mi?', en: 'Permanently delete this user?', de: 'Diesen Benutzer dauerhaft löschen?', fr: 'Supprimer définitivement cet utilisateur ?', ar: 'حذف هذا المستخدم نهائياً؟' }))) return;
    await axios.delete(cloudUrl(`/api/admin/users/${id}`), { headers });
    load();
  }

  // Shared between the desktop table row and the mobile card layout below,
  // so the two views can never drift apart on which actions a given user
  // gets (this is exactly what the mobile bug report turned out to be --
  // the actions weren't missing, they were just off-screen past the
  // table's unwrapped horizontal overflow on a narrow viewport).
  function UserActions({ u }: { u: UserRow }) {
    return (
      <>
        <button onClick={() => openEdit(u)} title={text(l, { tr: 'Düzelt', en: 'Edit', de: 'Bearbeiten', fr: 'Modifier', ar: 'تعديل' })}
          className="p-1.5 text-sim-accent hover:bg-sim-accent/10 transition-colors rounded">
          <Pencil size={16} />
        </button>
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
      </>
    );
  }

  const pending = users.filter(u => !u.is_approved && u.role === 'pending');
  // A single, unified list instead of separate pending/approved/all tabs --
  // pending registrations (the ones actually needing admin action) surface
  // at the top rather than being hidden behind a tab switch, with the rest
  // ordered newest-first below them.
  const displayed = [...users].sort((a, b) => {
    const aPending = !a.is_approved && a.role === 'pending';
    const bPending = !b.is_approved && b.role === 'pending';
    if (aPending !== bPending) return aPending ? -1 : 1;
    return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
  });

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
        {/* Header bar */}
        <div className="hud-panel mb-6 relative flex flex-wrap items-center px-4 py-2.5">
          <span className="hud-corner-tr" /><span className="hud-corner-bl" />
          <span className="font-share-tech tracking-widest" style={{ fontSize: 13, color: '#ffffff' }}>
            {text(l, { tr: 'KULLANICILAR', en: 'USERS', de: 'BENUTZER', fr: 'UTILISATEURS', ar: 'المستخدمون' })} ({users.length})
          </span>
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

        {/* Yeni kullanıcı ekle */}
        <div className="hud-panel mb-6 relative p-4 sm:p-5">
          <span className="hud-corner-tr" /><span className="hud-corner-bl" />
          <div className="font-orbitron font-bold tracking-widest mb-4" style={{ fontSize: 13, color: '#d4a838' }}>
            {text(l, { tr: '+ YENİ KULLANICI EKLE', en: '+ ADD NEW USER', de: '+ NEUEN BENUTZER HINZUFÜGEN', fr: '+ AJOUTER UN UTILISATEUR', ar: '+ إضافة مستخدم جديد' })}
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'Ad', en: 'First name', de: 'Vorname', fr: 'Prénom', ar: 'الاسم' })}
              value={newFirstName}
              onChange={e => setNewFirstName(e.target.value)}
            />
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'Soyad', en: 'Last name', de: 'Nachname', fr: 'Nom', ar: 'اللقب' })}
              value={newLastName}
              onChange={e => setNewLastName(e.target.value)}
            />
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'TC Kimlik No (11 hane)', en: 'National ID (11 digits)', de: 'Ausweisnummer (11 Ziffern)', fr: "N° d'identité (11 chiffres)", ar: 'رقم الهوية (11 رقمًا)' })}
              value={newTcNo}
              maxLength={11}
              onChange={e => setNewTcNo(e.target.value.replace(/\D/g, ''))}
            />
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'Kullanıcı kodu', en: 'User code', de: 'Benutzercode', fr: 'Code utilisateur', ar: 'رمز المستخدم' })}
              value={newCode}
              onChange={e => setNewCode(e.target.value)}
            />
            <input
              type="password"
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'Şifre (min 8 karakter)', en: 'Password (min 8 chars)', de: 'Passwort (min. 8 Zeichen)', fr: 'Mot de passe (min. 8 caractères)', ar: 'كلمة المرور (٨ أحرف على الأقل)' })}
              value={newPassword}
              onChange={e => setNewPassword(e.target.value)}
            />
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'Rumuz (opsiyonel)', en: 'Nickname (optional)', de: 'Spitzname (optional)', fr: 'Pseudo (facultatif)', ar: 'الاسم المستعار (اختياري)' })}
              value={newUsername}
              onChange={e => setNewUsername(e.target.value)}
            />
            <input
              className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
              style={{ fontSize: 14 }}
              placeholder={text(l, { tr: 'E-posta (bildirimler için)', en: 'Email (for notifications)', de: 'E-Mail (für Benachrichtigungen)', fr: 'E-mail (pour les notifications)', ar: 'البريد الإلكتروني (للإشعارات)' })}
              value={newEmail}
              onChange={e => setNewEmail(e.target.value)}
            />
          </div>
          <label className="flex items-center gap-2 mb-3 cursor-pointer select-none">
            <input type="checkbox" checked={newIsAdmin} onChange={e => setNewIsAdmin(e.target.checked)} />
            <span className="font-share-tech" style={{ fontSize: 13, color: '#ffffff' }}>{text(l, { tr: 'Admin yetkisi', en: 'Admin permission', de: 'Admin-Berechtigung', fr: 'Droits admin', ar: 'صلاحيات المسؤول' })}</span>
          </label>
          <p className="font-share-tech mb-3" style={{ fontSize: 11, color: 'rgba(212,168,56,0.75)' }}>
            {text(l, {
              tr: 'E-posta girilirse, bu kullanıcı çevrimdışıyken de acil durum bildirimleri e-posta ile iletilir.',
              en: 'If an email is provided, this user receives urgent notifications by email even while offline.',
              de: 'Bei angegebener E-Mail erhält dieser Benutzer auch offline dringende Benachrichtigungen per E-Mail.',
              fr: "Si un e-mail est fourni, cet utilisateur reçoit les notifications urgentes par e-mail même hors ligne.",
              ar: 'إذا تم إدخال بريد إلكتروني، سيتلقى هذا المستخدم إشعارات عاجلة عبر البريد حتى عند عدم الاتصال.',
            })}
          </p>
          {createError && (
            <p className="font-share-tech mb-3" style={{ fontSize: 12, color: '#e05a5a' }}>{createError}</p>
          )}
          <button
            onClick={createUser}
            disabled={creating || !newCode.trim() || !newPassword || !newFirstName.trim() || !newLastName.trim() || newTcNo.length !== 11}
            className="w-full py-2.5 font-share-tech tracking-widest transition-colors"
            style={{
              fontSize: 13,
              color: '#4f9ef7',
              background: 'rgba(79,158,247,0.12)',
              border: '1px solid rgba(79,158,247,0.4)',
              opacity: creating || !newCode.trim() || !newPassword || !newFirstName.trim() || !newLastName.trim() || newTcNo.length !== 11 ? 0.5 : 1,
              cursor: creating || !newCode.trim() || !newPassword || !newFirstName.trim() || !newLastName.trim() || newTcNo.length !== 11 ? 'default' : 'pointer',
            }}>
            {creating
              ? text(l, { tr: 'EKLENİYOR…', en: 'ADDING…', de: 'WIRD HINZUGEFÜGT…', fr: 'AJOUT…', ar: 'جارٍ الإضافة…' })
              : text(l, { tr: 'EKLE', en: 'ADD', de: 'HINZUFÜGEN', fr: 'AJOUTER', ar: 'إضافة' })}
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

        {/* Edit modal */}
        {editTarget && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ background: 'rgba(0,0,0,0.7)' }}>
            <div className="w-full max-w-md p-6" style={{ background: 'rgba(4,4,15,0.98)', border: '1px solid rgba(79,158,247,0.4)', maxHeight: '90vh', overflowY: 'auto' }}>
              <div className="font-orbitron text-sim-accent font-bold tracking-widest mb-4" style={{ fontSize: 14 }}>
                {text(l, { tr: 'KULLANICIYI DÜZENLE', en: 'EDIT USER', de: 'BENUTZER BEARBEITEN', fr: "MODIFIER L'UTILISATEUR", ar: 'تعديل المستخدم' })}
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'Ad', en: 'First name', de: 'Vorname', fr: 'Prénom', ar: 'الاسم' })}
                  value={editFirstName} onChange={e => setEditFirstName(e.target.value)} />
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'Soyad', en: 'Last name', de: 'Nachname', fr: 'Nom', ar: 'اللقب' })}
                  value={editLastName} onChange={e => setEditLastName(e.target.value)} />
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'TC Kimlik No (11 hane)', en: 'National ID (11 digits)', de: 'Ausweisnummer (11 Ziffern)', fr: "N° d'identité (11 chiffres)", ar: 'رقم الهوية (11 رقمًا)' })}
                  value={editTcNo} maxLength={11} onChange={e => setEditTcNo(e.target.value.replace(/\D/g, ''))} />
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'Kullanıcı kodu', en: 'User code', de: 'Benutzercode', fr: 'Code utilisateur', ar: 'رمز المستخدم' })}
                  value={editCode} onChange={e => setEditCode(e.target.value)} />
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'Rumuz (opsiyonel)', en: 'Nickname (optional)', de: 'Spitzname (optional)', fr: 'Pseudo (facultatif)', ar: 'الاسم المستعار (اختياري)' })}
                  value={editUsername} onChange={e => setEditUsername(e.target.value)} />
                <input className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'E-posta (bildirimler için)', en: 'Email (for notifications)', de: 'E-Mail (für Benachrichtigungen)', fr: 'E-mail (pour les notifications)', ar: 'البريد الإلكتروني (للإشعارات)' })}
                  value={editEmail} onChange={e => setEditEmail(e.target.value)} />
                <input type="password" className="w-full bg-sim-bg border border-sim-border px-3 py-2 font-share-tech text-sim-text focus:outline-none focus:border-sim-accent sm:col-span-2"
                  style={{ fontSize: 14 }}
                  placeholder={text(l, { tr: 'Yeni şifre (opsiyonel, boş bırakılırsa değişmez)', en: 'New password (optional, leave blank to keep current)', de: 'Neues Passwort (optional)', fr: 'Nouveau mot de passe (facultatif)', ar: 'كلمة مرور جديدة (اختياري)' })}
                  value={editPassword} onChange={e => setEditPassword(e.target.value)} />
              </div>
              <label className="flex items-center gap-2 mb-4 cursor-pointer select-none">
                <input type="checkbox" checked={editIsAdmin} onChange={e => setEditIsAdmin(e.target.checked)} />
                <span className="font-share-tech" style={{ fontSize: 13, color: '#ffffff' }}>{text(l, { tr: 'Admin yetkisi', en: 'Admin permission', de: 'Admin-Berechtigung', fr: 'Droits admin', ar: 'صلاحيات المسؤول' })}</span>
              </label>
              {editError && (
                <p className="font-share-tech mb-3" style={{ fontSize: 12, color: '#e05a5a' }}>{editError}</p>
              )}
              <div className="flex gap-2">
                <button onClick={saveEdit}
                  disabled={editSaving || !editCode.trim() || !editFirstName.trim() || !editLastName.trim() || editTcNo.length !== 11}
                  className="flex-1 py-2 font-share-tech tracking-widest text-sim-accent"
                  style={{
                    fontSize: 13, background: 'rgba(79,158,247,0.15)', border: '1px solid rgba(79,158,247,0.4)',
                    opacity: editSaving || !editCode.trim() || !editFirstName.trim() || !editLastName.trim() || editTcNo.length !== 11 ? 0.5 : 1,
                  }}>
                  {editSaving
                    ? text(l, { tr: 'KAYDEDİLİYOR…', en: 'SAVING…', de: 'WIRD GESPEICHERT…', fr: 'ENREGISTREMENT…', ar: 'جارٍ الحفظ…' })
                    : text(l, { tr: 'KAYDET', en: 'SAVE', de: 'SPEICHERN', fr: 'ENREGISTRER', ar: 'حفظ' })}
                </button>
                <button onClick={() => setEditTarget(null)}
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
          <>
            {/* Mobile: stacked cards -- a 7-column table has no way to fit a
                narrow viewport, and letting it overflow (the previous
                behavior) pushed the actions column off-screen entirely
                with no visual hint there was more to scroll to. */}
            <div className="flex flex-col gap-3 sm:hidden">
              {displayed.map(u => (
                <div key={u.id} className="hud-panel relative p-3.5">
                  <span className="hud-corner-tr" /><span className="hud-corner-bl" />
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div>
                      <div className="font-orbitron font-bold text-sim-accent" style={{ fontSize: 14 }}>{u.user_code}</div>
                      <div className="font-share-tech text-sim-text" style={{ fontSize: 14 }}>
                        {(u.first_name || u.last_name) ? `${u.first_name} ${u.last_name}`.trim() : (u.username ?? '—')}
                      </div>
                    </div>
                    <div className="flex flex-wrap justify-end gap-1">
                      {u.is_banned
                        ? <Badge label={text(l, { tr: 'BANLANDI', en: 'BANNED', de: 'GESPERRT', fr: 'BANNI', ar: 'محظور' })} color="#e05a5a" />
                        : u.is_approved
                          ? <Badge label={text(l, { tr: 'ONAYLANDI', en: 'APPROVED', de: 'GENEHMIGT', fr: 'APPROUVÉ', ar: 'موافَق عليه' })} color="#4ecb71" />
                          : <Badge label={text(l, { tr: 'BEKLIYOR', en: 'PENDING', de: 'AUSSTEHEND', fr: 'EN ATTENTE', ar: 'قيد الانتظار' })} color="#d4a838" />}
                      {u.role === 'admin' && <Badge label={text(l, { tr: 'ADMİN', en: 'ADMIN', de: 'ADMIN', fr: 'ADMIN', ar: 'مسؤول' })} color="#00d4ff" />}
                    </div>
                  </div>
                  <button
                    onClick={() => toggleTc(u.id)}
                    className="flex items-center gap-1.5 font-share-tech hover:text-sim-accent transition-colors mb-1"
                    style={{ fontSize: 13, color: '#8abda0' }}
                  >
                    {revealedTc.has(u.id) ? <Eye size={12} /> : <EyeOff size={12} />}
                    {revealedTc.has(u.id) ? (u.tc_no ?? '—') : maskTc(u.tc_no)}
                  </button>
                  <div className="font-share-tech mb-1 break-all" style={{ fontSize: 13, color: '#8abda0' }}>
                    {u.email.endsWith('@no-email.internal') ? '—' : u.email}
                  </div>
                  <div className="font-share-tech mb-3" style={{ fontSize: 12, color: '#6090a0' }}>
                    {new Date(u.created_at).toLocaleDateString(LOCALE_MAP[l] ?? 'en-US')}
                  </div>
                  <div className="flex items-center gap-1 flex-wrap border-t pt-2" style={{ borderColor: 'rgba(79,110,247,0.15)' }}>
                    <UserActions u={u} />
                  </div>
                </div>
              ))}
            </div>

            {/* Desktop/tablet: table, wrapped so it scrolls within its own
                box instead of blowing out the page width. */}
            <div className="hidden sm:block overflow-x-auto" style={{ border: '1px solid rgba(79,110,247,0.18)', background: 'rgba(4,4,15,0.9)' }}>
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
                        <span className="font-share-tech text-sim-text" style={{ fontSize: 14 }}>
                          {(u.first_name || u.last_name) ? `${u.first_name} ${u.last_name}`.trim() : (u.username ?? '—')}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <button
                          onClick={() => toggleTc(u.id)}
                          title={text(l, { tr: 'Göster/Gizle', en: 'Show/Hide', de: 'Anzeigen/Verbergen', fr: 'Afficher/Masquer', ar: 'إظهار/إخفاء' })}
                          className="flex items-center gap-1.5 font-share-tech hover:text-sim-accent transition-colors"
                          style={{ fontSize: 13, color: '#8abda0' }}
                        >
                          {revealedTc.has(u.id) ? <Eye size={12} /> : <EyeOff size={12} />}
                          {revealedTc.has(u.id) ? (u.tc_no ?? '—') : maskTc(u.tc_no)}
                        </button>
                      </td>
                      <td className="px-4 py-3">
                        <span className="font-share-tech" style={{ fontSize: 13, color: '#8abda0' }}>{u.email.endsWith('@no-email.internal') ? '—' : u.email}</span>
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
                          <UserActions u={u} />
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>

      <FooterBar mode="fixed" />
    </div>
  );
}
