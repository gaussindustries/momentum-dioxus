use dioxus::prelude::*;

#[component]
pub fn FinCalc(
    // bool with a default: if you don't pass it, it becomes false
    #[props(default)]
    overview: bool,

    // // example: optional string prop
    // #[props(default)]
    // title: Option<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

		if overview {
			Overview {}
		} else {
			Detailed {}
		}
    }
}


/*
	show graph of historical data
	data {
		all:
		assets:
		expenses:
		income:
	
	}

*/


fn Detailed () -> Element {
	rsx!{
        "Yessum"

	}
}

/*
	could very well be for injecting information quickly rather than
	having to dive into the full detailed version of the program,
	or rather just call how "things are going", the graphical aspect

	snapshot of assets

*/

fn Overview () -> Element {
	rsx!{
        "Overview mode FINCALC"

	}
}
