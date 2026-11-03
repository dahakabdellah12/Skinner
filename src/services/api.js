/**
 * Brawlhalla Skin Studio — High-Performance Native IPC Bridge Client
 * Communicates asynchronously with the Rust Native Engine
 */

let seq = 0;
const pendingRequests = new Map();
const eventListeners = new Set();
let isInitialized = false;

function handleIncomingMessage(raw) {
  try {
    const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
    if (!parsed) return;

    // Handle broadcast events
    if (parsed.type === 'event') {
      eventListeners.forEach((listener) => {
        try {
          listener(parsed.event, parsed.data);
        } catch (err) {
          console.error('[API Bridge] Event listener error:', err);
        }
      });
      return;
    }

    const { id, success, data, error } = parsed;
    if (id && pendingRequests.has(id)) {
      const { resolve, reject, timeoutId } = pendingRequests.get(id);
      clearTimeout(timeoutId);
      pendingRequests.delete(id);

      if (success) {
        resolve(data);
      } else {
        reject(new Error(error || 'Backend operation failed.'));
      }
    }
  } catch (err) {
    console.error('[API Bridge] Failed to parse incoming message:', err, raw);
  }
}

function initBridge() {
  if (isInitialized) return;

  // 1. Direct function hook called by Rust evaluate_script
  const previousQueue = window.__pendingRustResponses || [];
  window.__onRustResponse = handleIncomingMessage;

  // Flush any early messages that arrived before initialization
  if (Array.isArray(previousQueue)) {
    while (previousQueue.length > 0) {
      handleIncomingMessage(previousQueue.shift());
    }
  }

  // 2. Custom DOM event hook
  window.addEventListener('__rust_ipc_event__', (event) => {
    if (event && event.detail) {
      handleIncomingMessage(event.detail);
    }
  });

  // 3. Native WebView2 message listener
  if (window.chrome && window.chrome.webview && window.chrome.webview.addEventListener) {
    try {
      window.chrome.webview.addEventListener('message', (event) => {
        handleIncomingMessage(event.data);
      });
    } catch {}
  }

  isInitialized = true;
}

// Auto-initialize on load
if (typeof window !== 'undefined') {
  initBridge();

  window.addEventListener('error', (event) => {
    try {
      invoke('logClientError', {
        message: event.message || 'Window error',
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
        stack: event.error?.stack
      }).catch(() => {});
    } catch {}
  });

  window.addEventListener('unhandledrejection', (event) => {
    try {
      invoke('logClientError', {
        message: 'Unhandled Promise Rejection: ' + (event.reason?.message || String(event.reason)),
        stack: event.reason?.stack
      }).catch(() => {});
    } catch {}
  });
}

/**
 * Invoke a Rust backend action
 * @param {string} action
 * @param {any} payload
 * @param {number} timeoutMs
 * @returns {Promise<any>}
 */
export async function invoke(action, payload = null, timeoutMs = 45000) {
  initBridge();

  const id = `req_${Date.now()}_${++seq}`;

  const hasNativeBridge = Boolean(
    (typeof window !== 'undefined' && window.ipc && typeof window.ipc.postMessage === 'function') ||
    (typeof window !== 'undefined' && window.chrome && window.chrome.webview && typeof window.chrome.webview.postMessage === 'function')
  );

  if (hasNativeBridge) {
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        if (pendingRequests.has(id)) {
          pendingRequests.delete(id);
          reject(new Error(`Operation '${action}' timed out after ${timeoutMs / 1000}s.`));
        }
      }, timeoutMs);

      pendingRequests.set(id, { resolve, reject, timeoutId });

      try {
        const message = JSON.stringify({ id, action, payload });
        if (window.ipc && typeof window.ipc.postMessage === 'function') {
          window.ipc.postMessage(message);
        } else {
          window.chrome.webview.postMessage(message);
        }
      } catch (postErr) {
        clearTimeout(timeoutId);
        pendingRequests.delete(id);
        reject(postErr);
      }
    });
  }

  // Fallback for standalone browser development (mock data for hot reload testing)
  console.warn(`[API Bridge Mock] Invoking '${action}' without active bridge:`, payload);
  return getMockResponse(action, payload);
}

