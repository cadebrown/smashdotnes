use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: "main.css" }

        main { class: "shell",
            header { class: "topbar",
                a { class: "brand", href: "/", "SMASH.NES" }
                nav { class: "topnav",
                    a { href: "/play", "Play" }
                    a { href: "/docs", "Docs" }
                    a { href: "/info", "Info" }
                }
            }

            section { class: "hero",
                h1 { class: "title", "SMASH.NES" }
                p { class: "tagline", "Retro-fitted Smash-style combat on real NES constraints. Fast boot. Zero clutter." }
                a { class: "btn-play", href: "/play", "PLAY NOW" }
                p { class: "links",
                    a { href: "/docs", "Docs" }
                    span { " · " }
                    a { href: "/info", "Info" }
                }
            }
        }
    }
}
