use dioxus::prelude::*;
use crate::components::{Health};
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn HealthView() -> Element {
    rsx! {
		Health { overview:false }
    }
}
