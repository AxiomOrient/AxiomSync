use super::*;

impl AxiomSync {
    pub fn load_existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>> {
        self.repo.existing_raw_event_keys()
    }

    pub fn load_raw_events(&self) -> Result<Vec<RawEventRow>> {
        self.repo.load_raw_events()
    }

    pub fn load_derivation_inputs(&self) -> Result<DerivationInputs> {
        Ok(DerivationInputs {
            sessions: self.repo.load_sessions()?,
            turns: self.repo.load_turns()?,
            items: self.repo.load_items()?,
            anchors: self.repo.load_evidence_anchors()?,
        })
    }

    pub fn derivation_inputs_from_projection(
        &self,
        projection: &ProjectionPlan,
    ) -> DerivationInputs {
        DerivationInputs {
            sessions: projection.conv_sessions.clone(),
            turns: projection.conv_turns.clone(),
            items: projection.conv_items.clone(),
            anchors: projection.evidence_anchors.clone(),
        }
    }

    pub fn plan_ingest(
        &self,
        existing: &[ExistingRawEventKey],
        input: &ConnectorBatchInput,
    ) -> Result<IngestPlan> {
        plan_ingest(existing, input)
    }

    pub fn apply_ingest(&self, plan: &IngestPlan) -> Result<Value> {
        self.repo.apply_ingest_tx(plan)
    }

    pub fn plan_projection(&self, raw_events: &[RawEventRow]) -> Result<ProjectionPlan> {
        plan_projection(raw_events)
    }

    pub fn apply_projection(&self, plan: &ProjectionPlan) -> Result<Value> {
        self.repo.apply_projection_tx(plan)
    }

    pub fn plan_derivation_contexts(&self, inputs: &DerivationInputs) -> Vec<DerivationContext> {
        plan_derivation_contexts(&inputs.sessions, &inputs.turns, &inputs.items)
    }

    pub fn collect_derivation_enrichment(
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

    pub fn plan_derivation(
        &self,
        inputs: &DerivationInputs,
        enrichment: &DerivationEnrichment,
    ) -> Result<DerivePlan> {
        let contexts = self.plan_derivation_contexts(inputs);
        plan_derivation(
            &contexts,
            &inputs.turns,
            &inputs.items,
            &inputs.anchors,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_derivation(&self, plan: &DerivePlan) -> Result<Value> {
        self.repo.apply_derivation_tx(plan)
    }

    pub fn plan_replay(
        &self,
        raw_events: &[RawEventRow],
        enrichment: &DerivationEnrichment,
    ) -> Result<ReplayPlan> {
        plan_replay(
            raw_events,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_replay(&self, plan: &ReplayPlan) -> Result<Value> {
        self.repo.apply_replay_tx(plan)
    }

    pub fn plan_purge(
        &self,
        raw_events: &[RawEventRow],
        connector: Option<&str>,
        workspace_id: Option<&str>,
        enrichment: &DerivationEnrichment,
    ) -> Result<PurgePlan> {
        plan_purge(
            raw_events,
            connector,
            workspace_id,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_purge(&self, plan: &PurgePlan) -> Result<Value> {
        self.repo.apply_purge_tx(plan)
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        self.repo.doctor_report()
    }

    pub fn plan_repair(
        &self,
        raw_events: &[RawEventRow],
        ingest: &IngestPlan,
        enrichment: &DerivationEnrichment,
    ) -> Result<RepairPlan> {
        plan_repair(
            raw_events,
            ingest,
            &enrichment.extractions,
            &enrichment.verifications,
        )
    }

    pub fn apply_repair(&self, plan: &RepairPlan) -> Result<Value> {
        self.repo.apply_repair_tx(plan)
    }
}
