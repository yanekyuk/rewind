import { STEPS } from "../steps";

interface StepIndicatorProps {
  currentStep: number;
}

export function StepIndicator({ currentStep }: StepIndicatorProps) {
  return (
    <nav className="step-indicator" aria-label="Downgrade progress">
      <ol className="step-list">
        {STEPS.map((step, index) => {
          const isActive = index === currentStep;
          const isCompleted = index < currentStep;
          return (
            <li
              key={step.id}
              data-step={step.id}
              data-active={String(isActive)}
              data-completed={String(isCompleted)}
              className={[
                "step-item",
                isActive ? "step-item--active" : "",
                isCompleted ? "step-item--completed" : "",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <span className="step-number">{index + 1}</span>
              <span className="step-label">{step.label}</span>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
