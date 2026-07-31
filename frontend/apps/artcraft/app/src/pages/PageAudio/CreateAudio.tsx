import { useMemo } from "react";
import { PromptBoxAudio, usePromptAudioStore } from "@storyteller/ui-promptbox";
import { UploadAudioMedia, UploadImageMedia } from "@storyteller/api";
import {
  useOmniGenAudioModels,
  useAudioCostEstimate,
} from "@storyteller/omni-gen";
import { useGenerationJobs } from "@storyteller/ui-generation-list";
import { useDesktopUsername } from "~/components/generation-feed/useDesktopUsername";
import { AudioLines } from "lucide-react";
import {
  PromptShell,
  type CompletedFile,
} from "~/components/PromptShell";

// The whole page is the composer: no feed, no gallery — results are written
// straight to disk and the PromptShell shows the progress bar + receipt.
// Audio enqueues over HTTP (no Tauri task queue), so activity comes from the
// shared jobs poller instead of useComposerTasks.
const CreateAudio = () => {
  const { models } = useOmniGenAudioModels();

  // Cost estimate inputs come from the shared audio prompt store (the
  // promptbox owns the settings; the page only observes them).
  const selectedModelId = usePromptAudioStore((s) => s.selectedModelId);
  const referenceAudios = usePromptAudioStore((s) => s.referenceAudios);
  const referenceImages = usePromptAudioStore((s) => s.referenceImages);
  const selectedModel = useMemo(
    () =>
      models.find((m) => m.model === selectedModelId) ??
      models.find((m) => m.model === "suno_music") ??
      models[0],
    [models, selectedModelId],
  );
  const audioCredits = useAudioCostEstimate({
    model: selectedModel?.model ?? "",
    audioReferenceCount: referenceAudios.length,
    hasImageReference: referenceImages.length > 0,
    sampleRateHz: selectedModel?.sample_rate_hz_options?.length
      ? (selectedModel.sample_rate_hz_default ?? undefined)
      : undefined,
  });

  const username = useDesktopUsername();
  const feed = useGenerationJobs({ mediaType: "audio", enabled: !!username });

  const completed = useMemo<CompletedFile[]>(
    () =>
      feed.newlyCompleted
        .filter((item) => !!item.fullImage)
        .map((item) => ({ id: item.id, url: item.fullImage as string })),
    [feed.newlyCompleted],
  );

  return (
    <PromptShell
      icon={<AudioLines className="h-[17px] w-[17px]" />}
      busy={feed.inProgress.length > 0}
      completed={completed}
    >
      <PromptBoxAudio
        fullBleed
        models={models}
        uploadAudio={UploadAudioMedia}
        uploadImage={UploadImageMedia}
        credits={audioCredits}
        onEnqueuePressed={async () => {
          // Nudge the shared jobs poller so the in-flight state appears
          // immediately (one request can create several job tokens).
          window.dispatchEvent(new Event("task-queue-update"));
          window.dispatchEvent(new Event("credits-change"));
        }}
      />
    </PromptShell>
  );
};

export default CreateAudio;
