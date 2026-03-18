mod baseline;

pub use baseline::{
    BaselineHarnessRequest, BaselineHarnessRun, BaselineHarnessRunner, GeneratedArtifact,
    HarnessRunError, ValidationCheck, ValidationExecution, ValidationExecutor,
};

pub use crate::domain::{
    ArtifactEntrypoint, ArtifactEntrypointRole, ArtifactGroup, ArtifactIndex, ArtifactKind,
    ArtifactRef, CheckOutcome, CheckResult, CodexOutputFormat, CodexOutputRef, HarnessReplayRecord,
    HarnessResult, HarnessStatus, LogRef, LogStream, ValidationResultRef,
};
