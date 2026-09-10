use crate::plugins::render::{
    RenderFlow, RenderFlowValidationError, RenderFragmentDiagnosticSeverity,
    RenderFragmentMergeReport, compile_flow_plan,
};

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugGraphDumpState {
    pub revision: u64,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFragmentMergeInspection {
    pub package_id: Option<String>,
    pub source_path: Option<String>,
    pub source_revision: Option<u64>,
    pub generated_flow_id: Option<String>,
    pub provenance_count: usize,
    pub error_count: usize,
    pub lines: Vec<String>,
}

pub fn inspect_render_fragment_merge_report(
    report: &RenderFragmentMergeReport,
) -> RenderFragmentMergeInspection {
    let mut lines = Vec::<String>::new();
    if let Some(package_id) = &report.package_id {
        lines.push(format!("fragment_package: {}", package_id));
    }
    if let Some(source_path) = &report.source_path {
        lines.push(format!("source_path: {}", source_path));
    }
    if let Some(revision) = report.source_revision {
        lines.push(format!("source_revision: {}", revision.0));
    }
    for record in &report.provenance {
        lines.push(format!(
            "provenance: {:?} {} -> {} [{}:{}:{}]",
            record.element_kind,
            record.source_label,
            record.generated_label,
            record.package_id,
            record.fragment_id,
            record.namespace
        ));
    }
    for diagnostic in &report.diagnostics {
        lines.push(format!(
            "diagnostic: {:?} {:?} {}",
            diagnostic.severity, diagnostic.kind, diagnostic.message
        ));
    }

    RenderFragmentMergeInspection {
        package_id: report.package_id.as_ref().map(ToString::to_string),
        source_path: report.source_path.clone(),
        source_revision: report.source_revision.map(|revision| revision.0),
        generated_flow_id: report.generated_flow_id.clone(),
        provenance_count: report.provenance.len(),
        error_count: report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == RenderFragmentDiagnosticSeverity::Error)
            .count(),
        lines,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFlowGraphDump {
    pub flow_id: String,
    pub lines: Vec<String>,
}

pub fn dump_flow_graph(
    flow: &RenderFlow,
) -> Result<RenderFlowGraphDump, RenderFlowValidationError> {
    let compiled = compile_flow_plan(flow)?;
    let mut lines = Vec::<String>::new();

    lines.push(format!("flow: {}", flow.id()));
    lines.push("resources:".to_string());
    for resource in &flow.graph().resources.resources {
        lines.push(format!(
            "  - {} ({:?}, {:?})",
            resource.id(),
            resource,
            resource.lifetime()
        ));
    }

    lines.push("compiled_passes:".to_string());
    for pass in &compiled.render_passes {
        lines.push(format!(
            "  - {} [{} {:?}; authoring_index={}]",
            pass.pass_id(),
            pass.pass_label(),
            pass.node().kind,
            pass.authoring_index()
        ));
    }
    lines.push(
        "runtime_gpu_work: resolved after runtime resources, programs, pipelines, and bindings"
            .to_string(),
    );
    lines.push(format!(
        "compiler_diagnostics: {}",
        compiled.compiler_diagnostics.len()
    ));
    Ok(RenderFlowGraphDump {
        flow_id: flow.id().to_string(),
        lines,
    })
}
