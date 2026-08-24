use crate::error::artcraftx_error::ArtcraftXError;
use grok_consumer_client::endpoint_bindings::generate_image_websocket::grok_generate_image_websocket::GrokImageWebsocket;
use log::error;
use std::sync::{Arc, RwLock};

/// Shares one live Grok image websocket across the app.
///
/// [`GrokImageWebsocket`] is itself a cheap, thread-safe handle, so the manager
/// only guards the "which connection is current" slot.
#[derive(Clone, Default)]
pub struct GrokWebsocketManager {
  websocket: Arc<RwLock<Option<GrokImageWebsocket>>>,
}

impl GrokWebsocketManager {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn set_websocket(&self, websocket: GrokImageWebsocket) -> Result<(), ArtcraftXError> {
    let mut guard = self.websocket.write().map_err(|err| {
      error!("Error writing locked websocket: {}", err);
      ArtcraftXError::RwLockWriteError
    })?;
    *guard = Some(websocket);
    Ok(())
  }

  pub fn clear_websocket(&self) -> Result<(), ArtcraftXError> {
    let mut guard = self.websocket.write().map_err(|err| {
      error!("Error writing locked websocket: {}", err);
      ArtcraftXError::RwLockWriteError
    })?;
    *guard = None;
    Ok(())
  }

  pub fn grab_websocket(&self) -> Result<Option<GrokImageWebsocket>, ArtcraftXError> {
    let guard = self.websocket.read().map_err(|err| {
      error!("Error reading locked websocket: {}", err);
      ArtcraftXError::RwLockReadError
    })?;
    Ok(guard.clone())
  }
}
