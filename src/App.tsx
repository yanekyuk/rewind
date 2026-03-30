import { useState, useCallback, useEffect } from "react";
import { useAuth } from "./hooks/useAuth";
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

  // Auth gate
  if (!authenticated || currentView === "auth-gate") {
    return <LoginView auth={auth} />;
  }

  // Game library
  if (currentView === "game-library") {
    return (
      <GameLibrary
        username="Steam User"
        onSelectGame={handleSelectGame}
        onSignOut={handleSignOut}
      />
    );
  }

  // Game detail
  if (currentView === "game-detail" && selectedGame) {
    return (
      <GameDetail
        game={selectedGame}
        onBack={handleBackToLibrary}
        onChangeVersion={handleChangeVersion}
      />
    );
  }

  // Version select
  if (currentView === "version-select" && selectedGame) {
    return (
      <VersionSelect
        game={selectedGame}
        selectedManifestId={selectedManifestId}
        onSelectManifest={setSelectedManifestId}
        onBack={handleBackToDetail}
      />
    );
  }

  // Fallback: redirect to library
  return (
    <GameLibrary
      username="Steam User"
      onSelectGame={handleSelectGame}
      onSignOut={handleSignOut}
    />
  );
}

export default App;
