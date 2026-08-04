import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faCube,
  faFilm,
  faGlobe,
  faImage,
  faMusic,
} from "@fortawesome/pro-solid-svg-icons";
import { useExperimentalStore } from "@storyteller/ui-settings-modal";
import { useTabStore, TabId } from "~/pages/Stores/TabState";

export type AppId =
  | "IMAGE"
  | "VIDEO"
  | "AUDIO"
  | "EDIT"
  | "VIDEO_FRAME_EXTRACTOR"
  | "VIDEO_WATERMARK_REMOVAL"
  | "IMAGE_WATERMARK_REMOVAL"
  | "IMAGE_TO_3D_OBJECT"
  | "IMAGE_TO_3D_WORLD"
  | "REMOVE_BACKGROUND"
  | "ANGLES"
  | "STORYBOARD"
  | "BACKGROUND_CHANGE"
  | "VIDEO_EDITOR"
  | "MOODBOARD"
  | "SETTINGS";

export interface AppDescriptor {
  id: AppId;
  label: string;
  icon: IconDefinition;
  imageSrc?: string;
  description?: string;
  large?: boolean;
}

// The five modalities. Everything else stays reachable in code (goToApp)
// but is deliberately absent from the nav.
export const APP_DESCRIPTORS: AppDescriptor[] = [
  {
    id: "IMAGE",
    label: "Image",
    icon: faImage,
  },
  {
    id: "VIDEO",
    label: "Video",
    icon: faFilm,
  },
  {
    id: "AUDIO",
    label: "Audio",
    icon: faMusic,
  },
  {
    id: "IMAGE_TO_3D_OBJECT",
    label: "Meshes",
    icon: faCube,
  },
  {
    id: "IMAGE_TO_3D_WORLD",
    label: "Splats",
    icon: faGlobe,
  },
];

export const goToApp = (action?: string) => {
  if (
    action &&
    [
      "IMAGE",
      "VIDEO",
      "AUDIO",
      "VIDEO_FRAME_EXTRACTOR",
      "VIDEO_WATERMARK_REMOVAL",
      "IMAGE_WATERMARK_REMOVAL",
      "IMAGE_TO_3D_OBJECT",
      "IMAGE_TO_3D_WORLD",
      "REMOVE_BACKGROUND",
      "ANGLES",
      "STORYBOARD",
      "BACKGROUND_CHANGE",
      "VIDEO_EDITOR",
      "MOODBOARD",
      "SETTINGS",
    ].includes(action)
  ) {
    if (action === "STORYBOARD") {
      const { enabled, storyboardPageEnabled } =
        useExperimentalStore.getState();
      if (!enabled || !storyboardPageEnabled) return;
    }
    useTabStore.getState().setActiveTab(action as TabId);
  }
};
