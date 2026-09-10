use std::any::{Any, TypeId};
use std::collections::{BTreeMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};
use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use product::{FieldProductDiagnostic, ProductJobDescriptor};

use crate::runtime::jobs::diagnostics::{
    runtime_job_backpressure_diagnostic, runtime_job_disconnected_diagnostic,
    runtime_job_error_for_product, runtime_job_panic_diagnostic, runtime_job_stale_diagnostic,
};
use crate::runtime::jobs::task::RuntimeJob;
use crate::runtime::jobs::types::{
    RuntimeJobCompletion, RuntimeJobError, RuntimeJobExecutionMode, RuntimeJobExecutorConfig,
    RuntimeJobExecutorDiagnostics, RuntimeJobGeneration, RuntimeJobGenerationSnapshot,
    RuntimeJobHandle, RuntimeJobIssueKind, RuntimeJobIssueSnapshot, RuntimeJobKey,
    RuntimeJobPendingSnapshot, RuntimeJobStatus, RuntimeJobSubmissionError,
};

type ErasedOutput = Box<dyn Any + Send>;
const RECENT_JOB_ISSUE_LIMIT: usize = 64;

trait ErasedRuntimeJob: Send {
    fn execute_boxed(self: Box<Self>) -> ErasedRuntimeJobOutcome;
}

struct RuntimeJobEnvelope<J>
where
    J: RuntimeJob,
{
    handle: RuntimeJobHandle,
    product_job: ProductJobDescriptor,
    job: J,
}

impl<J> ErasedRuntimeJob for RuntimeJobEnvelope<J>
where
    J: RuntimeJob,
{
    fn execute_boxed(self: Box<Self>) -> ErasedRuntimeJobOutcome {
        let Self {
            handle,
            product_job,
            job,
        } = *self;
        let output_type_id = TypeId::of::<J::Output>();
        match catch_unwind(AssertUnwindSafe(|| job.execute())) {
            Ok(Ok(output)) => ErasedRuntimeJobOutcome {
                handle,
                product_job,
                status: RuntimeJobStatus::Completed,
                output_type_id,
                output: Some(Box::new(output)),
                diagnostics: Vec::new(),
            },
            Ok(Err(err)) => failed_outcome(handle, product_job, output_type_id, err),
            Err(_) => ErasedRuntimeJobOutcome {
                handle,
                product_job,
                status: RuntimeJobStatus::Failed,
                output_type_id,
                output: None,
                diagnostics: vec![runtime_job_panic_diagnostic()],
            },
        }
    }
}

fn failed_outcome(
    handle: RuntimeJobHandle,
    product_job: ProductJobDescriptor,
    output_type_id: TypeId,
    err: RuntimeJobError,
) -> ErasedRuntimeJobOutcome {
    let diagnostics = err
        .diagnostics
        .into_iter()
        .map(|diagnostic| runtime_job_error_for_product(diagnostic, &product_job))
        .collect();
    ErasedRuntimeJobOutcome {
        handle,
        product_job,
        status: RuntimeJobStatus::Failed,
        output_type_id,
        output: None,
        diagnostics,
    }
}

struct ErasedRuntimeJobOutcome {
    handle: RuntimeJobHandle,
    product_job: ProductJobDescriptor,
    status: RuntimeJobStatus,
    output_type_id: TypeId,
    output: Option<ErasedOutput>,
    diagnostics: Vec<FieldProductDiagnostic>,
}

impl ErasedRuntimeJobOutcome {
    fn mark_stale(&mut self, latest: RuntimeJobGeneration) {
        self.status = RuntimeJobStatus::Stale;
        self.output = None;
        self.diagnostics = vec![runtime_job_stale_diagnostic(self.handle, latest)];
    }

    fn into_typed<T: Send + 'static>(self) -> RuntimeJobCompletion<T> {
        let output = self
            .output
            .and_then(|output| output.downcast::<T>().ok())
            .map(|output| *output);
        RuntimeJobCompletion {
            handle: self.handle,
            product_job: self.product_job,
            status: self.status,
            output,
            diagnostics: self.diagnostics,
        }
    }
}

