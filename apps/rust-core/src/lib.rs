pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";
pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";
pub const MODEL_REGISTRY_METADATA_SCHEMA_VERSION: &str = "model_registry_metadata.v0";
pub const MODEL_REGISTRY_METADATA_SCOPE: &str = "local_synthetic_model_registry_metadata";
pub const MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION: &str = "model_evaluation_bundle.v0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeIdError {
    Empty,
    TooLong,
    InvalidPrefix,
    InvalidCharacter,
    RawIdentifier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionId(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobId(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobKind {
    CompareModelScores,
    RefreshEvidenceIndex,
    RunNativeInferenceCandidate,
    RenderWorkstationSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeInferenceRuntimeState {
    Unavailable,
    Available,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelRegistryState {
    ObservedSyntheticOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPromotionState {
    NotPromoted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeSummary {
    pub schema_version: &'static str,
    pub workspace_id: WorkspaceId,
    pub session_id: SessionId,
    pub total_job_count: u32,
    pub queued_job_count: u32,
    pub running_job_count: u32,
    pub failed_job_count: u32,
    pub last_event_label: &'static str,
    pub native_inference_state: NativeInferenceRuntimeState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistryMetadata {
    pub schema_version: &'static str,
    pub metadata_scope: &'static str,
    pub source_bundle_schema: &'static str,
    pub entries: &'static [ModelRegistryEntry],
    pub aggregate_summary: ModelRegistryAggregateSummary,
    pub safety_flags: ModelRegistrySafetyFlags,
    pub non_claims: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistryEntry {
    pub model_id: &'static str,
    pub registry_state: ModelRegistryState,
    pub promotion_state: ModelPromotionState,
    pub observed_source_schemas: &'static [&'static str],
    pub observed_source_names: &'static [&'static str],
    pub source_count: u32,
    pub has_score_rows: bool,
    pub human_review_required: bool,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistryAggregateSummary {
    pub model_count: u32,
    pub schemas_present: &'static [&'static str],
    pub models_with_score_rows: &'static [&'static str],
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistrySafetyFlags {
    pub local_only: bool,
    pub strict_json_loaded: bool,
    pub derived_from_evaluation_bundle_only: bool,
    pub input_paths_copied: bool,
    pub source_filenames_copied: bool,
    pub raw_identifiers_copied: bool,
    pub generated_artifact_references_copied: bool,
    pub secrets_detected: bool,
    pub report_payload_copied: bool,
    pub live_capture_used: bool,
    pub external_services_used: bool,
    pub deployment_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    WorkspaceOpened { workspace_id: WorkspaceId },
    SessionStarted {
        workspace_id: WorkspaceId,
        session_id: SessionId,
    },
    JobQueued {
        session_id: SessionId,
        job_id: JobId,
        kind: JobKind,
    },
    JobStateChanged { job_id: JobId, state: JobState },
}

impl WorkspaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["workspace-", "fixture-workspace-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["session-", "fixture-session-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl JobId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeIdError> {
        let value = value.into();
        validate_coarse_id(&value, &["job-", "fixture-job-"])?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RuntimeEvent {
    pub fn contract_summary() -> &'static str {
        "ARES local runtime boundary contract v0"
    }
}

impl NativeInferenceRuntimeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Available => "available",
            Self::Disabled => "disabled",
        }
    }
}

impl ModelRegistryState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ObservedSyntheticOnly => "observed_synthetic_only",
        }
    }
}

impl ModelPromotionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotPromoted => "not_promoted",
        }
    }
}

impl RuntimeSummary {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: RUNTIME_SUMMARY_SCHEMA_VERSION,
            workspace_id: WorkspaceId::new("fixture-workspace-alpha")
                .expect("static fixture workspace id must be valid"),
            session_id: SessionId::new("fixture-session-runtime-summary")
                .expect("static fixture session id must be valid"),
            total_job_count: 4,
            queued_job_count: 1,
            running_job_count: 1,
            failed_job_count: 0,
            last_event_label: "synthetic workstation snapshot rendered",
            native_inference_state: NativeInferenceRuntimeState::Disabled,
        }
    }
}

