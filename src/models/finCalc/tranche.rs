// src/models/finCalc/tranche.rs
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    OneTime,
    Daily,
    Weekly,
    Monthly,
    XDays(u32),
    XMonths(u32),
}

impl Frequency {
    pub fn label(&self) -> String {
        match self {
            Frequency::OneTime => "One-time".into(),
            Frequency::Daily => "Daily".into(),
            Frequency::Weekly => "Weekly".into(),
            Frequency::Monthly => "Monthly".into(),
            Frequency::XDays(n) => format!("Every {n} days"),
            Frequency::XMonths(n) => format!("Every {n} months"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranchePrimitives {
    created_at: OffsetDateTime,
    frequency: Frequency,
    amount: i64, // cents
}

impl TranchePrimitives {
    pub fn new(amount: i64, frequency: Frequency) -> Self {
        Self {
            created_at: OffsetDateTime::now_utc(),
            frequency,
            amount,
        }
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

pub trait Tranche {
    fn primitives(&self) -> &TranchePrimitives;
    fn primitives_mut(&mut self) -> &mut TranchePrimitives;

    fn get_frequency(&self) -> Frequency {
        self.primitives().frequency()
    }
    fn get_amount(&self) -> i64 {
        self.primitives().amount()
    }
    fn get_created_at(&self) -> OffsetDateTime {
        self.primitives().created_at()
    }

    fn set_amount(&mut self, new_amount: i64) {
        self.primitives_mut().amount = new_amount;
    }
    fn add_amount(&mut self, delta: i64) {
        self.primitives_mut().amount += delta;
    }
    fn subtract_amount(&mut self, delta: i64) {
        self.primitives_mut().amount -= delta;
    }
    fn set_frequency(&mut self, freq: Frequency) {
        self.primitives_mut().frequency = freq;
    }
}

#[derive(Debug, Clone)]
pub enum IncomeKind {
    Hourly {
        hourly_rate_cents: i64,
        avg_hours_per_week: f64,
    },
    Salary {
        yearly_cents: i64,
    },
    Other,
}

impl IncomeKind {
    pub fn label(&self) -> String {
        match self {
            IncomeKind::Hourly { .. } => "Hourly".into(),
            IncomeKind::Salary { .. } => "Salary".into(),
            IncomeKind::Other => "Other".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Income {
    name: String,
    kind: IncomeKind,
    data: TranchePrimitives,
}

impl Income {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> &IncomeKind {
        &self.kind
    }

    pub fn new_hourly(
        name: impl Into<String>,
        hourly_rate_cents: i64,
        avg_hours_per_week: f64,
    ) -> Self {
        let weekly = hourly_rate_cents as f64 * avg_hours_per_week;
        let monthly = (weekly * 52.0 / 12.0).round() as i64;

        Self {
            name: name.into(),
            kind: IncomeKind::Hourly {
                hourly_rate_cents,
                avg_hours_per_week,
            },
            data: TranchePrimitives::new(monthly, Frequency::Monthly),
        }
    }

    pub fn new_salary(name: impl Into<String>, yearly_cents: i64) -> Self {
        let monthly = yearly_cents / 12;
        Self {
            name: name.into(),
            kind: IncomeKind::Salary { yearly_cents },
            data: TranchePrimitives::new(monthly, Frequency::Monthly),
        }
    }
}

impl Tranche for Income {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }
    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Hard,
    Soft,
}

impl AssetKind {
    pub fn label(&self) -> &'static str {
        match self {
            AssetKind::Hard => "Hard",
            AssetKind::Soft => "Soft",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortfolioAsset {
    name: String,
    kind: AssetKind,
    data: TranchePrimitives,
}

impl PortfolioAsset {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> AssetKind {
        self.kind
    }

    pub fn new(name: impl Into<String>, kind: AssetKind, amount_cents: i64) -> Self {
        Self {
            name: name.into(),
            kind,
            data: TranchePrimitives::new(amount_cents, Frequency::OneTime),
        }
    }
}

impl Tranche for PortfolioAsset {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }
    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiabilityKind {
    Bill,
    Loan,
    Other,
}

impl LiabilityKind {
    pub fn label(&self) -> &'static str {
        match self {
            LiabilityKind::Bill => "Bill",
            LiabilityKind::Loan => "Loan",
            LiabilityKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Liability {
    name: String,
    kind: LiabilityKind,
    data: TranchePrimitives,
}

impl Liability {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> LiabilityKind {
        self.kind
    }

    pub fn new(
        name: impl Into<String>,
        kind: LiabilityKind,
        amount_cents: i64,
        frequency: Frequency,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            data: TranchePrimitives::new(amount_cents, frequency),
        }
    }
}

impl Tranche for Liability {
    fn primitives(&self) -> &TranchePrimitives {
        &self.data
    }
    fn primitives_mut(&mut self) -> &mut TranchePrimitives {
        &mut self.data
    }
}
