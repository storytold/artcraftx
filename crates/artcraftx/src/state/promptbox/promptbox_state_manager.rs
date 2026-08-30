use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::promptbox::promptbox_state::PromptboxState;
use errors::AnyhowResult;
use memory_store::clone_cell::CloneCell;

/// Shared, process-wide prompt box state: an in-memory copy that readers
/// clone, and a single write path that persists to disk.
#[derive(Clone)]
pub struct PromptboxStateManager {
  state: CloneCell<PromptboxState>,
  data_root: AppDataRoot,
}

impl PromptboxStateManager {
  pub fn load_or_default(data_root: &AppDataRoot) -> Self {
    Self {
      state: CloneCell::with_owned(PromptboxState::load_or_default(data_root)),
      data_root: data_root.clone(),
    }
  }

  pub fn get(&self) -> AnyhowResult<PromptboxState> {
    self.state.get_clone()
  }

  /// Apply a change, persist it, and return the updated snapshot. Nothing
  /// changes in memory if the write fails.
  pub fn update(&self, change: impl FnOnce(&mut PromptboxState)) -> AnyhowResult<PromptboxState> {
    let mut state = self.get()?;
    change(&mut state);
    state.save(&self.data_root)?;
    self.state.set_clone(&state)?;
    Ok(state)
  }
}
