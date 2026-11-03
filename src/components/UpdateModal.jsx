import React from 'react';
import { useLanguage } from '../i18n/LanguageContext';
import { Download, Pause, Play, X, ArrowUpCircle, ExternalLink } from 'lucide-react';

export default function UpdateModal({
  isOpen,
  updateInfo,
  updateProgress,
  isPaused,
  onProceed,
  onPause,
  onResume,
  onClose,
  onViewChangelog
}) {
  const { isArabic } = useLanguage();

  if (!isOpen || !updateInfo) return null;

  const isDownloading = updateProgress !== null;
  const progressPercent = updateProgress?.progress || 0;
  const isInstalling = updateProgress?.status === 'installing';

  // Format bytes helper
  const formatBytes = (bytes) => {
    if (!bytes || bytes <= 0) return '0 MB';
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.78)',
        backdropFilter: 'blur(6px)',
        WebkitBackdropFilter: 'blur(6px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 10001,
        padding: '20px',
        animation: 'updateModalFadeIn 0.2s ease-out'
      }}
      onClick={() => {
        if (!isDownloading) onClose();
      }}
    >
      <div
        className="card"
        style={{
          width: '100%',
          maxWidth: '520px',
          backgroundColor: 'var(--bg-card, #1E1E1E)',
          border: '1px solid var(--border-primary, #333)',
          borderRadius: '14px',
          boxShadow: '0 24px 48px rgba(0, 0, 0, 0.8)',
          padding: '24px',
          position: 'relative',
          direction: 'ltr'
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '18px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div
              style={{
                width: '40px',
                height: '40px',
                borderRadius: '10px',
                backgroundColor: 'rgba(59, 130, 246, 0.15)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--accent, #3B82F6)'
              }}
            >
              <ArrowUpCircle size={24} />
            </div>
            <div>
              <h2 style={{ fontSize: '17px', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>
                {isArabic
                  ? `يتوفر تحديث جديد: v${updateInfo.latestVersion}`
                  : `Software Update Available: v${updateInfo.latestVersion}`}
              </h2>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                {isArabic
                  ? `الإصدار الحالي: v${updateInfo.currentVersion}`
                  : `Current installed version: v${updateInfo.currentVersion}`}
              </span>
            </div>
          </div>

          {!isDownloading && (
            <button
              onClick={onClose}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                padding: '4px',
                borderRadius: '6px'
              }}
              title={isArabic ? 'إغلاق' : 'Close'}
            >
              <X size={18} />
            </button>
          )}
        </div>

        {/* Release Notes / Description */}
        {!isDownloading ? (
          <div style={{ marginBottom: '22px' }}>
            <p style={{ fontSize: '13px', lineHeight: 1.5, color: 'var(--text-secondary)', margin: '0 0 14px 0' }}>
              {isArabic
                ? `هل ترغب في متابعة تثبيت التحديث الآن؟ سيتم تحميل وتحديث Skinner تلقائياً بالكامل دون الحاجة لإعدادات يدوية.`
                : `Would you like to proceed with the update now? Skinner will be downloaded and updated automatically in-place.`}
            </p>

            {updateInfo.releaseTitle && (
              <div
                style={{
                  padding: '12px 14px',
                  backgroundColor: 'var(--bg-secondary, #1A1A1A)',
                  border: '1px solid var(--border-primary, #333)',
                  borderRadius: '8px',
                  fontSize: '12px',
                  color: 'var(--text-primary)'
                }}
              >
                <div style={{ fontWeight: 600, marginBottom: '4px', color: 'var(--accent-light, #60A5FA)' }}>
                  {updateInfo.releaseTitle}
                </div>
                {updateInfo.assetSizeFormatted && (
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                    {isArabic ? 'حجم التنزيل:' : 'Download size:'} {updateInfo.assetSizeFormatted}
                  </div>
                )}
              </div>
            )}
          </div>
        ) : (
          /* Progress Section */
          <div style={{ marginBottom: '22px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px', fontSize: '12px' }}>
              <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                {isInstalling
                  ? (isArabic ? 'جاري استخراج الملفات وتطبيق التحديث...' : 'Applying update and restarting...')
                  : isPaused
                    ? (isArabic ? 'تم إيقاف التنزيل مؤقتاً' : 'Download Paused')
                    : (isArabic ? 'جاري التحميل...' : 'Downloading update...')}
              </span>
              <span style={{ fontWeight: 700, color: 'var(--accent, #3B82F6)' }}>
                {progressPercent}%
              </span>
            </div>

            {/* Progress Bar Container */}
            <div
              style={{
                width: '100%',
                height: '8px',
                backgroundColor: 'var(--bg-secondary, #2A2A2A)',
                borderRadius: '4px',
                overflow: 'hidden',
                position: 'relative'
              }}
            >
              <div
                style={{
                  width: `${progressPercent}%`,
                  height: '100%',
                  backgroundColor: isPaused ? 'var(--warning, #F59E0B)' : 'var(--accent, #3B82F6)',
                  borderRadius: '4px',
                  transition: 'width 0.2s ease, background-color 0.2s ease'
                }}
              />
            </div>

            {/* Size metrics */}
            {updateProgress.bytesDownloaded && updateProgress.totalBytes > 0 && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '6px', fontSize: '11px', color: 'var(--text-muted)' }}>
                <span>{formatBytes(updateProgress.bytesDownloaded)} / {formatBytes(updateProgress.totalBytes)}</span>
                <span>{isPaused ? (isArabic ? 'متوقف' : 'Paused') : ''}</span>
              </div>
            )}
          </div>
        )}

        {/* Footer Actions */}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
          {!isDownloading ? (
            <>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={onClose}
                style={{ padding: '9px 18px', fontSize: '13px' }}
              >
                {isArabic ? 'لاحقاً' : 'Later'}
              </button>
              {onViewChangelog && (
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={onViewChangelog}
                  style={{ padding: '9px 16px', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '6px' }}
                >
                  <ExternalLink size={14} />
                  <span>{isArabic ? 'سجل التغييرات' : 'View Changelog'}</span>
                </button>
              )}
              <button
                type="button"
                className="btn btn-primary"
                onClick={onProceed}
                style={{ padding: '9px 20px', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '6px' }}
              >
                <Download size={15} />
                <span>
                  {isArabic ? 'تحديث الآن' : 'Update Now'}
                </span>
              </button>
            </>
          ) : (
            <div style={{ display: 'flex', gap: '8px', width: '100%', justifyContent: 'space-between', alignItems: 'center' }}>
              <div style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                {isInstalling ? (isArabic ? 'يرجى الانتظار...' : 'Please wait...') : ''}
              </div>

              {!isInstalling && (
                <div style={{ display: 'flex', gap: '8px' }}>
                  {isPaused ? (
                    <button
                      type="button"
                      className="btn btn-primary"
                      onClick={onResume}
                      style={{ padding: '7px 16px', fontSize: '12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Play size={14} />
                      <span>{isArabic ? 'استئناف' : 'Resume'}</span>
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={onPause}
                      style={{ padding: '7px 16px', fontSize: '12px', display: 'flex', alignItems: 'center', gap: '6px' }}
                    >
                      <Pause size={14} />
                      <span>{isArabic ? 'إيقاف مؤقت' : 'Pause'}</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <style>{`
        @keyframes updateModalFadeIn {
          from { opacity: 0; transform: scale(0.97); }
          to { opacity: 1; transform: scale(1); }
        }
      `}</style>
    </div>
  );
}
