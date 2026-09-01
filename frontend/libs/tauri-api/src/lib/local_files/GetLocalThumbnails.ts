import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

/**
 * Bulk thumbnail lookup/generation for local files. The backend hashes each
 * file (memoized in the local_files index), serves cached thumbnails from
 * `cache/thumbnails/{hash}.jpg|.webp`, and generates anything missing.
 * Render the returned paths via `convertFileSrc(...)`.
 */

export type LocalThumbnailStatus =
  | "ready"
  | "unsupported"
  | "missing_file"
  | "failed";

export interface LocalThumbnail {
  file_path: string;
  status: LocalThumbnailStatus;
  maybe_file_hash?: string;
  /** Absolute path of the static JPEG thumbnail (512px longest edge). */
  maybe_thumbnail_path?: string;
  /** Absolute path of the ~1s animated WebP preview (videos only). */
  maybe_animated_preview_path?: string;
}

interface GetLocalThumbnailsResponse {
  thumbnails: LocalThumbnail[];
}

interface GetLocalThumbnailsSuccess extends CommandResult {
  payload: GetLocalThumbnailsResponse;
}

export const GetLocalThumbnails = async (
  filePaths: string[],
): Promise<LocalThumbnail[]> => {
  if (filePaths.length === 0) return [];
  const result = (await invoke("get_local_thumbnails_command", {
    request: { file_paths: filePaths },
  })) as GetLocalThumbnailsSuccess;
  return result?.payload?.thumbnails ?? [];
};