enum WorkerCommand {
    Run(Box<dyn ErasedRuntimeJob>),
    Shutdown,
}

enum RuntimeJobSubmitFailure {
    Full,
    Disconnected,
}

struct WorkerPool {
    sender: Option<Sender<WorkerCommand>>,
    completions: Receiver<ErasedRuntimeJobOutcome>,
    workers: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    fn new(config: &RuntimeJobExecutorConfig) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(config.queue_capacity.max(1));
        let (completion_sender, completions) = crossbeam_channel::unbounded();
        let worker_count = config.worker_threads.max(1);
        let mut workers = Vec::with_capacity(worker_count);
        for index in 0..worker_count {
            let receiver: Receiver<WorkerCommand> = receiver.clone();
            let completion_sender = completion_sender.clone();
            let worker = thread::Builder::new()
                .name(format!("runenwerk-product-job-{index}"))
                .spawn(move || {
                    while let Ok(command) = receiver.recv() {
                        match command {
                            WorkerCommand::Run(job) => {
                                let _ = completion_sender.send(job.execute_boxed());
                            }
                            WorkerCommand::Shutdown => break,
                        }
                    }
                })
                .expect("runtime product job worker should spawn");
            workers.push(worker);
        }
        Self {
            sender: Some(sender),
            completions,
            workers,
        }
    }

    fn submit(&self, job: Box<dyn ErasedRuntimeJob>) -> Result<(), RuntimeJobSubmitFailure> {
        let Some(sender) = &self.sender else {
            return Err(RuntimeJobSubmitFailure::Disconnected);
        };
        sender
            .try_send(WorkerCommand::Run(job))
            .map_err(|err| match err {
                TrySendError::Full(_) => RuntimeJobSubmitFailure::Full,
                TrySendError::Disconnected(_) => RuntimeJobSubmitFailure::Disconnected,
            })
    }

    fn try_recv_completed(&self) -> Result<ErasedRuntimeJobOutcome, TryRecvError> {
        self.completions.try_recv()
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            for _ in 0..self.workers.len() {
                let _ = sender.send(WorkerCommand::Shutdown);
            }
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

struct WorkStealingPool {
    injector: Arc<Injector<Box<dyn ErasedRuntimeJob>>>,
    completions: Receiver<ErasedRuntimeJobOutcome>,
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    pending_count: Arc<AtomicUsize>,
    wake: Arc<(Mutex<()>, Condvar)>,
    queue_capacity: usize,
}

impl WorkStealingPool {
    fn new(config: &RuntimeJobExecutorConfig) -> Self {
        let injector = Arc::new(Injector::new());
        let (completion_sender, completions) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(AtomicBool::new(false));
        let pending_count = Arc::new(AtomicUsize::new(0));
        let wake = Arc::new((Mutex::new(()), Condvar::new()));
        let worker_count = config.worker_threads.max(1);
        let queue_capacity = config.queue_capacity.max(1);
        let locals = (0..worker_count)
            .map(|_| Worker::new_fifo())
            .collect::<Vec<_>>();
        let stealers = locals
            .iter()
            .map(Worker::stealer)
            .collect::<Vec<Stealer<Box<dyn ErasedRuntimeJob>>>>();
        let mut workers = Vec::with_capacity(worker_count);

        for (index, local) in locals.into_iter().enumerate() {
            let injector = Arc::clone(&injector);
            let completion_sender = completion_sender.clone();
            let shutdown = Arc::clone(&shutdown);
            let pending_count = Arc::clone(&pending_count);
            let wake = Arc::clone(&wake);
            let peer_stealers = stealers.clone();
            let worker = thread::Builder::new()
                .name(format!("runenwerk-product-steal-{index}"))
                .spawn(move || {
                    loop {
                        if let Some(job) = steal_next_job(&local, &injector, &peer_stealers) {
                            let _ = completion_sender.send(job.execute_boxed());
                            pending_count.fetch_sub(1, Ordering::SeqCst);
                            continue;
                        }
                        if shutdown.load(Ordering::SeqCst)
                            && pending_count.load(Ordering::SeqCst) == 0
                        {
                            break;
                        }
                        wait_for_runtime_job_signal(&wake);
                    }
                })
                .expect("runtime product work-stealing worker should spawn");
            workers.push(worker);
        }

        Self {
            injector,
            completions,
            workers,
            shutdown,
            pending_count,
            wake,
            queue_capacity,
        }
    }

    fn submit(&self, job: Box<dyn ErasedRuntimeJob>) -> Result<(), RuntimeJobSubmitFailure> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(RuntimeJobSubmitFailure::Disconnected);
        }
        if !self.try_reserve_slot() {
            return Err(RuntimeJobSubmitFailure::Full);
        }
        self.injector.push(job);
        self.wake.1.notify_one();
        Ok(())
    }

    fn try_recv_completed(&self) -> Result<ErasedRuntimeJobOutcome, TryRecvError> {
        self.completions.try_recv()
    }

    fn try_reserve_slot(&self) -> bool {
        loop {
            let pending = self.pending_count.load(Ordering::SeqCst);
            if pending >= self.queue_capacity {
                return false;
            }
            if self
                .pending_count
                .compare_exchange(
                    pending,
                    pending.saturating_add(1),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                return true;
            }
        }
    }
}

