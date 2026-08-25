mod tool;

use std::time::Instant;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, CodebaseFacts, PolicyConfig};

use super::Analysis;
use super::policies::Policies;
use super::selection::{Family, Selection};
use crate::args::{CheckOptions, OutputFormat};
use crate::report;
use crate::tools::{self, ToolReport};

pub(super) fn run(
    options: &CheckOptions,
    input: &AnalysisInput,
    config: &PolicyConfig,
    policies: &Policies,
    selection: Selection,
    config_path: &Utf8Path,
) -> Result<Analysis, Vec<AnalysisError>> {
    let mut facts = CodebaseFacts::default();
    if let Some(analysis) = run_syntax(policies, input, options, config_path, &mut facts)? {
        return Ok(analysis);
    }
    let tool_reports = match run_selected_tools(config, input, options, selection, config_path)? {
        ToolStep::Continue(reports) => reports,
        ToolStep::Stopped(analysis) => return Ok(analysis),
    };
    if let Some(diagnostics) =
        run_semantic(policies, config, input, options, config_path, &mut facts)?
    {
        return Ok(Analysis {
            diagnostics,
            tool_reports,
            stopped: true,
        });
    }
    for family in [Family::DeadCode, Family::Visibility] {
        if let Some(mut analysis) =
            run_policy(family, policies, &facts, options, input, config_path)
        {
            analysis.tool_reports = tool_reports;
            return Ok(analysis);
        }
    }
    Ok(Analysis {
        diagnostics: Vec::new(),
        tool_reports,
        stopped: false,
    })
}

fn run_syntax(
    policies: &Policies,
    input: &AnalysisInput,
    options: &CheckOptions,
    config_path: &Utf8Path,
    facts: &mut CodebaseFacts,
) -> Result<Option<Analysis>, Vec<AnalysisError>> {
    if policies.has_syntax_family() {
        collect("syntax", options, || Policies::collect_syntax(input, facts))?;
    }
    for family in [Family::Size, Family::Testing] {
        if let Some(analysis) = run_policy(family, policies, facts, options, input, config_path) {
            return Ok(Some(analysis));
        }
    }
    Ok(None)
}

enum ToolStep {
    Continue(Vec<ToolReport>),
    Stopped(Analysis),
}

fn run_selected_tools(
    config: &PolicyConfig,
    input: &AnalysisInput,
    options: &CheckOptions,
    selection: Selection,
    config_path: &Utf8Path,
) -> Result<ToolStep, Vec<AnalysisError>> {
    let mut tool_reports = Vec::new();
    for family in [Family::Clippy, Family::Rustdoc] {
        if !tools::enabled(&config.tools, selection, family) {
            continue;
        }
        let report = tool::run(family, config, input, options, config_path)?;
        let stopped = !report.diagnostics.is_empty();
        tool_reports.push(report);
        if stopped {
            return Ok(ToolStep::Stopped(Analysis {
                diagnostics: Vec::new(),
                tool_reports,
                stopped: true,
            }));
        }
    }
    Ok(ToolStep::Continue(tool_reports))
}

fn run_semantic(
    policies: &Policies,
    config: &PolicyConfig,
    input: &AnalysisInput,
    options: &CheckOptions,
    config_path: &Utf8Path,
    facts: &mut CodebaseFacts,
) -> Result<Option<Vec<policy_core::Diagnostic>>, Vec<AnalysisError>> {
    if !policies.has_semantic() {
        return Ok(None);
    }
    if !policies.has_syntax_family() {
        collect("syntax", options, || Policies::collect_syntax(input, facts))?;
    }
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started("semantic", options.color);
    }
    let started = Instant::now();
    let early = policies.collect_semantic_fail_fast(config, input, facts);
    if human {
        if early.is_ok() {
            report::progress::finished("semantic", started.elapsed());
        } else {
            report::progress::failed("semantic");
        }
    }
    let diagnostics = early?;
    if let Some(diagnostics) = &diagnostics
        && human
    {
        report::human::write_policy_diagnostics(diagnostics, input, config_path, options.color);
    }
    Ok(diagnostics)
}

fn collect(
    name: &str,
    options: &CheckOptions,
    operation: impl FnOnce() -> Result<(), Vec<AnalysisError>>,
) -> Result<(), Vec<AnalysisError>> {
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started(name, options.color);
    }
    let started = Instant::now();
    let result = operation();
    if human {
        if result.is_ok() {
            report::progress::finished(name, started.elapsed());
        } else {
            report::progress::failed(name);
        }
    }
    result
}

fn run_policy(
    family: Family,
    policies: &Policies,
    facts: &CodebaseFacts,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) -> Option<Analysis> {
    if !policies.has_family(family) {
        return None;
    }
    let name = family.policy_name();
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started(name, options.color);
    }
    let started = Instant::now();
    let mut diagnostics = policies.evaluate(family, facts);
    if human {
        report::progress::finished(name, started.elapsed());
    }
    if diagnostics.is_empty() {
        return None;
    }
    diagnostics.truncate(1);
    if human {
        report::human::write_policy_diagnostics(&diagnostics, input, config_path, options.color);
    }
    Some(Analysis {
        diagnostics,
        tool_reports: Vec::new(),
        stopped: true,
    })
}
