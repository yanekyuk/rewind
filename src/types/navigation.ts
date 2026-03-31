/**
 * View identifiers for the Steam UI navigation model.
 *
 * Navigation flow: auth-gate -> game-library -> game-detail -> version-select
 */
export type ViewId =
  | "auth-gate"
  | "game-library"
  | "game-detail"
  | "version-select";
