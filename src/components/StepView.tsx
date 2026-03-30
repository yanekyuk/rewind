import { STEPS } from "../steps";

interface StepViewProps {
  stepIndex: number;
}

export function StepView({ stepIndex }: StepViewProps) {
  const step = STEPS[stepIndex];

  return (
    <section className="step-view">
      <h2 className="step-view__title">{step.label}</h2>
      <p className="step-view__description">{step.description}</p>
    </section>
  );
}
