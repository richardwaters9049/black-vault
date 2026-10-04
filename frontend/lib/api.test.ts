import { afterEach, expect, it, vi } from "vitest";
import { ApiError, api } from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
});

it("returns a successful plain-text health response", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
    new Response("OK", { status: 200, headers: { "content-type": "text/plain" } }),
  ));

  await expect(api.healthCheck()).resolves.toBe("OK");
});

it("rejects an unhealthy HTTP response", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
    new Response("database unavailable", { status: 503, headers: { "content-type": "text/plain" } }),
  ));

  await expect(api.healthCheck()).rejects.toEqual(
    expect.objectContaining<ApiError>({ name: "ApiError", message: "Request failed", status: 503 }),
  );
});

it('preserves HTTP status for malformed or wrongly shaped JSON errors', async () => {
  for (const payload of ['{broken', 'null', '{"error":42}']) {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(payload, { status: 401, headers: { 'content-type': 'application/json' } })));
    await expect(api.getCurrentUser('test-token')).rejects.toEqual(expect.objectContaining({ name: 'ApiError', status: 401, message: 'Request failed' }));
  }
});
it('reports malformed success JSON through the API error contract', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('{broken', { status: 200, headers: { 'content-type': 'application/json' } })));
  await expect(api.getCurrentUser('test-token')).rejects.toEqual(expect.objectContaining({ name: 'ApiError', message: 'The server returned an invalid JSON response.' }));
});

it('normalises a network failure without exposing transport details', async () => {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('private transport details')));
  await expect(api.healthCheck()).rejects.toEqual(expect.objectContaining({ name: 'ApiError', message: 'Could not reach the server. Please try again.' }));
});
