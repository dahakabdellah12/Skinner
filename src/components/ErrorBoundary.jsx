import React from 'react';
import { api } from '../services/api';

export default class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    this.setState({ errorInfo });
    console.error('[ErrorBoundary caught error]:', error, errorInfo);
    try {
      api.invoke('logClientError', {
        message: error?.message || String(error),
        stack: error?.stack || '',
        componentStack: errorInfo?.componentStack || ''
      });
    } catch { }
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
    this.props.onReset?.();
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div style={{
          padding: '36px 28px',
          margin: '24px auto',
          maxWidth: '720px',
          background: 'rgba(239, 68, 68, 0.08)',
          border: '1px solid rgba(239, 68, 68, 0.3)',
          borderRadius: '14px',
          textAlign: 'center',
          color: '#f8fafc',
          backdropFilter: 'blur(8px)'
        }}>
          <div style={{
            width: '52px',
            height: '52px',
            borderRadius: '50%',
            background: 'rgba(239, 68, 68, 0.18)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 16px',
            color: '#ef4444'
          }}>
            <svg viewBox="0 0 24 24" width="30" height="30" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </div>

          <h3 style={{ fontSize: '18px', fontWeight: 800, marginBottom: '8px', color: '#fca5a5' }}>
            حدث خطأ أثناء تحميل واجهة المعاينة والتخصيص
          </h3>
          <p style={{ fontSize: '13px', color: '#94a3b8', marginBottom: '18px', lineHeight: 1.6 }}>
            تعذر عرض عناصر هذه الأسطورة مؤقتاً. يمكنك الرجوع لقائمة الأساطير واختيار أسطورة أخرى، أو إعادة ضبط التخصيصات.
          </p>

          {this.state.error && (
            <div style={{
              background: 'rgba(0, 0, 0, 0.45)',
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '8px',
              padding: '12px',
              fontSize: '12px',
              fontFamily: 'monospace',
              color: '#f87171',
              textAlign: 'left',
              direction: 'ltr',
              maxHeight: '120px',
              overflowY: 'auto',
              marginBottom: '20px'
            }}>
              {this.state.error.message || String(this.state.error)}
            </div>
          )}

          <div style={{ display: 'flex', gap: '12px', justifyContent: 'center', flexWrap: 'wrap' }}>
            <button
              className="button-primary"
              onClick={this.handleReset}
              style={{ padding: '10px 22px', fontSize: '13px' }}
            >
              العودة إلى قائمة الأساطير / Back to Legends
            </button>
            <button
              className="button-secondary"
              onClick={() => {
                try {
                  localStorage.removeItem('brawlhalla_legend_customizations');
                } catch { }
                window.location.reload();
              }}
              style={{ padding: '10px 20px', fontSize: '13px' }}
            >
              إعادة ضبط البيانات المخزنة / Clear Cache
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
