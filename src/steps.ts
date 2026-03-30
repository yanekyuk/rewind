export type StepId =
  | "select-game"
  | "enter-manifest"
  | "comparing"
  | "downloading"
  | "applying"
  | "complete";

export interface Step {
  id: StepId;
  label: string;
  description: string;
}

export const STEPS: Step[] = [
  {
    id: "select-game",
    label: "Select Game",
    description: "Choose an installed Steam game to downgrade.",
  },
  {
    id: "enter-manifest",
    label: "Enter Manifest ID",
    description:
      "Paste the target manifest ID from SteamDB for the version you want.",
  },
  {
    id: "comparing",
    label: "Comparing Versions",
    description:
      "Diffing current and target manifests to determine which files need downloading.",
  },
  {
    id: "downloading",
    label: "Downloading Files",
    description: "Downloading changed files from Steam via DepotDownloader.",
  },
  {
    id: "applying",
    label: "Applying Downgrade",
    description:
      "Applying downloaded files, patching the ACF manifest, and locking it.",
  },
  {
    id: "complete",
    label: "Complete",
    description:
      'Downgrade finished. Remember to set Steam update preference to "Only update when I launch."',
  },
];
