use crate::components::time::Time;
use dioxus::prelude::*;

/// Full-page scheduler. Rendered for `Route::TimeView` (`/Time`).
#[component]
pub fn TimeView() -> Element {
    rsx! {
        Time {}
    }
}
