import { describe, expect, it, vi } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import { errorMessage, formatFileSize, native, type DesktopInfo, type SelectedFile } from './native';

const info: DesktopInfo = { appName: 'Black Vault', version: '0.1.0', platform: 'macos', architecture: 'aarch64' };

describe('typed Tauri command boundary', () => {
  it('exchanges desktop information through invoke', async () => {
    vi.stubGlobal('isTauri', true);
    const handler = vi.fn(command => { expect(command).toBe('get_desktop_info'); return info; });
    mockIPC(handler);
    expect(await native.getDesktopInfo()).toEqual(info);
    expect(handler).toHaveBeenCalledOnce();
  });

  it('accepts selection and cancellation without sending a path to Rust', async () => {
    vi.stubGlobal('isTauri', true);
    const file: SelectedFile = { name: 'Café.txt', sizeBytes: 104 };
    const handler = vi.fn((command, args) => {
      expect(command).toBe('select_file');
      expect(args).toEqual({});
      return file;
    });
    mockIPC(handler);
    expect(await native.selectFile()).toEqual(file);
    mockIPC(() => null);
    expect(await native.selectFile()).toBeNull();
  });

  it('preserves structured native failures', async () => {
    vi.stubGlobal('isTauri', true);
    const error = { code: 'file_unavailable', message: 'The selected file is unavailable.' };
    mockIPC(() => { throw error; });
    await expect(native.selectFile()).rejects.toEqual(error);
    expect(errorMessage(error)).toBe(error.message);
    expect(errorMessage(null)).toContain('could not be completed');
  });

  it('refuses native operations in a browser preview', async () => {
    vi.stubGlobal('isTauri', false);
    await expect(native.selectFile()).rejects.toThrow('desktop application');
  });
});

it('formats empty, small and large file sizes', () => {
  expect(formatFileSize(0)).toBe('0 bytes');
  expect(formatFileSize(104)).toBe('104 bytes');
  expect(formatFileSize(1024)).toBe('1.0 KiB');
  expect(formatFileSize(1024 ** 2)).toBe('1.0 MiB');
  expect(formatFileSize(1024 ** 3)).toBe('1.0 GiB');
});

describe('local vault IPC contract', () => {
  it('passes secrets only to explicit creation and unlock commands', async () => {
    vi.stubGlobal('isTauri', true);
    const id = '0123456789abcdef0123456789abcdef';
    const secret = 'long private test password';
    const summary = { id, name: 'Personal', isLocked: false };
    const commands: Array<[string, unknown]> = [];
    mockIPC((command, args) => {
      commands.push([command, args]);
      if (command === 'create_vault' || command === 'unlock_vault') return summary;
      if (command === 'list_vaults') return [{ id, name: null, isLocked: true }];
      if (command === 'lock_vault') return null;
      throw new Error('Unexpected command');
    });
    expect(await native.createVault('Personal', secret)).toEqual(summary);
    expect(await native.listVaults()).toEqual([{ id, name: null, isLocked: true }]);
    expect(await native.unlockVault(id, secret)).toEqual(summary);
    await native.lockVault();
    expect(commands).toEqual([
      ['create_vault', { name: 'Personal', password: secret }],
      ['list_vaults', {}],
      ['unlock_vault', { vaultId: id, password: secret }],
      ['lock_vault', {}],
    ]);
  });
});

describe('Phase 03 transfer IPC contract', () => {
  it('sends a selected parent file and PIN to the explicit native command', async () => {
    vi.stubGlobal('isTauri', true);
    const nestedId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const fileId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const result = [{ id: fileId, name: 'private.txt', sizeBytes: 12 }];
    const handler = vi.fn((command, args) => {
      expect(command).toBe('transfer_parent_file_to_nested');
      expect(args).toEqual({ nestedId, password: '0427', fileId });
      return result;
    });
    mockIPC(handler);
    expect(await native.transferParentFileToNested(nestedId, '0427', fileId)).toEqual(result);
    expect(handler).toHaveBeenCalledOnce();
  });
});
