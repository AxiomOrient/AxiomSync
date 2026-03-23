pub struct ProjectionScope {
    pub session_ids: Vec<String>,
}

pub fn rebuild_projection(_scope: ProjectionScope) {
    // Pseudocode:
    // 1. scan ingress ledger in stable order
    // 2. upsert sessions
    // 3. upsert actors
    // 4. append entries in seq order
    // 5. attach artifacts
    // 6. attach anchors
    // 7. refresh links
}
