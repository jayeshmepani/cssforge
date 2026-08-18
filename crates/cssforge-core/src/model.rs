use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Safety {
    Safe,
    Review,
    Unsafe,
    Unsupported,
    NoOp,
}

impl Safety {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Review => "REVIEW",
            Self::Unsafe => "UNSAFE",
            Self::Unsupported => "UNSUPPORTED",
            Self::NoOp => "NO_OP",
        }
    }
}

impl fmt::Display for Safety {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SafetyLevel {
    AnalysisOnly,
    FormattingOnly,
    ProvenLocalRefactor,
    SemanticReview,
    Architectural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleId {
    NestPseudoClass,
    NestPseudoElement,
    NestAttribute,
    NestCompound,
    NestDescendant,
    NestCombinator,
    NestMedia,
    NestSupports,
    NestContainer,
    NestStartingStyle,
    FactorSelectorList,
    ConsolidateNot,
    ModernizeIs,
    ModernizeWhere,
    ModernizeMediaRange,
    MergeSameNamedLayer,
    MergeAdjacentMedia,
    MergeAdjacentSupports,
    MergeAdjacentContainer,
    MergeIdenticalScope,
    MergeIdenticalStartingStyle,
    MergeAdjacentIdenticalSelector,
    MergeIdenticalRuleBodies,
    FactorIdenticalStatesWithIs,
    GatherRelatedSelectorRules,
    PruneOverriddenDeclarations,
}

impl RuleId {
    pub const ALL: [RuleId; 26] = [
        RuleId::NestPseudoClass,
        RuleId::NestPseudoElement,
        RuleId::NestAttribute,
        RuleId::NestCompound,
        RuleId::NestDescendant,
        RuleId::NestCombinator,
        RuleId::NestMedia,
        RuleId::NestSupports,
        RuleId::NestContainer,
        RuleId::NestStartingStyle,
        RuleId::FactorSelectorList,
        RuleId::ConsolidateNot,
        RuleId::ModernizeIs,
        RuleId::ModernizeWhere,
        RuleId::ModernizeMediaRange,
        RuleId::MergeSameNamedLayer,
        RuleId::MergeAdjacentMedia,
        RuleId::MergeAdjacentSupports,
        RuleId::MergeAdjacentContainer,
        RuleId::MergeIdenticalScope,
        RuleId::MergeIdenticalStartingStyle,
        RuleId::MergeAdjacentIdenticalSelector,
        RuleId::MergeIdenticalRuleBodies,
        RuleId::FactorIdenticalStatesWithIs,
        RuleId::GatherRelatedSelectorRules,
        RuleId::PruneOverriddenDeclarations,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NestPseudoClass => "nest-pseudo-class",
            Self::NestPseudoElement => "nest-pseudo-element",
            Self::NestAttribute => "nest-attribute",
            Self::NestCompound => "nest-compound",
            Self::NestDescendant => "nest-descendant",
            Self::NestCombinator => "nest-combinator",
            Self::NestMedia => "nest-media",
            Self::NestSupports => "nest-supports",
            Self::NestContainer => "nest-container",
            Self::NestStartingStyle => "nest-starting-style",
            Self::FactorSelectorList => "factor-selector-list",
            Self::ConsolidateNot => "consolidate-not",
            Self::ModernizeIs => "modernize-is",
            Self::ModernizeWhere => "modernize-where",
            Self::ModernizeMediaRange => "modernize-media-range-syntax",
            Self::MergeSameNamedLayer => "merge-same-named-layer",
            Self::MergeAdjacentMedia => "merge-adjacent-media",
            Self::MergeAdjacentSupports => "merge-adjacent-supports",
            Self::MergeAdjacentContainer => "merge-adjacent-container",
            Self::MergeIdenticalScope => "merge-identical-scope",
            Self::MergeIdenticalStartingStyle => "merge-identical-starting-style",
            Self::MergeAdjacentIdenticalSelector => "merge-adjacent-identical-selector",
            Self::MergeIdenticalRuleBodies => "merge-identical-rule-bodies",
            Self::FactorIdenticalStatesWithIs => "factor-identical-states-with-is",
            Self::GatherRelatedSelectorRules => "gather-related-selector-rules",
            Self::PruneOverriddenDeclarations => "prune-overridden-declarations",
        }
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleSection {
    Modernize,
    Refactor,
}

impl RuleSection {
    pub const ALL: [RuleSection; 2] = [RuleSection::Modernize, RuleSection::Refactor];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Modernize => "MODERNIZE (Native Nesting, Range Syntax & Selectors)",
            Self::Refactor => "REFACTOR (Consolidation, Deduplication & Structural Cleanup)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleDefinition {
    pub id: RuleId,
    pub section: RuleSection,
    pub title: &'static str,
    pub category: &'static str,
    pub safety_level: SafetyLevel,
    pub description: &'static str,
}

pub fn rule_definitions() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: RuleId::NestPseudoClass,
            section: RuleSection::Modernize,
            title: "Nest pseudo-classes",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent same-parent pseudo-class rules such as .button:hover -> &:hover.",
        },
        RuleDefinition {
            id: RuleId::NestPseudoElement,
            section: RuleSection::Modernize,
            title: "Nest pseudo-elements",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent same-parent pseudo-elements such as .card::before -> &::before.",
        },
        RuleDefinition {
            id: RuleId::NestAttribute,
            section: RuleSection::Modernize,
            title: "Nest attribute states",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent attribute states such as .button[disabled] -> &[disabled].",
        },
        RuleDefinition {
            id: RuleId::NestCompound,
            section: RuleSection::Modernize,
            title: "Nest compound states",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent compound states such as .item.active -> &.active.",
        },
        RuleDefinition {
            id: RuleId::NestDescendant,
            section: RuleSection::Modernize,
            title: "Nest descendants",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent descendant selectors (.card .title -> .title) with exact relationship proof.",
        },
        RuleDefinition {
            id: RuleId::NestCombinator,
            section: RuleSection::Modernize,
            title: "Nest combinators",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest adjacent child and sibling combinators (> + ~) under their exact parent selector.",
        },
        RuleDefinition {
            id: RuleId::NestMedia,
            section: RuleSection::Modernize,
            title: "Nest local @media",
            category: "At-rules",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest an immediately-following @media block containing matching selector rules.",
        },
        RuleDefinition {
            id: RuleId::NestSupports,
            section: RuleSection::Modernize,
            title: "Nest local @supports",
            category: "At-rules",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest an immediately-following @supports block containing matching selector rules.",
        },
        RuleDefinition {
            id: RuleId::NestContainer,
            section: RuleSection::Modernize,
            title: "Nest local @container",
            category: "At-rules",
            safety_level: SafetyLevel::SemanticReview,
            description: "Nest an immediately-following @container block for matching selector rules.",
        },
        RuleDefinition {
            id: RuleId::NestStartingStyle,
            section: RuleSection::Modernize,
            title: "Nest @starting-style",
            category: "At-rules",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Nest an immediately-following @starting-style block for the matching parent selector.",
        },
        RuleDefinition {
            id: RuleId::FactorSelectorList,
            section: RuleSection::Modernize,
            title: "Factor selector lists",
            category: "Nesting",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Factor comma-separated selectors sharing a common base (.marker, .marker::before -> .marker { &, &::before }).",
        },
        RuleDefinition {
            id: RuleId::ConsolidateNot,
            section: RuleSection::Modernize,
            title: "Consolidate :not() selectors",
            category: "Selectors",
            safety_level: SafetyLevel::SemanticReview,
            description: "Consolidate chained :not() selectors like :not(a):not(b) into :not(a, b) (review required for additive specificity change).",
        },
        RuleDefinition {
            id: RuleId::ModernizeIs,
            section: RuleSection::Modernize,
            title: "Factor with :is()",
            category: "Selectors",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Factor selector-list alternatives with uniform specificity into :is(...) grouping.",
        },
        RuleDefinition {
            id: RuleId::ModernizeWhere,
            section: RuleSection::Modernize,
            title: "Modernize with :where()",
            category: "Selectors",
            safety_level: SafetyLevel::Architectural,
            description: "Factor selector-list alternatives into :where(...) for zero-specificity defaults (review required).",
        },
        RuleDefinition {
            id: RuleId::ModernizeMediaRange,
            section: RuleSection::Modernize,
            title: "Modernize media range syntax",
            category: "At-rules",
            safety_level: SafetyLevel::FormattingOnly,
            description: "Convert min/max-width and min/max-height media features to CSS Range Syntax (e.g. (width >= 800px)).",
        },
        RuleDefinition {
            id: RuleId::MergeSameNamedLayer,
            section: RuleSection::Refactor,
            title: "Merge same named @layer blocks",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Consolidate separated blocks belonging to the same named @layer into their canonical first occurrence.",
        },
        RuleDefinition {
            id: RuleId::MergeAdjacentMedia,
            section: RuleSection::Refactor,
            title: "Merge adjacent @media queries",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive @media blocks having identical query conditions into a single block.",
        },
        RuleDefinition {
            id: RuleId::MergeAdjacentSupports,
            section: RuleSection::Refactor,
            title: "Merge adjacent @supports queries",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive @supports blocks having identical feature conditions into a single block.",
        },
        RuleDefinition {
            id: RuleId::MergeAdjacentContainer,
            section: RuleSection::Refactor,
            title: "Merge adjacent @container queries",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive @container blocks having identical container name and query conditions.",
        },
        RuleDefinition {
            id: RuleId::MergeIdenticalScope,
            section: RuleSection::Refactor,
            title: "Merge adjacent @scope blocks",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive @scope blocks having identical root and limit parameters.",
        },
        RuleDefinition {
            id: RuleId::MergeIdenticalStartingStyle,
            section: RuleSection::Refactor,
            title: "Merge adjacent @starting-style blocks",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive top-level @starting-style blocks into a single block.",
        },
        RuleDefinition {
            id: RuleId::MergeAdjacentIdenticalSelector,
            section: RuleSection::Refactor,
            title: "Merge adjacent identical selectors",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine consecutive style rules sharing the exact same selector when no intervening rules exist.",
        },
        RuleDefinition {
            id: RuleId::MergeIdenticalRuleBodies,
            section: RuleSection::Refactor,
            title: "Merge identical rule bodies",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine selectors sharing identical declaration bodies into a unified comma-separated rule.",
        },
        RuleDefinition {
            id: RuleId::FactorIdenticalStatesWithIs,
            section: RuleSection::Refactor,
            title: "Factor identical states with :is()",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Combine multiple states of the same element sharing identical bodies into &:is(:hover, :focus, ...) form.",
        },
        RuleDefinition {
            id: RuleId::GatherRelatedSelectorRules,
            section: RuleSection::Refactor,
            title: "Gather related selector rules",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::SemanticReview,
            description: "Gather scattered related rules into the strongest existing parent (specificity wins; prefix nest beats appended `&` on a tie). Busy @media/@supports blocks with mixed selectors stay grouped. Review required for cascade crossing.",
        },
        RuleDefinition {
            id: RuleId::PruneOverriddenDeclarations,
            section: RuleSection::Refactor,
            title: "Prune overridden declarations & rules",
            category: "Structural Refactoring",
            safety_level: SafetyLevel::ProvenLocalRefactor,
            description: "Remove dead declarations and entire rules overridden by later identical selectors in the cascade.",
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Analysis,
    Conservative,
    Modern,
    Refactor,
    Aggressive,
    Custom,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Preset::Analysis,
        Preset::Conservative,
        Preset::Modern,
        Preset::Refactor,
        Preset::Aggressive,
        Preset::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Analysis => "Analysis",
            Self::Conservative => "Conservative",
            Self::Modern => "Modern",
            Self::Refactor => "Refactor",
            Self::Aggressive => "Aggressive",
            Self::Custom => "Custom",
        }
    }

