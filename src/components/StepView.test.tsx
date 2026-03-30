import { afterEach, describe, it, expect } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { StepView } from "./StepView";
import { STEPS } from "../steps";

afterEach(cleanup);

describe("StepView", () => {
  it("renders the step label as a heading", () => {
    render(<StepView stepIndex={0} />);
    expect(
      screen.getByRole("heading", { name: STEPS[0].label }),
    ).toBeInTheDocument();
  });

  it("renders the step description", () => {
    render(<StepView stepIndex={1} />);
    expect(screen.getByText(STEPS[1].description)).toBeInTheDocument();
  });

  it("renders correctly for each step index", () => {
    for (let i = 0; i < STEPS.length; i++) {
      const { unmount } = render(<StepView stepIndex={i} />);
      expect(
        screen.getByRole("heading", { name: STEPS[i].label }),
      ).toBeInTheDocument();
      expect(screen.getByText(STEPS[i].description)).toBeInTheDocument();
      unmount();
    }
  });
});
