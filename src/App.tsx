import { useState } from "react";
import { StepIndicator } from "./components/StepIndicator";
import { StepView } from "./components/StepView";
import { STEPS } from "./steps";
import "./App.css";

function App() {
  const [currentStep, setCurrentStep] = useState(0);

  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === STEPS.length - 1;

  return (
    <div className="app">
      <header className="app-header">
        <h1 className="app-title">Rewind</h1>
        <span className="app-subtitle">Steam Game Downgrader</span>
      </header>

      <div className="app-body">
        <aside className="app-sidebar">
          <StepIndicator currentStep={currentStep} />
        </aside>

        <main className="app-content">
          <StepView stepIndex={currentStep} />

          <div className="app-nav">
            <button
              className="app-nav__button"
              onClick={() => setCurrentStep((s) => s - 1)}
              disabled={isFirstStep}
            >
              Back
            </button>
            <button
              className="app-nav__button app-nav__button--primary"
              onClick={() => setCurrentStep((s) => s + 1)}
              disabled={isLastStep}
            >
              Next
            </button>
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;
