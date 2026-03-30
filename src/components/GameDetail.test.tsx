import { afterEach, describe, it, expect, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { GameDetail } from "./GameDetail";
import type { GameInfo } from "../types/game";

afterEach(cleanup);

const mockGame: GameInfo = {
  appid: "3321460",
  name: "Crimson Desert",
  buildid: "22560074",
  installdir: "Crimson Desert",
  depots: [{ depot_id: "3321461", manifest: "744665017", size: "133575233011" }],
  install_path: "/steamapps/common/Crimson Desert",
};

describe("GameDetail", () => {
  it("displays the game name", () => {
    render(
      <GameDetail game={mockGame} onBack={vi.fn()} onChangeVersion={vi.fn()} />,
    );
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
  });

  it("displays the game header image from Steam CDN", () => {
    render(
      <GameDetail game={mockGame} onBack={vi.fn()} onChangeVersion={vi.fn()} />,
    );
    const img = screen.getByRole("img");
    expect(img).toHaveAttribute(
      "src",
      "https://cdn.akamai.steamstatic.com/steam/apps/3321460/header.jpg",
    );
  });

  it("displays game metadata (app ID, build ID)", () => {
    render(
      <GameDetail game={mockGame} onBack={vi.fn()} onChangeVersion={vi.fn()} />,
    );
    expect(screen.getByText(/3321460/)).toBeInTheDocument();
    expect(screen.getByText(/22560074/)).toBeInTheDocument();
  });

  it("has a Change Version button", () => {
    render(
      <GameDetail game={mockGame} onBack={vi.fn()} onChangeVersion={vi.fn()} />,
    );
    expect(
      screen.getByRole("button", { name: /change version/i }),
    ).toBeInTheDocument();
  });

  it("calls onChangeVersion when the button is clicked", () => {
    const onChangeVersion = vi.fn();
    render(
      <GameDetail
        game={mockGame}
        onBack={vi.fn()}
        onChangeVersion={onChangeVersion}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /change version/i }));
    expect(onChangeVersion).toHaveBeenCalledOnce();
  });

  it("calls onBack when back button is clicked", () => {
    const onBack = vi.fn();
    render(
      <GameDetail game={mockGame} onBack={onBack} onChangeVersion={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