impl ModelRegistryMetadata {
    pub fn synthetic_fixture() -> Self {
        Self {
            schema_version: MODEL_REGISTRY_METADATA_SCHEMA_VERSION,
            metadata_scope: MODEL_REGISTRY_METADATA_SCOPE,
            source_bundle_schema: MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION,
            entries: MODEL_REGISTRY_METADATA_ENTRIES,
            aggregate_summary: ModelRegistryAggregateSummary {
                model_count: 4,
                schemas_present: MODEL_REGISTRY_AGGREGATE_SCHEMAS,
                models_with_score_rows: MODEL_REGISTRY_MODELS_WITH_SCORE_ROWS,
                deployment_allowed: false,
            },
            safety_flags: ModelRegistrySafetyFlags {
                local_only: true,
                strict_json_loaded: true,
                derived_from_evaluation_bundle_only: true,
                input_paths_copied: false,
                source_filenames_copied: false,
                raw_identifiers_copied: false,
                generated_artifact_references_copied: false,
                secrets_detected: false,
                report_payload_copied: false,
                live_capture_used: false,
                external_services_used: false,
                deployment_allowed: false,
            },
            non_claims: MODEL_REGISTRY_NON_CLAIMS,
        }
    }
}

const MODEL_REGISTRY_SCORE_SCHEMAS: &[&str] =
    &["model_disagreement_report.v0", "model_score_rows.v0"];
const MODEL_REGISTRY_SCORE_SOURCE_NAMES: &[&str] =
    &["model_disagreement_report_v0_001", "model_score_rows_v0_001"];
const MODEL_REGISTRY_INVESTIGATION_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
];
const MODEL_REGISTRY_INVESTIGATION_SOURCE_NAMES: &[&str] = &[
    "agentic_investigation_report_v0_001",
    "detection_candidate_report_v0_001",
];
const MODEL_REGISTRY_NATIVE_SCORE_SCHEMAS: &[&str] = &["model_score_rows.v0"];
const MODEL_REGISTRY_NATIVE_SCORE_SOURCE_NAMES: &[&str] = &["model_score_rows_v0_001"];
const MODEL_REGISTRY_AGGREGATE_SCHEMAS: &[&str] = &[
    "agentic_investigation_report.v0",
    "detection_candidate_report.v0",
    "model_disagreement_report.v0",
    "model_score_rows.v0",
];
const MODEL_REGISTRY_MODELS_WITH_SCORE_ROWS: &[&str] =
    &["isolation_forest", "pyod_ecod", "stdlib_linear_native"];
const MODEL_REGISTRY_NON_CLAIMS: &[&str] = &[
    "not_persistent_model_registry",
    "not_model_promotion_gate",
    "not_deployment_approval",
    "not_live_capture",
    "not_external_enrichment",
    "not_rule_deployment",
    "not_native_runtime_execution",
];
const MODEL_REGISTRY_METADATA_ENTRIES: &[ModelRegistryEntry] = &[
    ModelRegistryEntry {
        model_id: "isolation_forest",
        registry_state: ModelRegistryState::ObservedSyntheticOnly,
        promotion_state: ModelPromotionState::NotPromoted,
        observed_source_schemas: MODEL_REGISTRY_SCORE_SCHEMAS,
        observed_source_names: MODEL_REGISTRY_SCORE_SOURCE_NAMES,
        source_count: 2,
        has_score_rows: true,
        human_review_required: true,
        deployment_allowed: false,
    },
    ModelRegistryEntry {
        model_id: "model_disagreement",
        registry_state: ModelRegistryState::ObservedSyntheticOnly,
        promotion_state: ModelPromotionState::NotPromoted,
        observed_source_schemas: MODEL_REGISTRY_INVESTIGATION_SCHEMAS,
        observed_source_names: MODEL_REGISTRY_INVESTIGATION_SOURCE_NAMES,
        source_count: 2,
        has_score_rows: false,
        human_review_required: true,
        deployment_allowed: false,
    },
    ModelRegistryEntry {
        model_id: "pyod_ecod",
        registry_state: ModelRegistryState::ObservedSyntheticOnly,
        promotion_state: ModelPromotionState::NotPromoted,
        observed_source_schemas: MODEL_REGISTRY_SCORE_SCHEMAS,
        observed_source_names: MODEL_REGISTRY_SCORE_SOURCE_NAMES,
        source_count: 2,
        has_score_rows: true,
        human_review_required: true,
        deployment_allowed: false,
    },
    ModelRegistryEntry {
        model_id: "stdlib_linear_native",
        registry_state: ModelRegistryState::ObservedSyntheticOnly,
        promotion_state: ModelPromotionState::NotPromoted,
        observed_source_schemas: MODEL_REGISTRY_NATIVE_SCORE_SCHEMAS,
        observed_source_names: MODEL_REGISTRY_NATIVE_SCORE_SOURCE_NAMES,
        source_count: 1,
        has_score_rows: true,
        human_review_required: true,
        deployment_allowed: false,
    },
];

