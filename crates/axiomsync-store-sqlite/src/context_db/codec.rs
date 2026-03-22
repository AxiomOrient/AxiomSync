use super::*;
fn invalid_enum(name: &str, raw: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid {name} {raw}"),
        )),
    )
}

pub(super) fn parse_item_type(raw: String) -> rusqlite::Result<ItemType> {
    ItemType::parse(&raw).map_err(|_| invalid_enum("ItemType", raw))
}

pub(super) fn parse_selector_type(raw: String) -> rusqlite::Result<SelectorType> {
    SelectorType::parse(&raw).map_err(|_| invalid_enum("SelectorType", raw))
}

pub(super) fn parse_episode_status(raw: String) -> rusqlite::Result<EpisodeStatus> {
    EpisodeStatus::parse(&raw).map_err(|_| invalid_enum("EpisodeStatus", raw))
}

pub(super) fn parse_insight_kind(raw: String) -> rusqlite::Result<InsightKind> {
    InsightKind::parse(&raw).map_err(|_| invalid_enum("InsightKind", raw))
}

pub(super) fn parse_verification_kind(raw: String) -> rusqlite::Result<VerificationKind> {
    VerificationKind::parse(&raw).map_err(|_| invalid_enum("VerificationKind", raw))
}

pub(super) fn parse_verification_status(raw: String) -> rusqlite::Result<VerificationStatus> {
    VerificationStatus::parse(&raw).map_err(|_| invalid_enum("VerificationStatus", raw))
}
