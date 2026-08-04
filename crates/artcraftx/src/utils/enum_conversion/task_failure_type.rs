use enums::api_safe::by_table::generic_inference_jobs::frontend_failure_category_for_api_clients::FrontendFailureCategoryForApiClients;
use enums::by_table::generic_inference_jobs::frontend_failure_category::FrontendFailureCategory;
use sqlite_identifiers::enums::task_failure_type::TaskFailureType;

/// Convert the web API's `FrontendFailureCategory` to a Tauri-facing type, if there is a matching variant.
/// If there isn't a matching variant, return `Unknown`.
pub fn task_failure_type_from_frontend_failure_category(category: FrontendFailureCategory) -> TaskFailureType {
  match category {
    FrontendFailureCategory::ModelRulesViolation => TaskFailureType::RuleBansUserContent, // NB: This is a legacy enum value.
    FrontendFailureCategory::RuleBansUserImage => TaskFailureType::RuleBansUserImage,
    FrontendFailureCategory::RuleBansUserImageWithFaces => TaskFailureType::RuleBansUserImageWithFaces,
    FrontendFailureCategory::RuleBansUserTextPrompt => TaskFailureType::RuleBansUserTextPrompt,
    FrontendFailureCategory::RuleBansUserContent => TaskFailureType::RuleBansUserContent,
    FrontendFailureCategory::RuleBansGeneratedVideo => TaskFailureType::RuleBansGeneratedVideo,
    FrontendFailureCategory::RuleBansGeneratedAudio => TaskFailureType::RuleBansGeneratedAudio,
    FrontendFailureCategory::RuleBansGeneratedContent => TaskFailureType::RuleBansGeneratedContent,
    FrontendFailureCategory::NoForegroundSubjectDetected => TaskFailureType::NoForegroundSubjectDetected,
    FrontendFailureCategory::FormatNotSupported => TaskFailureType::FormatNotSupported,
    FrontendFailureCategory::GenerationFailed => TaskFailureType::GenerationFailed,
    _ => TaskFailureType::Unknown,
  }
}

/// Convert the API-client-facing `FrontendFailureCategoryForApiClients` to a Tauri-facing type.
/// `Unknown(String)` maps to `Unknown` with a debug log.
pub fn task_failure_type_from_frontend_failure_category_for_api(category: &FrontendFailureCategoryForApiClients) -> TaskFailureType {
  match category {
    FrontendFailureCategoryForApiClients::ModelRulesViolation => TaskFailureType::RuleBansUserContent,
    FrontendFailureCategoryForApiClients::RuleBansUserImage => TaskFailureType::RuleBansUserImage,
    FrontendFailureCategoryForApiClients::RuleBansUserImageWithFaces => TaskFailureType::RuleBansUserImageWithFaces,
    FrontendFailureCategoryForApiClients::RuleBansUserTextPrompt => TaskFailureType::RuleBansUserTextPrompt,
    FrontendFailureCategoryForApiClients::RuleBansUserContent => TaskFailureType::RuleBansUserContent,
    FrontendFailureCategoryForApiClients::RuleBansGeneratedVideo => TaskFailureType::RuleBansGeneratedVideo,
    FrontendFailureCategoryForApiClients::RuleBansGeneratedAudio => TaskFailureType::RuleBansGeneratedAudio,
    FrontendFailureCategoryForApiClients::RuleBansGeneratedContent => TaskFailureType::RuleBansGeneratedContent,
    FrontendFailureCategoryForApiClients::NoForegroundSubjectDetected => TaskFailureType::NoForegroundSubjectDetected,
    FrontendFailureCategoryForApiClients::FormatNotSupported => TaskFailureType::FormatNotSupported,
    FrontendFailureCategoryForApiClients::GenerationFailed => TaskFailureType::GenerationFailed,

    // Types ArtCraft doesn't care about
    FrontendFailureCategoryForApiClients::FaceNotDetected => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::KeepAliveElapsed => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::NotYetImplemented => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::RetryableWorkerError => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::FilesizeTooLarge => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::ImageDimensionsTooSmall => TaskFailureType::Unknown,
    FrontendFailureCategoryForApiClients::ImageDimensionsTooLarge => TaskFailureType::Unknown,

    // Unknown (future-proof) variant
    FrontendFailureCategoryForApiClients::Unknown(ref value) => {
      log::debug!("Unknown FrontendFailureCategoryForApiClients variant: {}", value);
      TaskFailureType::Unknown
    }
  }
}
