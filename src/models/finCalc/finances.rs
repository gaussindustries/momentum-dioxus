use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;
use crate::utils::json_store::{load_json, save_json, err_to_string};

pub const DEFAULT_FIN_PATH: &str = "assets/data/finances.json";

// --- money helper: accept 45.99 or 4599 (and turn into cents) ---
mod money_cents {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(cents: &i64, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // store as cents (integer) when writing
        s.serialize_i64(*cents)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        // accept: 4599, "4599", 45.99, "45.99"
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i) // assume already cents
                } else if let Some(f) = n.as_f64() {
                    Ok((f * 100.0).round() as i64)
                } else {
                    Err(de::Error::custom("Bad number for money"))
                }
            }
            serde_json::Value::String(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    Ok(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    Ok((f * 100.0).round() as i64)
                } else {
                    Err(de::Error::custom("Bad string for money"))
                }
            }
            _ => Err(de::Error::custom("Expected number or string for money")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancesFile {
    pub assets: Vec<AssetEntry>,
    pub income: Vec<IncomeEntry>,
    pub expenses: Vec<ExpenseEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: Uuid,
    pub name: String,
    #[serde(with = "money_cents")]
    pub value: i64, // cents
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeEntry {
    pub id: Uuid,
    pub date: Date,
    pub source: String,
    #[serde(with = "money_cents")]
    pub amount: i64, // cents
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseEntry {
    pub id: Uuid,
    pub date: Date,
    pub category: String,
    #[serde(with = "money_cents")]
    pub amount: i64, // cents
}

impl Default for FinancesFile {
    fn default() -> Self {
        Self { assets: vec![], income: vec![], expenses: vec![] }
    }
}
