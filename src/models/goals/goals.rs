use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub const DEFAULT_GOALS_PATH: &str = "assets/data/goals/goals.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalsFile {
    pub version: u32,
    pub roots: Vec<GoalNode>,
}

impl Default for GoalsFile {
    fn default() -> Self {
        Self { version: 1, roots: vec![] }
    }
}

/// High-level buckets you listed (and you can extend later).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalCategory {
    Health,
    Wealth,
    Research,
    Time,
    Other,
}

/// A link to “where this goal lives”.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextLink {
    pub label: String, // e.g. "Repo", "Notion", "Land listing", "Workout plan"
    pub kind: String,  // e.g. "path", "url", "note", "map"
    pub value: String, // e.g. "src/projects/guvnuh", "https://...", "assets/..."
}

/// What “done” means.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Metric {
    /// Checkbox goals (immaterial, or “did it / didn’t”).
    Boolean { done: bool },

    /// Numeric goal (money, hours, weight, pages, commits, etc).
    Numeric {
        unit: String, // "usd", "hours", "lbs", "pages", "sessions"
        current: f64,
        target: f64,
        /// Optional: if you want to show progress bounded
        #[serde(default)]
        clamp_0_100: bool,
    },
}

impl Metric {
    pub fn is_done(&self) -> bool {
        match self {
            Metric::Boolean { done } => *done,
            Metric::Numeric { current, target, .. } => current >= target,
        }
    }

    pub fn set_done(&mut self, done: bool) {
        match self {
            Metric::Boolean { done: d } => *d = done,
            Metric::Numeric { current, target, .. } => {
                // For numeric metrics, "done" snaps current to target (or resets to 0).
                *current = if done { *target } else { 0.0 };
            }
        }
    }
}

/// SMART fields. Keep them short but required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Smart {
    pub specific: String,
    pub measurable: String,
    pub achievable: String,
    pub relevant: String,
    pub time_bound: String,
}

/// The recursive goal node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalNode {
    pub id: Uuid,
    pub title: String,
    pub category: GoalCategory,

    /// Optional tags like: "new business", "family", "purchase land"
    #[serde(default)]
    pub tags: Vec<String>,

    pub smart: Smart,
    pub metric: Metric,

    #[serde(default)]
    pub context_links: Vec<ContextLink>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,

    /// If you want a “soft archive”
    #[serde(default)]
    pub archived: bool,

    /// Persisted completion flag (kept in sync with metric)
    #[serde(default)]
    pub completed: bool,

    /// Subgoals
    #[serde(default)]
    pub children: Vec<GoalNode>,
}

impl GoalNode {
    pub fn new(
        title: impl Into<String>,
        category: GoalCategory,
        smart: Smart,
        metric: Metric,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        let completed = metric.is_done();

        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            category,
            tags: vec![],
            smart,
            metric,
            context_links: vec![],
            created_at: now,
            updated_at: now,
            archived: false,
            completed,
            children: vec![],
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Source-of-truth: metric determines done; `completed` mirrors it.
    pub fn is_done(&self) -> bool {
        self.metric.is_done() || self.completed
    }

    /// Sets done in metric and mirrors to `completed`.
    pub fn set_done(&mut self, done: bool) {
        self.metric.set_done(done);
        self.completed = done;
        self.touch();
    }

    pub fn add_child(&mut self, child: GoalNode) {
        self.children.push(child);
        self.touch();
    }

    /// Find any node by id (mutable).
    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut GoalNode> {
        if self.id == id {
            return Some(self);
        }
        for c in &mut self.children {
            if let Some(found) = c.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// If you ever mutate metric.current directly, call this afterward.
    pub fn sync_completed_from_metric(&mut self) {
        self.completed = self.metric.is_done();
        self.touch();
    }
}

impl GoalsFile {
    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut GoalNode> {
        for r in &mut self.roots {
            if let Some(found) = r.find_mut(id) {
                return Some(found);
            }
        }
        None
    }
}
