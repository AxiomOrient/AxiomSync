use crate::mime::infer_mime;
use crate::models::{IndexRecord, SearchFilter};

#[derive(Debug)]
pub(super) struct NormalizedFilter {
    pub(super) tags: Vec<String>,
    pub(super) mime: Option<String>,
    pub(super) namespace_prefix: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) start_time: Option<i64>,
    pub(super) end_time: Option<i64>,
}

pub(super) fn record_matches_filter(
    record: &IndexRecord,
    filter: Option<&SearchFilter>,
    has_matching_leaf_descendant: impl FnOnce(&NormalizedFilter) -> bool,
) -> bool {
    let Some(normalized_filter) = normalize_filter(filter) else {
        return true;
    };

    if record.is_leaf {
        return leaf_matches_filter(record, &normalized_filter);
    }

    has_matching_leaf_descendant(&normalized_filter)
}

pub(super) fn normalize_filter(filter: Option<&SearchFilter>) -> Option<NormalizedFilter> {
    let filter = filter?;
    let tags = filter
        .tags
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    let mime = filter
        .mime
        .as_ref()
        .map(|mime| mime.trim().to_lowercase())
        .filter(|mime| !mime.is_empty());
    let namespace_prefix = filter
        .namespace_prefix
        .as_ref()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let kind = filter
        .kind
        .as_ref()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    if tags.is_empty()
        && mime.is_none()
        && namespace_prefix.is_none()
        && kind.is_none()
        && filter.start_time.is_none()
        && filter.end_time.is_none()
    {
        return None;
    }
    Some(NormalizedFilter {
        tags,
        mime,
        namespace_prefix,
        kind,
        start_time: filter.start_time,
        end_time: filter.end_time,
    })
}

pub(super) fn leaf_matches_filter(record: &IndexRecord, filter: &NormalizedFilter) -> bool {
    if !filter.tags.is_empty()
        && !filter.tags.iter().all(|wanted| {
            record
                .tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(wanted))
        })
    {
        return false;
    }

    if let Some(required_mime) = &filter.mime {
        let Some(record_mime) = infer_mime(record) else {
            return false;
        };
        if !record_mime.eq_ignore_ascii_case(required_mime) {
            return false;
        }
    }

    if let Some(namespace_prefix) = filter.namespace_prefix.as_deref() {
        let Some(namespace) = record_tag_value(record, "ns:") else {
            return false;
        };
        if namespace != namespace_prefix
            && !namespace
                .strip_prefix(namespace_prefix)
                .is_some_and(|tail| tail.starts_with('/'))
        {
            return false;
        }
    }

    if let Some(kind) = filter.kind.as_deref()
        && record_tag_value(record, "kind:") != Some(kind)
    {
        return false;
    }

    let event_time =
        record_tag_value(record, "event_time:").and_then(|raw| raw.parse::<i64>().ok());
    if let Some(start_time) = filter.start_time
        && event_time.is_none_or(|value| value < start_time)
    {
        return false;
    }
    if let Some(end_time) = filter.end_time
        && event_time.is_none_or(|value| value > end_time)
    {
        return false;
    }

    true
}

fn record_tag_value<'a>(record: &'a IndexRecord, prefix: &str) -> Option<&'a str> {
    record.tags.iter().find_map(|tag| tag.strip_prefix(prefix))
}
