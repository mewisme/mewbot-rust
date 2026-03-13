//! Wallet data stored in JSON file. Path: `data/wallet.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const WALLET_FILE: &str = "data/wallet.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWallet {
    /// Balance: min 0, max u64::MAX (18,446,744,073,709,551,615)
    pub balance: u64,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WalletData {
    pub users: HashMap<String, UserWallet>,
}

impl WalletData {
    /// Returns true if the user has been initialized (has an entry in wallet).
    pub fn has_user(&self, user_id: u64) -> bool {
        self.users.contains_key(&user_id.to_string())
    }

    /// Returns balance only if user is initialized; does not create entry. Min 0, max u64::MAX.
    pub fn get_balance_if_exists(&self, user_id: u64) -> Option<u64> {
        self.users.get(&user_id.to_string()).map(|u| u.balance)
    }

    #[allow(dead_code)]
    pub fn get_balance(&mut self, user_id: u64) -> u64 {
        self.users
            .entry(user_id.to_string())
            .or_insert_with(|| UserWallet {
                balance: 0,
                updated_at: None,
            })
            .balance
    }

    /// Add amount (clamped to 0..=u64::MAX). Balance never exceeds u64::MAX.
    pub fn add_balance(&mut self, user_id: u64, amount: i64, now_iso: &str) -> u64 {
        let amount = amount.max(0) as u64;
        let entry = self
            .users
            .entry(user_id.to_string())
            .or_insert_with(|| UserWallet {
                balance: 0,
                updated_at: None,
            });
        entry.balance = entry.balance.saturating_add(amount);
        entry.updated_at = Some(now_iso.to_string());
        entry.balance
    }

    /// Subtract amount (min balance 0). Fails if insufficient balance.
    pub fn subtract_balance(&mut self, user_id: u64, amount: i64, now_iso: &str) -> Result<u64, String> {
        let amount = amount.max(0) as u64;
        let entry = self
            .users
            .entry(user_id.to_string())
            .or_insert_with(|| UserWallet {
                balance: 0,
                updated_at: None,
            });
        if entry.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        entry.balance -= amount;
        entry.updated_at = Some(now_iso.to_string());
        Ok(entry.balance)
    }

    /// Initialize user wallet with given balance (create or reset). Min 0, max u64::MAX.
    pub fn init_user(&mut self, user_id: u64, balance: i64, now_iso: &str) {
        let balance = balance.max(0) as u64;
        self.users.insert(
            user_id.to_string(),
            UserWallet {
                balance,
                updated_at: Some(now_iso.to_string()),
            },
        );
    }

    /// Initialize user wallet only if not already present; returns true if created.
    pub fn init_user_if_new(&mut self, user_id: u64, balance: i64, now_iso: &str) -> bool {
        let key = user_id.to_string();
        if self.users.contains_key(&key) {
            return false;
        }
        let balance = balance.max(0) as u64;
        self.users.insert(
            key,
            UserWallet {
                balance,
                updated_at: Some(now_iso.to_string()),
            },
        );
        true
    }
}

pub async fn load_wallet() -> WalletData {
    let path = Path::new(WALLET_FILE);
    if !path.exists() {
        return WalletData::default();
    }
    match tokio::fs::read_to_string(path).await {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| WalletData::default()),
        Err(_) => WalletData::default(),
    }
}

pub async fn save_wallet(data: &WalletData) -> anyhow::Result<()> {
    let path = Path::new(WALLET_FILE);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let s = serde_json::to_string_pretty(data)?;
    tokio::fs::write(path, s).await?;
    Ok(())
}

/// Global lock for wallet file to avoid concurrent read/write.
pub static WALLET_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
