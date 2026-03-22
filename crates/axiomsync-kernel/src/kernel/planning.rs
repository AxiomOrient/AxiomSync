use super::*;

impl AxiomSync {
    pub fn plan_ingest(&self, input: &ConnectorBatchInput) -> Result<IngestPlan> {
        let existing = self.repo.existing_raw_event_keys()?;
        plan_ingest(&existing, input)
    }

    pub fn apply_ingest(&self, plan: &IngestPlan) -> Result<Value> {
        self.repo.apply_ingest_tx(plan)
    }

    pub fn plan_projection(&self) -> Result<ProjectionPlan> {
        plan_projection(&self.repo.load_raw_events()?)
    }

    pub fn apply_projection(&self, plan: &ProjectionPlan) -> Result<Value> {
        self.repo.apply_projection_tx(plan)
    }

    pub fn plan_replay(&self) -> Result<ReplayPlan> {
        let raw_events = self.repo.load_raw_events()?;
        self.plan_replay_for_raw_events(&raw_events)
    }

    pub fn apply_replay(&self, plan: &ReplayPlan) -> Result<Value> {
        self.repo.apply_replay_tx(plan)
    }

    pub fn derivation_contexts(&self) -> Result<Vec<DerivationContext>> {
        Ok(plan_derivation_contexts(
            &self.repo.load_sessions()?,
            &self.repo.load_turns()?,
            &self.repo.load_items()?,
        ))
    }

    pub fn plan_derivation(&self) -> Result<DerivePlan> {
        let sessions = self.repo.load_sessions()?;
        let turns = self.repo.load_turns()?;
        let items = self.repo.load_items()?;
        let anchors = self.repo.load_evidence_anchors()?;
        let contexts = plan_derivation_contexts(&sessions, &turns, &items);
        let enrichment = self.collect_derivation_inputs(&contexts)?;
        plan_derivation(
            &contexts,
            &turns,
            &items,
            &anchors,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_derivation(&self, plan: &DerivePlan) -> Result<Value> {
        self.repo.apply_derivation_tx(plan)
    }

    pub fn plan_purge(
        &self,
        connector: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<PurgePlan> {
        let raw_events = self.repo.load_raw_events()?;
        let replay_contexts =
            self.derive_contexts_after_purge(&raw_events, connector, workspace_id)?;
        plan_purge(
            &raw_events,
            connector,
            workspace_id,
            &replay_contexts.extractions,
            &replay_contexts.verifications,
        )
    }

    pub fn apply_purge(&self, plan: &PurgePlan) -> Result<Value> {
        self.repo.apply_purge_tx(plan)
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        self.repo.doctor_report()
    }

    pub fn plan_repair(&self, input: &ConnectorBatchInput) -> Result<RepairPlan> {
        let ingest = self.plan_ingest(input)?;
        let raw_events = self.repo.load_raw_events()?;
        let replay_contexts = self.derive_contexts_after_repair(&raw_events, &ingest)?;
        plan_repair(
            &raw_events,
            &ingest,
            &replay_contexts.extractions,
            &replay_contexts.verifications,
        )
    }

    pub fn apply_repair(&self, plan: &RepairPlan) -> Result<Value> {
        self.repo.apply_repair_tx(plan)
    }

    pub(crate) fn collect_derivation_inputs(
        &self,
        contexts: &[DerivationContext],
    ) -> Result<DerivationEnrichment> {
        let mut enrichment = DerivationEnrichment::default();
        for context in contexts {
            match self.llm.extract_episode(&context.transcript) {
                Ok(extraction) => {
                    enrichment
                        .extractions
                        .insert(context.episode_id.clone(), extraction);
                }
                Err(AxiomError::LlmUnavailable(_)) => {}
                Err(error) => return Err(error),
            }
            let mut verification_candidates = parse_verification_transcript(&context.transcript);
            match self.llm.synthesize_verifications(&context.transcript) {
                Ok(extractions) => verification_candidates.extend(extractions),
                Err(AxiomError::LlmUnavailable(_)) => {}
                Err(error) => return Err(error),
            }
            enrichment.verifications.insert(
                context.episode_id.clone(),
                merge_verification_extractions(&verification_candidates),
            );
        }
        Ok(enrichment)
    }

    pub(crate) fn plan_replay_for_raw_events(
        &self,
        raw_events: &[RawEventRow],
    ) -> Result<ReplayPlan> {
        let projection = plan_projection(raw_events)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        let enrichment = self.collect_derivation_inputs(&contexts)?;
        plan_replay(
            raw_events,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub(crate) fn derive_contexts_after_purge(
        &self,
        raw_events: &[RawEventRow],
        connector: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<DerivationEnrichment> {
        let mut surviving_events = Vec::new();
        for event in raw_events {
            if !crate::logic::raw_event_matches_purge(event, connector, workspace_id)? {
                surviving_events.push(event.clone());
            }
        }
        let projection = plan_projection(&surviving_events)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        self.collect_derivation_inputs(&contexts)
    }

    pub(crate) fn derive_contexts_after_repair(
        &self,
        raw_events: &[RawEventRow],
        ingest: &IngestPlan,
    ) -> Result<DerivationEnrichment> {
        let mut combined = raw_events.to_vec();
        combined.extend(ingest.adds.iter().map(|event| event.row.clone()));
        let projection = plan_projection(&combined)?;
        let contexts = plan_derivation_contexts(
            &projection.conv_sessions,
            &projection.conv_turns,
            &projection.conv_items,
        );
        self.collect_derivation_inputs(&contexts)
    }
}
