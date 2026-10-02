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
