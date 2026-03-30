import { describe, it, expect } from "vitest";
import type { ViewId } from "./navigation";

describe("ViewId type", () => {
  it("accepts all valid view identifiers", () => {
    const views: ViewId[] = [
      "auth-gate",
      "game-library",
      "game-detail",
      "version-select",
    ];
    expect(views).toHaveLength(4);
    expect(views).toContain("auth-gate");
    expect(views).toContain("game-library");
    expect(views).toContain("game-detail");
    expect(views).toContain("version-select");
  });
});
