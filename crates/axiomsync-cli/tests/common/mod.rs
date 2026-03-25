use axiomsync_kernel::AxiomSync;

pub fn apply_replay_plan(app: &AxiomSync) {
    let plan = app.build_replay_plan().expect("replay plan");
    app.apply_replay(&plan).expect("apply replay plan");
}