    pub fn enabled_rules(self) -> Vec<RuleId> {
        match self {
            Self::Analysis => vec![],
            Self::Conservative => vec![
                RuleId::NestPseudoClass,
                RuleId::NestPseudoElement,
                RuleId::NestAttribute,
                RuleId::NestCompound,
                RuleId::NestDescendant,
                RuleId::NestCombinator,
                RuleId::NestMedia,
                RuleId::NestSupports,
                RuleId::NestStartingStyle,
                RuleId::FactorSelectorList,
                RuleId::ModernizeIs,
                RuleId::ModernizeMediaRange,
                RuleId::MergeSameNamedLayer,
                RuleId::MergeAdjacentMedia,
                RuleId::MergeAdjacentSupports,
                RuleId::MergeAdjacentIdenticalSelector,
                RuleId::MergeIdenticalRuleBodies,
                RuleId::FactorIdenticalStatesWithIs,
            ],
            Self::Modern => vec![
                RuleId::NestPseudoClass,
                RuleId::NestPseudoElement,
                RuleId::NestAttribute,
                RuleId::NestCompound,
                RuleId::NestDescendant,
                RuleId::NestCombinator,
                RuleId::NestMedia,
                RuleId::NestSupports,
                RuleId::NestContainer,
                RuleId::NestStartingStyle,
                RuleId::FactorSelectorList,
                RuleId::ConsolidateNot,
                RuleId::ModernizeIs,
                RuleId::ModernizeMediaRange,
                RuleId::PruneOverriddenDeclarations,
            ],
            Self::Refactor => vec![
                RuleId::NestPseudoClass,
                RuleId::NestPseudoElement,
                RuleId::NestAttribute,
                RuleId::NestCompound,
                RuleId::NestDescendant,
                RuleId::NestCombinator,
                RuleId::NestMedia,
                RuleId::NestSupports,
                RuleId::NestContainer,
                RuleId::NestStartingStyle,
                RuleId::FactorSelectorList,
                RuleId::ConsolidateNot,
                RuleId::ModernizeIs,
                RuleId::ModernizeMediaRange,
                RuleId::MergeSameNamedLayer,
                RuleId::MergeAdjacentMedia,
                RuleId::MergeAdjacentSupports,
                RuleId::MergeAdjacentContainer,
                RuleId::MergeIdenticalScope,
                RuleId::MergeIdenticalStartingStyle,
                RuleId::MergeAdjacentIdenticalSelector,
                RuleId::MergeIdenticalRuleBodies,
                RuleId::FactorIdenticalStatesWithIs,
                RuleId::GatherRelatedSelectorRules,
                RuleId::PruneOverriddenDeclarations,
            ],
            Self::Aggressive => RuleId::ALL.to_vec(),
            Self::Custom => vec![],
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    DryRun,
    NewFile,
    OutDir,
    OverwriteWithBackup,
    Overwrite,
    Patch,
    Stdout,
}

impl OutputMode {
    pub const ALL: [OutputMode; 7] = [
        Self::DryRun,
        Self::NewFile,
        Self::OutDir,
        Self::OverwriteWithBackup,
        Self::Overwrite,
        Self::Patch,
        Self::Stdout,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::DryRun => "Dry run",
            Self::NewFile => "New file (*.modern.css)",
            Self::OutDir => "Output directory",
            Self::OverwriteWithBackup => "Overwrite + backup",
            Self::Overwrite => "Overwrite",
            Self::Patch => "Patch file",
            Self::Stdout => "stdout",
        }
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Proof {
    pub selector_set_equivalent: bool,
    pub specificity_equivalent: bool,
    pub cascade_context_equivalent: bool,
    pub source_order_equivalent: bool,
    pub layer_equivalent: bool,
    pub scope_equivalent: bool,
    pub declarations_exact: bool,
    pub important_exact: bool,
}

impl Proof {
    pub fn safe_local() -> Self {
        Self {
            selector_set_equivalent: true,
            specificity_equivalent: true,
            cascade_context_equivalent: true,
            source_order_equivalent: true,
            layer_equivalent: true,
            scope_equivalent: true,
            declarations_exact: true,
            important_exact: true,
        }
    }

    pub fn all_pass(&self) -> bool {
        self.selector_set_equivalent
            && self.specificity_equivalent
            && self.cascade_context_equivalent
            && self.source_order_equivalent
            && self.layer_equivalent
            && self.scope_equivalent
            && self.declarations_exact
            && self.important_exact
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub id: String,
    pub file: PathBuf,
    pub rules: Vec<RuleId>,
    pub safety: Safety,
    pub source_range: SourceRange,
    pub original: String,
    pub proposed: String,
    pub proof: Proof,
    pub warnings: Vec<String>,
    pub reason: String,
    #[serde(default = "default_true")]
    pub selected: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisStats {
    pub bytes: usize,
    pub top_level_style_rules: usize,
    pub top_level_at_rules: usize,
    pub declarations: usize,
    pub important_declarations: usize,
    pub custom_properties: usize,
    pub duplicate_selectors: usize,
    pub media_rules: usize,
    pub supports_rules: usize,
    pub container_rules: usize,
    pub layer_rules: usize,
    pub scope_rules: usize,
    pub starting_style_rules: usize,
    pub parse_errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub safety: Safety,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReport {
    pub path: PathBuf,
    pub parse_ok: bool,
    pub parse_error: Option<String>,
    pub stats: AnalysisStats,
    pub findings: Vec<Finding>,
    pub plans: Vec<PlanEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub files: usize,
    pub parse_errors: usize,
    pub rules_analyzed: usize,
    pub safe: usize,
    pub review: usize,
    pub unsafe_count: usize,
    pub unsupported: usize,
    pub no_op: usize,
    pub specificity_sensitive: usize,
    pub cascade_sensitive: usize,
    pub layer_sensitive: usize,
    pub scope_sensitive: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceReport {
    pub tool_version: String,
    pub spec_baseline: String,
    pub root: PathBuf,
    pub enabled_rules: Vec<RuleId>,
    pub files: Vec<FileReport>,
    pub summary: WorkspaceSummary,
}
