//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals. In this template, we define a Hero
//! component  to be used in our app.

pub mod alert_dialog;
pub mod aspect_ratio;
pub mod button;
pub mod calendar;
pub mod checkbox;
pub mod input;
pub mod navbar;
pub mod separator;
pub mod tabs;
pub mod tooltip;

mod fin_calc;
pub use fin_calc::FinCalc;

mod health;
pub use health::Health;

// mod goals;
// pub use goals::Goals;

mod jax_brain;
pub use jax_brain::JaxBrain;

mod graph_generator;
pub use graph_generator::GraphGenerator;

pub mod time;
pub use time::Time;

pub mod accordion;
pub mod context_menu;
pub mod date_picker;
pub mod popover;
pub mod select;
