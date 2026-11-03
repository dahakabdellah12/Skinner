import React, { useState, useRef, useEffect, useMemo } from 'react';

/**
 * Minimal Dark Dropdown component with embedded search & category badge
 * @param {Array<{value: any, label: string, searchKey?: string}>} options
 * @param {any} value
 * @param {function(any): void} onChange
 * @param {string} placeholder
 * @param {string} [headerTitle]
 */
export default function CustomDropdown({
  options = [],
  value,
  onChange,
  placeholder = 'اختر خياراً',
  headerTitle = '',
  showSearch = true
}) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const dropdownRef = useRef(null);
  const searchInputRef = useRef(null);

  const safeOptions = useMemo(() => (Array.isArray(options) ? options.filter(Boolean) : []), [options]);
  const selectedOption = safeOptions.find((opt) => String(opt.value) === String(value));

  // Close on outside click or escape
  useEffect(() => {
    function handleClickOutside(event) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target)) {
        setIsOpen(false);
      }
    }

    function handleKeyDown(event) {
      if (event.key === 'Escape') {
        setIsOpen(false);
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  const toggleDropdown = () => {
    setIsOpen((prev) => {
      const next = !prev;
      if (next) {
        setSearchQuery('');
        setTimeout(() => {
          searchInputRef.current?.focus();
        }, 50);
      }
      return next;
    });
  };

  const handleSelect = (val) => {
    onChange?.(val);
    setIsOpen(false);
    setSearchQuery('');
  };

  // Filter options based on user search query
  const filteredOptions = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return safeOptions;

    return safeOptions.filter((opt) => {
      if (String(opt.value) === '__NONE__') return true; // Keep default option accessible
      const labelMatch = opt.label && String(opt.label).toLowerCase().includes(q);
      const keyMatch = opt.searchKey && String(opt.searchKey).toLowerCase().includes(q);
      return labelMatch || keyMatch;
    });
  }, [safeOptions, searchQuery]);

  return (
    <div className={`dropdown ${isOpen ? 'open' : ''}`} ref={dropdownRef}>
      <button className="trigger" type="button" onClick={toggleDropdown}>
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {selectedOption ? selectedOption.label : placeholder}
        </span>
        <svg className="arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      <div className="menu">
        {/* Pinned Category / Header Title */}
        {headerTitle && (
          <div className="dropdown-menu-header">
            <span>{headerTitle}</span>
            <span className="dropdown-count-badge">{options.length - (options[0]?.value === '__NONE__' ? 1 : 0)}</span>
          </div>
        )}

        {/* Embedded Instant Search Filter (shown for large lists like weapons/costumes) */}
        {showSearch && options.length > 7 && (
          <div className="dropdown-search-wrapper" onClick={(e) => e.stopPropagation()}>
            <svg className="dropdown-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="11" cy="11" r="8" />
              <path d="M21 21l-4.35-4.35" />
            </svg>
            <input
              ref={searchInputRef}
              type="text"
              className="dropdown-search-input"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="بحث سريع (تصفية)..."
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  setIsOpen(false);
                }
              }}
            />
            {searchQuery && (
              <button
                type="button"
                className="dropdown-search-clear"
                onClick={() => setSearchQuery('')}
                title="مسح"
              >
                ✕
              </button>
            )}
          </div>
        )}

        {/* Scrollable Options List */}
        <div className="dropdown-options-list">
          {filteredOptions.length === 0 ? (
            <div className="dropdown-empty-message">
              لا توجد خيارات مطابقة للبحث
            </div>
          ) : (
            filteredOptions.map((opt, idx) => {
              const isSelected = String(opt.value) === String(value);
              return (
                <div
                  key={`${String(opt.value)}_${idx}`}
                  className={`option ${isSelected ? 'active' : ''}`}
                  onClick={() => handleSelect(opt.value)}
                >
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {opt.label}
                  </span>
                  {isSelected && (
                    <svg className="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
