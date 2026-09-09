use crate::scheduler::system::{ParamSlotDescriptor, SystemId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamSlotId {
    pub system_id: SystemId,
    pub path: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSlotMetadata {
    pub id: ParamSlotId,
    pub name: Option<&'static str>,
    pub kind: &'static str,
    pub label: &'static str,
    pub type_name: &'static str,
    pub children: Vec<ParamSlotMetadata>,
}

pub(crate) fn param_slot_metadata_for_descriptors(
    system_id: SystemId,
    descriptors: &[ParamSlotDescriptor],
) -> Vec<ParamSlotMetadata> {
    descriptors
        .iter()
        .enumerate()
        .map(|(slot_index, descriptor)| {
            let path = vec![slot_index];
            param_slot_metadata_for_descriptor(system_id, path, descriptor)
        })
        .collect()
}

fn param_slot_metadata_for_descriptor(
    system_id: SystemId,
    path: Vec<usize>,
    descriptor: &ParamSlotDescriptor,
) -> ParamSlotMetadata {
    let children = descriptor
        .children
        .iter()
        .enumerate()
        .map(|(child_index, child)| {
            let mut child_path = path.clone();
            child_path.push(child_index);
            param_slot_metadata_for_descriptor(system_id, child_path, child)
        })
        .collect();

    ParamSlotMetadata {
        id: ParamSlotId { system_id, path },
        name: descriptor.name,
        kind: descriptor.kind,
        label: descriptor.label,
        type_name: descriptor.type_name,
        children,
    }
}
