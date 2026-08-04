use chrono::{DateTime, Utc};
use artcraft_enums::common::payments_namespace::PaymentsNamespace;
use serde_derive::{Deserialize, Serialize};
use artcraft_tokens::tokens::users::UserToken;
use artcraft_tokens::tokens::wallets::WalletToken;

pub const LIST_USER_WALLETS_PATH: &str = "/v1/moderation/wallets/user/{user_token}/list";

#[derive(Deserialize)]
pub struct ListUserWalletsPathInfo {
  pub user_token: UserToken,
}

#[derive(Serialize)]
pub struct ListUserWalletsResponse {
  pub success: bool,
  pub wallets: Vec<ListUserWalletsEntry>,
}

#[derive(Serialize)]
pub struct ListUserWalletsEntry {
  pub token: WalletToken,
  pub wallet_namespace: PaymentsNamespace,
  pub banked_credits: u32,
  pub monthly_credits: u32,
  pub version: i32,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}
