import React, { useEffect } from 'react';
import { CheckCircle2, AlertCircle, Info, X } from 'lucide-react';

export default function Toast({ toast, onDismiss }) {
  const hasToast = Boolean(toast && toast.message);

  useEffect(() => {
    if (!hasToast) return;
    const timer = setTimeout(() => {
      onDismiss();
    }, 4500);
    return () => clearTimeout(timer);
  }, [hasToast, toast, onDismiss]);

  if (!hasToast) return null;

  const type = toast?.type || (toast?.isSuccess ? 'success' : 'error');
  const isSuccess = type === 'success';
  const isInfo = type === 'info';
  const toastClass = isSuccess ? 'toast-success' : isInfo ? 'toast-info' : 'toast-danger';

  return (
    <div className={`toast-banner ${toastClass}`}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        {isSuccess ? (
          <CheckCircle2 size={18} />
        ) : isInfo ? (
          <Info size={18} style={{ color: 'var(--gold, #F5C542)' }} />
        ) : (
          <AlertCircle size={18} />
        )}
        <span>{toast.message}</span>
      </div>
      <button
        onClick={onDismiss}
        style={{
          background: 'none',
          border: 'none',
          color: 'inherit',
          cursor: 'pointer',
          padding: '2px',
          display: 'flex',
          alignItems: 'center'
        }}
      >
        <X size={16} />
      </button>
    </div>
  );
}
