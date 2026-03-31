import { describe, it, expect } from "bun:test";
import type { SteamDBManifest } from "./steamdb";

describe("SteamDBManifest type", () => {
  it("represents a manifest extracted from SteamDB with all fields", () => {
    const manifest: SteamDBManifest = {
      manifest_id: "7446650175280810671",
      date: "2026-03-15",
      branch: "public",
    };
    expect(manifest.manifest_id).toBe("7446650175280810671");
    expect(manifest.date).toBe("2026-03-15");
    expect(manifest.branch).toBe("public");
  });

  it("allows optional fields to be undefined", () => {
    const manifest: SteamDBManifest = {
      manifest_id: "7446650175280810671",
    };
    expect(manifest.manifest_id).toBe("7446650175280810671");
    expect(manifest.date).toBeUndefined();
    expect(manifest.branch).toBeUndefined();
  });
});
