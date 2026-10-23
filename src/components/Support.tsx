import React, { useEffect, useMemo, useState } from 'react';
import QRCode from 'qrcode';
import support from '../about/support.json';
import { useT } from '../i18n';

type SupportKey = keyof typeof support;

const ORDER: SupportKey[] = ['card', 'iban', 'paypal', 'binance', 'usdtTrc20'];
const CRYPTO: SupportKey[] = ['binance', 'usdtTrc20'];

/** Заполненные владельцем реквизиты; пустые поля не показываем. */
export function supportEntries(): Array<{ key: SupportKey; value: string }> {
  return ORDER.map((key) => ({ key, value: (support[key] || '').trim() })).filter((e) => e.value);
}

/** QR без innerHTML: матрица qrcode → один SVG path. */
const Qr: React.FC<{ value: string; label: string }> = ({ value, label }) => {
  const path = useMemo(() => {
    const { modules } = QRCode.create(value, { errorCorrectionLevel: 'M' });
    const size = modules.size;
    let d = '';
    for (let r = 0; r < size; r++) {
      for (let c = 0; c < size; c++) {
        if (modules.get(r, c)) d += `M${c} ${r}h1v1h-1z`;
      }
    }
    return { d, size };
  }, [value]);

  return (
    <svg
      className="support-qr"
      viewBox={`0 0 ${path.size} ${path.size}`}
      role="img"
      aria-label={label}
      shapeRendering="crispEdges"
    >
      <path d={path.d} fill="currentColor" />
    </svg>
  );
};

interface SupportProps {
  /** compact — короткая полоска для боковой панели, без QR. */
  compact?: boolean;
  onOpen?: () => void;
}

export const Support: React.FC<SupportProps> = ({ compact = false, onOpen }) => {
  const t = useT();
  const entries = supportEntries();
  const [copied, setCopied] = useState<SupportKey | null>(null);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(null), 1500);
    return () => clearTimeout(timer);
  }, [copied]);

  if (entries.length === 0) return null;

  const copy = async (key: SupportKey, value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(key);
    } catch (err) {
      console.error(err);
    }
  };

  if (compact) {
    return (
      <button type="button" className="support-strip" onClick={onOpen}>
        <span className="support-strip__heart" aria-hidden="true">
          ♥
        </span>
        <span>{t('support.title')}</span>
      </button>
    );
  }

  return (
    <div className="grp support">
      <h3>
        {t('support.title')} <span>{t('support.sub')}</span>
      </h3>
      <div className="rows">
        {entries.map(({ key, value }) => (
          <button
            key={key}
            type="button"
            className="row support-row"
            onClick={() => void copy(key, value)}
            title={t('support.copy')}
          >
            <div className="nm">
              {t(`support.${key}`)}
              <span className="support-value">{value}</span>
            </div>
            {CRYPTO.includes(key) && (
              <Qr value={value} label={t('support.qr', { label: t(`support.${key}`) })} />
            )}
            <div className={`ok ${copied === key ? '' : 'support-hint'}`} aria-live="polite">
              {copied === key ? t('support.copied') : t('support.copy')}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
};
