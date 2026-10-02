import { afterEach, vi } from 'vitest';
import { clearMocks } from '@tauri-apps/api/mocks';

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

// JSDOM intentionally has no Canvas renderer. The globe is decorative, so tests
// only need getContext() to fail quietly rather than emit implementation noise.
HTMLCanvasElement.prototype.getContext = vi.fn(() => null) as typeof HTMLCanvasElement.prototype.getContext;

afterEach(() => { clearMocks(); vi.unstubAllGlobals(); });
