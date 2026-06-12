use crate::components::{FinCalc, Health, JaxBrain, Time};
use dioxus::prelude::*;
/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Overview() -> Element {
    rsx! {
        div { //put overview shit here
            h1 {class:"text-center", "Overview" }

            Time { overview: true }

            // Goals { overview:true }

            FinCalc { overview:true }

            Health { overview:true }

            JaxBrain { overview:true }
        }
    }
}
