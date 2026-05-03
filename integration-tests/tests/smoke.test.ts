import { describe, it, expect } from "vitest";
import { mf, mfUrl } from "./mf";

describe("non-RPC routes", () => {
  it("GET /healthz returns 200 + plaintext body", async () => {
    const res = await mf.dispatchFetch(`${mfUrl}/healthz`);
    expect(res.status).toBe(200);
    expect(await res.text()).toMatch(/ok/i);
  });

  it("GET /oauth/callback echoes code + state", async () => {
    const res = await mf.dispatchFetch(`${mfUrl}/oauth/callback?code=abc&state=xyz`);
    expect(res.status).toBe(200);
    const body = await res.text();
    expect(body).toContain("abc");
    expect(body).toContain("xyz");
  });

  it("GET /verify-email returns the bypass message", async () => {
    const res = await mf.dispatchFetch(`${mfUrl}/verify-email?token=irrelevant`);
    expect(res.status).toBe(200);
    expect(await res.text()).toMatch(/bypassed/i);
  });
});
