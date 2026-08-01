import React from 'react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';

const WRAPPER_BASE: React.CSSProperties = {
  textAlign: 'center',
  background: 'rgba(3,3,16,0.97)',
  borderTop: '1px solid rgba(0,232,135,0.15)',
};

interface FooterBarProps {
  mode?: 'fixed' | 'flow' | 'inline';
  className?: string;
  style?: React.CSSProperties;
}

export default function FooterBar({ mode = 'fixed', className = '', style }: FooterBarProps) {
  const { lang, updatePercent, updateReady } = useSimStore();
  const bannerVisible = updatePercent !== null || updateReady !== null;
  const wrapperStyle: React.CSSProperties =
    mode === 'fixed'
      ? { ...WRAPPER_BASE, position: 'fixed', bottom: bannerVisible ? 44 : 0, left: 0, right: 0, zIndex: 50, padding: '5px 12px', display: 'flex', justifyContent: 'center', alignItems: 'center', transition: 'bottom 0.2s ease' }
      : mode === 'flow'
      ? { ...WRAPPER_BASE, flexShrink: 0, padding: '4px 10px', display: 'flex', justifyContent: 'center', alignItems: 'center' }
      : { padding: '0', background: 'transparent', border: 'none' };

  const companyName = text(lang as LangCode, {
    tr: 'Bold Askeri Teknoloji ve Savunma Sanayi A.Ş.',
    en: 'Bold Military Technology and Defense Industry Inc.',
    de: 'Bold Militärtechnologie und Verteidigungsindustrie AG',
    fr: 'Bold Technologie Militaire et Industrie de Défense S.A.',
    ar: 'شركة Bold للتكنولوجيا العسكرية وصناعة الدفاع',
  });
  const footerLabel = text(lang as LangCode, {
    tr: 'Tüm hakları saklıdır.',
    en: 'All rights reserved.',
    de: 'Alle Rechte vorbehalten.',
    fr: 'Tous droits réservés.',
    ar: 'جميع الحقوق محفوظة.',
  });

  return (
    <div style={{ ...wrapperStyle, ...style }} className={className}>
      <span className="footer-bar-text">
        RST Q-Nation 200120401018 © 2026 · <span className="footer-bar-company">{companyName}</span>
        <span className="footer-bar-rights">{footerLabel}</span>
      </span>
    </div>
  );
}
