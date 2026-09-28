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