impl Drop for WorkStealingPool {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.wake.1.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn steal_next_job(
    local: &Worker<Box<dyn ErasedRuntimeJob>>,
    injector: &Injector<Box<dyn ErasedRuntimeJob>>,
    stealers: &[Stealer<Box<dyn ErasedRuntimeJob>>],
) -> Option<Box<dyn ErasedRuntimeJob>> {
    if let Some(job) = local.pop() {
        return Some(job);
    }
    if let Some(job) = retry_steal(|| injector.steal()) {
        return Some(job);
    }
    for stealer in stealers {
        if let Some(job) = retry_steal(|| stealer.steal()) {
            return Some(job);
        }
    }
    None
}

fn retry_steal<T>(mut steal: impl FnMut() -> Steal<T>) -> Option<T> {
    loop {
        match steal() {
            Steal::Success(value) => return Some(value),
            Steal::Empty => return None,
            Steal::Retry => thread::yield_now(),
        }
    }
}

fn wait_for_runtime_job_signal(wake: &Arc<(Mutex<()>, Condvar)>) {
    let (lock, signal) = &**wake;
    let guard = lock
        .lock()
        .expect("runtime job worker wake lock should not be poisoned");
    let _ = signal.wait_timeout(guard, Duration::from_millis(10));
}

enum RuntimeJobBackend {
    Serial,
    WorkerPool(WorkerPool),
    WorkStealing(WorkStealingPool),
}

pub struct RuntimeJobExecutorResource {
    config: RuntimeJobExecutorConfig,
    backend: RuntimeJobBackend,
    latest_generations: BTreeMap<RuntimeJobKey, RuntimeJobGeneration>,
    pending_jobs: BTreeMap<RuntimeJobHandle, RuntimeJobHandle>,
    recent_issues: VecDeque<RuntimeJobIssueSnapshot>,
    completed: VecDeque<ErasedRuntimeJobOutcome>,
    next_submission_sequence: u64,
    submitted_count: u64,
    completed_count: u64,
    failed_count: u64,
    stale_count: u64,
    rejected_count: u64,
    last_drain_count: usize,
    last_drain_limit_hit: bool,
}

impl Default for RuntimeJobExecutorResource {
    fn default() -> Self {
        Self::with_config(RuntimeJobExecutorConfig::default())
    }
}

impl runen_ecs::Resource for RuntimeJobExecutorResource {}

impl RuntimeJobExecutorResource {
    pub fn with_config(config: RuntimeJobExecutorConfig) -> Self {
        let config = config.normalized();
        let backend = match config.execution_mode {
            RuntimeJobExecutionMode::Serial => RuntimeJobBackend::Serial,
            RuntimeJobExecutionMode::WorkerPool => {
                RuntimeJobBackend::WorkerPool(WorkerPool::new(&config))
            }
            RuntimeJobExecutionMode::WorkStealing => {
                RuntimeJobBackend::WorkStealing(WorkStealingPool::new(&config))
            }
        };
        Self {
            config,
            backend,
            latest_generations: BTreeMap::new(),
            pending_jobs: BTreeMap::new(),
            recent_issues: VecDeque::new(),
            completed: VecDeque::new(),
            next_submission_sequence: 0,
            submitted_count: 0,
            completed_count: 0,
            failed_count: 0,
            stale_count: 0,
            rejected_count: 0,
            last_drain_count: 0,
            last_drain_limit_hit: false,
        }
    }

