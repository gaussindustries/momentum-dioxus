use crate::{components::navbar::*, Route};
use dioxus::prelude::*;
const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn NavbarLayout() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: NAVBAR_CSS }

		div {
			class: "p-2 ",

			Navbar {
				NavbarNav {
					index: 0usize,  
					NavbarTrigger { "💭" }
					NavbarContent {
						NavbarItem { 
							index: 0usize,
							value: "home".to_string(),
							to: Route::Overview {  },
							div{class:"text-center",
								"Overview"
							}
						}
						NavbarItem { 
							index: 1usize,
							value: "goals".to_string(),
							to: Route::GoalView {  },
							div{class:"text-center",
								"Goals 🎯"
							}
						}
						
						NavbarItem {
							index: 2usize,
							value: "health".to_string(),
							to: Route::HealthView {  },
							div{class:"text-center",
								"Health 💪"
							}
						}
						NavbarItem { 
							index: 3usize,
							value: "fincalc".to_string(),
							to: Route::FinCalcView {  },
							div{class:"text-center",
								"Wealth 💸"
							}
						}
						NavbarItem { 
							index: 4usize,
							value: "jaxbrain".to_string(),
							to: Route::JaxBrainView {  },
							div{ class:"flex justify-center gap-2 items-center",
								"Jax Brain"
								img {class:"h-[30px]", src: asset! { "assets/images/digital_brain.png"}}
							}
						}
					}
				}
			}
		}
		div { class:"flex justify-center",
			Outlet::<Route> {}
		}
	}
}
