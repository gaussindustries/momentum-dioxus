//! The shared time/schedule store.
//!
//! Provide it ONCE near the root of your app (above the `Router`) with
//! [`use_provide_time`]. Every other component — the calendar itself, plus
//! Health / JaxBrain / FinCalc — reaches it with [`use_time`] and can read
//! occurrences or push new events. Any mutation autosaves to JSON.

use chrono::{NaiveDate, NaiveDateTime};
use dioxus::prelude::*;
use uuid::Uuid;

use super::model::{Event, EventId, EventSource, Occurrence, Recurrence, When};
use super::storage::{self, SavedState};

/// Cheap, `Copy` handle to the schedule. Hand it around freely.
/// `PartialEq` is required because it's passed as a component prop (Dioxus
/// memoizes props); `Signal` compares by identity, so this is cheap.
#[derive(Clone, Copy, PartialEq)]
pub struct TimeStore {
    events: Signal<Vec<Event>>,
}

/// Call once, high in the tree (in your top-level `App`, before the `Router`).
/// Loads persisted state, provides the context, and wires autosave.
pub fn use_provide_time() -> TimeStore {
    // Load synchronously, exactly once.
    let saved = use_hook(storage::load);
    let events = use_signal(|| saved.events.clone());

    let store = TimeStore { events };
    use_context_provider(|| store);

    // Persist whenever events change.
    use_effect(move || {
        let snapshot = SavedState {
            version: 1,
            events: events.read().clone(),
        };
        storage::save(&snapshot);
    });

    store
}

/// Grab the shared schedule from any descendant component.
pub fn use_time() -> TimeStore {
    use_context::<TimeStore>()
}

impl TimeStore {
    /// Raw signal access if you need to render/iterate the stored events.
    pub fn events(&self) -> Signal<Vec<Event>> {
        self.events
    }

    /// Insert a fully-built event, assigning it a fresh UUID. Returns that id.
    /// The `id` field on the passed-in event is ignored/overwritten.
    pub fn add_event(&self, mut ev: Event) -> EventId {
        let mut events = self.events;
        let id = Uuid::new_v4();
        ev.id = id;
        events.write().push(ev);
        id
    }

    /// Replace an existing event (matched by id). No-op if the id is unknown.
    pub fn update_event(&self, ev: Event) {
        let mut events = self.events;
        let mut guard = events.write();
        if let Some(slot) = guard.iter_mut().find(|e| e.id == ev.id) {
            *slot = ev;
        }
    }

    pub fn remove_event(&self, id: EventId) {
        let mut events = self.events;
        events.write().retain(|e| e.id != id);
    }

    /// Remove every event originating from a given sub-app. Used by sub-apps
    /// (e.g. FinCalc) to re-sync idempotently: clear their old projections,
    /// then re-add the current set.
    pub fn remove_by_source(&self, source: EventSource) {
        let mut events = self.events;
        events.write().retain(|e| e.source != source);
    }

    pub fn get(&self, id: EventId) -> Option<Event> {
        self.events.read().iter().find(|e| e.id == id).cloned()
    }

    // -- Convenience constructors for the sub-apps -------------------------

    /// One-line all-day event. Handy for FinCalc ("bill due"), JaxBrain
    /// ("review note"), Health ("rest day"), etc.
    pub fn add_all_day(
        &self,
        source: EventSource,
        title: impl Into<String>,
        date: NaiveDate,
        link: Option<String>,
    ) -> EventId {
        self.add_event(Event {
            id: Uuid::nil(),
            title: title.into(),
            notes: String::new(),
            when: When::AllDay { date },
            source,
            recurrence: None,
            link,
        })
    }

    /// One-line timed block.
    pub fn add_timed(
        &self,
        source: EventSource,
        title: impl Into<String>,
        start: NaiveDateTime,
        end: NaiveDateTime,
        link: Option<String>,
    ) -> EventId {
        self.add_event(Event {
            id: Uuid::nil(),
            title: title.into(),
            notes: String::new(),
            when: When::Timed { start, end },
            source,
            recurrence: None,
            link,
        })
    }

    /// Same as [`TimeStore::add_timed`] but recurring.
    pub fn add_recurring(
        &self,
        source: EventSource,
        title: impl Into<String>,
        start: NaiveDateTime,
        end: NaiveDateTime,
        recurrence: Recurrence,
        link: Option<String>,
    ) -> EventId {
        self.add_event(Event {
            id: Uuid::nil(),
            title: title.into(),
            notes: String::new(),
            when: When::Timed { start, end },
            source,
            recurrence: Some(recurrence),
            link,
        })
    }

    // -- Querying ----------------------------------------------------------

    /// All occurrences whose date falls in `[from, to]`, sorted by start time.
    pub fn occurrences_in(&self, from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
        let events = self.events.read();
        let mut out: Vec<Occurrence> = Vec::new();
        for e in events.iter() {
            out.extend(e.occurrences(from, to));
        }
        out.sort_by(|a, b| a.start.cmp(&b.start).then(a.title.cmp(&b.title)));
        out
    }

    /// Occurrences for a single day.
    pub fn occurrences_on(&self, date: NaiveDate) -> Vec<Occurrence> {
        self.occurrences_in(date, date)
    }
}
