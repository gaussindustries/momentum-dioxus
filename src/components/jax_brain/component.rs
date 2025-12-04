use dioxus::prelude::*;

#[component]
pub fn JaxBrain(
    // bool with a default: if you don't pass it, it becomes false
    #[props(default)]
    overview: bool,

    // // example: optional string prop
    // #[props(default)]
    // title: Option<String>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        div {
            if overview {
                "Overview mode Jax Brain"
            } else {
                "Detail mode Jax Brain"
            }
        }
    }
}
