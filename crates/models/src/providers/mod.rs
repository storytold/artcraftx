//! Which providers (services) offer which models.
//!
//! A model's capabilities live in [`crate::configs`]; *where it can run* lives
//! here. Not every provider has every model: first-party Grok only runs the
//! Grok Imagine models, first-party Midjourney only the Midjourney models,
//! and video / mesh / splat / audio are ArtCraft (plus a couple of
//! credential-backed services) only. The frontend uses these tables to keep
//! provider + model + account choices valid; the enqueue commands and the
//! router still guard on their own.
//!
//! Each offering may carry `overrides`: a provider-specific replacement config
//! for a model whose behavior differs on that provider. None do today.

pub mod audio_providers;
pub mod image_providers;
pub mod mesh_providers;
pub mod provider_offering;
pub mod splat_providers;
pub mod video_providers;

#[cfg(test)]
pub(crate) mod tests_common {
  use crate::providers::provider_offering::ProviderOffering;
  use std::collections::HashSet;
  use std::fmt::Debug;
  use std::hash::Hash;

  /// Every enabled model is offered by at least one provider, every offered
  /// model is a known enabled model, and no provider lists a model twice.
  pub fn check_offerings<M: Copy + Eq + Hash + Debug, C>(offerings: &[ProviderOffering<M, C>], enabled_models: &[M]) {
    let enabled: HashSet<M> = enabled_models.iter().copied().collect();
    let mut offered_anywhere: HashSet<M> = HashSet::new();
    let mut providers_seen = HashSet::new();
    for offering in offerings {
      assert!(providers_seen.insert(offering.provider), "{:?} listed twice", offering.provider);
      let mut seen = HashSet::new();
      for offered in &offering.models {
        assert!(seen.insert(offered.model), "{:?} lists {:?} twice", offering.provider, offered.model);
        assert!(enabled.contains(&offered.model), "{:?} offers unknown/disabled {:?}", offering.provider, offered.model);
        offered_anywhere.insert(offered.model);
      }
    }
    for model in &enabled {
      assert!(offered_anywhere.contains(model), "{model:?} is offered by no provider");
    }
  }
}
