#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EcsTelemetrySnapshot {
    pub query_matching_calls: u64,
    pub query_matching_nanos: u64,
    pub query_matching_candidates: u64,
    pub query_matching_matches: u64,
    pub query_iter_calls: u64,
    pub query_iter_nanos: u64,
    pub query_get_calls: u64,
    pub query_get_nanos: u64,
    pub query_single_calls: u64,
    pub query_single_nanos: u64,
    pub changed_check_calls: u64,
    pub changed_check_nanos: u64,
    pub added_check_calls: u64,
    pub added_check_nanos: u64,
    pub runtime_plan_calls: u64,
    pub runtime_plan_nanos: u64,
    pub runtime_stage_calls: u64,
    pub runtime_stage_nanos: u64,
    pub runtime_flush_calls: u64,
    pub runtime_flush_nanos: u64,
    pub runtime_flush_command_queues: u64,
    pub schedule_plan_build_calls: u64,
    pub schedule_plan_build_nanos: u64,
    pub schedule_plan_conflict_checks: u64,
    pub schedule_plan_stage_count: u64,
}

#[cfg(feature = "telemetry")]
mod imp {
    use super::EcsTelemetrySnapshot;
    use std::sync::atomic::{AtomicU64, Ordering};

    static QUERY_MATCHING_CALLS: AtomicU64 = AtomicU64::new(0);
    static QUERY_MATCHING_NANOS: AtomicU64 = AtomicU64::new(0);
    static QUERY_MATCHING_CANDIDATES: AtomicU64 = AtomicU64::new(0);
    static QUERY_MATCHING_MATCHES: AtomicU64 = AtomicU64::new(0);
    static QUERY_ITER_CALLS: AtomicU64 = AtomicU64::new(0);
    static QUERY_ITER_NANOS: AtomicU64 = AtomicU64::new(0);
    static QUERY_GET_CALLS: AtomicU64 = AtomicU64::new(0);
    static QUERY_GET_NANOS: AtomicU64 = AtomicU64::new(0);
    static QUERY_SINGLE_CALLS: AtomicU64 = AtomicU64::new(0);
    static QUERY_SINGLE_NANOS: AtomicU64 = AtomicU64::new(0);
    static CHANGED_CHECK_CALLS: AtomicU64 = AtomicU64::new(0);
    static CHANGED_CHECK_NANOS: AtomicU64 = AtomicU64::new(0);
    static ADDED_CHECK_CALLS: AtomicU64 = AtomicU64::new(0);
    static ADDED_CHECK_NANOS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_PLAN_CALLS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_PLAN_NANOS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_STAGE_CALLS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_STAGE_NANOS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_FLUSH_CALLS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_FLUSH_NANOS: AtomicU64 = AtomicU64::new(0);
    static RUNTIME_FLUSH_COMMAND_QUEUES: AtomicU64 = AtomicU64::new(0);
    static SCHEDULE_PLAN_BUILD_CALLS: AtomicU64 = AtomicU64::new(0);
    static SCHEDULE_PLAN_BUILD_NANOS: AtomicU64 = AtomicU64::new(0);
    static SCHEDULE_PLAN_CONFLICT_CHECKS: AtomicU64 = AtomicU64::new(0);
    static SCHEDULE_PLAN_STAGE_COUNT: AtomicU64 = AtomicU64::new(0);

