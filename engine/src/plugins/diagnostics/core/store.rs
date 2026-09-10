use super::model::DiagnosticsFrameReport;
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DiagnosticsReportStoreResource {
    pub reports: VecDeque<DiagnosticsFrameReport>,
    pub latest: Option<DiagnosticsFrameReport>,
    pub next_report_sequence: u64,
}

impl DiagnosticsReportStoreResource {
    pub fn allocate_report_sequence(&mut self) -> u64 {
        self.next_report_sequence = self.next_report_sequence.saturating_add(1);
        self.next_report_sequence
    }

    pub fn push_report(&mut self, report: DiagnosticsFrameReport, retention_max_reports: usize) {
        self.latest = Some(report.clone());
        self.reports.push_back(report);
        let retention = retention_max_reports.max(1);
        while self.reports.len() > retention {
            self.reports.pop_front();
        }
    }

    pub fn latest_report(&self) -> Option<&DiagnosticsFrameReport> {
        self.latest.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::diagnostics::core::model::DiagnosticsFrameReport;

    #[test]
    fn retention_keeps_last_n_reports() {
        let mut store = DiagnosticsReportStoreResource::default();
        for idx in 0..5_u64 {
            let mut report = DiagnosticsFrameReport::new_pending(idx, idx);
            report.assign_identity(store.allocate_report_sequence());
            store.push_report(report, 2);
        }

        assert_eq!(store.reports.len(), 2);
        assert_eq!(
            store.reports.front().map(|value| value.frame_index),
            Some(3)
        );
        assert_eq!(store.reports.back().map(|value| value.frame_index), Some(4));
    }
}
