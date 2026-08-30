// Wire types shared by every modality's listing. These mirror the serde form
// of the Rust `models` crate (`crates/models/src/enums`); the strings are the
// ids the backend accepts on generate requests.

// `models::enums::GenerationProvider`
export type ListingProvider = "artcraft" | "fal" | "grok" | "midjourney" | "sora" | "world_labs";

// `models::enums::ModelCreator` (snake_case)
export type ListingCreator = string;

// `models::enums::ModelTag` (camelCase, same values as `ModelTag`)
export type ListingTag = string;

// `models::enums::CommonAspectRatio` / `CommonResolution` / `CommonQuality` /
// `CommonBitrate` (snake_case; same values as the frontend enums)
export type ListingAspectRatio = string;
export type ListingResolution = string;
export type ListingQuality = string;
export type ListingBitrate = string;

// `models::enums::LegacyVideoSize`
export type ListingLegacyVideoSize = "landscape" | "portrait" | "square";

// Fields every modality's config carries.
export interface ListingModelBase {
  model: string;
  model_creator: ListingCreator;
  full_name: string;
  selector_name: string;
  selector_description: string;
  extra_info?: string;
  selector_badges: string[];
  tags: ListingTag[];
  providers: ListingProvider[];
  progress_bar_ms: number;
  // Absent = no limit.
  text_prompt_max_length?: number;
  is_disabled: boolean;
}
