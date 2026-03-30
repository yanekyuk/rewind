import { useState } from "react";
import { StepIndicator } from "./components/StepIndicator";
import { StepView } from "./components/StepView";
import { AuthInput } from "./components/AuthInput";
import { GameSelect } from "./components/GameSelect";
import { ManifestSelect } from "./components/ManifestSelect";
import { STEPS } from "./steps";
import type { GameInfo } from "./types/game";
import "./App.css";

function App() {
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedGame, setSelectedGame] = useState<GameInfo | null>(null);
  const [selectedManifestId, setSelectedManifestId] = useState<string | null>(
    null,
  );

  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === STEPS.length - 1;
  const currentStepId = STEPS[currentStep].id;
  const isGameSelectStep = currentStepId === "select-game";
  const isAuthStep = currentStepId === "authenticate";
  const isVersionSelectStep = currentStepId === "select-version";
  const isNextDisabled =
    isLastStep ||
    (isGameSelectStep && selectedGame === null) ||
    (isVersionSelectStep && selectedManifestId === null);

  const renderStepContent = () => {
    if (isGameSelectStep) {
      return (
        <GameSelect
          selectedGame={selectedGame}
          onSelectGame={setSelectedGame}
        />
      );
    }

    if (isAuthStep) {
      return <AuthInput />;
    }

    if (isVersionSelectStep && selectedGame) {
      return (
        <ManifestSelect
          selectedGame={selectedGame}
          selectedManifestId={selectedManifestId}
          onSelectManifest={setSelectedManifestId}
        />
      );
    }

    return <StepView stepIndex={currentStep} />;
  };

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
          {renderStepContent()}

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
