pub struct DeriveScope {
    pub session_ids: Vec<String>,
}

pub fn rebuild_derivations(_scope: DeriveScope) {
    // Pseudocode:
    // 1. select evidence-bearing entry windows
    // 2. segment reusable episodes
    // 3. derive claims from episodes
    // 4. derive procedures only when evidence spans >= threshold
    // 5. mark stale derived rows by extractor version
}
