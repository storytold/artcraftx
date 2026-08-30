use crate::enums::generation_provider::GenerationProvider;
use serde_derive::Serialize;

/// The models one provider offers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProviderOffering<Model, Config> {
  pub provider: GenerationProvider,
  /// In the provider's display order.
  pub models: Vec<OfferedModel<Model, Config>>,
}

/// One model as offered by one provider.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OfferedModel<Model, Config> {
  pub model: Model,
  /// When set, this provider's version of the model differs from the base
  /// config and this replaces it. `None` = same as the base config.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub overrides: Option<Config>,
}

impl<Model, Config> OfferedModel<Model, Config> {
  pub fn same_as_base(model: Model) -> Self {
    Self { model, overrides: None }
  }
}

impl<Model: Copy + PartialEq, Config> ProviderOffering<Model, Config> {
  /// Every model this provider offers (base config, no overrides).
  pub fn of(provider: GenerationProvider, models: &[Model]) -> Self {
    Self {
      provider,
      models: models.iter().copied().map(OfferedModel::same_as_base).collect(),
    }
  }

  pub fn offers(&self, model: Model) -> bool {
    self.models.iter().any(|offered| offered.model == model)
  }
}

/// The providers that offer `model`, in table order (the first is the
/// default provider for the model).
pub fn providers_for_model<Model: Copy + PartialEq, Config>(
  offerings: &[ProviderOffering<Model, Config>],
  model: Model,
) -> Vec<GenerationProvider> {
  offerings.iter()
      .filter(|offering| offering.offers(model))
      .map(|offering| offering.provider)
      .collect()
}

/// Whether `provider` offers `model`.
pub fn is_offered<Model: Copy + PartialEq, Config>(
  offerings: &[ProviderOffering<Model, Config>],
  provider: GenerationProvider,
  model: Model,
) -> bool {
  offerings.iter().any(|offering| offering.provider == provider && offering.offers(model))
}
