import { useState, useCallback, useEffect } from "react";
import { useAuth } from "./hooks/useAuth";
import { AppShell } from "./components/AppShell";
import { LoginView } from "./components/LoginView";
import { GameLibrary } from "./components/GameLibrary";
import { GameSidebar } from "./components/GameSidebar";
import { GameDetail } from "./components/GameDetail";
import { VersionSelect } from "./components/VersionSelect";
import type { ViewId } from "./types/navigation";
import type { GameInfo } from "./types/game";
import "./App.css";

function App() {
  const auth = useAuth();
  const { authenticated, signOut } = auth;
  const [currentView, setCurrentView] = useState<ViewId>("auth-gate");
  const [selectedGame, setSelectedGame] = useState<GameInfo | null>(null);
  const [selectedManifestId, setSelectedManifestId] = useState<string | null>(
    null,
  );

  useEffect(() => {
    if (authenticated) {
      setCurrentView("game-library");
    }
  }, [authenticated]);

  const handleSelectGame = useCallback((game: GameInfo) => {
    setSelectedGame(game);
    setCurrentView("game-detail");
  }, []);

  const handleBackToLibrary = useCallback(() => {
    setSelectedGame(null);
    setSelectedManifestId(null);
    setCurrentView("game-library");
  }, []);

  const handleChangeVersion = useCallback(() => {
    setSelectedManifestId(null);
    setCurrentView("version-select");
  }, []);

  const handleBackToDetail = useCallback(() => {
    setSelectedManifestId(null);
    setCurrentView("game-detail");
  }, []);

  const handleSignOut = useCallback(async () => {
    await signOut();
    setSelectedGame(null);
    setSelectedManifestId(null);
    setCurrentView("auth-gate");
  }, [signOut]);

  const canGoBack = currentView !== "game-library" && currentView !== "auth-gate";
  const handleBack = useCallback(() => {
    if (currentView === "version-select") {
      handleBackToDetail();
    } else if (currentView === "game-detail") {
      handleBackToLibrary();
    }
  }, [currentView, handleBackToDetail, handleBackToLibrary]);

  if (!authenticated || currentView === "auth-gate") {
    return <LoginView auth={auth} />;
  }

  const inGameView = currentView === "game-detail" || currentView === "version-select";

  return (
    <AppShell
      username={auth.username ?? "Steam User"}
      canGoBack={canGoBack}
      onBack={handleBack}
      onLibrary={handleBackToLibrary}
      onSignOut={handleSignOut}
    >
      <div className="library-layout">
        {/* Sidebar — slides in/out independently */}
        <div className={`game-sidebar-wrap ${inGameView ? "game-sidebar-wrap--in" : ""}`}>
          <GameSidebar
            selectedAppId={selectedGame?.appid ?? null}
            onSelectGame={handleSelectGame}
          />
        </div>

        {/* Main content area — views fade within this */}
        <div className="library-layout__main">
          <div className={`view-fade ${!inGameView ? "view-fade--visible" : ""}`}>
            <GameLibrary onSelectGame={handleSelectGame} />
          </div>

          <div className={`view-fade ${inGameView ? "view-fade--visible" : ""}`}>
            {selectedGame && currentView === "game-detail" && (
              <GameDetail
                key={selectedGame.appid}
                game={selectedGame}
                onChangeVersion={handleChangeVersion}
              />
            )}

            {selectedGame && currentView === "version-select" && (
              <VersionSelect
                game={selectedGame}
                selectedManifestId={selectedManifestId}
                onSelectManifest={setSelectedManifestId}
              />
            )}
          </div>
        </div>
      </div>
    </AppShell>
  );
}

export default App;
