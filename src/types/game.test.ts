import { describe, it, expect } from "vitest";
import type { GameInfo, DepotInfo } from "./game";

describe("GameInfo type", () => {
  it("accepts a valid GameInfo object", () => {
    const depot: DepotInfo = {
      depot_id: "3321461",
      manifest: "7446650175280810671",
      size: "133575233011",
    };

    const game: GameInfo = {
      appid: "3321460",
      name: "Crimson Desert",
      buildid: "22560074",
      installdir: "Crimson Desert",
      depots: [depot],
      install_path: "/home/user/.local/share/Steam/steamapps/common/Crimson Desert",
    };

    expect(game.appid).toBe("3321460");
    expect(game.name).toBe("Crimson Desert");
    expect(game.buildid).toBe("22560074");
    expect(game.installdir).toBe("Crimson Desert");
    expect(game.depots).toHaveLength(1);
    expect(game.depots[0].depot_id).toBe("3321461");
    expect(game.install_path).toContain("Crimson Desert");
  });

  it("accepts a GameInfo with empty depots", () => {
    const game: GameInfo = {
      appid: "440",
      name: "Team Fortress 2",
      buildid: "12345",
      installdir: "Team Fortress 2",
      depots: [],
      install_path: "/steamapps/common/Team Fortress 2",
    };

    expect(game.depots).toHaveLength(0);
  });
});
