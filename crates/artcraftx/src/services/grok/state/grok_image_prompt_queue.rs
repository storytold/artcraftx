use crate::error::artcraftx_error::ArtcraftXError;
use log::error;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use grok_consumer_client::endpoint_bindings::generate_image_websocket::messages::websocket_client_message::FastAspectRatio;

#[derive(Clone)]
pub struct GrokImagePromptQueue {
  pub prompt_queue: Arc<Mutex<VecDeque<PromptItem>>>,
}


#[derive(Clone, Debug)]
pub struct PromptItem {
  /// Local database task ID
  pub task_id: String,
  
  /// Text prompt
  pub prompt: String,
  
  /// The aspect ratio of the image
  pub aspect_ratio: FastAspectRatio,
}

impl GrokImagePromptQueue {
  pub fn new() -> Self {
    Self {
      prompt_queue: Arc::new(Mutex::new(VecDeque::new())),
    }
  }
  
  pub fn enqueue(&self, prompt_item: PromptItem) -> Result<(), ArtcraftXError> {
    match self.prompt_queue.lock() {
      Ok(mut queue) => {
        queue.push_back(prompt_item);
        Ok(())
      },
      Err(err) => {
        error!("Error locking prompt queue: {:?}", err);
        Err(ArtcraftXError::MutexLockError)
      },
    }
  }
  
  pub fn dequeue(&self) -> Result<Option<PromptItem>, ArtcraftXError> {
    match self.prompt_queue.lock() {
      Ok(mut queue) => {
        Ok(queue.pop_front())
      },
      Err(err) => {
        error!("Error locking prompt queue: {:?}", err);
        Err(ArtcraftXError::MutexLockError)
      },
    }
  }

  pub fn is_empty(&self) -> Result<bool, ArtcraftXError> {
    match self.prompt_queue.lock() {
      Ok(queue) => {
        Ok(queue.is_empty())
      },
      Err(err) => {
        error!("Error locking prompt queue: {:?}", err);
        Err(ArtcraftXError::MutexLockError)
      },
    }
  }

  pub fn len(&self) -> Result<usize, ArtcraftXError> {
    match self.prompt_queue.lock() {
      Ok(queue) => {
        Ok(queue.len())
      },
      Err(err) => {
        error!("Error locking prompt queue: {:?}", err);
        Err(ArtcraftXError::MutexLockError)
      },
    }
  }
}
