extern crate self as ecs;

mod bundle;
mod commands;
mod component;
mod entity;
mod errors;
pub mod prelude;
pub mod query;
pub mod reflect;
mod storage;
pub mod system;
pub mod telemetry;
mod world;
pub use bundle::Bundle;
#[doc(hidden)]
pub use bundle::{BundleComponentDescriptor, BundleComponents};
pub use commands::{BatchCommands, Commands, DeferredCommand};
pub use component::{Component, ComponentState, Resource, StatefulComponent};
pub use ecs_macros::{Bundle, Component, Reflect, Resource, StatefulComponent, SystemParam};
pub use entity::{Entity, EntityAllocator};
pub use errors::{CommandError, EntityAllocationError, EntityError, QueryError, ResourceError};
pub use query::{
    Added, Changed, Orphaned, Query, QueryAccess, QueryOrphaned, QueryOrphanedState, QueryState,
    QueryTypeAccess, With, Without, query_snapshot_source_generation,
};
pub use reflect::{
    EnumInfo, EnumVariantInfo, FieldInfo, Reflect, ReflectShape, ReflectValueMut, ReflectValueRef,
    StructInfo, StructValueMut, StructValueRef, TypeInfo, TypeRegistry,
};
pub use system::{
    ConfiguredSystem, IntoSystem, IntoSystemConfigs, IntoSystemSetKey, ParamSlotDescriptor,
    ParamSlotId, ParamSlotMetadata, Res, ResMut, ResView, Runtime, RuntimePlanBarrierReport,
    RuntimePlanConflictReport, RuntimePlanDiagnosticReport, RuntimePlanPhaseReport,
    RuntimePlanReport, RuntimePlanStageReport, RuntimePlanSystemReport, RuntimePlanWaveReport,
    SystemConfigExt, SystemId, SystemParam, SystemParamContext, SystemParamError,
};
pub use world::{
    ComponentChangeKind, ComponentChangeRecord, ComponentTypeKey, EntityMut, EntityRef, Mut,
    ResourceChangeKind, ResourceChangeRecord, ResourceTypeKey, World,
};
