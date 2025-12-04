use dioxus::prelude::*;
use crate::components::{JaxBrain};
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn JaxBrainView() -> Element {
    rsx! {
		JaxBrain { overview:false }
    }
}
