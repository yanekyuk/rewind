import { useState, useCallback, useEffect } from "react";
import { useAuth } from "./hooks/useAuth";
import { AppShell } from "./components/AppShell";
import { LoginView } from "./components/LoginView";
import { GameLibrary } from "./components/GameLibrary";
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

  // Navigate to library when auth state changes to authenticated
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

  // Resolve back action based on current view
  const canGoBack = currentView !== "game-library";
  const handleBack = useCallback(() => {
    if (currentView === "version-select") {
      handleBackToDetail();
    } else if (currentView === "game-detail") {
      handleBackToLibrary();
    }
  }, [currentView, handleBackToDetail, handleBackToLibrary]);

  // Auth gate — no shell
  if (!authenticated || currentView === "auth-gate") {
    return <LoginView auth={auth} />;
  }

  // All authenticated views share the shell
  return (
    <AppShell
      username={auth.username ?? "Steam User"}
      canGoBack={canGoBack}
      onBack={handleBack}
      onSignOut={handleSignOut}
    >
      {currentView === "game-library" && (
        <GameLibrary onSelectGame={handleSelectGame} />
      )}

      {currentView === "game-detail" && selectedGame && (
        <GameDetail
          game={selectedGame}
          onChangeVersion={handleChangeVersion}
        />
      )}

      {currentView === "version-select" && selectedGame && (
        <VersionSelect
          game={selectedGame}
          selectedManifestId={selectedManifestId}
          onSelectManifest={setSelectedManifestId}
        />
      )}
    </AppShell>
  );
}

export default App;
