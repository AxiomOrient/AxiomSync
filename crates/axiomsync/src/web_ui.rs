use maud::{DOCTYPE, Markup, html};
use serde_json::Value;

use crate::domain::RunbookRecord;

fn layout(title: &str, body: Markup) -> String {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                style {
                    r#"
                    :root { color-scheme: light; font-family: "Iosevka Aile", "IBM Plex Sans", sans-serif; }
                    body { margin: 0; background: linear-gradient(160deg, #f7f3ea, #eef5ef); color: #1d2a22; }
                    main { max-width: 920px; margin: 0 auto; padding: 32px 20px 80px; }
                    h1, h2, h3 { font-family: "IBM Plex Serif", "Georgia", serif; }
                    .card { background: rgba(255,255,255,0.86); border: 1px solid #d7dfd2; border-radius: 18px; padding: 18px 20px; margin: 14px 0; box-shadow: 0 10px 30px rgba(30,50,35,0.08); }
                    code, pre { font-family: "Iosevka", monospace; }
                    pre { white-space: pre-wrap; background: #f3f6f2; padding: 12px; border-radius: 12px; }
                    a { color: #0e5e52; text-decoration: none; }
                    ul { padding-left: 20px; }
                    "# }
            }
            body {
                main { (body) }
            }
        }
    }
    .into_string()
}

pub fn index(runbooks: &[RunbookRecord]) -> String {
    layout(
        "AxiomSync Episodes",
        html! {
            h1 { "AxiomSync Renewal Kernel" }
            p { "SQLite-backed conversation ledger and runbook surface." }
            @for runbook in runbooks {
                article.card {
                    h2 {
                        a href=(format!("/episodes/{}", runbook.episode_id)) {
                            (runbook.problem)
                        }
                    }
                    p { code { (runbook.episode_id) } }
                    @if let Some(fix) = &runbook.fix {
                        p { (fix) }
                    }
                    @if !runbook.commands.is_empty() {
                        p { "Commands: " (runbook.commands.join(" | ")) }
                    }
                }
            }
        },
    )
}

pub fn episode(runbook: &RunbookRecord) -> String {
    layout(
        "Episode",
        html! {
            h1 { (runbook.problem) }
            p { code { (runbook.episode_id) } }
            @if let Some(root_cause) = &runbook.root_cause {
                section.card {
                    h2 { "Root Cause" }
                    p { (root_cause) }
                }
            }
            @if let Some(fix) = &runbook.fix {
                section.card {
                    h2 { "Fix" }
                    p { (fix) }
                }
            }
            section.card {
                h2 { "Commands" }
                ul {
                    @for command in &runbook.commands {
                        li { code { (command) } }
                    }
                }
            }
            section.card {
                h2 { "Verification" }
                ul {
                    @for verification in &runbook.verification {
                        li {
                            strong { (&verification.kind) " · " (&verification.status) }
                            @if let Some(summary) = &verification.summary {
                                " " (summary)
                            }
                        }
                    }
                }
            }
            section.card {
                h2 { "Evidence" }
                ul {
                    @for uri in &runbook.evidence {
                        li { code { (uri) } }
                    }
                }
            }
        },
    )
}

pub fn connectors(status: &Value) -> String {
    layout(
        "Connectors",
        html! {
            h1 { "Connector Status" }
            div.card {
                pre { (serde_json::to_string_pretty(status).unwrap_or_else(|_| "{}".to_string())) }
            }
        },
    )
}
