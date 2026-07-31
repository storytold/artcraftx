import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { faFilm, faImage, faMusic } from "@fortawesome/pro-solid-svg-icons";
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
  | "MOODBOARD";

export interface AppDescriptor {
  id: AppId;
  label: string;
  icon: IconDefinition;
  imageSrc?: string;
  description?: string;
  large?: boolean;
}

export const APP_DESCRIPTORS: AppDescriptor[] = [
  {
    id: "IMAGE",
    label: "Create Image",
    icon: faImage,
  },
  {
    id: "VIDEO",
    label: "Create Video",
    icon: faFilm,
  },
  {
    id: "AUDIO",
    label: "Create Audio",
    icon: faMusic,
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
