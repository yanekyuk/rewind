import { describe, it, expect } from "vitest";
import { STEPS, StepId } from "./steps";

describe("STEPS", () => {
  it("defines exactly 6 steps", () => {
    expect(STEPS).toHaveLength(6);
  });

  it("has the correct step IDs in order", () => {
    const ids = STEPS.map((s) => s.id);
    expect(ids).toEqual([
      "select-game",
      "enter-manifest",
      "comparing",
      "downloading",
      "applying",
      "complete",
    ]);
  });

  it("each step has a label and description", () => {
    for (const step of STEPS) {
      expect(step.label).toBeTruthy();
      expect(step.description).toBeTruthy();
    }
  });

  it("exports StepId type matching step IDs", () => {
    const id: StepId = "select-game";
    expect(STEPS.find((s) => s.id === id)).toBeDefined();
  });
});
