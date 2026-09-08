use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::Arc;

const RADIX_BITS: usize = 4;
const RADIX_MASK: u64 = (1 << RADIX_BITS) - 1;
const RADIX_DEPTH: usize = u64::BITS as usize / RADIX_BITS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderObjectId(NonZeroU64);

impl RenderObjectId {
    fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    pub const fn raw(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RenderSceneRevision(u64);

impl RenderSceneRevision {
    pub const INITIAL: Self = Self(0);

    pub const fn raw(self) -> u64 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderSceneOperationKind {
    Insert,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderSceneOperation {
    object_id: RenderObjectId,
    kind: RenderSceneOperationKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderSceneUpdate {
    operations: Vec<RenderSceneOperation>,
}

impl RenderSceneUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, object_id: RenderObjectId) -> &mut Self {
        self.operations.push(RenderSceneOperation {
            object_id,
            kind: RenderSceneOperationKind::Insert,
        });
        self
    }

    pub fn remove(&mut self, object_id: RenderObjectId) -> &mut Self {
        self.operations.push(RenderSceneOperation {
            object_id,
            kind: RenderSceneOperationKind::Remove,
        });
        self
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSceneChangeSet {
    Incremental {
        inserted: Arc<[RenderObjectId]>,
        removed: Arc<[RenderObjectId]>,
    },
    FullResync,
}

impl RenderSceneChangeSet {
    fn incremental(inserted: Vec<RenderObjectId>, removed: Vec<RenderObjectId>) -> Self {
        Self::Incremental {
            inserted: Arc::from(inserted),
            removed: Arc::from(removed),
        }
    }

    pub fn inserted(&self) -> Option<&[RenderObjectId]> {
        match self {
            Self::Incremental { inserted, .. } => Some(inserted),
            Self::FullResync => None,
        }
    }

    pub fn removed(&self) -> Option<&[RenderObjectId]> {
        match self {
            Self::Incremental { removed, .. } => Some(removed),
            Self::FullResync => None,
        }
    }

    pub const fn is_full_resync(&self) -> bool {
        matches!(self, Self::FullResync)
    }

    pub fn is_empty_incremental(&self) -> bool {
        matches!(
            self,
            Self::Incremental { inserted, removed }
                if inserted.is_empty() && removed.is_empty()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MembershipNode {
    count: usize,
    terminal: bool,
    children: BTreeMap<u8, Arc<MembershipNode>>,
}

impl MembershipNode {
    fn empty() -> Self {
        Self {
            count: 0,
            terminal: false,
            children: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneMembership {
    root: Arc<MembershipNode>,
}

impl Default for SceneMembership {
    fn default() -> Self {
        Self {
            root: Arc::new(MembershipNode::empty()),
        }
    }
}

impl SceneMembership {
    fn len(&self) -> usize {
        self.root.count
    }

    fn is_empty(&self) -> bool {
        self.root.count == 0
    }

    fn contains(&self, object_id: RenderObjectId) -> bool {
        let mut node = self.root.as_ref();
        let raw = object_id.raw();
        for depth in 0..RADIX_DEPTH {
            let key = radix_digit(raw, depth);
            let Some(child) = node.children.get(&key) else {
                return false;
            };
            node = child.as_ref();
        }
        node.terminal
    }

    fn inserted(&self, object_id: RenderObjectId) -> (Self, usize) {
        debug_assert!(!self.contains(object_id));
        let (root, copied_nodes) = insert_node(&self.root, object_id.raw(), 0);
        (Self { root }, copied_nodes)
    }

    fn removed(&self, object_id: RenderObjectId) -> (Self, usize) {
        debug_assert!(self.contains(object_id));
        let (root, copied_nodes) = remove_node(&self.root, object_id.raw(), 0);
        (Self { root }, copied_nodes)
    }

    fn object_ids(&self) -> Vec<RenderObjectId> {
        let mut object_ids = Vec::with_capacity(self.len());
        collect_object_ids(&self.root, 0, 0, &mut object_ids);
        object_ids
    }
}

fn radix_digit(raw: u64, depth: usize) -> u8 {
    debug_assert!(depth < RADIX_DEPTH);
    let shift = (RADIX_DEPTH - depth - 1) * RADIX_BITS;
    ((raw >> shift) & RADIX_MASK) as u8
}

fn insert_node(node: &Arc<MembershipNode>, raw: u64, depth: usize) -> (Arc<MembershipNode>, usize) {
    let mut updated = node.as_ref().clone();
    updated.count += 1;

    if depth == RADIX_DEPTH {
        debug_assert!(!updated.terminal);
        updated.terminal = true;
        return (Arc::new(updated), 1);
    }

    let key = radix_digit(raw, depth);
    let child = node
        .children
        .get(&key)
        .cloned()
        .unwrap_or_else(|| Arc::new(MembershipNode::empty()));
    let (updated_child, copied_nodes) = insert_node(&child, raw, depth + 1);
    updated.children.insert(key, updated_child);
    (Arc::new(updated), copied_nodes + 1)
}

fn remove_node(node: &Arc<MembershipNode>, raw: u64, depth: usize) -> (Arc<MembershipNode>, usize) {
    let mut updated = node.as_ref().clone();
    updated.count -= 1;

    if depth == RADIX_DEPTH {
        debug_assert!(updated.terminal);
        updated.terminal = false;
        return (Arc::new(updated), 1);
    }

    let key = radix_digit(raw, depth);
    let child = node
        .children
        .get(&key)
        .expect("validated scene membership removal path must exist");
    let (updated_child, copied_nodes) = remove_node(child, raw, depth + 1);
    if updated_child.count == 0 {
        updated.children.remove(&key);
    } else {
        updated.children.insert(key, updated_child);
    }
    (Arc::new(updated), copied_nodes + 1)
}

fn collect_object_ids(
    node: &Arc<MembershipNode>,
    depth: usize,
    prefix: u64,
    output: &mut Vec<RenderObjectId>,
) {
    if depth == RADIX_DEPTH {
        if node.terminal {
            output.push(
                RenderObjectId::from_raw(prefix)
                    .expect("renderer scene membership must contain only non-zero object IDs"),
            );
        }
        return;
    }

    for (digit, child) in &node.children {
        collect_object_ids(
            child,
            depth + 1,
            (prefix << RADIX_BITS) | u64::from(*digit),
            output,
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSceneSnapshot {
    revision: RenderSceneRevision,
    membership: SceneMembership,
}

impl RenderSceneSnapshot {
    pub const fn revision(&self) -> RenderSceneRevision {
        self.revision
    }

    pub fn len(&self) -> usize {
        self.membership.len()
    }

    pub fn is_empty(&self) -> bool {
        self.membership.is_empty()
    }

    pub fn contains(&self, object_id: RenderObjectId) -> bool {
        self.membership.contains(object_id)
    }

    pub fn object_ids(&self) -> Vec<RenderObjectId> {
        self.membership.object_ids()
    }

    pub fn membership_eq(&self, other: &Self) -> bool {
        self.membership == other.membership
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSceneCommit {
    snapshot: RenderSceneSnapshot,
    change_set: RenderSceneChangeSet,
}

impl RenderSceneCommit {
    pub const fn revision(&self) -> RenderSceneRevision {
        self.snapshot.revision()
    }

    pub const fn snapshot(&self) -> &RenderSceneSnapshot {
        &self.snapshot
    }

    pub const fn change_set(&self) -> &RenderSceneChangeSet {
        &self.change_set
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSceneResync {
    snapshot: RenderSceneSnapshot,
    change_set: RenderSceneChangeSet,
}

impl RenderSceneResync {
    pub const fn snapshot(&self) -> &RenderSceneSnapshot {
        &self.snapshot
    }

    pub const fn change_set(&self) -> &RenderSceneChangeSet {
        &self.change_set
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderObjectIdAllocationError {
    Exhausted,
}

impl fmt::Display for RenderObjectIdAllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => write!(f, "RenderObjectId allocator exhausted"),
        }
    }
}

impl Error for RenderObjectIdAllocationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSceneCommitError {
    ConflictingOperations { object_id: RenderObjectId },
    ObjectAlreadyPresent { object_id: RenderObjectId },
    ObjectMissing { object_id: RenderObjectId },
    RevisionExhausted,
}

impl fmt::Display for RenderSceneCommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingOperations { object_id } => {
                write!(
                    f,
                    "RenderSceneUpdate contains conflicting operations for {object_id:?}"
                )
            }
            Self::ObjectAlreadyPresent { object_id } => {
                write!(f, "RenderObjectId {object_id:?} is already present")
            }
            Self::ObjectMissing { object_id } => {
                write!(f, "RenderObjectId {object_id:?} is not present")
            }
            Self::RevisionExhausted => write!(f, "RenderSceneRevision exhausted"),
        }
    }
}

impl Error for RenderSceneCommitError {}

#[derive(Debug)]
pub struct RenderSceneStore {
    revision: RenderSceneRevision,
    membership: SceneMembership,
    next_object_raw: u64,
}

impl Default for RenderSceneStore {
    fn default() -> Self {
        Self {
            revision: RenderSceneRevision::INITIAL,
            membership: SceneMembership::default(),
            next_object_raw: 1,
        }
    }
}

impl RenderSceneStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn revision(&self) -> RenderSceneRevision {
        self.revision
    }

    pub fn snapshot(&self) -> RenderSceneSnapshot {
        RenderSceneSnapshot {
            revision: self.revision,
            membership: self.membership.clone(),
        }
    }

    pub fn allocate_object_id(&mut self) -> Result<RenderObjectId, RenderObjectIdAllocationError> {
        if self.next_object_raw == u64::MAX {
            return Err(RenderObjectIdAllocationError::Exhausted);
        }

        let raw = self.next_object_raw;
        self.next_object_raw += 1;
        Ok(RenderObjectId::from_raw(raw).expect("renderer allocator never issues zero"))
    }

    pub fn commit(
        &mut self,
        update: RenderSceneUpdate,
    ) -> Result<RenderSceneCommit, RenderSceneCommitError> {
        let validated = self.validate_update(&update)?;
        if validated.inserted.is_empty() && validated.removed.is_empty() {
            return Ok(RenderSceneCommit {
                snapshot: self.snapshot(),
                change_set: RenderSceneChangeSet::incremental(Vec::new(), Vec::new()),
            });
        }

        let next_revision = self
            .revision
            .checked_next()
            .ok_or(RenderSceneCommitError::RevisionExhausted)?;

        let mut next_membership = self.membership.clone();
        for object_id in &validated.inserted {
            next_membership = next_membership.inserted(*object_id).0;
        }
        for object_id in &validated.removed {
            next_membership = next_membership.removed(*object_id).0;
        }

        self.membership = next_membership;
        self.revision = next_revision;

        Ok(RenderSceneCommit {
            snapshot: self.snapshot(),
            change_set: RenderSceneChangeSet::incremental(validated.inserted, validated.removed),
        })
    }

    pub fn full_resync(&self) -> RenderSceneResync {
        RenderSceneResync {
            snapshot: self.snapshot(),
            change_set: RenderSceneChangeSet::FullResync,
        }
    }

    fn validate_update(
        &self,
        update: &RenderSceneUpdate,
    ) -> Result<ValidatedRenderSceneUpdate, RenderSceneCommitError> {
        let mut normalized = BTreeMap::<RenderObjectId, RenderSceneOperationKind>::new();
        let mut conflicts = BTreeSet::<RenderObjectId>::new();

        for operation in &update.operations {
            if normalized
                .insert(operation.object_id, operation.kind)
                .is_some()
            {
                conflicts.insert(operation.object_id);
            }
        }

        if let Some(object_id) = conflicts.first().copied() {
            return Err(RenderSceneCommitError::ConflictingOperations { object_id });
        }

        let mut inserted = Vec::new();
        let mut removed = Vec::new();
        for (object_id, kind) in normalized {
            match kind {
                RenderSceneOperationKind::Insert => {
                    if self.membership.contains(object_id) {
                        return Err(RenderSceneCommitError::ObjectAlreadyPresent { object_id });
                    }
                    inserted.push(object_id);
                }
                RenderSceneOperationKind::Remove => {
                    if !self.membership.contains(object_id) {
                        return Err(RenderSceneCommitError::ObjectMissing { object_id });
                    }
                    removed.push(object_id);
                }
            }
        }

        Ok(ValidatedRenderSceneUpdate { inserted, removed })
    }
}

#[derive(Debug)]
struct ValidatedRenderSceneUpdate {
    inserted: Vec<RenderObjectId>,
    removed: Vec<RenderObjectId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_one(store: &mut RenderSceneStore, object_id: RenderObjectId) -> RenderSceneCommit {
        let mut update = RenderSceneUpdate::new();
        update.insert(object_id);
        store.commit(update).expect("single insert should commit")
    }

    #[test]
    fn empty_scene_has_defined_initial_revision() {
        let store = RenderSceneStore::new();
        let snapshot = store.snapshot();

        assert_eq!(snapshot.revision(), RenderSceneRevision::INITIAL);
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.object_ids(), Vec::<RenderObjectId>::new());
    }

    #[test]
    fn allocation_is_monotonic_non_reusing_and_does_not_advance_scene_revision() {
        let mut store = RenderSceneStore::new();
        let first = store
            .allocate_object_id()
            .expect("first ID should allocate");
        let second = store
            .allocate_object_id()
            .expect("second ID should allocate");

        assert_eq!(first.raw(), 1);
        assert_eq!(second.raw(), 2);
        assert_eq!(store.revision(), RenderSceneRevision::INITIAL);

        insert_one(&mut store, first);
        let mut remove = RenderSceneUpdate::new();
        remove.remove(first);
        store.commit(remove).expect("remove should commit");

        let third = store
            .allocate_object_id()
            .expect("third ID should allocate");
        assert_eq!(third.raw(), 3);
        assert_ne!(third, first);
    }

    #[test]
    fn insert_and_remove_publish_precise_structural_changes() {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("ID should allocate");

        let insert = insert_one(&mut store, object_id);
        assert_eq!(insert.revision().raw(), 1);
        assert!(insert.snapshot().contains(object_id));
        assert_eq!(insert.change_set().inserted(), Some(&[object_id][..]));
        assert_eq!(insert.change_set().removed(), Some(&[][..]));

        let mut remove_update = RenderSceneUpdate::new();
        remove_update.remove(object_id);
        let remove = store.commit(remove_update).expect("remove should commit");
        assert_eq!(remove.revision().raw(), 2);
        assert!(!remove.snapshot().contains(object_id));
        assert_eq!(remove.change_set().inserted(), Some(&[][..]));
        assert_eq!(remove.change_set().removed(), Some(&[object_id][..]));
    }

    #[test]
    fn duplicate_insert_rejects_without_publication() {
        let mut store = RenderSceneStore::new();
        let present = store.allocate_object_id().expect("ID should allocate");
        let absent = store.allocate_object_id().expect("ID should allocate");
        insert_one(&mut store, present);
        let before = store.snapshot();

        let mut update = RenderSceneUpdate::new();
        update.insert(absent).insert(present);
        assert_eq!(
            store.commit(update),
            Err(RenderSceneCommitError::ObjectAlreadyPresent { object_id: present })
        );
        assert_eq!(store.snapshot(), before);
        assert!(!store.snapshot().contains(absent));
    }

    #[test]
    fn missing_remove_rejects_without_publication() {
        let mut store = RenderSceneStore::new();
        let missing = store.allocate_object_id().expect("ID should allocate");
        let before = store.snapshot();

        let mut update = RenderSceneUpdate::new();
        update.remove(missing);
        assert_eq!(
            store.commit(update),
            Err(RenderSceneCommitError::ObjectMissing { object_id: missing })
        );
        assert_eq!(store.snapshot(), before);
    }

    #[test]
    fn same_object_multi_operation_conflict_rejects_before_membership_validation() {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("ID should allocate");

        let mut update = RenderSceneUpdate::new();
        update.insert(object_id).remove(object_id);
        assert_eq!(
            store.commit(update),
            Err(RenderSceneCommitError::ConflictingOperations { object_id })
        );
        assert_eq!(store.revision(), RenderSceneRevision::INITIAL);
        assert!(store.snapshot().is_empty());
    }

    #[test]
    fn empty_update_is_accepted_no_op() {
        let mut store = RenderSceneStore::new();
        let before = store.snapshot();
        let commit = store
            .commit(RenderSceneUpdate::new())
            .expect("empty update should be accepted");

        assert_eq!(commit.snapshot(), &before);
        assert_eq!(commit.revision(), RenderSceneRevision::INITIAL);
        assert!(commit.change_set().is_empty_incremental());
        assert_eq!(store.snapshot(), before);
    }

    #[test]
    fn multi_operation_commit_advances_revision_once() {
        let mut store = RenderSceneStore::new();
        let first = store.allocate_object_id().expect("ID should allocate");
        let second = store.allocate_object_id().expect("ID should allocate");
        let third = store.allocate_object_id().expect("ID should allocate");
        insert_one(&mut store, first);

        let mut update = RenderSceneUpdate::new();
        update.remove(first).insert(second).insert(third);
        let commit = store
            .commit(update)
            .expect("multi-operation update should commit");

        assert_eq!(commit.revision().raw(), 2);
        assert_eq!(commit.snapshot().object_ids(), vec![second, third]);
        assert_eq!(commit.change_set().inserted(), Some(&[second, third][..]));
        assert_eq!(commit.change_set().removed(), Some(&[first][..]));
    }

    #[test]
    fn retained_snapshot_remains_immutable_after_later_commits() {
        let mut store = RenderSceneStore::new();
        let first = store.allocate_object_id().expect("ID should allocate");
        let second = store.allocate_object_id().expect("ID should allocate");
        let first_commit = insert_one(&mut store, first);
        let retained = first_commit.snapshot().clone();

        insert_one(&mut store, second);

        assert_eq!(retained.revision().raw(), 1);
        assert_eq!(retained.object_ids(), vec![first]);
        assert_eq!(store.snapshot().object_ids(), vec![first, second]);
    }

    #[test]
    fn full_and_incremental_construction_are_membership_equivalent() {
        let mut full = RenderSceneStore::new();
        let full_ids = [
            full.allocate_object_id().expect("ID should allocate"),
            full.allocate_object_id().expect("ID should allocate"),
            full.allocate_object_id().expect("ID should allocate"),
        ];
        let mut full_update = RenderSceneUpdate::new();
        for object_id in full_ids {
            full_update.insert(object_id);
        }
        full.commit(full_update)
            .expect("full construction should commit");

        let mut incremental = RenderSceneStore::new();
        let incremental_ids = [
            incremental
                .allocate_object_id()
                .expect("ID should allocate"),
            incremental
                .allocate_object_id()
                .expect("ID should allocate"),
            incremental
                .allocate_object_id()
                .expect("ID should allocate"),
        ];
        for object_id in incremental_ids {
            insert_one(&mut incremental, object_id);
        }

        assert!(full.snapshot().membership_eq(&incremental.snapshot()));
        assert_eq!(
            full.snapshot().object_ids(),
            incremental.snapshot().object_ids()
        );
        assert_ne!(full.revision(), incremental.revision());
    }

    #[test]
    fn full_resync_is_explicit_and_does_not_advance_revision() {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("ID should allocate");
        insert_one(&mut store, object_id);
        let revision = store.revision();

        let resync = store.full_resync();

        assert!(resync.change_set().is_full_resync());
        assert_eq!(resync.snapshot().revision(), revision);
        assert_eq!(resync.snapshot().object_ids(), vec![object_id]);
        assert_eq!(store.revision(), revision);
    }

    #[test]
    fn small_membership_change_path_copy_is_bounded_by_id_width() {
        let mut store = RenderSceneStore::new();
        for _ in 0..4096 {
            let object_id = store.allocate_object_id().expect("ID should allocate");
            insert_one(&mut store, object_id);
        }

        let next = store.allocate_object_id().expect("ID should allocate");
        let (_, insert_copies) = store.membership.inserted(next);
        assert_eq!(insert_copies, RADIX_DEPTH + 1);

        let existing = store.snapshot().object_ids()[2048];
        let (_, remove_copies) = store.membership.removed(existing);
        assert_eq!(remove_copies, RADIX_DEPTH + 1);
    }

    #[test]
    fn allocation_exhaustion_is_explicit_and_does_not_wrap() {
        let mut store = RenderSceneStore {
            next_object_raw: u64::MAX,
            ..RenderSceneStore::default()
        };

        assert_eq!(
            store.allocate_object_id(),
            Err(RenderObjectIdAllocationError::Exhausted)
        );
        assert_eq!(store.revision(), RenderSceneRevision::INITIAL);
    }

    #[test]
    fn revision_exhaustion_rejects_without_mutating_membership() {
        let mut store = RenderSceneStore {
            revision: RenderSceneRevision(u64::MAX),
            ..RenderSceneStore::default()
        };
        let object_id = store.allocate_object_id().expect("ID should allocate");
        let before = store.snapshot();
        let mut update = RenderSceneUpdate::new();
        update.insert(object_id);

        assert_eq!(
            store.commit(update),
            Err(RenderSceneCommitError::RevisionExhausted)
        );
        assert_eq!(store.snapshot(), before);
    }
}