    pub fn config(&self) -> &RuntimeJobExecutorConfig {
        &self.config
    }

    pub fn submit<J>(&mut self, job: J) -> Result<RuntimeJobHandle, RuntimeJobSubmissionError>
    where
        J: RuntimeJob,
    {
        let product_job = job.product_job();
        let key = job.key();
        let generation = job.generation();
        self.next_submission_sequence = self.next_submission_sequence.saturating_add(1);
        let handle = RuntimeJobHandle {
            key,
            generation,
            submission_sequence: self.next_submission_sequence,
            product_job_id: product_job.job_id,
        };
        let envelope = RuntimeJobEnvelope {
            handle,
            product_job,
            job,
        };

        match &self.backend {
            RuntimeJobBackend::Serial => {
                self.record_submitted(handle);
                self.completed.push_back(Box::new(envelope).execute_boxed());
                Ok(handle)
            }
            RuntimeJobBackend::WorkerPool(pool) => {
                let result = pool.submit(Box::new(envelope));
                self.record_pool_submission_result(handle, key, generation, result)
            }
            RuntimeJobBackend::WorkStealing(pool) => {
                let result = pool.submit(Box::new(envelope));
                self.record_pool_submission_result(handle, key, generation, result)
            }
        }
    }

    pub fn drain_completed<T>(&mut self) -> Vec<RuntimeJobCompletion<T>>
    where
        T: Send + 'static,
    {
        self.collect_worker_completions();

        let mut completions = Vec::new();
        let mut remaining = VecDeque::new();
        let output_type_id = TypeId::of::<T>();
        self.last_drain_count = 0;
        self.last_drain_limit_hit = false;
        while let Some(mut outcome) = self.completed.pop_front() {
            if outcome.output_type_id != output_type_id {
                remaining.push_back(outcome);
                continue;
            }
            if completions.len() >= self.config.completion_drain_budget {
                self.last_drain_limit_hit = true;
                remaining.push_back(outcome);
                continue;
            }
            self.apply_staleness(&mut outcome);
            self.record_completion_outcome(&outcome);
            self.last_drain_count = self.last_drain_count.saturating_add(1);
            completions.push(outcome.into_typed::<T>());
        }
        self.completed = remaining;
        completions
    }

    pub fn diagnostics(&mut self) -> RuntimeJobExecutorDiagnostics {
        self.collect_worker_completions();
        RuntimeJobExecutorDiagnostics {
            execution_mode: self.config.execution_mode,
            worker_threads: self.config.worker_threads,
            queue_capacity: self.config.queue_capacity,
            completion_drain_budget: self.config.completion_drain_budget,
            submitted_count: self.submitted_count,
            completed_count: self.completed_count,
            failed_count: self.failed_count,
            stale_count: self.stale_count,
            rejected_count: self.rejected_count,
            pending_count: self.pending_jobs.len() as u64,
            buffered_completion_count: self.completed.len(),
            last_drain_count: self.last_drain_count,
            last_drain_limit_hit: self.last_drain_limit_hit,
            latest_generations: self
                .latest_generations
                .iter()
                .map(|(key, generation)| RuntimeJobGenerationSnapshot {
                    key: *key,
                    generation: *generation,
                })
                .collect(),
            pending_jobs: self
                .pending_jobs
                .values()
                .copied()
                .map(|handle| RuntimeJobPendingSnapshot { handle })
                .collect(),
            recent_issues: self.recent_issues.iter().cloned().collect(),
        }
    }

    fn collect_worker_completions(&mut self) {
        match &self.backend {
            RuntimeJobBackend::Serial => {}
            RuntimeJobBackend::WorkerPool(pool) => {
                while let Ok(outcome) = pool.try_recv_completed() {
                    self.completed.push_back(outcome);
                }
            }
            RuntimeJobBackend::WorkStealing(pool) => {
                while let Ok(outcome) = pool.try_recv_completed() {
                    self.completed.push_back(outcome);
                }
            }
        }
    }

