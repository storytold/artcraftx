import { Modal } from "@storyteller/ui-modal";
import { useEffect, useState } from "react";
import { twMerge } from "tailwind-merge";
import { MiscSettingsPane } from "./panes/MiscSettingsPane";
import { DownloadsSettingsPane } from "./panes/DownloadsSettingsPane";
import { AudioSettingsPane } from "./panes/AudioSettingsPane";
import { AccountSettingsPane } from "./panes/AccountSettings/AccountSettingsPane";
import { AboutSettingsPane } from "./panes/AboutSettingsPane";
import { ExperimentalSettingsPane } from "./panes/ExperimentalSettingsPane";
import { gtagEvent } from "@storyteller/google-analytics";
import { BillingSettingsPane } from "./panes/BillingSettingsPane";
import { AppearanceSettingsPane } from "./panes/AppearanceSettingsPane";
import { Button } from "@storyteller/ui-button";
import { useExperimentalStore } from "./experimental-store";
import { ExperimentalConfirmModal } from "./ExperimentalConfirmModal";

export type SettingsSection =
  | "general"
  | "downloads"
  | "appearance"
  | "accounts"
  | "alerts"
  | "about"
  | "billing"
  | "experimental";

export interface SettingsContentProps {
  globalAccountLogoutCallback: () => void;
  onStoryboardPageDisable?: () => void;
  initialSection?: SettingsSection;
}

interface SettingsModalProps extends SettingsContentProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * The settings body (sidebar + panes), independent of any container so it can
 * fill a page or a modal. The host provides the bounded height.
 */
export const SettingsContent = ({
  globalAccountLogoutCallback,
  onStoryboardPageDisable,
  initialSection = "general",
}: SettingsContentProps) => {
  const [selectedSection, setSelectedSection] =
    useState<SettingsSection>(initialSection);

  const experimentalEnabled = useExperimentalStore((s) => s.enabled);
  const disableExperimental = useExperimentalStore((s) => s.disable);
  const [isResetConfirmOpen, setIsResetConfirmOpen] = useState(false);

  // If experimental gets disabled while user is on that pane, fall back to General
  useEffect(() => {
    if (!experimentalEnabled && selectedSection === "experimental") {
      setSelectedSection("general");
    }
  }, [experimentalEnabled, selectedSection]);

  const sections = [
    { id: "general" as const, label: "General" },
    { id: "downloads" as const, label: "Downloads" },
    { id: "accounts" as const, label: "Accounts" },
    { id: "billing" as const, label: "Plan & Credits" },
    { id: "alerts" as const, label: "Alerts" },
    { id: "about" as const, label: "About" },
    ...(experimentalEnabled
      ? [{ id: "experimental" as const, label: "Experimental" }]
      : []),
  ];

  const renderContent = () => {
    switch (selectedSection) {
      case "appearance":
        return <AppearanceSettingsPane />;
      case "alerts":
        return <AudioSettingsPane />;
      case "general":
        return <MiscSettingsPane />;
      case "downloads":
        return <DownloadsSettingsPane />;
      case "accounts":
        return (
          <AccountSettingsPane
            globalAccountLogoutCallback={globalAccountLogoutCallback}
          />
        );
      case "about":
        return <AboutSettingsPane />;
      case "billing":
        return <BillingSettingsPane />;
      case "experimental":
        return (
          <ExperimentalSettingsPane
            onStoryboardPageDisable={onStoryboardPageDisable}
          />
        );
    }
  };

  const handleConfirmReset = () => {
    disableExperimental();
    gtagEvent("reset_experimental_menu", {});
    setIsResetConfirmOpen(false);
  };

  return (
    <>
      <div className="flex h-full">
        {/* Index rail: mono spec labels, signal edge on the open section —
            the marketing site's spec-sheet row language. */}
        <nav
          aria-label="Settings sections"
          className="flex w-52 shrink-0 flex-col border-r border-line px-4 pb-4 pt-5"
        >
          <span className="ax-marker text-mud">Settings</span>
          <div className="mt-4 flex flex-col gap-0.5">
            {sections.map((section) => (
              <button
                key={section.id}
                aria-current={
                  section.id === selectedSection ? "true" : undefined
                }
                className={twMerge(
                  "flex h-8 items-center rounded-ax-sm px-2.5 text-left font-mono text-[11px] uppercase tracking-[0.18em] transition-colors duration-150",
                  "focus-visible:outline focus-visible:outline-1 focus-visible:outline-signal/60",
                  section.id === selectedSection
                    ? "bg-bone/[0.06] text-bone shadow-[inset_2px_0_0_#4d7cfb]"
                    : "text-ash hover:bg-bone/[0.03] hover:text-putty",
                )}
                onClick={() => {
                  gtagEvent("switch_settings_section", {
                    section: section.id,
                  });
                  setSelectedSection(section.id);
                }}
              >
                {section.label}
              </button>
            ))}
          </div>
          <div className="mt-auto pt-4">
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-mud">
              ArtCraft-X
            </span>
          </div>
        </nav>

        {/* Section pane: hairline-ruled header row, scrolling body. */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line px-6">
            <h2 className="ax-display text-[15px]">
              {sections.find((s) => s.id === selectedSection)?.label}
            </h2>
            {experimentalEnabled && selectedSection === "experimental" && (
              <div className="flex items-center gap-3">
                <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-mud">
                  Experimental on
                </span>
                <Button
                  variant="destructive"
                  onClick={() => setIsResetConfirmOpen(true)}
                  className="px-2 py-1 text-xs"
                >
                  Reset
                </Button>
              </div>
            )}
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
            <div className="text-sm">{renderContent()}</div>
          </div>
        </div>
      </div>
      <ExperimentalConfirmModal
        isOpen={isResetConfirmOpen}
        onClose={() => setIsResetConfirmOpen(false)}
        onConfirm={handleConfirmReset}
        title="Reset experimental settings?"
        text="This will hide the Experimental section and clear any experimental settings. You can unlock it again from the About page."
        confirmText="Reset"
      />
    </>
  );
};

/** Modal wrapper kept for hosts that still want settings as an overlay. */
export const SettingsModal = ({
  isOpen,
  onClose,
  globalAccountLogoutCallback,
  onStoryboardPageDisable,
  initialSection = "general",
}: SettingsModalProps) => {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      className="max-w-3xl"
      childPadding={false}
    >
      <div className="h-[560px]">
        {/* Remount on open so initialSection applies each time. */}
        <SettingsContent
          key={String(isOpen)}
          globalAccountLogoutCallback={globalAccountLogoutCallback}
          onStoryboardPageDisable={onStoryboardPageDisable}
          initialSection={initialSection}
        />
      </div>
    </Modal>
  );
};

export default SettingsModal;
