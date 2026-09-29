import { expect, it } from 'vitest';
import { DEFAULT_IDLE_MS, hasSessionExpired } from './sessionTimeout';

it('expires at five minutes of inactivity, not before', () => {
  expect(hasSessionExpired(1000, 1000 + DEFAULT_IDLE_MS - 1)).toBe(false);
  expect(hasSessionExpired(1000, 1000 + DEFAULT_IDLE_MS)).toBe(true);
});
it('fails closed if the system clock goes backwards or timestamps are invalid', () => {
  expect(hasSessionExpired(1000, 999)).toBe(true);
  expect(hasSessionExpired(Number.NaN, 1000)).toBe(true);
});
it('detects elapsed wall-clock time after delayed timers or sleep', () => {
  expect(hasSessionExpired(1000, 1000 + 60 * 60 * 1000)).toBe(true);
});
