use crate::{components::navbar::*, Route};
use dioxus::prelude::*;
const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn NavbarLayout() -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: NAVBAR_CSS }

		div {
			class: "p-2 bg-primary-color text-secondary-color flex justify-between",

			Navbar {
				NavbarNav {
					index: 0usize,  
					NavbarTrigger { "💭" }
					NavbarContent {
						NavbarItem { 
							index: 0usize,
							value: "home".to_string(),
							to: Route::Overview {  },
							"Overview"
						}
						NavbarItem { 
							index: 1usize,
							value: "fincalc".to_string(),
							to: Route::FinCalcView {  },
							"Wealth 💸"
						}
					}
				}
			}
		}

		Outlet::<Route> {}
	}
}
