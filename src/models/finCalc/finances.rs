// src/models/finCalc/finances.rs
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// Seed/sample path (checked into the repo). The live app reads/writes a copy
/// outside the project tree — see `default_finances_path()` in the component —
/// so editing doesn't trigger `dx serve` rebuild loops.
pub const DEFAULT_FIN_PATH: &str = "assets/data/finances.json";

// ---------------------------------------------------------------------------
// Frequency — how often a cash flow repeats. Supersedes tranche.rs::Frequency
// (this one is serde-friendly and carries the monthly-normalization math the
// dashboard needs).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frequency {
    OneTime,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    EveryNDays(u32),
    EveryNMonths(u32),
}

impl Default for Frequency {
    fn default() -> Self {
        Frequency::Monthly
    }
}

impl Frequency {
    pub fn label(&self) -> String {
        match self {
            Frequency::OneTime => "One-time".into(),
            Frequency::Daily => "Daily".into(),
            Frequency::Weekly => "Weekly".into(),
            Frequency::Monthly => "Monthly".into(),
            Frequency::Yearly => "Yearly".into(),
            Frequency::EveryNDays(n) => format!("Every {n} days"),
            Frequency::EveryNMonths(n) => format!("Every {n} months"),
        }
    }

    /// The set shown in the UI dropdown. The `EveryN*` variants are still valid
    /// in the model/JSON; they're just not surfaced as menu items.
    pub const UI: [Frequency; 5] = [
        Frequency::OneTime,
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Yearly,
    ];

    pub fn from_label(s: &str) -> Frequency {
        match s {
            "One-time" => Frequency::OneTime,
            "Daily" => Frequency::Daily,
            "Weekly" => Frequency::Weekly,
            "Yearly" => Frequency::Yearly,
            _ => Frequency::Monthly,
        }
    }

    /// How many times this recurs per month, on average. Used to normalize all
    /// cash flows to a common monthly figure. `OneTime` returns 0 (it isn't a
    /// recurring monthly burden).
    pub fn per_month(&self) -> f64 {
        // 365.2425 days/yr, 12 months/yr.
        match self {
            Frequency::OneTime => 0.0,
            Frequency::Daily => 365.2425 / 12.0,
            Frequency::Weekly => 52.1775 / 12.0,
            Frequency::Monthly => 1.0,
            Frequency::Yearly => 1.0 / 12.0,
            Frequency::EveryNDays(n) if *n > 0 => (365.2425 / *n as f64) / 12.0,
            Frequency::EveryNMonths(n) if *n > 0 => 1.0 / *n as f64,
            _ => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Categorization for net-worth lines
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Cash,
    Investment,
    Property,
    Vehicle,
    Other,
}

impl Default for AssetKind {
    fn default() -> Self {
        AssetKind::Other
    }
}

impl AssetKind {
    pub const ALL: [AssetKind; 5] = [
        AssetKind::Cash,
        AssetKind::Investment,
        AssetKind::Property,
        AssetKind::Vehicle,
        AssetKind::Other,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            AssetKind::Cash => "Cash",
            AssetKind::Investment => "Investment",
            AssetKind::Property => "Property",
            AssetKind::Vehicle => "Vehicle",
            AssetKind::Other => "Other",
        }
    }
    pub fn from_label(s: &str) -> AssetKind {
        AssetKind::ALL
            .into_iter()
            .find(|k| k.label() == s)
            .unwrap_or(AssetKind::Other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityKind {
    CreditCard,
    Loan,
    Mortgage,
    Other,
}

impl Default for LiabilityKind {
    fn default() -> Self {
        LiabilityKind::Other
    }
}

impl LiabilityKind {
    pub const ALL: [LiabilityKind; 4] = [
        LiabilityKind::CreditCard,
        LiabilityKind::Loan,
        LiabilityKind::Mortgage,
        LiabilityKind::Other,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            LiabilityKind::CreditCard => "Credit card",
            LiabilityKind::Loan => "Loan",
            LiabilityKind::Mortgage => "Mortgage",
            LiabilityKind::Other => "Other",
        }
    }
    pub fn from_label(s: &str) -> LiabilityKind {
        LiabilityKind::ALL
            .into_iter()
            .find(|k| k.label() == s)
            .unwrap_or(LiabilityKind::Other)
    }
}

// ---------------------------------------------------------------------------
// Flexible money (de)serialization: accepts 4599, "4599", 45.99, or "45.99".
// Always stored as integer cents.
// ---------------------------------------------------------------------------

mod money_cents {
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(cents: &i64, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_i64(*cents)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i) // already cents
                } else if let Some(f) = n.as_f64() {
                    Ok((f * 100.0).round() as i64)
                } else {
                    Err(de::Error::custom("bad number for money"))
                }
            }
            serde_json::Value::String(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    Ok(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    Ok((f * 100.0).round() as i64)
                } else {
                    Err(de::Error::custom("bad string for money"))
                }
            }
            _ => Err(de::Error::custom("expected number or string for money")),
        }
    }
}

fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

// ---------------------------------------------------------------------------
// The persisted file
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinancesFile {
    #[serde(default)]
    pub assets: Vec<AssetEntry>,
    #[serde(default)]
    pub liabilities: Vec<LiabilityEntry>,
    #[serde(default)]
    pub income: Vec<CashFlow>,
    #[serde(default)]
    pub expenses: Vec<CashFlow>,
    /// Dated snapshots for the trend chart. One point per calendar day.
    #[serde(default)]
    pub history: Vec<Snapshot>,
}

impl FinancesFile {
    pub fn total_assets(&self) -> i64 {
        self.assets.iter().map(|a| a.value).sum()
    }
    pub fn total_liabilities(&self) -> i64 {
        self.liabilities.iter().map(|l| l.balance).sum()
    }
    pub fn net_worth(&self) -> i64 {
        self.total_assets() - self.total_liabilities()
    }
    pub fn monthly_income(&self) -> i64 {
        monthly_total(&self.income)
    }
    pub fn monthly_expenses(&self) -> i64 {
        monthly_total(&self.expenses)
    }
    pub fn monthly_net(&self) -> i64 {
        self.monthly_income() - self.monthly_expenses()
    }

    /// Capture today's figures as a history point. Upserts: if the most recent
    /// point is already today's, it's overwritten, so we keep one point per day.
    pub fn record_snapshot(&mut self) {
        let snap = Snapshot {
            date: today(),
            net_worth: self.net_worth(),
            monthly_income: self.monthly_income(),
            monthly_expenses: self.monthly_expenses(),
        };
        match self.history.last_mut() {
            Some(last) if last.date == snap.date => *last = snap,
            _ => self.history.push(snap),
        }
    }
}

/// Sum a set of cash flows normalized to a per-month figure (in cents).
pub fn monthly_total(flows: &[CashFlow]) -> i64 {
    flows
        .iter()
        .map(|f| (f.amount as f64 * f.frequency.per_month()).round() as i64)
        .sum()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub kind: AssetKind,
    #[serde(with = "money_cents")]
    pub value: i64, // cents
}

impl AssetEntry {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New asset".into(),
            kind: AssetKind::Cash,
            value: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiabilityEntry {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub kind: LiabilityKind,
    #[serde(with = "money_cents")]
    pub balance: i64, // cents owed
}

impl LiabilityEntry {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New debt".into(),
            kind: LiabilityKind::Other,
            balance: 0,
        }
    }
}

/// A recurring (or one-time) inflow or outflow. Used for both income and
/// expenses — the list it lives in determines the sign/meaning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlow {
    pub id: Uuid,
    pub name: String,
    #[serde(with = "money_cents")]
    pub amount: i64, // cents per occurrence
    #[serde(default)]
    pub frequency: Frequency,
    /// Anchor / next occurrence date. Drives calendar projection.
    #[serde(default = "today")]
    pub date: Date,
}

impl CashFlow {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            amount: 0,
            frequency: Frequency::Monthly,
            date: today(),
        }
    }

    /// This flow's contribution to a monthly budget, in cents.
    pub fn per_month_cents(&self) -> i64 {
        (self.amount as f64 * self.frequency.per_month()).round() as i64
    }
}

/// A dated point for the trend chart. Money fields are plain cents (they're
/// derived figures, not user-entered, so they skip the flexible money parser).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub date: Date,
    pub net_worth: i64,
    pub monthly_income: i64,
    pub monthly_expenses: i64,
}

impl Snapshot {
    pub fn take_home(&self) -> i64 {
        self.monthly_income - self.monthly_expenses
    }
}