fn validate_coarse_id(value: &str, allowed_prefixes: &[&str]) -> Result<(), RuntimeIdError> {
    if value.is_empty() {
        return Err(RuntimeIdError::Empty);
    }
    if value.len() > 96 {
        return Err(RuntimeIdError::TooLong);
    }
    if !allowed_prefixes
        .iter()
        .any(|allowed_prefix| value.starts_with(allowed_prefix))
    {
        return Err(RuntimeIdError::InvalidPrefix);
    }
    if value.contains('.') || value.contains(':') || value.contains('@') {
        return Err(RuntimeIdError::RawIdentifier);
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_')
    {
        return Err(RuntimeIdError::InvalidCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_coarse_runtime_identifiers() {
        let workspace = WorkspaceId::new("workspace-alpha").unwrap();
        let session = SessionId::new("session-alpha").unwrap();
        let job = JobId::new("job-alpha").unwrap();

        assert_eq!(workspace.as_str(), "workspace-alpha");
        assert_eq!(session.as_str(), "session-alpha");
        assert_eq!(job.as_str(), "job-alpha");
    }

    #[test]
    fn rejects_raw_identifier_shapes() {
        assert_eq!(
            WorkspaceId::new("workspace-alpha.raw").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
        assert_eq!(
            SessionId::new("session-alpha:01").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
        assert_eq!(
            JobId::new("job-alpha@example").unwrap_err(),
            RuntimeIdError::RawIdentifier
        );
    }

    #[test]
    fn emits_static_runtime_summary_fixture() {
        let summary = RuntimeSummary::synthetic_fixture();

        assert_eq!(summary.schema_version, RUNTIME_SUMMARY_SCHEMA_VERSION);
        assert_eq!(summary.workspace_id.as_str(), "fixture-workspace-alpha");
        assert_eq!(summary.session_id.as_str(), "fixture-session-runtime-summary");
        assert_eq!(summary.total_job_count, 4);
        assert_eq!(summary.queued_job_count, 1);
        assert_eq!(summary.running_job_count, 1);
        assert_eq!(summary.failed_job_count, 0);
        assert_eq!(summary.last_event_label, "synthetic workstation snapshot rendered");
        assert_eq!(summary.native_inference_state.as_str(), "disabled");
    }

    #[test]
    fn emits_static_model_registry_metadata_fixture() {
        let metadata = ModelRegistryMetadata::synthetic_fixture();

        assert_eq!(metadata.schema_version, MODEL_REGISTRY_METADATA_SCHEMA_VERSION);
        assert_eq!(metadata.metadata_scope, MODEL_REGISTRY_METADATA_SCOPE);
        assert_eq!(
            metadata.source_bundle_schema,
            MODEL_REGISTRY_SOURCE_BUNDLE_SCHEMA_VERSION
        );
        assert_eq!(metadata.entries.len(), 4);
        assert_eq!(metadata.entries[0].model_id, "isolation_forest");
        assert_eq!(
            metadata.entries[0].registry_state.as_str(),
            "observed_synthetic_only"
        );
        assert_eq!(metadata.entries[0].promotion_state.as_str(), "not_promoted");
        assert_eq!(metadata.entries[0].source_count, 2);
        assert!(metadata.entries[0].has_score_rows);
        assert!(metadata.entries[0].human_review_required);
        assert!(!metadata.entries[0].deployment_allowed);
        assert_eq!(metadata.entries[3].model_id, "stdlib_linear_native");
        assert_eq!(metadata.aggregate_summary.model_count, 4);
        assert_eq!(
            metadata.aggregate_summary.models_with_score_rows,
            &["isolation_forest", "pyod_ecod", "stdlib_linear_native"]
        );
        assert!(!metadata.aggregate_summary.deployment_allowed);
        assert!(metadata.safety_flags.local_only);
        assert!(metadata.safety_flags.strict_json_loaded);
        assert!(metadata.safety_flags.derived_from_evaluation_bundle_only);
        assert!(!metadata.safety_flags.deployment_allowed);
        assert_eq!(
            metadata.non_claims,
            &[
                "not_persistent_model_registry",
                "not_model_promotion_gate",
                "not_deployment_approval",
                "not_live_capture",
                "not_external_enrichment",
                "not_rule_deployment",
                "not_native_runtime_execution"
            ]
        );
    }
}
