use tauri::AppHandle;

use crate::commands::generate::generate_error::{CredentialProblemReason, GenerateError};
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::functional_events::show_credential_error_modal_event::ShowCredentialErrorModalEvent;

/// If the error is a credential problem, tell the frontend to show the
/// dismissable credential-error modal.
pub async fn maybe_notify_frontend_of_credential_errors(
  app: &AppHandle,
  error: &GenerateError,
) {
  let GenerateError::CredentialProblem(reason) = error else {
    return;
  };

  let message = match reason {
    CredentialProblemReason::NoCredentialSupplied => {
      "No account selected. Pick an account in the account selector, \
       or add one in Settings → Accounts.".to_string()
    }
    CredentialProblemReason::CredentialNotFound { credential_id } => {
      format!(
        "The selected account no longer exists (credential {}). \
         Pick another account and try again.",
        credential_id,
      )
    }
    CredentialProblemReason::CredentialNotUsable { reason, .. } => {
      format!("The selected account can't be used for this request: {}", reason)
    }
  };

  ShowCredentialErrorModalEvent { message }.send_infallible(app);
}
