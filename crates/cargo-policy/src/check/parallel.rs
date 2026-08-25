mod reporting;

use core::time::Duration;
use std::sync::mpsc;
use std::time::Instant;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, CodebaseFacts, Diagnostic, PolicyConfig};

use super::Analysis;
use super::policies::Policies;
use super::selection::Selection;
use crate::args::{CheckOptions, OutputFormat};
use crate::report;
use crate::tools::{RunEvent, ToolReport};

enum Event {
    Policies(Result<Vec<Diagnostic>, Vec<AnalysisError>>, Duration),
    Tool(RunEvent),
    ToolsComplete(Result<(), AnalysisError>),
}

struct State {
    diagnostics: Vec<Diagnostic>,
    tool_reports: Vec<ToolReport>,
    errors: Vec<AnalysisError>,
    policies_complete: bool,
    tools_complete: bool,
    active_tool: Option<&'static str>,
}

impl State {
    fn new(policies_complete: bool, tools_complete: bool) -> Self {
        Self {
            diagnostics: Vec::new(),
            tool_reports: Vec::new(),
            errors: Vec::new(),
            policies_complete,
            tools_complete,
            active_tool: None,
        }
    }

    const fn complete(&self) -> bool {
        self.policies_complete && self.tools_complete
    }

    fn finish(self) -> Result<Analysis, Vec<AnalysisError>> {
        if self.errors.is_empty() {
            Ok(Analysis {
                diagnostics: self.diagnostics,
                tool_reports: self.tool_reports,
                stopped: false,
            })
        } else {
            Err(self.errors)
        }
    }
}

pub(super) fn run(
    options: &CheckOptions,
    input: &AnalysisInput,
    config: &PolicyConfig,
    policies: &Policies,
    selection: Selection,
    config_path: &Utf8Path,
) -> Result<Analysis, Vec<AnalysisError>> {
    let run_policies = !policies.is_empty();
    let run_tools = crate::tools::any_enabled(&config.tools, selection);
    if run_policies && options.format == OutputFormat::Human {
        report::progress::started("policies", options.color);
    }
    let (sender, receiver) = mpsc::channel();
    let mut state = State::new(!run_policies, !run_tools);
    std::thread::scope(|scope| {
        if run_policies {
            let sender = sender.clone();
            scope.spawn(move || {
                let started = Instant::now();
                let result = analyze_policies(policies, config, input);
                let _ = sender.send(Event::Policies(result, started.elapsed()));
            });
        }
        if run_tools {
            let sender = sender.clone();
            scope.spawn(move || {
                let result = crate::tools::run_each(input, &config.tools, selection, |event| {
                    let _ = sender.send(Event::Tool(event));
                });
                let _ = sender.send(Event::ToolsComplete(result));
            });
        }
        drop(sender);
        receive(&receiver, &mut state, options, input, config_path);
    });
    state.finish()
}

fn analyze_policies(
    policies: &Policies,
    config: &PolicyConfig,
    input: &AnalysisInput,
) -> Result<Vec<Diagnostic>, Vec<AnalysisError>> {
    let mut facts = CodebaseFacts::default();
    if policies.has_syntax() {
        Policies::collect_syntax(input, &mut facts)?;
    }
    if policies.has_semantic() {
        policies.collect_semantic(config, input, &mut facts)?;
    }
    Ok(policies.evaluate_all(&facts))
}

fn receive(
    receiver: &mpsc::Receiver<Event>,
    state: &mut State,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    while !state.complete() {
        let Ok(event) = receiver.recv() else {
            state.errors.push(AnalysisError::new(
                "an Axiom check worker stopped before reporting its result",
            ));
            break;
        };
        handle(event, state, options, input, config_path);
    }
}

fn handle(
    event: Event,
    state: &mut State,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    match event {
        Event::Policies(result, elapsed) => {
            state.policies_complete = true;
            reporting::policies(&result, elapsed, options, input, config_path);
            match result {
                Ok(items) => state.diagnostics = items,
                Err(mut items) => state.errors.append(&mut items),
            }
        }
        Event::Tool(event) => reporting::tool(
            event,
            &mut state.active_tool,
            &mut state.tool_reports,
            options,
            input,
            config_path,
        ),
        Event::ToolsComplete(result) => {
            state.tools_complete = true;
            if let Err(error) = result {
                if options.format == OutputFormat::Human {
                    report::progress::failed(state.active_tool.unwrap_or("tools"));
                }
                state.errors.push(error);
            }
        }
    }
}
