import {
  faDash,
  faSquare,
  faWindowRestore,
  faXmark,
} from "@fortawesome/pro-regular-svg-icons";
import { faGear, faImages } from "@fortawesome/pro-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { signal } from "@preact/signals-react";
import { useSignals } from "@preact/signals-react/runtime";
import { getCreatorIcon, ModelCreator } from "@storyteller/model-list";
import { useCreditsState } from "@storyteller/credits";
import { gtagEvent } from "@storyteller/google-analytics";
import { ProviderBillingModal } from "@storyteller/provider-billing-modal";
import { ProviderSetupModal } from "@storyteller/provider-setup-modal";
import { useSubscriptionState } from "@storyteller/subscription";
import {
  useCreditsBalanceChangedEvent,
  useSubscriptionPlanChangedEvent,
} from "@storyteller/tauri-events";
import {
  useTauriPlatform,
  useTauriWindowControls,
} from "@storyteller/tauri-utils";
import { Button } from "@storyteller/ui-button";
import {
  GalleryModal,
  galleryModalLightboxVisible,
  galleryModalVisibleDuringDrag,
  galleryModalVisibleViewMode,
} from "@storyteller/ui-gallery-modal";
import {
  MenuIconItem,
  MenuIconSelector,
} from "@storyteller/ui-menu-icon-selector";
import { CostBreakdownModal, CreditsModal } from "@storyteller/ui-pricing-modal";
import { GalleryAutoplayToggle } from "@storyteller/ui-generation-list";
import { SettingsModal } from "@storyteller/ui-settings-modal";
import { Tooltip } from "@storyteller/ui-tooltip";
import { useEffect, useRef, useState } from "react";
import { APP_DESCRIPTORS, goToApp } from "~/config/appMenu";
import {
  applyMakeVideoFromImage,
  applyRecreateFromPromptData,
  downloadMediaFileToDisk,
} from "~/components/generation-feed/desktopMediaActions";
import { useStoryboardStore } from "~/pages/PageStoryboard";
import { useImageTo3DStore } from "~/pages/PageImageTo3DObject/ImageTo3DStore";
import { useImageTo3DWorldStore } from "~/pages/PageImageTo3DWorld/ImageTo3DWorldStore";
import { useRemoveBackgroundStore } from "~/pages/PageRemoveBackground/RemoveBackgroundStore";
import { TabId, useTabStore } from "~/pages/Stores/TabState";
import { AUTH_STATUS } from "~/enums";
import { authentication } from "~/signals";
import { setLogoutStates } from "~/signals/authentication/utilities";
import {
  galleryModalDeleteMedia,
  galleryModalSubscribeToMediaEvents,
} from "~/Helpers/galleryModalTauriBindings";
import { TaskQueue } from "./TaskQueue";

interface Props {
  pageName: string;
}

// Settings section type to match the SettingsModal component
type SettingsSection =
  | "general"
  | "accounts"
  | "alerts"
  | "about"
  | "provider_priority"
  | "billing";

const SWITCHER_THROTTLE_TIME = 500; // milliseconds
const CREDITS_POLL_INTERVAL = 60_000; // milliseconds

// NB: See `TabState` for the default tab.
const appMenuTabs: MenuIconItem[] = [
  ...APP_DESCRIPTORS.map((d) => ({
    id: d.id,
    label: d.label,
    icon: <FontAwesomeIcon icon={d.icon} />,
    imageSrc: d.imageSrc,
    description: d.description,
    large: d.large,
  })),
];

export const topNavMediaId = signal<string>("");
export const topNavMediaUrl = signal<string>("");