    pub fn reset() {
        QUERY_MATCHING_CALLS.store(0, Ordering::Relaxed);
        QUERY_MATCHING_NANOS.store(0, Ordering::Relaxed);
        QUERY_MATCHING_CANDIDATES.store(0, Ordering::Relaxed);
        QUERY_MATCHING_MATCHES.store(0, Ordering::Relaxed);
        QUERY_ITER_CALLS.store(0, Ordering::Relaxed);
        QUERY_ITER_NANOS.store(0, Ordering::Relaxed);
        QUERY_GET_CALLS.store(0, Ordering::Relaxed);
        QUERY_GET_NANOS.store(0, Ordering::Relaxed);
        QUERY_SINGLE_CALLS.store(0, Ordering::Relaxed);
        QUERY_SINGLE_NANOS.store(0, Ordering::Relaxed);
        CHANGED_CHECK_CALLS.store(0, Ordering::Relaxed);
        CHANGED_CHECK_NANOS.store(0, Ordering::Relaxed);
        ADDED_CHECK_CALLS.store(0, Ordering::Relaxed);
        ADDED_CHECK_NANOS.store(0, Ordering::Relaxed);
        RUNTIME_PLAN_CALLS.store(0, Ordering::Relaxed);
        RUNTIME_PLAN_NANOS.store(0, Ordering::Relaxed);
        RUNTIME_STAGE_CALLS.store(0, Ordering::Relaxed);
        RUNTIME_STAGE_NANOS.store(0, Ordering::Relaxed);
        RUNTIME_FLUSH_CALLS.store(0, Ordering::Relaxed);
        RUNTIME_FLUSH_NANOS.store(0, Ordering::Relaxed);
        RUNTIME_FLUSH_COMMAND_QUEUES.store(0, Ordering::Relaxed);
        SCHEDULE_PLAN_BUILD_CALLS.store(0, Ordering::Relaxed);
        SCHEDULE_PLAN_BUILD_NANOS.store(0, Ordering::Relaxed);
        SCHEDULE_PLAN_CONFLICT_CHECKS.store(0, Ordering::Relaxed);
        SCHEDULE_PLAN_STAGE_COUNT.store(0, Ordering::Relaxed);
    }

    pub fn snapshot() -> EcsTelemetrySnapshot {
        EcsTelemetrySnapshot {
            query_matching_calls: QUERY_MATCHING_CALLS.load(Ordering::Relaxed),
            query_matching_nanos: QUERY_MATCHING_NANOS.load(Ordering::Relaxed),
            query_matching_candidates: QUERY_MATCHING_CANDIDATES.load(Ordering::Relaxed),
            query_matching_matches: QUERY_MATCHING_MATCHES.load(Ordering::Relaxed),
            query_iter_calls: QUERY_ITER_CALLS.load(Ordering::Relaxed),
            query_iter_nanos: QUERY_ITER_NANOS.load(Ordering::Relaxed),
            query_get_calls: QUERY_GET_CALLS.load(Ordering::Relaxed),
            query_get_nanos: QUERY_GET_NANOS.load(Ordering::Relaxed),
            query_single_calls: QUERY_SINGLE_CALLS.load(Ordering::Relaxed),
            query_single_nanos: QUERY_SINGLE_NANOS.load(Ordering::Relaxed),
            changed_check_calls: CHANGED_CHECK_CALLS.load(Ordering::Relaxed),
            changed_check_nanos: CHANGED_CHECK_NANOS.load(Ordering::Relaxed),
            added_check_calls: ADDED_CHECK_CALLS.load(Ordering::Relaxed),
            added_check_nanos: ADDED_CHECK_NANOS.load(Ordering::Relaxed),
            runtime_plan_calls: RUNTIME_PLAN_CALLS.load(Ordering::Relaxed),
            runtime_plan_nanos: RUNTIME_PLAN_NANOS.load(Ordering::Relaxed),
            runtime_stage_calls: RUNTIME_STAGE_CALLS.load(Ordering::Relaxed),
            runtime_stage_nanos: RUNTIME_STAGE_NANOS.load(Ordering::Relaxed),
            runtime_flush_calls: RUNTIME_FLUSH_CALLS.load(Ordering::Relaxed),
            runtime_flush_nanos: RUNTIME_FLUSH_NANOS.load(Ordering::Relaxed),
            runtime_flush_command_queues: RUNTIME_FLUSH_COMMAND_QUEUES.load(Ordering::Relaxed),
            schedule_plan_build_calls: SCHEDULE_PLAN_BUILD_CALLS.load(Ordering::Relaxed),
            schedule_plan_build_nanos: SCHEDULE_PLAN_BUILD_NANOS.load(Ordering::Relaxed),
            schedule_plan_conflict_checks: SCHEDULE_PLAN_CONFLICT_CHECKS.load(Ordering::Relaxed),
            schedule_plan_stage_count: SCHEDULE_PLAN_STAGE_COUNT.load(Ordering::Relaxed),
        }
    }

