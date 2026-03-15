use crate::core::data_file;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const WALLET_FILE: &str = "wallet.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWallet {
    pub balance: u64,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    pub users: HashMap<String, UserWallet>,
    #[serde(default)]
    pub unit: String,
}

impl Default for WalletData {
    fn default() -> Self {
        Self {
            users: HashMap::new(),
            unit: String::new(),
        }
    }
}

impl WalletData {
    pub fn has_user(&self, user_id: u64) -> bool {
        self.users.contains_key(&user_id.to_string())
    }

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
    match data_file::load(WALLET_FILE).await {
        Ok(s) if !s.is_empty() => {
            serde_json::from_str(&s).unwrap_or_else(|_| WalletData::default())
        }
        _ => WalletData::default(),
    }
}

pub async fn save_wallet(data: &WalletData) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(data)?;
    data_file::save(WALLET_FILE, &s).await
}

pub static WALLET_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
