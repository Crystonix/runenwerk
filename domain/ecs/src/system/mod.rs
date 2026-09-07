mod extract;
mod param_metadata;
mod params;
mod plan_report;
mod runtime;

pub use extract::{SystemParam, SystemParamContext, SystemParamError};
pub use param_metadata::{ParamSlotId, ParamSlotMetadata};
pub use params::{Res, ResMut, ResView};
pub use plan_report::{
    RuntimePlanBarrierReport, RuntimePlanConflictReport, RuntimePlanDiagnosticReport,
    RuntimePlanPhaseReport, RuntimePlanReport, RuntimePlanStageReport, RuntimePlanSystemReport,
    RuntimePlanWaveReport,
};
pub use runtime::{
    ConfiguredSystem, IntoSystem, IntoSystemConfigs, IntoSystemSetKey, Runtime, SystemConfigExt,
};
pub use scheduler::system::{ParamSlotDescriptor, SystemId};
