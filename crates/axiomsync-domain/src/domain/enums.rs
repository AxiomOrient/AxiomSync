use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{AxiomError, Result};

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            pub fn parse(value: &str) -> Result<Self> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    other => Err(AxiomError::Validation(format!(
                        "invalid {} {}",
                        stringify!($name),
                        other
                    ))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

    };
}

string_enum!(ItemType {
    UserMsg => "user_msg",
    AssistantMsg => "assistant_msg",
    ToolCall => "tool_call",
    ToolResult => "tool_result",
    FileChange => "file_change",
    Diff => "diff",
    Plan => "plan",
});

string_enum!(SelectorType {
    TextSpan => "text_span",
    JsonPointer => "json_pointer",
    DiffHunk => "diff_hunk",
    ArtifactRange => "artifact_range",
    DomSelector => "dom_selector",
});

string_enum!(EpisodeStatus {
    Open => "open",
    Solved => "solved",
    Abandoned => "abandoned",
});

string_enum!(InsightKind {
    Problem => "problem",
    Fix => "fix",
    RootCause => "root_cause",
    Decision => "decision",
    Command => "command",
    Snippet => "snippet",
});

string_enum!(VerificationKind {
    Test => "test",
    CommandExit => "command_exit",
    DiffApplied => "diff_applied",
    HumanConfirm => "human_confirm",
});

string_enum!(VerificationStatus {
    Pass => "pass",
    Fail => "fail",
    Partial => "partial",
    Unknown => "unknown",
});