    pub fn record_query_matching(duration_nanos: u64, candidates: u64, matches: u64) {
        QUERY_MATCHING_CALLS.fetch_add(1, Ordering::Relaxed);
        QUERY_MATCHING_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
        QUERY_MATCHING_CANDIDATES.fetch_add(candidates, Ordering::Relaxed);
        QUERY_MATCHING_MATCHES.fetch_add(matches, Ordering::Relaxed);
    }
    pub fn record_query_iter(duration_nanos: u64) {
        QUERY_ITER_CALLS.fetch_add(1, Ordering::Relaxed);
        QUERY_ITER_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_query_get(duration_nanos: u64) {
        QUERY_GET_CALLS.fetch_add(1, Ordering::Relaxed);
        QUERY_GET_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_query_single(duration_nanos: u64) {
        QUERY_SINGLE_CALLS.fetch_add(1, Ordering::Relaxed);
        QUERY_SINGLE_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_changed_check(duration_nanos: u64) {
        CHANGED_CHECK_CALLS.fetch_add(1, Ordering::Relaxed);
        CHANGED_CHECK_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_added_check(duration_nanos: u64) {
        ADDED_CHECK_CALLS.fetch_add(1, Ordering::Relaxed);
        ADDED_CHECK_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_runtime_plan(duration_nanos: u64) {
        RUNTIME_PLAN_CALLS.fetch_add(1, Ordering::Relaxed);
        RUNTIME_PLAN_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_runtime_stage(duration_nanos: u64) {
        RUNTIME_STAGE_CALLS.fetch_add(1, Ordering::Relaxed);
        RUNTIME_STAGE_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
    }
    pub fn record_runtime_flush(duration_nanos: u64, command_queues: u64) {
        RUNTIME_FLUSH_CALLS.fetch_add(1, Ordering::Relaxed);
        RUNTIME_FLUSH_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
        RUNTIME_FLUSH_COMMAND_QUEUES.fetch_add(command_queues, Ordering::Relaxed);
    }
    pub fn record_schedule_plan_build(duration_nanos: u64, conflict_checks: u64, stage_count: u64) {
        SCHEDULE_PLAN_BUILD_CALLS.fetch_add(1, Ordering::Relaxed);
        SCHEDULE_PLAN_BUILD_NANOS.fetch_add(duration_nanos, Ordering::Relaxed);
        SCHEDULE_PLAN_CONFLICT_CHECKS.fetch_add(conflict_checks, Ordering::Relaxed);
        SCHEDULE_PLAN_STAGE_COUNT.fetch_add(stage_count, Ordering::Relaxed);
    }
}

#[cfg(not(feature = "telemetry"))]
mod imp {
    use super::EcsTelemetrySnapshot;
    pub fn reset() {}
    pub fn snapshot() -> EcsTelemetrySnapshot {
        EcsTelemetrySnapshot::default()
    }
    pub fn record_query_matching(_duration_nanos: u64, _candidates: u64, _matches: u64) {}
    pub fn record_query_iter(_duration_nanos: u64) {}
    pub fn record_query_get(_duration_nanos: u64) {}
    pub fn record_query_single(_duration_nanos: u64) {}
    pub fn record_changed_check(_duration_nanos: u64) {}
    pub fn record_added_check(_duration_nanos: u64) {}
    pub fn record_runtime_plan(_duration_nanos: u64) {}
    pub fn record_runtime_stage(_duration_nanos: u64) {}
    pub fn record_runtime_flush(_duration_nanos: u64, _command_queues: u64) {}
    pub fn record_schedule_plan_build(
        _duration_nanos: u64,
        _conflict_checks: u64,
        _stage_count: u64,
    ) {
    }
}

pub fn reset() {
    imp::reset();
}
pub fn snapshot() -> EcsTelemetrySnapshot {
    imp::snapshot()
}

pub fn snapshot_delta(
    before: &EcsTelemetrySnapshot,
    after: &EcsTelemetrySnapshot,
) -> EcsTelemetrySnapshot {
    EcsTelemetrySnapshot {
        query_matching_calls: after
            .query_matching_calls
            .saturating_sub(before.query_matching_calls),
        query_matching_nanos: after
            .query_matching_nanos
            .saturating_sub(before.query_matching_nanos),
        query_matching_candidates: after
            .query_matching_candidates
            .saturating_sub(before.query_matching_candidates),
        query_matching_matches: after
            .query_matching_matches
            .saturating_sub(before.query_matching_matches),
        query_iter_calls: after
            .query_iter_calls
            .saturating_sub(before.query_iter_calls),
        query_iter_nanos: after
            .query_iter_nanos
            .saturating_sub(before.query_iter_nanos),
        query_get_calls: after.query_get_calls.saturating_sub(before.query_get_calls),
        query_get_nanos: after.query_get_nanos.saturating_sub(before.query_get_nanos),
        query_single_calls: after
            .query_single_calls
            .saturating_sub(before.query_single_calls),
        query_single_nanos: after
            .query_single_nanos
            .saturating_sub(before.query_single_nanos),
        changed_check_calls: after
            .changed_check_calls
            .saturating_sub(before.changed_check_calls),
        changed_check_nanos: after
            .changed_check_nanos
            .saturating_sub(before.changed_check_nanos),
        added_check_calls: after
            .added_check_calls
            .saturating_sub(before.added_check_calls),
        added_check_nanos: after
            .added_check_nanos
            .saturating_sub(before.added_check_nanos),
        runtime_plan_calls: after
            .runtime_plan_calls
            .saturating_sub(before.runtime_plan_calls),
        runtime_plan_nanos: after
            .runtime_plan_nanos
            .saturating_sub(before.runtime_plan_nanos),
        runtime_stage_calls: after
            .runtime_stage_calls
            .saturating_sub(before.runtime_stage_calls),
        runtime_stage_nanos: after
            .runtime_stage_nanos
            .saturating_sub(before.runtime_stage_nanos),
        runtime_flush_calls: after
            .runtime_flush_calls
            .saturating_sub(before.runtime_flush_calls),
        runtime_flush_nanos: after
            .runtime_flush_nanos
            .saturating_sub(before.runtime_flush_nanos),
        runtime_flush_command_queues: after
            .runtime_flush_command_queues
            .saturating_sub(before.runtime_flush_command_queues),
        schedule_plan_build_calls: after
            .schedule_plan_build_calls
            .saturating_sub(before.schedule_plan_build_calls),
        schedule_plan_build_nanos: after
            .schedule_plan_build_nanos
            .saturating_sub(before.schedule_plan_build_nanos),
        schedule_plan_conflict_checks: after
            .schedule_plan_conflict_checks
            .saturating_sub(before.schedule_plan_conflict_checks),
        schedule_plan_stage_count: after
            .schedule_plan_stage_count
            .saturating_sub(before.schedule_plan_stage_count),
    }
}

pub(crate) fn record_query_matching(duration_nanos: u64, candidates: u64, matches: u64) {
    imp::record_query_matching(duration_nanos, candidates, matches);
}
pub(crate) fn record_query_iter(duration_nanos: u64) {
    imp::record_query_iter(duration_nanos);
}
pub(crate) fn record_query_get(duration_nanos: u64) {
    imp::record_query_get(duration_nanos);
}
pub(crate) fn record_query_single(duration_nanos: u64) {
    imp::record_query_single(duration_nanos);
}
pub(crate) fn record_changed_check(duration_nanos: u64) {
    imp::record_changed_check(duration_nanos);
}
pub(crate) fn record_added_check(duration_nanos: u64) {
    imp::record_added_check(duration_nanos);
}
pub(crate) fn record_runtime_plan(duration_nanos: u64) {
    imp::record_runtime_plan(duration_nanos);
}
pub(crate) fn record_runtime_stage(duration_nanos: u64) {
    imp::record_runtime_stage(duration_nanos);
}
pub(crate) fn record_runtime_flush(duration_nanos: u64, command_queues: u64) {
    imp::record_runtime_flush(duration_nanos, command_queues);
}
pub(crate) fn record_schedule_plan_build(
    duration_nanos: u64,
    conflict_checks: u64,
    stage_count: u64,
) {
    imp::record_schedule_plan_build(duration_nanos, conflict_checks, stage_count);
}
