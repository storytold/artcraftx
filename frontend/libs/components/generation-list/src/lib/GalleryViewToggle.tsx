import { LayoutGrid, List, type LucideIcon } from "lucide-react";
import { Tooltip } from "@storyteller/ui-tooltip";
import {
  useGalleryViewStore,
  type GalleryViewMode,
} from "./gallery-view-store";

const OPTIONS: {
  mode: GalleryViewMode;
  icon: LucideIcon;
  label: string;
}[] = [
  { mode: "grid", icon: LayoutGrid, label: "Grid view" },
  { mode: "list", icon: List, label: "List view" },
];

// Segmented grid/list switch for the create-page galleries. Lives in the
// TopBar; the selection is shared with the gallery via useGalleryViewStore.
export function GalleryViewToggle() {
  const viewMode = useGalleryViewStore((s) => s.viewMode);
  const setViewMode = useGalleryViewStore((s) => s.setViewMode);

  return (
    <div
      role="group"
      aria-label="Gallery layout"
      className="flex items-center gap-0.5 rounded-lg border border-white/[0.08] bg-white/[0.04] p-0.5"
    >
      {OPTIONS.map(({ mode, icon: Icon, label }) => {
        const active = viewMode === mode;
        return (
          <Tooltip key={mode} content={label} position="bottom" delay={300}>
            <button
              type="button"
              aria-label={label}
              aria-pressed={active}
              onClick={() => setViewMode(mode)}
              className={`flex h-6 w-6 items-center justify-center rounded-md text-xs transition-colors ${
                active
                  ? "bg-white/10 text-white"
                  : "text-white/45 hover:text-white/80"
              }`}
            >
              <Icon size="1em" />
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}
