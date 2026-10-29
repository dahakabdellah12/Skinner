import pkg from '../package.json';

// Single source of truth for the frontend version (kept in sync with the .NET build).
export const APP_VERSION = pkg.version;
export default APP_VERSION;