// Convenient Named API Wrappers
export const api = {
  invoke,
  onEvent: (listener) => {
    eventListeners.add(listener);
    return () => eventListeners.delete(listener);
  },
  getSystemInfo: () => invoke('getSystemInfo'),
  getCatalog: () => invoke('getCatalog'),
  validateModification: (req) => invoke('validateModification', req),
  applyModification: (req) => invoke('applyModification', req, 60000),
  applyAdvancedModification: (req) => invoke('applyAdvancedModification', req, 60000),
  restoreCostume: (swfFile) => invoke('restoreCostume', { swfFile }),
  restoreAll: () => invoke('restoreAll'),
  getPresets: () => invoke('getPresets'),
  savePreset: (preset) => invoke('savePreset', preset),
  deletePreset: (id) => invoke('deletePreset', { id }),
  duplicatePreset: (id, newName) => invoke('duplicatePreset', { id, newName }),
  exportPreset: (id) => invoke('exportPreset', { id }),
  importPreset: (json) => invoke('importPreset', { json }),
  getBackups: () => invoke('getBackups'),
  restoreBackupFile: (relativePath) => invoke('restoreBackupFile', { relativePath }),
  getDiagnostics: () => invoke('getDiagnostics'),
  launchGame: () => invoke('launchGame'),
  restartGame: () => invoke('restartGame'),
  bringGameToFront: () => invoke('bringGameToFront'),
  isGameRunning: () => invoke('isGameRunning'),
  renderPreview: (swfFile, symbol) => invoke('renderPreview', { swfFile, symbol }),
  getAssetSvg: (params) => invoke('getAssetSvg', params),
  batchGetAssetSvgs: (items) => invoke('batchGetAssetSvgs', { items }, 60000),
  getVerificationSteps: (params) => invoke('getVerificationSteps', params),
  windowMinimize: () => invoke('windowMinimize'),
  windowMaximize: () => invoke('windowMaximize'),
  isWindowMaximized: () => invoke('isWindowMaximized'),
  windowClose: () => invoke('windowClose'),
  browseGameExe: () => invoke('browseGameExe'),
  selectGamePath: (path) => invoke('selectGamePath', { path }),
  clearAssetCache: () => invoke('clearAssetCache'),
  repairGameInstallation: () => invoke('repairGameInstallation', null, 60000),
  checkForUpdates: (repo) => invoke('checkForUpdates', { repo }, 15000),
  installUpdate: (downloadUrl, version, sha256, assetName, manifestUrl, signatureUrl) =>
    invoke('installUpdate', { downloadUrl, version, sha256, assetName, manifestUrl, signatureUrl }, 600000),
  pauseUpdate: () => invoke('pauseUpdate'),
  resumeUpdate: () => invoke('resumeUpdate'),
  cancelUpdate: () => invoke('cancelUpdate'),
  setGitHubRepo: (repo) => invoke('setGitHubRepo', { repo }),
  getGitHubRepo: () => invoke('getGitHubRepo'),
  openExternalUrl: (url) => invoke('openExternalUrl', { url })
};

export default api;

function getMockResponse(action, _payload) {
  switch (action) {
    case 'getSystemInfo':
      return Promise.resolve({
        gamePath: 'D:\\SteamLibrary\\steamapps\\common\\Brawlhalla',
        version: '11.00.36258',
        buildId: '25942757',
        discoveredSwfCount: 564,
        isValid: true,
        activeModCount: 0,
        totalLegends: 72,
        totalCostumes: 795,
        totalWeaponSkins: 2137,
        totalColorSchemes: 82
      });
    case 'getCatalog':
      return Promise.resolve({
        legends: [],
        costumes: [],
        weaponSkins: [],
        colorSchemes: [],
        weaponTypes: ['Axe', 'Battle Boots', 'Blasters', 'Bow', 'Cannon', 'Gauntlets', 'Greatsword', 'Hammer', 'Katars', 'Orb', 'Rocket Lance', 'Scythe', 'Spear', 'Sword']
      });
    default:
      return Promise.resolve(null);
  }
}
