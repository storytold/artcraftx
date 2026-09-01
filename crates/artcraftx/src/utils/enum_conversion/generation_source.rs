use crate::events::generation_events::common::GenerationServiceProvider;
use core_types::enums::generation_source::GenerationSource;

// TODO(bt,2025-07-15): Get rid of GenerationServiceProvider
pub fn to_generation_service_provider(source: GenerationSource) -> GenerationServiceProvider {
  match source {
    GenerationSource::Artcraft
    | GenerationSource::ArtcraftLocal
    | GenerationSource::ArtcraftCookies
    | GenerationSource::ArtcraftApi => GenerationServiceProvider::Artcraft,
    GenerationSource::Fal
    | GenerationSource::FalApi => GenerationServiceProvider::Fal,
    GenerationSource::Grok
    | GenerationSource::GrokCookies
    | GenerationSource::XAiApi => GenerationServiceProvider::Grok,
    GenerationSource::Midjourney
    | GenerationSource::MidjourneyCookies => GenerationServiceProvider::Midjourney,
    GenerationSource::Sora
    | GenerationSource::SoraCookies
    | GenerationSource::OpenAiApi => GenerationServiceProvider::Sora,
    GenerationSource::WorldLabs
    | GenerationSource::WorldLabsCookies => GenerationServiceProvider::WorldLabs,
    GenerationSource::Higgsfield
    | GenerationSource::HiggsfieldCookies => GenerationServiceProvider::Higgsfield,
    // No tasks are generated from these sources (yet); fall back to Artcraft.
    GenerationSource::MagnificCookies
    | GenerationSource::OpenArtCookies
    | GenerationSource::RunwayCookies
    | GenerationSource::ReplicateApi => GenerationServiceProvider::Artcraft,
  }
}