export const TopBar = ({ pageName }: Props) => {
  useSignals();

  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);
  const [settingsSection, setSettingsSection] =
    useState<SettingsSection>("general");

  const { isDesktop, isMaximized, minimize, toggleMaximize, close } =
    useTauriWindowControls();
  const platform = useTauriPlatform();

  const handleOpenGalleryModal = () => {
    galleryModalVisibleViewMode.value = true;
    galleryModalVisibleDuringDrag.value = true;
    gtagEvent("open_gallery_modal", { tab: tabStore.activeTabId });
  };

  const tabStore = useTabStore();

  const [disableSwitcher, setDisableSwitcher] = useState(false);
  const switcherThrottle = useRef(false);

  // Just calling this function kills the app:
  const subscriptionStore = useSubscriptionState();

  // Fetch credits + subscription on entering LOGGED_IN, then poll credits every
  // 60s. Reading via getState() inside the effect keeps the dep array honest
  // (the only real dep is the auth status). Earlier versions had a 1s setTimeout
  // band-aid to outrun intermediate auth states; gating on LOGGED_IN replaces it.
  const authStatus = authentication.status.value;
  useEffect(() => {
    if (authStatus !== AUTH_STATUS.LOGGED_IN) return;

    void useCreditsState.getState().fetchFromServer();
    void useSubscriptionState.getState().fetchFromServer();

    const interval = setInterval(() => {
      void useCreditsState.getState().fetchFromServer();
      console.log("TopBar: Polled credits");
    }, CREDITS_POLL_INTERVAL);

    // Imperative refresh requests, e.g. dispatched by the shared
    // useGenerationJobs hook when it observes a newly-failed job (the server
    // may have refunded the charge).
    const handleCreditsChange = () => {
      void useCreditsState.getState().fetchFromServer();
    };
    window.addEventListener("credits-change", handleCreditsChange);

    return () => {
      clearInterval(interval);
      window.removeEventListener("credits-change", handleCreditsChange);
    };
  }, [authStatus]);

  useCreditsBalanceChangedEvent(async () => {
    useCreditsState.getState().fetchFromServer();
  });

  useSubscriptionPlanChangedEvent(async () => {
    subscriptionStore.fetchFromServer();
  });

  const disableTabSwitcher = () => {
    return disableSwitcher;
  };

  const downloadFile = downloadMediaFileToDisk;

  const handleTurnIntoVideoFromGallery = applyMakeVideoFromImage;

  const handleRecreateFromGallery = applyRecreateFromPromptData;

  const handleRemoveBackgroundFromGallery = async (url: string) => {
    try {
      useRemoveBackgroundStore.getState().setPendingExternalUrl(url);
      useTabStore.getState().setActiveTab("REMOVE_BACKGROUND");
      galleryModalVisibleViewMode.value = false;
      galleryModalVisibleDuringDrag.value = false;
      galleryModalLightboxVisible.value = false;
    } catch (e) {
      // no-op
    }
  };

  const handleMake3DObjectFromGallery = async (
    url: string,
    mediaId?: string,
  ) => {
    try {
      if (mediaId) {
        useImageTo3DStore.getState().setPendingExternalImage(url, mediaId);
      }
      useTabStore.getState().setActiveTab("IMAGE_TO_3D_OBJECT");
      galleryModalVisibleViewMode.value = false;
      galleryModalVisibleDuringDrag.value = false;
      galleryModalLightboxVisible.value = false;
    } catch (e) {
      // no-op
    }
  };

  const handleMake3DWorldFromGallery = async (
    url: string,
    mediaId?: string,
  ) => {
    try {
      if (mediaId) {
        useImageTo3DWorldStore.getState().setPendingExternalImage(url, mediaId);
      }
      useTabStore.getState().setActiveTab("IMAGE_TO_3D_WORLD");
      galleryModalVisibleViewMode.value = false;
      galleryModalVisibleDuringDrag.value = false;
      galleryModalLightboxVisible.value = false;
    } catch (e) {
      // no-op
    }
  };

  const getPageTitle = (): string => {
    switch (tabStore.activeTabId) {
      case "IMAGE":
        return "Create Image";
      case "VIDEO":
        return "Create Video";
      case "AUDIO":
        return "Create Audio";
      case "EDIT":
        return "Edit Image";
      case "VIDEO_FRAME_EXTRACTOR":
        return "Video Frame Extractor";
      case "VIDEO_WATERMARK_REMOVAL":
        return "Video Watermark Remover";
      case "IMAGE_WATERMARK_REMOVAL":
        return "Image Watermark Remover";
      case "IMAGE_TO_3D_OBJECT":
        return "Image to 3D Object";
      case "IMAGE_TO_3D_WORLD":
        return "Image to 3D World";
      case "BACKGROUND_CHANGE":
        return "Background Change";
      default:
        return "Artcraft";
    }
  };

  const pageTitle = getPageTitle();

  // Pick logo based on current theme (light uses black logo; others use white)
  const [_logoSrc, setLogoSrc] = useState<string>(
    "/resources/logo/artcraft-logo-color-white.svg",
  );
  useEffect(() => {
    const computeLogo = () => {
      const root = document.documentElement;
      const isLight = root.classList.contains("theme-light");
      setLogoSrc(
        isLight
          ? "/resources/logo/artcraft-logo-color-black.svg"
          : "/resources/logo/artcraft-logo-color-white.svg",
      );
    };
    computeLogo();
    const mo = new MutationObserver((muts) => {
      for (const m of muts) {
        if (m.type === "attributes" && m.attributeName === "class") {
          computeLogo();
          break;
        }
      }
    });
    mo.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => mo.disconnect();
  }, []);

  return (
    <>
      <header
        className="fixed left-0 top-0 z-[60] w-full border-b border-ui-panel-border bg-ui-background"
        data-tauri-drag-region
      >
        <nav
          className="mx-auto grid h-[56px] w-screen grid-cols-3 items-center justify-between ps-3"
          aria-label="navigation"
          data-tauri-drag-region
        >
          <div
            className={`flex items-center gap-3 ${platform === "macos" ? "ml-14" : ""}`}
            data-tauri-drag-region
          >
            {/* <div className="mr-2" data-tauri-drag-region>
              <span className="sr-only" data-tauri-drag-region>
                ArtCraft
              </span>
              <img
                className="h-[24px] w-auto"
                src={logoSrc}
                alt="ArtCraft Logo"
                data-tauri-drag-region
              />
            </div> */}
            <MenuIconSelector
              menuItems={appMenuTabs}
              activeMenu={tabStore.activeTabId}
              disabled={disableTabSwitcher()}
              onMenuChange={(tabId) => {
                gtagEvent("switch_tab", { tab: tabId });

                // Prevent a second input if the switcher is throttled.
                if (switcherThrottle.current) {
                  return;
                }
                switcherThrottle.current = true;
                setDisableSwitcher(true);

                useTabStore.getState().setActiveTab(tabId as TabId);
                setTimeout(() => {
                  // Clear the throttle
                  switcherThrottle.current = false;
                  // Trigger a new re-render (important)
                  setDisableSwitcher(false);
                }, SWITCHER_THROTTLE_TIME);
              }}
              className="no-drag w-fit"
            />
          </div>

          <div
            className="flex items-center justify-center gap-2 font-medium"
            data-tauri-drag-region
          >
            <h1
              className="flex items-center gap-2.5 text-base-fg"
              data-tauri-drag-region
            >
              {getCreatorIcon(
                ModelCreator.ArtCraft,
                "h-5 w-5 icon-auto-contrast opacity-50",
              )}
              {pageTitle}
            </h1>
          </div>

          <div className="flex justify-end gap-2" data-tauri-drag-region>
            <div className="no-drag flex items-center gap-1.5">
              {tabStore.activeTabId === "VIDEO" && <GalleryAutoplayToggle />}

              <Tooltip content="Settings" position="bottom" delay={300}>
                <Button
                  variant="secondary"
                  icon={faGear}
                  className="h-[34px] w-[34px]"
                  onClick={() => {
                    setSettingsSection("general");
                    setIsSettingsModalOpen(true);
                    gtagEvent("open_settings_modal");
                  }}
                />
              </Tooltip>

              <Button
                variant="secondary"
                icon={faImages}
                onClick={handleOpenGalleryModal}
              >
                <span className="hidden whitespace-nowrap text-base-fg xl:block">
                  My Library
                </span>
              </Button>

              {/* <Activity /> */}
              <TaskQueue />
            </div>

            <div className="no-drag">
              {/* TODO(bt,2025-09-12): This was the old auth buttons that didn't work. We need to remove this and clean up the DOM. */}
            </div>

            {isDesktop && platform !== "macos" && (
              <div className="no-drag flex items-center">
                <Button
                  variant="secondary"
                  className="h-[32px] w-[44px] rounded-none border-0 bg-transparent text-base-fg opacity-70 shadow-none hover:bg-ui-controls/20 hover:opacity-100"
                  onClick={minimize}
                >
                  <FontAwesomeIcon icon={faDash} className="text-xs" />
                </Button>
                <Button
                  variant="secondary"
                  className="h-[32px] w-[44px] rounded-none border-0 bg-transparent text-base-fg opacity-70 shadow-none hover:bg-ui-controls/20 hover:opacity-100"
                  onClick={toggleMaximize}
                >
                  <FontAwesomeIcon
                    icon={isMaximized ? faWindowRestore : faSquare}
                    className="text-xs"
                  />
                </Button>
                <Button
                  variant="secondary"
                  className="h-[32px] w-[44px] rounded-none border-0 bg-transparent text-base-fg opacity-70 shadow-none hover:bg-red/10 hover:text-red"
                  onClick={close}
                >
                  <FontAwesomeIcon icon={faXmark} className="text-lg" />
                </Button>
              </div>
            )}
          </div>
        </nav>
      </header>

      <SettingsModal
        isOpen={isSettingsModalOpen}
        onClose={() => setIsSettingsModalOpen(false)}
        globalAccountLogoutCallback={() => {
          setIsSettingsModalOpen(false);
          setLogoutStates();
        }}
        onStoryboardPageDisable={() => {
          useStoryboardStore.getState().reset();
          goToApp("IMAGE");
        }}
        initialSection={settingsSection}
      />

      <GalleryModal
        mode="view"
        onDownloadClicked={downloadFile}
        onTurnIntoVideoClicked={handleTurnIntoVideoFromGallery}
        onRemoveBackgroundClicked={handleRemoveBackgroundFromGallery}
        onMake3DObjectClicked={handleMake3DObjectFromGallery}
        onMake3DWorldClicked={handleMake3DWorldFromGallery}
        onRecreateClicked={handleRecreateFromGallery}
        onDeleteMedia={galleryModalDeleteMedia}
        subscribeToMediaEvents={galleryModalSubscribeToMediaEvents}
      />

      <ProviderSetupModal />
      <ProviderBillingModal isVideoPage={tabStore.activeTabId === "VIDEO"} />
      <CreditsModal />
      <CostBreakdownModal activeTabId={tabStore.activeTabId} />
    </>
  );
};
