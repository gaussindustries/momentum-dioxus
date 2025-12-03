use dioxus::prelude::*;
use crate::components::{FinCalc};
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Overview() -> Element {
    rsx! {
        //put overview shit here
		FinCalc { overview:true }
    }
}
