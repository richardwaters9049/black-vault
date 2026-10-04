import { invoke, isTauri } from '@tauri-apps/api/core';

export interface DesktopInfo {
  appName: string;
  version: string;
  platform: string;
  architecture: string;
}
export interface BiometricStatus {
  available: boolean;
  kind: 'touch_id' | 'face_id' | 'optic_id' | 'biometric' | 'none';
  platform: string;
}
export interface BiometricAuthResult {
  authenticated: boolean;
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
export interface NestedVaultSummary { id: string; fileCount: number }
export interface VaultFile { id: string; name: string; sizeBytes: number }
export interface NativeError { code: string; message: string }
export type ContactTrustState = 'unverified' | 'verified' | 'key_changed' | 'revoked';
export interface PublicIdentity {
  fingerprint: string;
  document: string;
}
export interface TrustedContact {
  name: string;
  fingerprint: string;
  trust: ContactTrustState;
  previousVerifiedFingerprint: string | null;
}

interface Commands {
  get_biometric_status: BiometricStatus;
  authenticate_biometric: BiometricAuthResult;
  get_desktop_info: DesktopInfo;
  select_file: SelectedFile | null;
  list_vaults: VaultSummary[];
  create_vault: VaultSummary;
  unlock_vault: VaultSummary;
  lock_vault: void;
  import_vault_file: VaultFile | null;
  describe_dropped_file: SelectedFile;
  import_dropped_file: VaultFile;
  import_example_file: VaultFile;
  delete_locked_vault: void;
  list_vault_files: VaultFile[];
  list_nested_vaults: NestedVaultSummary[];
  lock_selected_files: NestedVaultSummary;
  unlock_nested_vault: VaultFile[];
  restore_nested_vault: VaultFile[];
  create_nested_text_file: VaultFile[];
  manage_nested_file: VaultFile[];
  transfer_parent_file_to_nested: VaultFile[];
  read_nested_text_file: string;
  create_vault_text_file: VaultFile;
  read_vault_text_file: string;
  update_vault_text_file: VaultFile;
  delete_vault_file: void;
  decrypt_vault_file: VaultFile;
  export_vault_file: VaultFile | null;
  get_public_identity: PublicIdentity;
  list_contacts: TrustedContact[];
  import_contact_identity: TrustedContact;
  verify_contact_fingerprint: TrustedContact;
  revoke_contact: TrustedContact;
}

function call<K extends keyof Commands>(command: K, args?: Record<string, unknown>): Promise<Commands[K]> {
  if (!isTauri()) return Promise.reject(new Error('Open Black Vault as a desktop application to use native features.'));
  return invoke<Commands[K]>(command, args);
}
export const native = {
  getBiometricStatus: () => call('get_biometric_status'),
  authenticateBiometric: () => call('authenticate_biometric'),
  getDesktopInfo: () => call('get_desktop_info'),
  selectFile: () => call('select_file'),
  listVaults: () => call('list_vaults'),
  createVault: (name: string, password: string) => call('create_vault', { name, password }),
  unlockVault: (vaultId: string, password: string) => call('unlock_vault', { vaultId, password }),
  lockVault: () => call('lock_vault'),
  importVaultFile: () => call('import_vault_file'),
  describeDroppedFile: () => call('describe_dropped_file'),
  importDroppedFile: () => call('import_dropped_file'),
  importExampleFile: (example: string) => call('import_example_file', { example }),
  deleteLockedVault: (vaultId: string, confirmation: string) => call('delete_locked_vault', { vaultId, confirmation }),
  listVaultFiles: () => call('list_vault_files'),
  listNestedVaults: () => call('list_nested_vaults'),
  lockSelectedFiles: (name: string, password: string, fileIds: string[]) => call('lock_selected_files', { name, password, fileIds }),
  unlockNestedVault: (nestedId: string, password: string) => call('unlock_nested_vault', { nestedId, password }),
  createNestedTextFile: (nestedId: string, password: string, name: string, content: string) => call('create_nested_text_file', { nestedId, password, name, content }),
  manageNestedFile: (nestedId: string, password: string, fileId: string, action: 'edit' | 'delete' | 'transfer_to_parent', name?: string, content?: string) => call('manage_nested_file', { nestedId, password, fileId, action, name, content }),
  transferParentFileToNested: (nestedId: string, password: string, fileId: string) => call('transfer_parent_file_to_nested', { nestedId, password, fileId }),
  readNestedTextFile: (nestedId: string, password: string, fileId: string) => call('read_nested_text_file', { nestedId, password, fileId }),
  restoreNestedVault: (nestedId: string, password: string) => call('restore_nested_vault', { nestedId, password }),
  createVaultTextFile: (name: string, content: string) => call('create_vault_text_file', { name, content }),
  readVaultTextFile: (fileId: string) => call('read_vault_text_file', { fileId }),
  updateVaultTextFile: (fileId: string, name: string, content: string) => call('update_vault_text_file', { fileId, name, content }),
  deleteVaultFile: (fileId: string) => call('delete_vault_file', { fileId }),
  decryptVaultFile: (fileId: string) => call('decrypt_vault_file', { fileId }),
  exportVaultFile: (fileId: string) => call('export_vault_file', { fileId }),
  getPublicIdentity: () => call('get_public_identity'),
  listContacts: () => call('list_contacts'),
  importContactIdentity: (name: string, document: string) => call('import_contact_identity', { name, document }),
  verifyContactFingerprint: (name: string, fingerprint: string) => call('verify_contact_fingerprint', { name, fingerprint }),
  revokeContact: (name: string) => call('revoke_contact', { name }),
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
