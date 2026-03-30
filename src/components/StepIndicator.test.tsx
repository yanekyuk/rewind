import { afterEach, describe, it, expect } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { StepIndicator } from "./StepIndicator";
import { STEPS } from "../steps";

afterEach(cleanup);

describe("StepIndicator", () => {
  it("renders all step labels", () => {
    render(<StepIndicator currentStep={0} />);
    for (const step of STEPS) {
      expect(screen.getByText(step.label)).toBeInTheDocument();
    }
  });

  it("marks the current step as active", () => {
    render(<StepIndicator currentStep={2} />);
    const activeItem = screen.getByText(STEPS[2].label).closest("[data-step]");
    expect(activeItem).toHaveAttribute("data-active", "true");
  });

  it("marks previous steps as completed", () => {
    render(<StepIndicator currentStep={3} />);
    for (let i = 0; i < 3; i++) {
      const item = screen.getByText(STEPS[i].label).closest("[data-step]");
      expect(item).toHaveAttribute("data-completed", "true");
    }
  });

  it("does not mark future steps as active or completed", () => {
    render(<StepIndicator currentStep={1} />);
    for (let i = 2; i < STEPS.length; i++) {
      const item = screen.getByText(STEPS[i].label).closest("[data-step]");
      expect(item).toHaveAttribute("data-active", "false");
      expect(item).toHaveAttribute("data-completed", "false");
    }
  });
});
