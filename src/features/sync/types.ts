export interface SyncStatus {
  enabled: boolean;
  configured: boolean;
  lastRevision: string;
  lastSyncAt: string | null;
  lastError: string | null;
}
