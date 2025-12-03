use dioxus::prelude::*;
use crate::components::{FinCalc};
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn FinCalcView() -> Element {
    rsx! {
		FinCalc { overview:false }
    }
}
