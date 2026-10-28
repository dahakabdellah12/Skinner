/**
 * Brawlhalla Skin Studio — WebView2 IPC Bridge Client
 * Communicates asynchronously with the C# .NET Backend
 */

let seq = 0;
const pendingRequests = new Map();
let isInitialized = false;

function initBridge() {
  if (isInitialized) return;
  if (window.chrome && window.chrome.webview) {
    window.chrome.webview.addEventListener('message', (event) => {
      try {
        const raw = typeof event.data === 'string' ? JSON.parse(event.data) : event.data;
        const { id, success, data, error } = raw;
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
        console.error('[API Bridge] Failed to parse message:', err, event.data);
      }
    });
    isInitialized = true;
  }
}

// Auto-initialize on load
if (typeof window !== 'undefined') {
  initBridge();
}

/**
 * Invoke a C# backend action
 * @param {string} action
 * @param {any} payload
 * @param {number} timeoutMs
 * @returns {Promise<any>}
 */
export async function invoke(action, payload = null, timeoutMs = 45000) {
  initBridge();

  const id = `req_${Date.now()}_${++seq}`;

  // Check if WebView2 host is available
  if (window.chrome && window.chrome.webview) {
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
        window.chrome.webview.postMessage(message);
      } catch (postErr) {
        clearTimeout(timeoutId);
        pendingRequests.delete(id);
        reject(postErr);
      }
    });
  }

  // Fallback for standalone browser development (mock data for hot reload testing)
  console.warn(`[API Bridge Mock] Invoking '${action}' without active WebView2 host:`, payload);
  return getMockResponse(action, payload);
}

// Convenient Named API Wrappers
export const api = {
  invoke,
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
        legends: [
          {
            heroId: 3,
            heroName: 'Viking',
            displayName: 'Bödvar',
            weapon1: 'Hammer',
            weapon2: 'Sword',
            primarySwf: 'Gfx_Viking.swf',
            costumes: [
              { costumeId: 3, costumeName: 'Viking', displayName: 'Classic Bödvar', isDefault: true, swfFile: 'Gfx_Viking.swf', symbolSuffix: 'Viking', baseColors: {} },
              { costumeId: 54, costumeName: 'Bear', displayName: 'Beardvar', isDefault: false, swfFile: 'Gfx_Viking.swf', symbolSuffix: 'Bear', baseColors: {} },
              { costumeId: 98, costumeName: 'CyberViking', displayName: 'Cyber Bödvar', isDefault: false, swfFile: 'Gfx_Viking.swf', symbolSuffix: 'CyberViking', baseColors: {} }
            ]
          },
          {
            heroId: 2,
            heroName: 'Cowgirl',
            displayName: 'Cassidy',
            weapon1: 'Hammer',
            weapon2: 'Blasters',
            primarySwf: 'Gfx_Cowgirl.swf',
            costumes: [
              { costumeId: 2, costumeName: 'Cowgirl', displayName: 'Classic Cassidy', isDefault: true, swfFile: 'Gfx_Cowgirl.swf', symbolSuffix: 'Cowgirl', baseColors: {} }
            ]
          }
        ],
        costumes: [
          { costumeId: 3, costumeName: 'Viking', ownerHero: 'Viking', displayName: 'Classic Bödvar', isDefault: true, swfFile: 'Gfx_Viking.swf', symbolSuffix: 'Viking', baseColors: {} },
          { costumeId: 54, costumeName: 'Bear', ownerHero: 'Viking', displayName: 'Beardvar', isDefault: false, swfFile: 'Gfx_Viking.swf', symbolSuffix: 'Bear', baseColors: {} },
          { costumeId: 2, costumeName: 'Cowgirl', ownerHero: 'Cowgirl', displayName: 'Classic Cassidy', isDefault: true, swfFile: 'Gfx_Cowgirl.swf', symbolSuffix: 'Cowgirl', baseColors: {} }
        ],
        weaponSkins: [
          { weaponSkinId: 2, weaponSkinName: 'HammerViking', baseWeapon: 'Hammer', ownerHero: 'Viking', displayName: "Bödvar's Hammer", swfFile: 'Gfx_Viking.swf', symbolSuffix: 'VikingHammer1', isDefault: true },
          { weaponSkinId: 3, weaponSkinName: 'HammerCowgirl', baseWeapon: 'Hammer', ownerHero: 'Cowgirl', displayName: "Cassidy's Hammer", swfFile: 'Gfx_Cowgirl.swf', symbolSuffix: 'Cowgirl01', isDefault: true }
        ],
        colorSchemes: [
          { schemeId: 1, schemeName: 'Red', displayName: 'Red', rarity: 'Common', indicatorColor: 0xCF1F31, swapColors: {} },
          { schemeId: 2, schemeName: 'Blue', displayName: 'Blue', rarity: 'Common', indicatorColor: 0x2563EB, swapColors: {} }
        ],
        weaponTypes: ['Axe', 'Battle Boots', 'Blasters', 'Bow', 'Cannon', 'Gauntlets', 'Greatsword', 'Hammer', 'Katars', 'Orb', 'Rocket Lance', 'Scythe', 'Spear', 'Sword']
      });
    case 'validateModification':
      return Promise.resolve({
        isCompatible: true,
        level: 'Compatible',
        diagnostics: ['Mock: Native intra-SWF replacement verified.'],
        mappedSymbols: ['a_Torso1_Viking -> a_Torso1_Bear'],
        missingSymbols: [],
        requiresCrossSwfImport: false
      });
    case 'applyModification':
      return Promise.resolve({
        success: true,
        message: 'Mock: Successfully applied cosmetic modification.',
        modifiedFiles: ['Gfx_Viking.swf'],
        backupId: '20261027_120000',
        hashesBefore: {},
        hashesAfter: {},
        operationLog: ['Mock modification applied.'],
        appliedAt: new Date().toISOString()
      });
    case 'getPresets':
      return Promise.resolve([]);
    case 'getBackups':
      return Promise.resolve([]);
    case 'getDiagnostics':
      return Promise.resolve({
        installation: { gamePath: 'D:\\SteamLibrary\\steamapps\\common\\Brawlhalla', version: '11.00', buildId: '25942757', discoveredSwfCount: 564, isValid: true },
        totalLegendsDiscovered: 72,
        totalCostumesDiscovered: 795,
        totalWeaponSkinsDiscovered: 2137,
        totalColorSchemesDiscovered: 82,
        modifiedFilesDetected: [],
        backupArchivesCount: 1,
        isSteamRunning: true,
        healthChecks: ['✓ Brawlhalla.exe verified.', '✓ Game.swz verified.']
      });
    case 'launchGame':
      return Promise.resolve(true);
    case 'getVerificationSteps':
      return Promise.resolve([
        '1. Ensure Brawlhalla is launched to the Main Menu.',
        '2. Navigate to Offline Play -> Training Mode.',
        '3. Inspect character appearance and attacks.'
      ]);
    default:
      return Promise.resolve(null);
  }
}
