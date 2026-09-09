use crate::World;
use crate::scheduler::access::{AccessConflict, SystemAccess};
use crate::scheduler::label::{ScheduleKey, ScheduleLabel, SystemSet, SystemSetKey};
use anyhow::Result;

pub type RunnableSystemFn = Box<dyn FnMut(&mut World) -> Result<()>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SystemId(u64);

impl SystemId {
    pub const fn unassigned() -> Self {
        Self(u64::MAX)
    }

    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSlotDescriptor {
    pub name: Option<&'static str>,
    pub kind: &'static str,
    pub label: &'static str,
    pub type_name: &'static str,
    pub children: Vec<ParamSlotDescriptor>,
}

impl ParamSlotDescriptor {
    pub fn leaf(kind: &'static str, label: &'static str, type_name: &'static str) -> Self {
        Self {
            name: None,
            kind,
            label,
            type_name,
            children: Vec::new(),
        }
    }

    pub fn group(
        kind: &'static str,
        label: &'static str,
        type_name: &'static str,
        children: Vec<ParamSlotDescriptor>,
    ) -> Self {
        Self {
            name: None,
            kind,
            label,
            type_name,
            children,
        }
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    pub fn named_child(name: &'static str, descriptor: ParamSlotDescriptor) -> Self {
        descriptor.with_name(name)
    }
}

pub struct RegisteredSystem {
    id: SystemId,
    name: String,
    label: ScheduleKey,
    sets: Vec<SystemSetKey>,
    before_sets: Vec<SystemSetKey>,
    after_sets: Vec<SystemSetKey>,
    param_slots: Vec<ParamSlotDescriptor>,
    access: SystemAccess,
    run: RunnableSystemFn,
}

impl RegisteredSystem {
    pub fn new<L>(
        name: impl Into<String>,
        access: SystemAccess,
        run: impl FnMut(&mut World) -> Result<()> + 'static,
    ) -> Result<Self>
    where
        L: ScheduleLabel,
    {
        let name = name.into();
        access
            .validate_internal()
            .map_err(|conflict| internal_access_error(&name, &conflict))?;
        Ok(Self {
            id: SystemId::unassigned(),
            name,
            label: L::key(),
            sets: Vec::new(),
            before_sets: Vec::new(),
            after_sets: Vec::new(),
            param_slots: Vec::new(),
            access,
            run: Box::new(run),
        })
    }

    pub fn with_set<S: SystemSet>(mut self) -> Self {
        self.with_set_key(S::key());
        self
    }

    pub fn with_set_key(&mut self, key: SystemSetKey) -> &mut Self {
        if !self.sets.contains(&key) {
            self.sets.push(key);
        }
        self
    }

    pub fn before_set<S: SystemSet>(mut self) -> Self {
        self.before_set_key(S::key());
        self
    }

    pub fn before_set_key(&mut self, key: SystemSetKey) -> &mut Self {
        if !self.before_sets.contains(&key) {
            self.before_sets.push(key);
        }
        self
    }

    pub fn after_set<S: SystemSet>(mut self) -> Self {
        self.after_set_key(S::key());
        self
    }

    pub fn after_set_key(&mut self, key: SystemSetKey) -> &mut Self {
        if !self.after_sets.contains(&key) {
            self.after_sets.push(key);
        }
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn label(&self) -> ScheduleKey {
        self.label
    }

    pub fn sets(&self) -> &[SystemSetKey] {
        &self.sets
    }

    pub fn before_sets(&self) -> &[SystemSetKey] {
        &self.before_sets
    }

    pub fn after_sets(&self) -> &[SystemSetKey] {
        &self.after_sets
    }

    pub fn access(&self) -> &SystemAccess {
        &self.access
    }

    pub fn set_param_slots(&mut self, param_slots: Vec<ParamSlotDescriptor>) {
        self.param_slots = param_slots;
    }

    pub fn param_slots(&self) -> &[ParamSlotDescriptor] {
        &self.param_slots
    }

    pub fn run(&mut self, world: &mut World) -> Result<()> {
        (self.run)(world)
    }

    pub(crate) fn assign_id(&mut self, id: SystemId) {
        self.id = id;
    }
}

fn internal_access_error(system_name: &str, conflict: &AccessConflict) -> anyhow::Error {
    anyhow::anyhow!(
        "system '{system_name}' has conflicting access: {}",
        conflict.diagnostic_message()
    )
}
