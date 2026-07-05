pub const RUNTIME_CONTRACT_VERSION: &str = "rust_runtime_contract.v0";
pub const RUNTIME_SUMMARY_SCHEMA_VERSION: &str = "runtime_summary.v0";

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
}
