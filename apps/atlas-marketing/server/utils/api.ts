// Single source of truth for the API version segment - must match
// atlas-backend's own `API_PREFIX` (apps/atlas-backend/src/lib.rs). Every
// request this proxy builds itself, rather than passing through the
// browser's own path, imports this instead of typing 'v1' by hand.
export const API_VERSION = 'v1';
