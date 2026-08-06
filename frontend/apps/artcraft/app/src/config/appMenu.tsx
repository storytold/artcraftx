import { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import {
  faCube,
  faFilm,
  faGlobe,
  faImage,
  faMusic,
} from "@fortawesome/pro-solid-svg-icons";
import { useTabStore, TabId } from "~/pages/Stores/TabState";

export type AppId =
  | "IMAGE"
  | "VIDEO"
  | "AUDIO"
  | "IMAGE_TO_3D_OBJECT"
  | "IMAGE_TO_3D_WORLD"
  | "SETTINGS";

export interface AppDescriptor {
  id: AppId;
  label: string;
  icon: IconDefinition;
  imageSrc?: string;
  description?: string;
  large?: boolean;
}

// The five modalities.
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
      "IMAGE_TO_3D_OBJECT",
      "IMAGE_TO_3D_WORLD",
      "SETTINGS",
    ].includes(action)
  ) {
    useTabStore.getState().setActiveTab(action as TabId);
  }
};