    fn apply_staleness(&self, outcome: &mut ErasedRuntimeJobOutcome) {
        let Some(latest) = self.latest_generations.get(&outcome.handle.key).copied() else {
            return;
        };
        if latest > outcome.handle.generation {
            outcome.mark_stale(latest);
        }
    }

    fn record_submitted(&mut self, handle: RuntimeJobHandle) {
        self.latest_generations
            .insert(handle.key, handle.generation);
        self.pending_jobs.insert(handle, handle);
        self.submitted_count = self.submitted_count.saturating_add(1);
    }

    fn record_pool_submission_result(
        &mut self,
        handle: RuntimeJobHandle,
        key: RuntimeJobKey,
        generation: RuntimeJobGeneration,
        result: Result<(), RuntimeJobSubmitFailure>,
    ) -> Result<RuntimeJobHandle, RuntimeJobSubmissionError> {
        match result {
            Ok(()) => {
                self.record_submitted(handle);
                Ok(handle)
            }
            Err(RuntimeJobSubmitFailure::Full) => {
                let diagnostics = vec![runtime_job_backpressure_diagnostic(
                    self.config.queue_capacity,
                )];
                self.record_rejection(key, generation, handle.product_job_id, diagnostics.clone());
                Err(RuntimeJobSubmissionError::new(key, generation, diagnostics))
            }
            Err(RuntimeJobSubmitFailure::Disconnected) => {
                let diagnostics = vec![runtime_job_disconnected_diagnostic()];
                self.record_rejection(key, generation, handle.product_job_id, diagnostics.clone());
                Err(RuntimeJobSubmissionError::new(key, generation, diagnostics))
            }
        }
    }

    fn record_rejection(
        &mut self,
        key: RuntimeJobKey,
        generation: RuntimeJobGeneration,
        product_job_id: product::ProductJobId,
        diagnostics: Vec<FieldProductDiagnostic>,
    ) {
        self.rejected_count = self.rejected_count.saturating_add(1);
        self.push_issue(RuntimeJobIssueSnapshot {
            kind: RuntimeJobIssueKind::Rejected,
            handle: None,
            key,
            generation,
            product_job_id: Some(product_job_id),
            diagnostics,
        });
    }

    fn record_completion_outcome(&mut self, outcome: &ErasedRuntimeJobOutcome) {
        self.pending_jobs.remove(&outcome.handle);
        match outcome.status {
            RuntimeJobStatus::Completed => {
                self.completed_count = self.completed_count.saturating_add(1);
            }
            RuntimeJobStatus::Failed => {
                self.failed_count = self.failed_count.saturating_add(1);
                self.push_issue(RuntimeJobIssueSnapshot {
                    kind: RuntimeJobIssueKind::Failed,
                    handle: Some(outcome.handle),
                    key: outcome.handle.key,
                    generation: outcome.handle.generation,
                    product_job_id: Some(outcome.product_job.job_id),
                    diagnostics: outcome.diagnostics.clone(),
                });
            }
            RuntimeJobStatus::Stale => {
                self.stale_count = self.stale_count.saturating_add(1);
                self.push_issue(RuntimeJobIssueSnapshot {
                    kind: RuntimeJobIssueKind::Stale,
                    handle: Some(outcome.handle),
                    key: outcome.handle.key,
                    generation: outcome.handle.generation,
                    product_job_id: Some(outcome.product_job.job_id),
                    diagnostics: outcome.diagnostics.clone(),
                });
            }
        }
    }

