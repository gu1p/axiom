use core::time::Duration;
use std::sync::mpsc;
use std::time::Instant;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, Diagnostic, PolicyConfig};

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
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            tool_reports: Vec::new(),
            errors: Vec::new(),
            policies_complete: false,
            tools_complete: false,
            active_tool: None,
        }
    }

    const fn complete(&self) -> bool {
        self.policies_complete && self.tools_complete
    }

    fn finish(self) -> Result<(Vec<Diagnostic>, Vec<ToolReport>), Vec<AnalysisError>> {
        if self.errors.is_empty() {
            Ok((self.diagnostics, self.tool_reports))
        } else {
            Err(self.errors)
        }
    }
}

pub(super) fn run(
    options: &CheckOptions,
    input: &AnalysisInput,
    config: &PolicyConfig,
    config_path: &Utf8Path,
) -> Result<(Vec<Diagnostic>, Vec<ToolReport>), Vec<AnalysisError>> {
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started("policies");
    }
    let (sender, receiver) = mpsc::channel();
    let mut state = State::new();
    std::thread::scope(|scope| {
        let policy_sender = sender.clone();
        scope.spawn(move || {
            let started = Instant::now();
            let result = super::analyze_policies(config, input);
            let _ = policy_sender.send(Event::Policies(result, started.elapsed()));
        });
        let tool_sender = sender.clone();
        scope.spawn(move || {
            let result = crate::tools::run_each(input, &config.tools, |event| {
                let _ = tool_sender.send(Event::Tool(event));
            });
            let _ = tool_sender.send(Event::ToolsComplete(result));
        });
        drop(sender);
        receive(&receiver, &mut state, options, input, config_path);
    });
    state.finish()
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
            handle_policies(result, elapsed, state, options, input, config_path);
        }
        Event::Tool(event) => handle_tool(event, state, options, input, config_path),
        Event::ToolsComplete(result) => handle_tools_complete(result, state, options),
    }
}

fn handle_policies(
    result: Result<Vec<Diagnostic>, Vec<AnalysisError>>,
    elapsed: Duration,
    state: &mut State,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    state.policies_complete = true;
    if options.format == OutputFormat::Human {
        match &result {
            Ok(items) => {
                report::progress::finished("policies", elapsed);
                report::human::write_policy_diagnostics(items, input, config_path, options.color);
            }
            Err(_) => report::progress::failed("policies"),
        }
    }
    match result {
        Ok(items) => state.diagnostics = items,
        Err(mut items) => state.errors.append(&mut items),
    }
}

fn handle_tool(
    event: RunEvent,
    state: &mut State,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    match event {
        RunEvent::Started(name) => {
            state.active_tool = Some(name);
            if options.format == OutputFormat::Human {
                report::progress::started(name);
            }
        }
        RunEvent::Finished(report, elapsed) => {
            state.active_tool = None;
            if options.format == OutputFormat::Human {
                report::progress::finished(report.name, elapsed);
                report::human::write_tool_report(&report, input, config_path, options.color);
            }
            state.tool_reports.push(report);
        }
    }
}

fn handle_tools_complete(
    result: Result<(), AnalysisError>,
    state: &mut State,
    options: &CheckOptions,
) {
    state.tools_complete = true;
    if let Err(error) = result {
        if options.format == OutputFormat::Human {
            report::progress::failed(state.active_tool.unwrap_or("tools"));
        }
        state.errors.push(error);
    }
}
