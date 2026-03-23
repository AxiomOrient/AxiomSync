use crate::domain::CaseRecord;
use maud::{DOCTYPE, Markup, html};

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

pub fn index(cases: &[CaseRecord]) -> String {
    layout(
        "AxiomSync Cases",
        html! {
            h1 { "AxiomSync Agent Memory Kernel" }
            p { "SQLite-backed case, thread, and evidence surface for universal agent records." }
            @for case_record in cases {
                article.card {
                    h2 {
                        a href=(format!("/cases/{}", case_record.case_id)) {
                            (case_record.problem)
                        }
                    }
                    p { code { (case_record.case_id) } }
                    @if let Some(resolution) = &case_record.resolution {
                        p { (resolution) }
                    }
                    @if !case_record.commands.is_empty() {
                        p { "Commands: " (case_record.commands.join(" | ")) }
                    }
                }
            }
        },
    )
}

pub fn case_page(case_record: &CaseRecord) -> String {
    layout(
        "Case",
        html! {
            h1 { (case_record.problem) }
            p { code { (case_record.case_id) } }
            @if let Some(root_cause) = &case_record.root_cause {
                section.card {
                    h2 { "Root Cause" }
                    p { (root_cause) }
                }
            }
            @if let Some(resolution) = &case_record.resolution {
                section.card {
                    h2 { "Resolution" }
                    p { (resolution) }
                }
            }
            section.card {
                h2 { "Commands" }
                ul {
                    @for command in &case_record.commands {
                        li { code { (command) } }
                    }
                }
            }
            section.card {
                h2 { "Verification" }
                ul {
                    @for verification in &case_record.verification {
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
                    @for uri in &case_record.evidence {
                        li { code { (uri) } }
                    }
                }
            }
        },
    )
}

pub fn episode(runbook: &crate::domain::RunbookRecord) -> String {
    case_page(&runbook.clone().into())
}
