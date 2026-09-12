export interface Config {
  schemaVersion: number;
  watchDir: string;
  scanIntervalMs: number;
  batchWindowMs: number;
  debounceMs: number;
  duplicatePolicy: string;
  versionTimestampPrefix: boolean;
  destinationMap: Record<string, string>;
  categoryDestinations: Record<string, string>;
  flattenToCategories: boolean;
  allowExternalDestinations: boolean;
  ignoredExtensions: string[];
  ignoredNames: string[];
  rules: Record<string, string>;
  undoToastMs: number;
  autoDismissTimeoutMs: number;
  launchAtLogin: boolean;
  startMinimizedToTray: boolean;
  pausedUntil: number | null;
}

export interface QueueItem {
  id: string;
  path: string;
  fileName: string;
  ext: string;
  size: number;
  suggested: string;
  detectedAt: string;
}

export interface HistoryEntry {
  id: string;
  source: string;
  destination: string;
  archived: string | null;
  rule: string | null;
  timestamp: string;
  undone: boolean;
}

export interface Status {
  watchDir: string;
  pending: number;
  paused: boolean;
  scanning: boolean;
  lastScan: string | null;
}

export interface MoveOutcome {
  source: string;
  destination: string;
  archived: string | null;
  moved: boolean;
  note: string | null;
}