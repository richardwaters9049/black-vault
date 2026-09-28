import { afterEach, vi } from 'vitest';
import { clearMocks } from '@tauri-apps/api/mocks';

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
afterEach(() => { clearMocks(); vi.unstubAllGlobals(); });
