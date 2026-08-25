mod parallel;
mod policies;
pub(crate) mod selection;
mod sequential;

use policy_cargo::Workspace;
use policy_core::{AnalysisError, Diagnostic, Level, PolicyConfig};

use crate::{
    args::{CheckOptions, OutputFormat},
    report,
    tools::ToolReport,
};

use self::policies::Policies;
use self::selection::Selection;

pub(super) struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub tool_reports: Vec<ToolReport>,
    pub stopped: bool,
}

pub fn run(options: &CheckOptions) -> u8 {
    let setup = match setup(options) {
        Ok(setup) => setup,
        Err(errors) => return operational(options, &errors, None),
    };
    let input = match setup.workspace.load(&setup.config.sources) {
        Ok(input) => input,
        Err(errors) => return operational(options, &errors, None),
    };
    let config_path_for_report = setup
        .config_path
        .strip_prefix(&input.workspace_root)
        .unwrap_or(&setup.config_path);
    let analysis = match analyze(options, &setup, &input, config_path_for_report) {
        Ok(analysis) => analysis,
        Err(errors) => return operational(options, &errors, Some(&input)),
    };
    write_result(options, &analysis, &input, &setup.config_path);
    exit_code(&analysis)
}

struct Setup {
    workspace: Workspace,
    config_path: camino::Utf8PathBuf,
    config: PolicyConfig,
    selection: Selection,
    policies: Policies,
}

fn setup(options: &CheckOptions) -> Result<Setup, Vec<AnalysisError>> {
    let workspace = match Workspace::discover(options.manifest_path.as_deref()) {
        Ok(workspace) => workspace,
        Err(error) => return Err(vec![AnalysisError::new(error.to_string())]),
    };
    let config_path = options
        .config
        .clone()
        .unwrap_or_else(|| workspace.policy_path());
    let config = match PolicyConfig::load(&config_path) {
        Ok(config) => config,
        Err(error) => return Err(vec![AnalysisError::new(error.to_string())]),
    };
    if let Err(error) = crate::semantic::validate_config(workspace.root(), config.semantic.as_ref())
    {
        return Err(vec![error]);
    }
    let selection = Selection::from_options(options);
    let policies = Policies::prepare(&config, selection, options.ignore_warnings)
        .map_err(|error| vec![error])?;
    Ok(Setup {
        workspace,
        config_path,
        config,
        selection,
        policies,
    })
}

fn analyze(
    options: &CheckOptions,
    setup: &Setup,
    input: &policy_core::AnalysisInput,
    config_path: &camino::Utf8Path,
) -> Result<Analysis, Vec<AnalysisError>> {
    if options.fail_fast {
        sequential::run(
            options,
            input,
            &setup.config,
            &setup.policies,
            setup.selection,
            config_path,
        )
    } else {
        parallel::run(
            options,
            input,
            &setup.config,
            &setup.policies,
            setup.selection,
            config_path,
        )
    }
}

fn write_result(
    options: &CheckOptions,
    analysis: &Analysis,
    input: &policy_core::AnalysisInput,
    config_path: &camino::Utf8Path,
) {
    match options.format {
        OutputFormat::Human => report::summary::write(
            &analysis.diagnostics,
            &analysis.tool_reports,
            input,
            analysis.stopped,
        ),
        OutputFormat::Json => report::check(
            options,
            input,
            config_path,
            &analysis.diagnostics,
            &analysis.tool_reports,
            analysis.stopped,
        ),
    }
}

fn exit_code(analysis: &Analysis) -> u8 {
    u8::from(
        analysis.stopped
            || analysis
                .diagnostics
                .iter()
                .any(|item| item.level == Level::Deny)
            || analysis.tool_reports.iter().any(|report| {
                report
                    .diagnostics
                    .iter()
                    .any(|item| item.level == Level::Deny)
            }),
    )
}

fn operational(
    options: &CheckOptions,
    errors: &[AnalysisError],
    input: Option<&policy_core::AnalysisInput>,
) -> u8 {
    match options.format {
        OutputFormat::Human => report::human::operational(errors, input, options.color),
        OutputFormat::Json => report::json::operational(errors),
    }
    2
}
