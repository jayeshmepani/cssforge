use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use cssforge_core::{
    OutputMode, OutputOptions, Preset, RuleSection, Safety, analyze_workspace,
    apply_selected_plans, discover_css_files, rule_definitions, write_result,
};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "cssforge",
    version,
    about = "Safety-first semantic CSS refactoring and modernization workbench"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Launch the interactive terminal workbench.
    Interactive {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Analyze CSS and report safe/review/unsupported findings.
    Analyze {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum, default_value = "conservative")]
        preset: PresetArg,
        #[arg(long)]
        json: bool,
    },
    /// Generate transformation plans without writing files.
    Plan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum, default_value = "conservative")]
        preset: PresetArg,
        #[arg(long)]
        json: bool,
    },
    /// Apply selected safe transformations.
    Apply {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum, default_value = "conservative")]
        preset: PresetArg,
        #[arg(long, value_enum, default_value = "new-file")]
        output: OutputArg,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long, default_value = ".modern.css")]
        suffix: String,
        /// Required for destructive overwrite modes.
        #[arg(long)]
        yes: bool,
        /// Include REVIEW transformations when a rule produces them.
        #[arg(long)]
        allow_review: bool,
        #[arg(long)]
        report_file: Option<PathBuf>,
    },
    /// List the concrete transformation rules implemented by this build.
    Rules,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PresetArg {
    Analysis,
    Conservative,
    Modern,
    Refactor,
    Aggressive,
}

