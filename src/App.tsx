import { useState } from "react";
import { StepIndicator } from "./components/StepIndicator";
import { StepView } from "./components/StepView";
import { GameSelect } from "./components/GameSelect";
import { STEPS } from "./steps";
import type { GameInfo } from "./types/game";
import "./App.css";

function App() {
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedGame, setSelectedGame] = useState<GameInfo | null>(null);

  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === STEPS.length - 1;
  const isGameSelectStep = STEPS[currentStep].id === "select-game";
  const isNextDisabled = isLastStep || (isGameSelectStep && selectedGame === null);

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
          {isGameSelectStep ? (
            <GameSelect
              selectedGame={selectedGame}
              onSelectGame={setSelectedGame}
            />
          ) : (
            <StepView stepIndex={currentStep} />
          )}

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
              disabled={isNextDisabled}
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
