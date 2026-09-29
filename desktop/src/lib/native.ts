import { invoke, isTauri } from '@tauri-apps/api/core';

export interface DesktopInfo {
  appName: string;
  version: string;
  platform: string;
  architecture: string;
}
export interface SelectedFile {
  name: string;
  sizeBytes: number;
}
export interface VaultSummary {
  id: string;
  // Names are encrypted on disk and returned only for an unlocked vault.
  name: string | null;
  isLocked: boolean;
}
export interface VaultFile { id: string; name: string; sizeBytes: number }
export interface NativeError { code: string; message: string }

interface Commands {
  get_desktop_info: DesktopInfo;
  select_file: SelectedFile | null;
  list_vaults: VaultSummary[];
  create_vault: VaultSummary;
  unlock_vault: VaultSummary;
  lock_vault: void;
  import_vault_file: VaultFile | null;
  import_example_file: VaultFile;
  delete_locked_vault: void;
  list_vault_files: VaultFile[];
  export_vault_file: VaultFile | null;
}

function call<K extends keyof Commands>(command: K, args?: Record<string, unknown>): Promise<Commands[K]> {
  if (!isTauri()) return Promise.reject(new Error('Open Black Vault as a desktop application to use native features.'));
  return invoke<Commands[K]>(command, args);
}
export const native = {
  getDesktopInfo: () => call('get_desktop_info'),
  selectFile: () => call('select_file'),
  listVaults: () => call('list_vaults'),
  createVault: (name: string, password: string) => call('create_vault', { name, password }),
  unlockVault: (vaultId: string, password: string) => call('unlock_vault', { vaultId, password }),
  lockVault: () => call('lock_vault'),
  importVaultFile: () => call('import_vault_file'),
  importExampleFile: (example: string) => call('import_example_file', { example }),
  deleteLockedVault: (vaultId: string, confirmation: string) => call('delete_locked_vault', { vaultId, confirmation }),
  listVaultFiles: () => call('list_vault_files'),
  exportVaultFile: (fileId: string) => call('export_vault_file', { fileId }),
};

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string' && error.trim()) return error;
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
    return error.message;
  }
  return 'The native operation could not be completed. Please try again.';
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
}