    fn push_issue(&mut self, issue: RuntimeJobIssueSnapshot) {
        self.recent_issues.push_back(issue);
        if self.recent_issues.len() > RECENT_JOB_ISSUE_LIMIT {
            self.recent_issues.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use product::{
        ProductIdentity, ProductJobDescriptor, ProductKind, ProductScaleBand, ProductScope,
    };

    use super::*;
    use crate::runtime::jobs::RuntimeJobResult;

    const RUNTIME_JOB_TEST_TIMEOUT: Duration = Duration::from_secs(30);

    #[derive(Debug)]
    struct TestJob {
        id: u64,
        generation: u64,
        value: u64,
    }

    impl RuntimeJob for TestJob {
        type Output = u64;

        fn product_job(&self) -> ProductJobDescriptor {
            ProductJobDescriptor::new(
                product::ProductJobId::new(self.id),
                ProductKind::new("runtime_test"),
                "engine.runtime.test",
                ProductIdentity::new(self.id),
                ProductScope::non_spatial("runtime-test"),
                ProductScaleBand::Preview,
            )
        }

        fn generation(&self) -> RuntimeJobGeneration {
            RuntimeJobGeneration::new(self.generation)
        }

        fn execute(self) -> RuntimeJobResult<Self::Output> {
            Ok(self.value)
        }
    }

    #[test]
    fn runtime_job_serial_executor_completes_typed_job_deterministically() {
        let mut executor = RuntimeJobExecutorResource::default();

        executor
            .submit(TestJob {
                id: 1,
                generation: 1,
                value: 42,
            })
            .expect("serial job should submit");

        let completions = executor.drain_completed::<u64>();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].status, RuntimeJobStatus::Completed);
        assert_eq!(completions[0].output, Some(42));
    }

    #[test]
    fn runtime_job_worker_executor_matches_serial_result() {
        let mut executor =
            RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::worker_pool(1, 4));

        executor
            .submit(TestJob {
                id: 2,
                generation: 1,
                value: 7,
            })
            .expect("worker job should submit");

        let completions = wait_for_completions::<u64>(&mut executor, 1);
        assert_eq!(completions[0].status, RuntimeJobStatus::Completed);
        assert_eq!(completions[0].output, Some(7));
    }

    #[test]
    fn runtime_job_work_stealing_executor_matches_serial_results() {
        let mut config = RuntimeJobExecutorConfig::work_stealing(2, 8);
        config.completion_drain_budget = 1;
        let mut executor = RuntimeJobExecutorResource::with_config(config);

        for id in 1..=4 {
            executor
                .submit(TestJob {
                    id,
                    generation: 1,
                    value: id * 10,
                })
                .expect("work-stealing job should submit");
        }

        let mut values = wait_for_completions::<u64>(&mut executor, 4)
            .into_iter()
            .map(|completion| {
                assert_eq!(completion.status, RuntimeJobStatus::Completed);
                completion
                    .output
                    .expect("completed job should carry output")
            })
            .collect::<Vec<_>>();
        values.sort_unstable();
        assert_eq!(values, vec![10, 20, 30, 40]);
    }

    #[test]
    fn runtime_job_newer_generation_marks_older_completion_stale() {
        let mut executor = RuntimeJobExecutorResource::default();

        executor
            .submit(TestJob {
                id: 3,
                generation: 1,
                value: 1,
            })
            .unwrap();
        executor
            .submit(TestJob {
                id: 3,
                generation: 2,
                value: 2,
            })
            .unwrap();

        let completions = executor.drain_completed::<u64>();
        assert_eq!(completions.len(), 2);
        assert_eq!(completions[0].status, RuntimeJobStatus::Stale);
        assert_eq!(completions[0].output, None);
        assert_eq!(completions[1].status, RuntimeJobStatus::Completed);
        assert_eq!(completions[1].output, Some(2));
    }