impl From<PresetArg> for Preset {
    fn from(value: PresetArg) -> Self {
        match value {
            PresetArg::Analysis => Preset::Analysis,
            PresetArg::Conservative => Preset::Conservative,
            PresetArg::Modern => Preset::Modern,
            PresetArg::Refactor => Preset::Refactor,
            PresetArg::Aggressive => Preset::Aggressive,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputArg {
    DryRun,
    NewFile,
    OutDir,
    OverwriteWithBackup,
    Overwrite,
    Patch,
    Stdout,
}

impl From<OutputArg> for OutputMode {
    fn from(value: OutputArg) -> Self {
        match value {
            OutputArg::DryRun => OutputMode::DryRun,
            OutputArg::NewFile => OutputMode::NewFile,
            OutputArg::OutDir => OutputMode::OutDir,
            OutputArg::OverwriteWithBackup => OutputMode::OverwriteWithBackup,
            OutputArg::Overwrite => OutputMode::Overwrite,
            OutputArg::Patch => OutputMode::Patch,
            OutputArg::Stdout => OutputMode::Stdout,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Interactive {
        path: PathBuf::from("."),
    }) {
        Command::Interactive { path } => {
            let outcome = cssforge_tui::run(path)?;
            for text in outcome.stdout {
                println!("{text}");
            }
        }
        Command::Analyze { path, preset, json } => analyze_command(path, preset.into(), json)?,
        Command::Plan { path, preset, json } => plan_command(path, preset.into(), json)?,
        Command::Apply {
            path,
            preset,
            output,
            out_dir,
            suffix,
            yes,
            allow_review,
            report_file,
        } => apply_command(
            path,
            preset.into(),
            output.into(),
            out_dir,
            suffix,
            yes,
            allow_review,
            report_file,
        )?,
        Command::Rules => rules_command(),
    }
    Ok(())
}

fn analyze_command(path: PathBuf, preset: Preset, json: bool) -> Result<()> {
    let files = discover_css_files(&path)?;
    let report = analyze_workspace(&path, &files, &preset.enabled_rules())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let s = &report.summary;
    println!("CSSForge semantic analysis");
    println!("root: {}", path.display());
    println!("files: {}", s.files);
    println!("rules analyzed: {}", s.rules_analyzed);
    println!("SAFE: {}", s.safe);
    println!("REVIEW: {}", s.review);
    println!("UNSAFE: {}", s.unsafe_count);
    println!("UNSUPPORTED: {}", s.unsupported);
    println!("parse errors: {}", s.parse_errors);
    println!();
    for file in &report.files {
        println!(
            "{}  parse={}  styles={} at-rules={} declarations={} !important={} plans={}",
            file.path.display(),
            if file.parse_ok { "ok" } else { "error" },
            file.stats.top_level_style_rules,
            file.stats.top_level_at_rules,
            file.stats.declarations,
            file.stats.important_declarations,
            file.plans.len()
        );
        for finding in &file.findings {
            println!(
                "  [{}] {} — {}",
                finding.safety, finding.title, finding.detail
            );
        }
    }
    Ok(())
}

fn plan_command(path: PathBuf, preset: Preset, json: bool) -> Result<()> {
    let files = discover_css_files(&path)?;
    let report = analyze_workspace(&path, &files, &preset.enabled_rules())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    for file in &report.files {
        for plan in &file.plans {
            println!("{} [{}] {}", plan.id, plan.safety, plan.file.display());
            println!(
                "  rules: {}",
                plan.rules
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("  reason: {}", plan.reason);
            println!(
                "  proof: selector={} specificity={} cascade={} order={} declarations={} important={}",
                plan.proof.selector_set_equivalent,
                plan.proof.specificity_equivalent,
                plan.proof.cascade_context_equivalent,
                plan.proof.source_order_equivalent,
                plan.proof.declarations_exact,
                plan.proof.important_exact,
            );
            println!();
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_command(
    path: PathBuf,
    preset: Preset,
    output_mode: OutputMode,
    out_dir: Option<PathBuf>,
    suffix: String,
    yes: bool,
    allow_review: bool,
    report_file: Option<PathBuf>,
) -> Result<()> {
    let destructive = matches!(
        output_mode,
        OutputMode::Overwrite | OutputMode::OverwriteWithBackup
    );
    if destructive && !yes {
        bail!("destructive output mode requires --yes");
    }

    let files = discover_css_files(&path)?;
    let report = analyze_workspace(&path, &files, &preset.enabled_rules())?;
    let options = OutputOptions {
        mode: output_mode,
        root: path.clone(),
        out_dir,
        suffix,
    };

    let mut changed = 0usize;
    for file in &report.files {
        let original = fs::read_to_string(&file.path)
            .with_context(|| format!("failed to read {}", file.path.display()))?;
        let mut plans = file.plans.clone();
        for plan in &mut plans {
            plan.selected =
                plan.safety == Safety::Safe || (allow_review && plan.safety == Safety::Review);
        }
        let transformed = apply_selected_plans(&original, &plans, allow_review)?;
        if transformed == original {
            continue;
        }
        changed += 1;
        let result = write_result(&file.path, &original, &transformed, &options)?;
        if let Some(stdout) = result.stdout {
            println!("/* {} */\n{}", file.path.display(), stdout);
        } else {
            println!("{}: {}", file.path.display(), result.message);
            if let Some(target) = result.written {
                println!("  -> {}", target.display());
            }
            if let Some(backup) = result.backup {
                println!("  backup: {}", backup.display());
            }
        }
    }

    if let Some(report_file) = report_file {
        let json = serde_json::to_string_pretty(&report)?;
        if let Some(parent) = report_file.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&report_file, json)?;
        eprintln!("report: {}", report_file.display());
    }

    eprintln!("changed files: {changed}");
    Ok(())
}

fn rules_command() {
    println!(
        "DISCLAIMER: CSSForge is a strictly forward semantic modernization & refactoring workbench."
    );
    println!(
        "Backward/reverse demodernization is unsupported. Always maintain Git backups before applying changes.\n"
    );
    for section in RuleSection::ALL {
        println!("=== {} ===", section.label());
        for rule in rule_definitions()
            .into_iter()
            .filter(|r| r.section == section)
        {
            println!(
                "{:<32} {:<22} {:?}",
                rule.id, rule.category, rule.safety_level
            );
            println!("  {}", rule.description);
        }
        println!();
    }
}
