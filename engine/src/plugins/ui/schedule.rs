use crate::runtime::IntoSystemSetKey;
use runen_ecs::SystemSetKey;

/// Stable system labels reserved for the engine-owned UI runtime.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UiRuntimeSet {
    Foundation,
    Report,
    RenderPublication,
}

impl IntoSystemSetKey for UiRuntimeSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::Foundation => SystemSetKey::of::<UiRuntimeSet>("UiRuntimeSet::Foundation"),
            Self::Report => SystemSetKey::of::<UiRuntimeSet>("UiRuntimeSet::Report"),
            Self::RenderPublication => {
                SystemSetKey::of::<UiRuntimeSet>("UiRuntimeSet::RenderPublication")
            }
        }
    }
}
