use dioxus::prelude::*;
use crate::components::{Goals};
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn GoalView() -> Element {
    rsx! {
		Goals { overview:false }
    }
}