    #[test]
    fn runtime_job_panic_becomes_failed_completion() {
        struct PanicJob;

        impl RuntimeJob for PanicJob {
            type Output = u64;

            fn product_job(&self) -> ProductJobDescriptor {
                ProductJobDescriptor::new(
                    product::ProductJobId::new(4),
                    ProductKind::new("runtime_test"),
                    "engine.runtime.test",
                    ProductIdentity::new(4),
                    ProductScope::non_spatial("runtime-test"),
                    ProductScaleBand::Preview,
                )
            }

            fn generation(&self) -> RuntimeJobGeneration {
                RuntimeJobGeneration::new(1)
            }

            fn execute(self) -> RuntimeJobResult<Self::Output> {
                panic!("intentional runtime job panic")
            }
        }

        let mut executor = RuntimeJobExecutorResource::default();
        executor.submit(PanicJob).unwrap();

        let completions = executor.drain_completed::<u64>();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].status, RuntimeJobStatus::Failed);
        assert!(completions[0].output.is_none());
        assert!(!completions[0].diagnostics.is_empty());
    }

    #[test]
    fn runtime_job_diagnostics_expose_pending_and_recent_issues() {
        struct PanicJob;

        impl RuntimeJob for PanicJob {
            type Output = u64;

            fn product_job(&self) -> ProductJobDescriptor {
                ProductJobDescriptor::new(
                    product::ProductJobId::new(40),
                    ProductKind::new("runtime_test"),
                    "engine.runtime.test",
                    ProductIdentity::new(40),
                    ProductScope::non_spatial("runtime-test"),
                    ProductScaleBand::Preview,
                )
            }

            fn generation(&self) -> RuntimeJobGeneration {
                RuntimeJobGeneration::new(1)
            }

            fn execute(self) -> RuntimeJobResult<Self::Output> {
                panic!("intentional runtime job panic")
            }
        }

        let mut executor = RuntimeJobExecutorResource::default();
        executor.submit(PanicJob).unwrap();
        assert_eq!(executor.diagnostics().pending_jobs.len(), 1);

        let completions = executor.drain_completed::<u64>();
        assert_eq!(completions[0].status, RuntimeJobStatus::Failed);
        let diagnostics = executor.diagnostics();
        assert_eq!(diagnostics.pending_jobs.len(), 0);
        assert_eq!(diagnostics.failed_count, 1);
        assert!(
            diagnostics
                .recent_issues
                .iter()
                .any(|issue| issue.kind == RuntimeJobIssueKind::Failed)
        );
    }

    #[test]
    fn runtime_job_worker_backpressure_is_reported() {
        struct BlockingJob {
            id: u64,
            started: Arc<AtomicBool>,
            release: Arc<AtomicBool>,
        }

        impl RuntimeJob for BlockingJob {
            type Output = u64;

            fn product_job(&self) -> ProductJobDescriptor {
                ProductJobDescriptor::new(
                    product::ProductJobId::new(self.id),
                    ProductKind::new("runtime_test"),
                    "engine.runtime.test",
                    ProductIdentity::new(self.id),
                    ProductScope::non_spatial("runtime-test"),
                    ProductScaleBand::Preview,
                )
            }

            fn generation(&self) -> RuntimeJobGeneration {
                RuntimeJobGeneration::new(1)
            }

            fn execute(self) -> RuntimeJobResult<Self::Output> {
                self.started.store(true, Ordering::SeqCst);
                while !self.release.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(self.id)
            }
        }

        let started = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let mut executor =
            RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::worker_pool(1, 1));
        executor
            .submit(BlockingJob {
                id: 10,
                started: started.clone(),
                release: release.clone(),
            })
            .unwrap();
        wait_until(|| started.load(Ordering::SeqCst));
        executor
            .submit(BlockingJob {
                id: 11,
                started: Arc::new(AtomicBool::new(false)),
                release: release.clone(),
            })
            .unwrap();
        let err = executor
            .submit(BlockingJob {
                id: 12,
                started: Arc::new(AtomicBool::new(false)),
                release: release.clone(),
            })
            .expect_err("bounded worker queue should report backpressure");
        release.store(true, Ordering::SeqCst);

        assert!(!err.diagnostics.is_empty());
    }

    fn wait_for_completions<T: Send + 'static>(
        executor: &mut RuntimeJobExecutorResource,
        expected: usize,
    ) -> Vec<RuntimeJobCompletion<T>> {
        let started = Instant::now();
        let mut completions = Vec::with_capacity(expected);
        loop {
            completions.extend(executor.drain_completed::<T>());
            if completions.len() >= expected {
                return completions;
            }
            assert!(
                started.elapsed() < RUNTIME_JOB_TEST_TIMEOUT,
                "timed out waiting for runtime job completion"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn wait_until(condition: impl Fn() -> bool) {
        let started = Instant::now();
        while !condition() {
            assert!(
                started.elapsed() < RUNTIME_JOB_TEST_TIMEOUT,
                "timed out waiting for worker job to start"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
