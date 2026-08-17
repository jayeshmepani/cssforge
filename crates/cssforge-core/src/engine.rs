use crate::{
    model::{
        AnalysisStats, FileReport, Finding, PlanEntry, Proof, RuleId, Safety, SourceRange,
        WorkspaceReport, WorkspaceSummary,
    },
    scanner::{
        NodeKind, SourceNode, count_ascii_case_insensitive_outside_comments,
        count_top_level_declarations, is_whitespace_only, scan_nodes,
    },
};
use anyhow::{Context, Result};
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use similar::TextDiff;
use std::{
    collections::{HashMap, HashSet},
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

const SPEC_BASELINE: &str = "2026-08-17";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity {
    pub ids: usize,
    pub classes: usize,
    pub elements: usize,
}

pub fn calculate_specificity(selector: &str) -> Specificity {
    let mut ids = 0;
    let mut classes = 0;
    let mut elements = 0;
    let bytes = selector.as_bytes();
    let mut i = 0;
    let mut in_attr = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'[' {
            in_attr = true;
            classes += 1;
            i += 1;
            continue;
        }
        if b == b']' {
            in_attr = false;
            i += 1;
            continue;
        }
        if in_attr {
            i += 1;
            continue;
        }

        if b == b'#' {
            ids += 1;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }

        if b == b'.' {
            classes += 1;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }

        if b == b':' {
            if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                elements += 1;
                i += 2;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
                {
                    i += 1;
                }
            } else {
                let start_name = i + 1;
                let mut end_name = start_name;
                while end_name < bytes.len()
                    && (bytes[end_name].is_ascii_alphanumeric() || bytes[end_name] == b'-')
                {
                    end_name += 1;
                }
                let pseudo_name = &selector[start_name..end_name];
                if pseudo_name == "where" {
                    if end_name < bytes.len() && bytes[end_name] == b'(' {
                        if let Some(close_p) = find_matching_paren(selector, end_name) {
                            i = close_p + 1;
                            continue;
                        }
                    }
                } else if pseudo_name == "is" || pseudo_name == "not" || pseudo_name == "has" {
                    if end_name < bytes.len() && bytes[end_name] == b'(' {
                        if let Some(close_p) = find_matching_paren(selector, end_name) {
                            let inner = &selector[end_name + 1..close_p];
                            let max_inner = split_top_level_comma(inner)
                                .into_iter()
                                .map(|s| calculate_specificity(s.trim()))
                                .max()
                                .unwrap_or_default();
                            ids += max_inner.ids;
                            classes += max_inner.classes;
                            elements += max_inner.elements;
                            i = close_p + 1;
                            continue;
                        }
                    }
                    classes += 1;
                } else {
                    classes += 1;
                }
                i = end_name;
            }
            continue;
        }

        if (b.is_ascii_alphabetic() || b == b'*')
            && (i == 0
                || bytes[i - 1].is_ascii_whitespace()
                || bytes[i - 1] == b'>'
                || bytes[i - 1] == b'+'
                || bytes[i - 1] == b'~'
                || bytes[i - 1] == b'|')
        {
            if b != b'*' {
                elements += 1;
            }
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    Specificity {
        ids,
        classes,
        elements,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationKind {
    PseudoClass,
    PseudoElement,
    Attribute,
    Compound,
    Descendant,
    Combinator,
}

impl RelationKind {
    fn rule(self) -> RuleId {
        match self {
            Self::PseudoClass => RuleId::NestPseudoClass,
            Self::PseudoElement => RuleId::NestPseudoElement,
            Self::Attribute => RuleId::NestAttribute,
            Self::Compound => RuleId::NestCompound,
            Self::Descendant => RuleId::NestDescendant,
            Self::Combinator => RuleId::NestCombinator,
        }
    }
}

#[derive(Debug, Clone)]
enum ConditionalInner {
    Direct {
        body_range: Range<usize>,
    },
    Nested {
        nested_selector: String,
        body_range: Range<usize>,
    },
}

#[derive(Debug, Clone)]
enum ClusterChild {
    Style {
        node: SourceNode,
        relation: RelationKind,
        nested_selector: String,
    },
    Conditional {
        node: SourceNode,
        rule: RuleId,
        inners: Vec<ConditionalInner>,
    },
}

impl ClusterChild {
    fn node(&self) -> &SourceNode {
        match self {
            Self::Style { node, .. } | Self::Conditional { node, .. } => node,
        }
    }

    fn rule(&self) -> RuleId {
        match self {
            Self::Style { relation, .. } => relation.rule(),
            Self::Conditional { rule, .. } => *rule,
        }
    }
}

pub fn analyze_file(path: &Path, enabled_rules: &[RuleId]) -> Result<FileReport> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read CSS file {}", path.display()))?;
    analyze_source(path.to_path_buf(), &source, enabled_rules)
}

pub fn analyze_workspace(
    root: &Path,
    files: &[PathBuf],
    enabled_rules: &[RuleId],
) -> Result<WorkspaceReport> {
    let mut reports = Vec::with_capacity(files.len());
    let mut next_id = 1usize;

    for path in files {
        let mut report = analyze_file(path, enabled_rules)?;
        for plan in &mut report.plans {
            plan.id = format!("T-{next_id:06}");
            next_id += 1;
        }
        reports.push(report);
    }

    let mut summary = WorkspaceSummary {
        files: reports.len(),
        ..WorkspaceSummary::default()
    };

    for report in &reports {
        if !report.parse_ok {
            summary.parse_errors += 1;
        }
        summary.rules_analyzed +=
            report.stats.top_level_style_rules + report.stats.top_level_at_rules;
        for plan in &report.plans {
            match plan.safety {
                Safety::Safe => summary.safe += 1,
                Safety::Review => summary.review += 1,
                Safety::Unsafe => summary.unsafe_count += 1,
                Safety::Unsupported => summary.unsupported += 1,
                Safety::NoOp => summary.no_op += 1,
            }
            if plan
                .warnings
                .iter()
                .any(|w| w.to_ascii_lowercase().contains("specificity"))
            {
                summary.specificity_sensitive += 1;
            }
            if plan.warnings.iter().any(|w| {
                w.to_ascii_lowercase().contains("cascade")
                    || w.to_ascii_lowercase().contains("source order")
            }) {
                summary.cascade_sensitive += 1;
            }
            if plan
                .warnings
                .iter()
                .any(|w| w.to_ascii_lowercase().contains("layer"))
            {
                summary.layer_sensitive += 1;
            }
            if plan
                .warnings
                .iter()
                .any(|w| w.to_ascii_lowercase().contains("scope"))
            {
                summary.scope_sensitive += 1;
            }
        }
    }

    Ok(WorkspaceReport {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        spec_baseline: SPEC_BASELINE.to_string(),
        root: root.to_path_buf(),
        enabled_rules: enabled_rules.to_vec(),
        files: reports,
        summary,
    })
}

fn analyze_source(path: PathBuf, source: &str, enabled_rules: &[RuleId]) -> Result<FileReport> {
    let parse_result = StyleSheet::parse(
        source,
        ParserOptions {
            filename: path.display().to_string(),
            error_recovery: false,
            ..ParserOptions::default()
        },
    );

    let parse_error = parse_result.err().map(|err| format!("{err:?}"));
    let parse_ok = parse_error.is_none();
    let nodes = scan_nodes(source, 0..source.len());
    let stats = collect_stats(source, &nodes, parse_ok);
    let mut findings = collect_findings(source, &nodes);

    if let Some(error) = &parse_error {
        findings.push(Finding {
            safety: Safety::Unsupported,
            title: "Semantic parse failed".into(),
            detail: error.clone(),
        });
    }

    let plans = if parse_ok && !enabled_rules.is_empty() {
        build_plans_recursive(&path, source, &nodes, enabled_rules)
    } else {
        Vec::new()
    };

    Ok(FileReport {
        path,
        parse_ok,
        parse_error,
        stats,
        findings,
        plans,
    })
}

fn collect_stats(source: &str, nodes: &[SourceNode], parse_ok: bool) -> AnalysisStats {
    let mut stats = AnalysisStats {
        bytes: source.len(),
        parse_errors: usize::from(!parse_ok),
        important_declarations: count_ascii_case_insensitive_outside_comments(source, "!important"),
        ..AnalysisStats::default()
    };
    let mut selector_counts: HashMap<String, usize> = HashMap::new();

    for node in nodes {
        match &node.kind {
            NodeKind::Style => {
                stats.top_level_style_rules += 1;
                let selector = node.prelude(source).to_string();
                *selector_counts.entry(selector).or_default() += 1;
                if let Some(body) = node.body(source) {
                    stats.declarations += count_top_level_declarations(body);
                    stats.custom_properties += count_custom_properties(body);
                }
            }
            NodeKind::AtBlock { name, .. } => {
                stats.top_level_at_rules += 1;
                match name.as_str() {
                    "media" => stats.media_rules += 1,
                    "supports" => stats.supports_rules += 1,
                    "container" => stats.container_rules += 1,
                    "layer" => stats.layer_rules += 1,
                    "scope" => stats.scope_rules += 1,
                    "starting-style" => stats.starting_style_rules += 1,
                    _ => {}
                }
            }
            NodeKind::AtStatement { .. } => stats.top_level_at_rules += 1,
        }
    }

    stats.duplicate_selectors = selector_counts.values().filter(|&&count| count > 1).count();
    stats
}

fn count_custom_properties(body: &str) -> usize {
    body.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("--") && trimmed.contains(':')
        })
        .count()
}

fn collect_findings(source: &str, nodes: &[SourceNode]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let selectors: HashSet<String> = nodes
        .iter()
        .filter(|n| matches!(&n.kind, NodeKind::Style))
        .map(|n| n.prelude(source).to_string())
        .collect();

    let mut selector_occurrences: HashMap<String, usize> = HashMap::new();

    for node in nodes {
        match &node.kind {
            NodeKind::Style => {
                let selector = node.prelude(source);
                *selector_occurrences.entry(selector.to_string()).or_default() += 1;

                if contains_top_level_comma(selector) {
                    let branches = split_top_level_comma(selector);
                    let specs: Vec<Specificity> = branches.iter().map(|b| calculate_specificity(b.trim())).collect();
                    let has_mixed = specs.windows(2).any(|w| w[0] != w[1]);
                    if has_mixed {
                        findings.push(Finding {
                            safety: Safety::Review,
                            title: "Mixed-specificity selector list detected".into(),
                            detail: format!("{selector}: contains branches with differing specificities; factoring into :is() or parent nesting would raise lower-specificity branches."),
                        });
                    } else {
                        findings.push(Finding {
                            safety: Safety::Review,
                            title: "Selector list kept flat".into(),
                            detail: format!("{selector}: parent selector lists require per-branch specificity proof before native nesting."),
                        });
                    }
                }

                if let Some(base) = bem_base_candidate(selector) {
                    if selectors.contains(base) {
                        findings.push(Finding {
                            safety: Safety::Unsupported,
                            title: "BEM token concatenation is not native nesting".into(),
                            detail: format!("{selector} resembles {base} + a BEM suffix; CSS nesting cannot safely generate &__element or &--modifier."),
                        });
                    }
                }

                if let Some(body) = node.body(source) {
                    if body.trim().is_empty() {
                        findings.push(Finding {
                            safety: Safety::Review,
                            title: "Empty rule block detected".into(),
                            detail: format!("{selector} contains no declarations or nested rules."),
                        });
                    }

                    let mut seen_props: HashMap<String, String> = HashMap::new();
                    for line in body.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("/*") || trimmed.starts_with('*') || !trimmed.contains(':') {
                            continue;
                        }
                        if let Some((prop, val)) = trimmed.split_once(':') {
                            let prop = prop.trim().to_ascii_lowercase();
                            let val = val.trim().trim_end_matches(';').trim().to_string();
                            if let Some(prev_val) = seen_props.get(&prop) {
                                if prev_val == &val {
                                    findings.push(Finding {
                                        safety: Safety::Review,
                                        title: "Exact duplicate declaration detected".into(),
                                        detail: format!("In {selector}: property '{prop}: {val}' is declared multiple times with identical value."),
                                    });
                                }
                            } else {
                                seen_props.insert(prop, val);
                            }
                        }
                    }

                    if selector.contains(" .") && !selector.contains(":has(") {
                        findings.push(Finding {
                            safety: Safety::Review,
                            title: "Potential :has() relational candidate".into(),
                            detail: format!("{selector}: parent-child descendant relationship could be expressed with :has() if container-targeting is intended (advisory)."),
                        });
                    }
                }
            }
            NodeKind::AtBlock { name, .. } => match name.as_str() {
                "layer" => findings.push(Finding {
                    safety: Safety::Review,
                    title: "Cascade layer context detected".into(),
                    detail: "@layer participates in cascade ordering and reverses layer precedence for !important; automatic layer architecture is not applied.".into(),
                }),
                "scope" => findings.push(Finding {
                    safety: Safety::Review,
                    title: "Scope context detected".into(),
                    detail: "@scope adds scope proximity to the cascade; scope architecture remains advisory.".into(),
                }),
                "container" => findings.push(Finding {
                    safety: Safety::Review,
                    title: "Container query context detected".into(),
                    detail: "@container depends on eligible ancestor containers; media-to-container conversion is not inferred from CSS alone.".into(),
                }),
                "starting-style" => findings.push(Finding {
                    safety: Safety::Review,
                    title: "Starting-style context detected".into(),
                    detail: "@starting-style is temporal transition state; this build never invents it from ordinary declarations.".into(),
                }),
                _ => {}
            },
            NodeKind::AtStatement { .. } => {}
        }
    }

    for (sel, count) in selector_occurrences {
        if count > 1 {
            findings.push(Finding {
                safety: Safety::Review,
                title: "Duplicate selector in stylesheet".into(),
                detail: format!("'{sel}' appears {count} times in the stylesheet; non-adjacent occurrences must not be merged across intervening rules."),
            });
        }
    }

    findings
}

fn bem_base_candidate(selector: &str) -> Option<&str> {
    let trimmed = selector.trim();
    let idx = trimmed.find("__").or_else(|| trimmed.find("--"))?;
    if idx == 0 {
        None
    } else {
        Some(&trimmed[..idx])
    }
}

const TRANSPARENT_AT_RULES: &[&str] = &["layer", "scope", "media", "supports", "container"];

fn build_plans_recursive(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled_rules: &[RuleId],
) -> Vec<PlanEntry> {
    let mut plans = build_plans(path, source, nodes, enabled_rules);

    for node in nodes {
        if let NodeKind::AtBlock { name, .. } = &node.kind {
            if TRANSPARENT_AT_RULES.contains(&name.as_str()) {
                let is_covered = plans
                    .iter()
                    .any(|p| p.source_range.start <= node.start && node.end <= p.source_range.end);
                if !is_covered {
                    if let Some(body_range) = &node.body_range {
                        let inner_nodes = scan_nodes(source, body_range.clone());
                        if !inner_nodes.is_empty() {
                            let inner_plans =
                                build_plans_recursive(path, source, &inner_nodes, enabled_rules);
                            plans.extend(inner_plans);
                        }
                    }
                }
            }
        }
    }

    plans.sort_by(|a, b| {
        a.source_range
            .start
            .cmp(&b.source_range.start)
            .then_with(|| b.source_range.end.cmp(&a.source_range.end))
    });
    let mut disjoint = Vec::with_capacity(plans.len());
    let mut last_end = 0;
    for p in plans {
        if p.source_range.start >= last_end {
            last_end = p.source_range.end;
            disjoint.push(p);
        }
    }

    disjoint
}

fn build_plans(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled_rules: &[RuleId],
) -> Vec<PlanEntry> {
    let enabled: HashSet<RuleId> = enabled_rules.iter().copied().collect();
    let mut plans = Vec::new();

    // 1. Structural At-rule refactorings across top-level nodes
    plan_merge_same_named_layers(path, source, nodes, &enabled, &mut plans);
    plan_merge_adjacent_at_blocks(path, source, nodes, &enabled, &mut plans);
    plan_gather_consecutive_conditions_by_selector(path, source, nodes, &enabled, &mut plans);
    plan_merge_adjacent_identical_selectors(path, source, nodes, &enabled, &mut plans);
    plan_gather_related_selector_rules(path, source, nodes, &enabled, &mut plans);
    plan_merge_identical_rule_bodies(path, source, nodes, &enabled, &mut plans);
    plan_factor_identical_states_with_is(path, source, nodes, &enabled, &mut plans);
    plan_factor_multi_selector_cluster_with_is(path, source, nodes, &enabled, &mut plans);
    plan_nest_in_place_adjacent_states(path, source, nodes, &enabled, &mut plans);

    let mut i = 0usize;

    while i < nodes.len() {
        let parent = &nodes[i];

        // ModernizeMediaRange on at-rules
        if enabled.contains(&RuleId::ModernizeMediaRange) {
            if let NodeKind::AtBlock { name, .. } = &parent.kind {
                if name == "media" || name == "container" {
                    let prelude = parent.prelude(source);
                    if let Some(modernized) = modernize_media_query_str(prelude) {
                        plans.push(PlanEntry {
                            id: String::new(),
                            file: path.to_path_buf(),
                            rules: vec![RuleId::ModernizeMediaRange],
                            safety: Safety::Safe,
                            source_range: SourceRange {
                                start: parent.prelude_range.start,
                                end: parent.prelude_range.end,
                            },
                            original: source[parent.prelude_range.clone()].to_string(),
                            proposed: modernized,
                            proof: Proof::safe_local(),
                            warnings: Vec::new(),
                            reason: "Modernize legacy media/container feature syntax to CSS Range Syntax (e.g. (width >= 800px)).".to_string(),
                            selected: true,
                        });
                    }
                }
            }
        }

        if !matches!(&parent.kind, NodeKind::Style) {
            i += 1;
            continue;
        }

        let parent_selector = parent.prelude(source);

        // FactorSelectorList
        if contains_top_level_comma(parent_selector) {
            let parent_indent = line_indent(source, parent.start);
            let parent_body_range = parent.body_range.clone();
            let unit = parent_body_range
                .as_ref()
                .and_then(|r| detect_indent_unit(source, r.clone()))
                .unwrap_or_else(|| "  ".to_string());

            if enabled.contains(&RuleId::FactorSelectorList) {
                if let Some(body_range) = &parent.body_range {
                    let body = &source[body_range.clone()];
                    if let Some(mut factored) =
                        factor_selector_list(parent_selector, body, &parent_indent, &unit)
                    {
                        let branches: Vec<&str> = split_top_level_comma(parent_selector)
                            .into_iter()
                            .map(|s| s.trim())
                            .collect();
                        let base = branches[0];

                        // Check if subsequent adjacent style rules share base (e.g. .notice:hover)
                        let mut cursor = i + 1;
                        let mut prev_end = parent.end;
                        let mut extra_children = Vec::new();

                        while cursor < nodes.len() {
                            let next = &nodes[cursor];
                            if !is_whitespace_only(source, prev_end..next.start) {
                                break;
                            }
                            if matches!(&next.kind, NodeKind::Style) {
                                if let Some((rel, nested_sel)) =
                                    selector_relation(base, next.prelude(source))
                                {
                                    if enabled.contains(&rel.rule()) {
                                        extra_children.push(ClusterChild::Style {
                                            node: next.clone(),
                                            relation: rel,
                                            nested_selector: nested_sel,
                                        });
                                        prev_end = next.end;
                                        cursor += 1;
                                        continue;
                                    }
                                }
                            }
                            break;
                        }

                        let end_offset = if extra_children.is_empty() {
                            parent.end
                        } else {
                            let nested_indent = format!("{parent_indent}{unit}");
                            let inner_decl_indent = format!("{nested_indent}{unit}");
                            let mut extra_rendered = String::new();

                            for ch in &extra_children {
                                if let ClusterChild::Style {
                                    node: ch_node,
                                    nested_selector,
                                    ..
                                } = ch
                                {
                                    extra_rendered.push('\n');
                                    extra_rendered.push_str(&nested_indent);
                                    extra_rendered.push_str(nested_selector.trim());
                                    extra_rendered.push_str(" {\n");
                                    if let Some(ch_body_range) = &ch_node.body_range {
                                        for line in source[ch_body_range.clone()].lines() {
                                            let trimmed = line.trim();
                                            if !trimmed.is_empty() {
                                                extra_rendered.push_str(&inner_decl_indent);
                                                extra_rendered.push_str(trimmed);
                                                extra_rendered.push('\n');
                                            }
                                        }
                                    }
                                    extra_rendered.push_str(&nested_indent);
                                    extra_rendered.push_str("}\n");
                                }
                            }

                            if let Some(close_brace_pos) = factored.rfind('}') {
                                factored.insert_str(close_brace_pos, &extra_rendered);
                            }
                            prev_end
                        };

                        plans.push(PlanEntry {
                            id: String::new(),
                            file: path.to_path_buf(),
                            rules: vec![RuleId::FactorSelectorList],
                            safety: Safety::Safe,
                            source_range: SourceRange {
                                start: parent.start,
                                end: end_offset,
                            },
                            original: source[parent.start..end_offset].to_string(),
                            proposed: factored,
                            proof: Proof::safe_local(),
                            warnings: Vec::new(),
                            reason: "Factor comma-separated selectors sharing a common base element into nested form.".to_string(),
                            selected: true,
                        });
                        i = cursor;
                        continue;
                    }
                }
            }

            if enabled.contains(&RuleId::ModernizeIs) {
                if let Some((factored_sel, uniform)) = factor_with_is(parent_selector) {
                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::ModernizeIs],
                        safety: if uniform { Safety::Safe } else { Safety::Review },
                        source_range: SourceRange {
                            start: parent.prelude_range.start,
                            end: parent.prelude_range.end,
                        },
                        original: source[parent.prelude_range.clone()].to_string(),
                        proposed: factored_sel,
                        proof: Proof {
                            specificity_equivalent: uniform,
                            ..Proof::safe_local()
                        },
                        warnings: if uniform { Vec::new() } else { vec!["Mixed branch specificity: :is() takes the specificity of its most specific argument.".into()] },
                        reason: "Factor common selector prefix/suffix into :is(...) grouping.".to_string(),
                        selected: true,
                    });
                    i += 1;
                    continue;
                }
            }

            if enabled.contains(&RuleId::ModernizeWhere) {
                if let Some(factored_where) = factor_with_where(parent_selector) {
                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::ModernizeWhere],
                        safety: Safety::Review,
                        source_range: SourceRange {
                            start: parent.prelude_range.start,
                            end: parent.prelude_range.end,
                        },
                        original: source[parent.prelude_range.clone()].to_string(),
                        proposed: factored_where,
                        proof: Proof {
                            specificity_equivalent: false,
                            ..Proof::safe_local()
                        },
                        warnings: vec!["Specificity zeroed to 0-0-0 by :where()".into()],
                        reason: "Convert selector list to :where(...) for zero-specificity defaults (review required).".to_string(),
                        selected: true,
                    });
                    i += 1;
                    continue;
                }
            }

            i += 1;
            continue;
        }

        if parent_selector.contains("::") {
            i += 1;
            continue;
        }

        let mut children = Vec::new();
        let mut cursor = i + 1;
        let mut previous_end = parent.end;

        while cursor < nodes.len() {
            let node = &nodes[cursor];
            if !is_whitespace_only(source, previous_end..node.start) {
                break;
            }

            if matches!(&node.kind, NodeKind::Style) {
                if let Some((relation, nested_selector)) =
                    selector_relation(parent_selector, node.prelude(source))
                {
                    if enabled.contains(&relation.rule()) {
                        children.push(ClusterChild::Style {
                            node: node.clone(),
                            relation,
                            nested_selector,
                        });
                        previous_end = node.end;
                        cursor += 1;
                        continue;
                    }
                }
            }

            if let Some(child) = conditional_child(source, parent_selector, node, &enabled) {
                previous_end = node.end;
                children.push(child);
                cursor += 1;
                continue;
            }

            break;
        }

        if !children.is_empty() {
            let last_end = children.last().expect("non-empty cluster").node().end;
            let proposed = render_cluster(source, parent, &children);
            let mut rules = Vec::new();
            for child in &children {
                let rule = child.rule();
                if !rules.contains(&rule) {
                    rules.push(rule);
                }
            }
            plans.push(PlanEntry {
                id: String::new(),
                file: path.to_path_buf(),
                rules,
                safety: Safety::Safe,
                source_range: SourceRange {
                    start: parent.start,
                    end: last_end,
                },
                original: source[parent.start..last_end].to_string(),
                proposed,
                proof: Proof::safe_local(),
                warnings: Vec::new(),
                reason: format!(
                    "{} immediately adjacent rule(s) share the exact parent selector and can be nested without crossing comments or unrelated rules.",
                    children.len()
                ),
                selected: true,
            });
            i = cursor;
        } else {
            if enabled.contains(&RuleId::ConsolidateNot) && matches!(&parent.kind, NodeKind::Style)
            {
                let prelude = parent.prelude(source);
                if let Some((consolidated, _uniform)) = consolidate_not_in_selector(prelude) {
                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::ConsolidateNot],
                        safety: Safety::Review,
                        source_range: SourceRange {
                            start: parent.prelude_range.start,
                            end: parent.prelude_range.end,
                        },
                        original: source[parent.prelude_range.clone()].to_string(),
                        proposed: consolidated,
                        proof: Proof {
                            specificity_equivalent: false,
                            ..Proof::safe_local()
                        },
                        warnings: vec!["Specificity reduced: chained :not() has additive specificity; comma-separated :not() takes only the maximum argument specificity.".into()],
                        reason: "Consolidate chained :not() selectors into a single comma-separated :not() list (review required for specificity drop).".to_string(),
                        selected: true,
                    });
                }
            }
            i += 1;
        }
    }

    plans
}

fn plan_merge_same_named_layers(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::MergeSameNamedLayer) {
        return;
    }
    let mut layer_groups: HashMap<String, Vec<&SourceNode>> = HashMap::new();
    for node in nodes {
        if let NodeKind::AtBlock { name, .. } = &node.kind {
            if name == "layer" {
                let prelude = node.prelude(source).trim();
                if let Some(layer_name) = prelude.strip_prefix("@layer") {
                    let layer_name = layer_name.trim();
                    if !layer_name.is_empty() && !layer_name.contains('{') {
                        layer_groups
                            .entry(layer_name.to_string())
                            .or_default()
                            .push(node);
                    }
                }
            }
        }
    }

    let enabled_rules_vec: Vec<RuleId> = enabled.iter().copied().collect();

    for (layer_name, blocks) in layer_groups {
        if blocks.len() > 1 {
            let first = blocks[0];
            let parent_indent = line_indent(source, first.start);
            let first_body_range = first.body_range.as_ref().unwrap();
            let unit = detect_indent_unit(source, first_body_range.clone())
                .unwrap_or_else(|| "  ".to_string());
            let nested_indent = format!("{parent_indent}{unit}");

            let mut merged_body = String::new();
            for b in &blocks {
                if let Some(body_range) = &b.body_range {
                    let inner_nodes = scan_nodes(source, body_range.clone());
                    let inner_plans =
                        build_plans_recursive(path, source, &inner_nodes, &enabled_rules_vec);
                    let body_text = &source[body_range.clone()];
                    let modernized_body = if inner_plans.is_empty() {
                        body_text.to_string()
                    } else {
                        let mut local_plans = Vec::new();
                        for p in inner_plans {
                            if p.source_range.start >= body_range.start
                                && p.source_range.end <= body_range.end
                            {
                                let mut local_p = p.clone();
                                local_p.source_range.start -= body_range.start;
                                local_p.source_range.end -= body_range.start;
                                local_plans.push(local_p);
                            }
                        }
                        apply_selected_plans(body_text, &local_plans, true)
                            .unwrap_or_else(|_| body_text.to_string())
                    };

                    for line in modernized_body.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            merged_body.push_str(&nested_indent);
                            merged_body.push_str(trimmed);
                            merged_body.push('\n');
                        }
                    }
                }
            }

            let proposed_first =
                format!("{parent_indent}@layer {layer_name} {{\n{merged_body}{parent_indent}}}");
            plans.push(PlanEntry {
                id: String::new(),
                file: path.to_path_buf(),
                rules: vec![RuleId::MergeSameNamedLayer],
                safety: Safety::Safe,
                source_range: SourceRange {
                    start: first.start,
                    end: first.end,
                },
                original: source[first.start..first.end].to_string(),
                proposed: proposed_first,
                proof: Proof::safe_local(),
                warnings: Vec::new(),
                reason: format!(
                    "Consolidate {} separated blocks of @layer {} into first occurrence.",
                    blocks.len(),
                    layer_name
                ),
                selected: true,
            });

            for subsequent in &blocks[1..] {
                plans.push(PlanEntry {
                    id: String::new(),
                    file: path.to_path_buf(),
                    rules: vec![RuleId::MergeSameNamedLayer],
                    safety: Safety::Safe,
                    source_range: SourceRange {
                        start: subsequent.start,
                        end: subsequent.end,
                    },
                    original: source[subsequent.start..subsequent.end].to_string(),
                    proposed: String::new(),
                    proof: Proof::safe_local(),
                    warnings: Vec::new(),
                    reason: format!(
                        "Remove consolidated subsequent block of @layer {}.",
                        layer_name
                    ),
                    selected: true,
                });
            }
        }
    }
}

fn plan_merge_adjacent_at_blocks(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if let NodeKind::AtBlock { name, .. } = &first.kind {
            let rule = match name.as_str() {
                "media" => RuleId::MergeAdjacentMedia,
                "supports" => RuleId::MergeAdjacentSupports,
                "container" => RuleId::MergeAdjacentContainer,
                "scope" => RuleId::MergeIdenticalScope,
                "starting-style" => RuleId::MergeIdenticalStartingStyle,
                _ => {
                    i += 1;
                    continue;
                }
            };

            if !enabled.contains(&rule) {
                i += 1;
                continue;
            }

            let first_prelude = first.prelude(source).trim();
            let mut cluster = vec![first];
            let mut cursor = i + 1;
            let mut prev_end = first.end;

            while cursor < nodes.len() {
                let next = &nodes[cursor];
                if !is_whitespace_only(source, prev_end..next.start) {
                    break;
                }
                if let NodeKind::AtBlock {
                    name: next_name, ..
                } = &next.kind
                {
                    if next_name == name && next.prelude(source).trim() == first_prelude {
                        cluster.push(next);
                        prev_end = next.end;
                        cursor += 1;
                        continue;
                    }
                }
                break;
            }

            if cluster.len() > 1 {
                let last = cluster.last().unwrap();
                let parent_indent = line_indent(source, first.start);
                let first_body_range = first.body_range.as_ref().unwrap();
                let unit = detect_indent_unit(source, first_body_range.clone())
                    .unwrap_or_else(|| "  ".to_string());
                let nested_indent = format!("{parent_indent}{unit}");

                let mut merged_body = String::new();
                for c in &cluster {
                    if let Some(body_range) = &c.body_range {
                        let body_text = &source[body_range.clone()];
                        for line in body_text.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                merged_body.push_str(&nested_indent);
                                merged_body.push_str(trimmed);
                                merged_body.push('\n');
                            }
                        }
                    }
                }

                let proposed =
                    format!("{parent_indent}{first_prelude} {{\n{merged_body}{parent_indent}}}");
                plans.push(PlanEntry {
                    id: String::new(),
                    file: path.to_path_buf(),
                    rules: vec![rule],
                    safety: Safety::Safe,
                    source_range: SourceRange {
                        start: first.start,
                        end: last.end,
                    },
                    original: source[first.start..last.end].to_string(),
                    proposed,
                    proof: Proof::safe_local(),
                    warnings: Vec::new(),
                    reason: format!(
                        "Merge {} adjacent identical {} blocks into a single block.",
                        cluster.len(),
                        first_prelude
                    ),
                    selected: true,
                });
                i = cursor;
                continue;
            }
        }
        i += 1;
    }
}

fn plan_gather_consecutive_conditions_by_selector(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::NestMedia) && !enabled.contains(&RuleId::NestSupports) {
        return;
    }

    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if let NodeKind::AtBlock { name, .. } = &first.kind {
            if name == "media" || name == "supports" {
                if let Some(target_sel) = extract_single_style_selector(source, first) {
                    let mut cluster = vec![first];
                    let mut cursor = i + 1;
                    let mut prev_end = first.end;

                    while cursor < nodes.len() {
                        let next = &nodes[cursor];
                        if !is_whitespace_only(source, prev_end..next.start) {
                            break;
                        }
                        if let NodeKind::AtBlock {
                            name: next_name, ..
                        } = &next.kind
                        {
                            if next_name == "media" || next_name == "supports" {
                                if let Some(next_sel) = extract_single_style_selector(source, next)
                                {
                                    if next_sel == target_sel {
                                        cluster.push(next);
                                        prev_end = next.end;
                                        cursor += 1;
                                        continue;
                                    }
                                }
                            }
                        }
                        break;
                    }

                    if cluster.len() > 1 {
                        let last = cluster.last().unwrap();
                        let parent_indent = line_indent(source, first.start);
                        let first_body_range = first.body_range.as_ref().unwrap();
                        let unit = detect_indent_unit(source, first_body_range.clone())
                            .unwrap_or_else(|| "  ".to_string());
                        let nested_indent = format!("{parent_indent}{unit}");
                        let inner_decl_indent = format!("{nested_indent}{unit}");

                        let mut body_out = String::new();
                        for (idx, &c) in cluster.iter().enumerate() {
                            if idx > 0 {
                                body_out.push('\n');
                            }
                            let at_header = c.prelude(source).trim();
                            body_out.push_str(&nested_indent);
                            body_out.push_str(at_header);
                            body_out.push_str(" {\n");

                            let c_body_range = c.body_range.as_ref().unwrap();
                            let inner_nodes = scan_nodes(source, c_body_range.clone());
                            for in_node in &inner_nodes {
                                if let Some(in_body_range) = &in_node.body_range {
                                    for line in source[in_body_range.clone()].lines() {
                                        let trimmed = line.trim();
                                        if !trimmed.is_empty() {
                                            body_out.push_str(&inner_decl_indent);
                                            body_out.push_str(trimmed);
                                            body_out.push('\n');
                                        }
                                    }
                                }
                            }

                            body_out.push_str(&nested_indent);
                            body_out.push_str("}\n");
                        }

                        let proposed =
                            format!("{parent_indent}{target_sel} {{\n{body_out}{parent_indent}}}");
                        plans.push(PlanEntry {
                            id: String::new(),
                            file: path.to_path_buf(),
                            rules: vec![RuleId::NestMedia, RuleId::NestSupports],
                            safety: Safety::Safe,
                            source_range: SourceRange {
                                start: first.start,
                                end: last.end,
                            },
                            original: source[first.start..last.end].to_string(),
                            proposed,
                            proof: Proof::safe_local(),
                            warnings: Vec::new(),
                            reason: format!(
                                "Gather {} consecutive condition blocks targeting '{}' into a single component rule.",
                                cluster.len(),
                                target_sel
                            ),
                            selected: true,
                        });
                        i = cursor;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
}

fn extract_single_style_selector<'a>(source: &'a str, at_node: &SourceNode) -> Option<&'a str> {
    let body_range = at_node.body_range.as_ref()?;
    let inner_nodes = scan_nodes(source, body_range.clone());
    if inner_nodes.len() == 1 && matches!(&inner_nodes[0].kind, NodeKind::Style) {
        Some(inner_nodes[0].prelude(source).trim())
    } else {
        None
    }
}

fn plan_nest_in_place_adjacent_states(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::NestPseudoClass) {
        return;
    }

    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if matches!(&first.kind, NodeKind::Style) {
            let first_sel = first.prelude(source).trim();
            if let Some(base) = extract_base_target(first_sel) {
                let mut cluster = vec![first];
                let mut cursor = i + 1;
                let mut prev_end = first.end;

                while cursor < nodes.len() {
                    let next = &nodes[cursor];
                    if !is_whitespace_only(source, prev_end..next.start) {
                        break;
                    }
                    if matches!(&next.kind, NodeKind::Style) {
                        let next_sel = next.prelude(source).trim();
                        if let Some(next_base) = extract_base_target(next_sel) {
                            if next_base == base {
                                cluster.push(next);
                                prev_end = next.end;
                                cursor += 1;
                                continue;
                            }
                        }
                    }
                    break;
                }

                if cluster.len() > 1 {
                    let last = cluster.last().unwrap();
                    let parent_indent = line_indent(source, first.start);
                    let first_body_range = first.body_range.as_ref().unwrap();
                    let unit = detect_indent_unit(source, first_body_range.clone())
                        .unwrap_or_else(|| "  ".to_string());
                    let nested_indent = format!("{parent_indent}{unit}");
                    let inner_decl_indent = format!("{nested_indent}{unit}");

                    let mut out = format!("{parent_indent}{base} {{\n");
                    for (idx, &c) in cluster.iter().enumerate() {
                        if idx > 0 {
                            out.push('\n');
                        }
                        let c_sel = c.prelude(source).trim();
                        let remainder = &c_sel[base.len()..];
                        let nested_sel = if remainder.starts_with(':')
                            || remainder.starts_with('[')
                            || remainder.starts_with('.')
                            || remainder.starts_with('#')
                        {
                            format!("&{remainder}")
                        } else {
                            remainder.trim().to_string()
                        };

                        out.push_str(&nested_indent);
                        out.push_str(&nested_sel);
                        out.push_str(" {\n");

                        if let Some(c_body_range) = &c.body_range {
                            for line in source[c_body_range.clone()].lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    out.push_str(&inner_decl_indent);
                                    out.push_str(trimmed);
                                    out.push('\n');
                                }
                            }
                        }

                        out.push_str(&nested_indent);
                        out.push_str("}\n");
                    }

                    out.push_str(&parent_indent);
                    out.push('}');

                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::NestPseudoClass, RuleId::NestAttribute],
                        safety: Safety::Safe,
                        source_range: SourceRange {
                            start: first.start,
                            end: last.end,
                        },
                        original: source[first.start..last.end].to_string(),
                        proposed: out,
                        proof: Proof::safe_local(),
                        warnings: Vec::new(),
                        reason: format!(
                            "Nest {} adjacent state rules for '{}' in place without moving.",
                            cluster.len(),
                            base
                        ),
                        selected: true,
                    });
                    i = cursor;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn extract_base_target(selector: &str) -> Option<&str> {
    if contains_top_level_comma(selector) {
        return None;
    }
    if let Some(pos) = selector.find(':') {
        if pos > 0 && !selector[pos..].starts_with("::") {
            let base = &selector[..pos];
            if !base.is_empty() {
                return Some(base);
            }
        }
    }
    if let Some(pos) = selector.find('[') {
        if pos > 0 {
            let base = &selector[..pos];
            if !base.is_empty() {
                return Some(base);
            }
        }
    }
    None
}

fn parse_rule_body_items(body_str: &str) -> (Vec<String>, Vec<String>) {
    let mut declarations = Vec::new();
    let mut nested_rules = Vec::new();

    let mut depth = 0usize;
    let mut current_block = String::new();
    let mut current_decl = String::new();
    let mut in_comment = false;
    let bytes = body_str.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if in_comment {
            current_decl.push(bytes[i] as char);
            current_block.push(bytes[i] as char);
            if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                current_decl.push('/');
                current_block.push('/');
                i += 2;
                in_comment = false;
                continue;
            }
            i += 1;
            continue;
        }

        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            in_comment = true;
            current_decl.push('/');
            current_decl.push('*');
            current_block.push('/');
            current_block.push('*');
            i += 2;
            continue;
        }

        let b = bytes[i];
        if b == b'{' {
            depth += 1;
            if depth == 1 {
                current_block = current_decl.clone();
                current_decl.clear();
            }
            current_block.push('{');
            i += 1;
            continue;
        } else if b == b'}' {
            if depth > 0 {
                depth -= 1;
                current_block.push('}');
                if depth == 0 {
                    let trimmed = current_block.trim().to_string();
                    if !trimmed.is_empty() {
                        nested_rules.push(trimmed);
                    }
                    current_block.clear();
                    current_decl.clear();
                }
            }
            i += 1;
            continue;
        }

        if depth > 0 {
            current_block.push(b as char);
        } else {
            if b == b';' {
                current_decl.push(';');
                let trimmed = current_decl.trim().to_string();
                if !trimmed.is_empty() {
                    declarations.push(trimmed);
                }
                current_decl.clear();
            } else if b == b'\n' {
                let trimmed = current_decl.trim();
                if !trimmed.is_empty() && trimmed.contains(':') && !trimmed.ends_with('{') {
                    let rest = body_str[i + 1..].trim_start();
                    if !rest.starts_with('{') {
                        declarations.push(trimmed.to_string());
                        current_decl.clear();
                    } else {
                        current_decl.push('\n');
                    }
                } else {
                    current_decl.push('\n');
                }
            } else {
                current_decl.push(b as char);
            }
        }
        i += 1;
    }

    let trailing_decl = current_decl.trim().to_string();
    if !trailing_decl.is_empty() && trailing_decl.contains(':') {
        declarations.push(trailing_decl);
    }

    (declarations, nested_rules)
}

fn extract_related_nested_selector(base: &str, candidate_sel: &str) -> Option<String> {
    if candidate_sel == base {
        return None;
    }
    if !candidate_sel.starts_with(base) {
        return None;
    }
    let rem_raw = &candidate_sel[base.len()..];
    let rem = rem_raw.trim_start();
    if rem.is_empty() {
        return None;
    }
    // Only attach '&' when the suffix is directly touching the base (no whitespace).
    // e.g. `.foo:hover` → `&:hover` but `.foo :not(*)` → `:not(*)` (descendant, no &).
    let directly_attached = !rem_raw.starts_with(|c: char| c.is_whitespace());
    if rem.starts_with(':') || rem.starts_with('[') || rem.starts_with('.') || rem.starts_with('#') {
        if directly_attached {
            return Some(format!("&{rem}"));
        } else {
            // Whitespace-separated: descendant combinator, no &
            return Some(rem.to_string());
        }
    }
    if rem.starts_with('+') || rem.starts_with('>') || rem.starts_with('~') {
        let first_char = &rem[..1];
        let rest = rem[1..].trim_start();
        return Some(format!("{first_char} {rest}"));
    }
    if rem_raw.starts_with(' ') {
        return Some(rem.to_string());
    }
    None
}

fn format_merged_rule(
    first_sel: &str,
    parent_indent: &str,
    unit: &str,
    cluster: &[&SourceNode],
    source: &str,
) -> String {
    let nested_indent = format!("{parent_indent}{unit}");
    let inner_indent = format!("{nested_indent}{unit}");

    let mut all_decls = Vec::new();
    let mut all_nested_rules = Vec::new();

    for c in cluster {
        let cand_sel = c.prelude(source).trim();
        if let Some(body_range) = &c.body_range {
            let body_str = &source[body_range.clone()];
            if cand_sel == first_sel {
                let (decls, nested) = parse_rule_body_items(body_str);
                all_decls.extend(decls);
                all_nested_rules.extend(nested);
            } else if let Some(rel_sel) = extract_related_nested_selector(first_sel, cand_sel) {
                let (decls, nested) = parse_rule_body_items(body_str);
                if nested.is_empty() {
                    let mut rel_body = String::new();
                    for d in &decls {
                        rel_body.push_str(&format!("{d}\n"));
                    }
                    all_nested_rules.push(format!("{rel_sel} {{\n    {rel_body}}}"));
                } else {
                    let mut rel_body_lines = Vec::new();
                    for d in &decls {
                        rel_body_lines.push(format!("    {d}"));
                    }
                    for nr in &nested {
                        rel_body_lines.push(nr.clone());
                    }
                    let rel_body = rel_body_lines.join("\n");
                    all_nested_rules.push(format!("{rel_sel} {{\n{rel_body}\n}}"));
                }
            }
        }
    }

    let mut body_lines = Vec::new();

    for d in &all_decls {
        body_lines.push(format!("{nested_indent}{d}"));
    }

    if !all_decls.is_empty() && !all_nested_rules.is_empty() {
        body_lines.push(String::new());
    }

    for (idx, nr) in all_nested_rules.iter().enumerate() {
        let lines: Vec<&str> = nr.lines().collect();
        if lines.is_empty() {
            continue;
        }
        let first_line = lines[0].trim();
        body_lines.push(format!("{nested_indent}{first_line}"));

        for mid_line in &lines[1..lines.len().saturating_sub(1)] {
            let m_trimmed = mid_line.trim();
            if m_trimmed.is_empty() {
                body_lines.push(String::new());
            } else {
                body_lines.push(format!("{inner_indent}{m_trimmed}"));
            }
        }

        if lines.len() > 1 {
            let last_line = lines.last().unwrap().trim();
            body_lines.push(format!("{nested_indent}{last_line}"));
        }

        if idx < all_nested_rules.len() - 1 {
            body_lines.push(String::new());
        }
    }

    let body_content = body_lines.join("\n");
    format!("{parent_indent}{first_sel} {{\n{body_content}\n{parent_indent}}}")
}

fn plan_merge_adjacent_identical_selectors(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::MergeAdjacentIdenticalSelector) {
        return;
    }
    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if matches!(&first.kind, NodeKind::Style) {
            let first_sel = first.prelude(source).trim();
            let mut cluster = vec![first];
            let mut cursor = i + 1;
            let mut prev_end = first.end;

            while cursor < nodes.len() {
                let next = &nodes[cursor];
                if !is_whitespace_only(source, prev_end..next.start) {
                    break;
                }
                if matches!(&next.kind, NodeKind::Style) && next.prelude(source).trim() == first_sel
                {
                    cluster.push(next);
                    prev_end = next.end;
                    cursor += 1;
                    continue;
                }
                break;
            }

            if cluster.len() > 1 {
                let last = cluster.last().unwrap();
                let parent_indent = line_indent(source, first.start);
                let first_body_range = first.body_range.as_ref().unwrap();
                let unit = detect_indent_unit(source, first_body_range.clone())
                    .unwrap_or_else(|| "    ".to_string());

                let proposed = format_merged_rule(first_sel, &parent_indent, &unit, &cluster, source);

                plans.push(PlanEntry {
                    id: String::new(),
                    file: path.to_path_buf(),
                    rules: vec![RuleId::MergeAdjacentIdenticalSelector],
                    safety: Safety::Safe,
                    source_range: SourceRange {
                        start: first.start,
                        end: last.end,
                    },
                    original: source[first.start..last.end].to_string(),
                    proposed,
                    proof: Proof::safe_local(),
                    warnings: Vec::new(),
                    reason: format!(
                        "Merge {} adjacent identical selector rules for '{}' into a single block.",
                        cluster.len(),
                        first_sel
                    ),
                    selected: true,
                });
                i = cursor;
                continue;
            }
        }
        i += 1;
    }
}

fn plan_gather_related_selector_rules(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::GatherRelatedSelectorRules) {
        return;
    }

    let mut base_candidates = Vec::new();

    for node in nodes {
        if matches!(&node.kind, NodeKind::Style) {
            let sel = node.prelude(source).trim();
            if !sel.is_empty() && !sel.starts_with('&') && !sel.starts_with('+') && !sel.starts_with('>') && !sel.starts_with('~') {
                let base = if let Some(colon_pos) = sel.find(':') {
                    sel[..colon_pos].trim()
                } else if let Some(bracket_pos) = sel.find('[') {
                    sel[..bracket_pos].trim()
                } else {
                    sel
                };
                if !base.is_empty() && !base_candidates.contains(&base) {
                    base_candidates.push(base);
                }
            }
        }
    }

    for base in base_candidates {
        let mut cluster: Vec<&SourceNode> = Vec::new();
        for node in nodes {
            if matches!(&node.kind, NodeKind::Style) {
                let sel = node.prelude(source).trim();
                if sel == base || extract_related_nested_selector(base, sel).is_some() {
                    cluster.push(node);
                }
            }
        }

        if cluster.len() > 1 {
            let mut is_non_adjacent = false;
            for window in cluster.windows(2) {
                let prev = window[0];
                let next = window[1];
                if !is_whitespace_only(source, prev.end..next.start) {
                    is_non_adjacent = true;
                    break;
                }
            }

            if is_non_adjacent {
                let first = cluster[0];
                let parent_indent = line_indent(source, first.start);
                let first_body_range = first.body_range.as_ref().unwrap();
                let unit = detect_indent_unit(source, first_body_range.clone())
                    .unwrap_or_else(|| "    ".to_string());

                let proposed = format_merged_rule(base, &parent_indent, &unit, &cluster, source);

                plans.push(PlanEntry {
                    id: String::new(),
                    file: path.to_path_buf(),
                    rules: vec![RuleId::GatherRelatedSelectorRules],
                    safety: Safety::Review,
                    source_range: SourceRange {
                        start: first.start,
                        end: first.end,
                    },
                    original: source[first.start..first.end].to_string(),
                    proposed,
                    proof: Proof {
                        selector_set_equivalent: true,
                        specificity_equivalent: true,
                        cascade_context_equivalent: false,
                        source_order_equivalent: false,
                        layer_equivalent: true,
                        scope_equivalent: true,
                        declarations_exact: true,
                        important_exact: true,
                    },
                    warnings: vec![format!(
                        "Gathered {} related occurrences of '{}' across lines; review cascade ordering.",
                        cluster.len(),
                        base
                    )],
                    reason: format!(
                        "Gather {} related rules for '{}' into the canonical first selector block.",
                        cluster.len(),
                        base
                    ),
                    selected: true,
                });

                for sec in &cluster[1..] {
                    let mut sec_end = sec.end;
                    if source[sec_end..].starts_with("\r\n") {
                        sec_end += 2;
                    } else if source[sec_end..].starts_with('\n') {
                        sec_end += 1;
                    }

                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::GatherRelatedSelectorRules],
                        safety: Safety::Review,
                        source_range: SourceRange {
                            start: sec.start,
                            end: sec_end,
                        },
                        original: source[sec.start..sec_end].to_string(),
                        proposed: String::new(),
                        proof: Proof::safe_local(),
                        warnings: Vec::new(),
                        reason: format!(
                            "Remove non-adjacent gathered rule for '{}' at line {}.",
                            base,
                            line_number(source, sec.start)
                        ),
                        selected: true,
                    });
                }
            }
        }
    }
}

fn plan_factor_identical_states_with_is(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::FactorIdenticalStatesWithIs) {
        return;
    }
    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if matches!(&first.kind, NodeKind::Style) {
            let first_sel = first.prelude(source).trim();
            if let Some(colon_pos) = first_sel.find(':') {
                if !first_sel[colon_pos..].starts_with("::") {
                    let base = &first_sel[..colon_pos];
                    if !base.is_empty() && !base.contains(' ') {
                        let first_body = first.body(source).unwrap_or("").trim();
                        let mut cluster = vec![first];
                        let mut cursor = i + 1;
                        let mut prev_end = first.end;

                        while cursor < nodes.len() {
                            let next = &nodes[cursor];
                            if !is_whitespace_only(source, prev_end..next.start) {
                                break;
                            }
                            if matches!(&next.kind, NodeKind::Style) {
                                let next_sel = next.prelude(source).trim();
                                if next_sel.starts_with(base)
                                    && next_sel[base.len()..].starts_with(':')
                                    && !next_sel[base.len()..].starts_with("::")
                                    && next.body(source).unwrap_or("").trim() == first_body
                                {
                                    cluster.push(next);
                                    prev_end = next.end;
                                    cursor += 1;
                                    continue;
                                }
                            }
                            break;
                        }

                        if cluster.len() > 1 {
                            let last = cluster.last().unwrap();
                            let pseudos: Vec<&str> = cluster
                                .iter()
                                .map(|c| {
                                    let s = c.prelude(source).trim();
                                    &s[base.len()..]
                                })
                                .collect();
                            let is_inner = pseudos.join(", ");
                            let parent_indent = line_indent(source, first.start);
                            let first_body_range = first.body_range.as_ref().unwrap();
                            let unit = detect_indent_unit(source, first_body_range.clone())
                                .unwrap_or_else(|| "  ".to_string());
                            let nested_indent = format!("{parent_indent}{unit}");
                            let inner_decl_indent = format!("{nested_indent}{unit}");

                            let mut decls = String::new();
                            for line in source[first_body_range.clone()].lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    decls.push_str(&inner_decl_indent);
                                    decls.push_str(trimmed);
                                    decls.push('\n');
                                }
                            }

                            let proposed = format!(
                                "{parent_indent}{base} {{\n{nested_indent}&:is({is_inner}) {{\n{decls}{nested_indent}}}\n{parent_indent}}}"
                            );
                            plans.push(PlanEntry {
                                id: String::new(),
                                file: path.to_path_buf(),
                                rules: vec![RuleId::FactorIdenticalStatesWithIs],
                                safety: Safety::Safe,
                                source_range: SourceRange {
                                    start: first.start,
                                    end: last.end,
                                },
                                original: source[first.start..last.end].to_string(),
                                proposed,
                                proof: Proof::safe_local(),
                                warnings: Vec::new(),
                                reason: format!(
                                    "Factor {} identical state rules for '{}' into &:is({}) form.",
                                    cluster.len(),
                                    base,
                                    is_inner
                                ),
                                selected: true,
                            });
                            i = cursor;
                            continue;
                        }
                    }
                }
            }
        }
        i += 1;
    }
}

fn plan_factor_multi_selector_cluster_with_is(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::ModernizeIs) {
        return;
    }

    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if matches!(&first.kind, NodeKind::Style) {
            let first_sel = first.prelude(source).trim();
            if let Some((base_prefixes, first_suffix)) = extract_multi_branch_pattern(first_sel) {
                let mut cluster = vec![(first, first_suffix)];
                let mut cursor = i + 1;
                let mut prev_end = first.end;

                while cursor < nodes.len() {
                    let next = &nodes[cursor];
                    if !is_whitespace_only(source, prev_end..next.start) {
                        break;
                    }
                    if matches!(&next.kind, NodeKind::Style) {
                        let next_sel = next.prelude(source).trim();
                        if let Some((next_prefixes, next_suffix)) =
                            extract_multi_branch_pattern(next_sel)
                        {
                            if next_prefixes == base_prefixes {
                                cluster.push((next, next_suffix));
                                prev_end = next.end;
                                cursor += 1;
                                continue;
                            }
                        }
                    }
                    break;
                }

                if cluster.len() > 1 {
                    let (last_node, _) = cluster.last().unwrap();
                    let parent_indent = line_indent(source, first.start);
                    let first_body_range = first.body_range.as_ref().unwrap();
                    let unit = detect_indent_unit(source, first_body_range.clone())
                        .unwrap_or_else(|| "  ".to_string());
                    let nested_indent = format!("{parent_indent}{unit}");
                    let inner_decl_indent = format!("{nested_indent}{unit}");

                    let is_header = format!(":is({})", base_prefixes.join(", "));
                    let mut out = format!("{parent_indent}{is_header} {{\n");
                    let mut has_direct_decls = false;

                    // 1. Direct declarations from base rules (suffix == None)
                    for &(c_node, ref suffix) in &cluster {
                        if suffix.is_none() {
                            if let Some(c_body_range) = &c_node.body_range {
                                for line in source[c_body_range.clone()].lines() {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        out.push_str(&nested_indent);
                                        out.push_str(trimmed);
                                        out.push('\n');
                                        has_direct_decls = true;
                                    }
                                }
                            }
                        }
                    }

                    // 2. Nested child rules (suffix == Some(sub_sel))
                    for (c_idx, &(c_node, ref suffix)) in cluster.iter().enumerate() {
                        if let Some(sub_sel) = suffix {
                            if has_direct_decls || c_idx > 0 {
                                out.push('\n');
                            }
                            out.push_str(&nested_indent);
                            out.push_str(sub_sel);
                            out.push_str(" {\n");
                            if let Some(c_body_range) = &c_node.body_range {
                                for line in source[c_body_range.clone()].lines() {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        out.push_str(&inner_decl_indent);
                                        out.push_str(trimmed);
                                        out.push('\n');
                                    }
                                }
                            }
                            out.push_str(&nested_indent);
                            out.push_str("}\n");
                        }
                    }

                    out.push_str(&parent_indent);
                    out.push('}');

                    let specificities: Vec<Specificity> = base_prefixes
                        .iter()
                        .map(|p| calculate_specificity(p))
                        .collect();
                    let uniform = specificities.windows(2).all(|w| w[0] == w[1]);

                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::ModernizeIs],
                        safety: Safety::Safe,
                        source_range: SourceRange {
                            start: first.start,
                            end: last_node.end,
                        },
                        original: source[first.start..last_node.end].to_string(),
                        proposed: out,
                        proof: Proof::safe_local(),
                        warnings: if uniform { Vec::new() } else { vec!["Notice: :is() takes the specificity of its most specific argument.".into()] },
                        reason: format!("Factor multi-selector cluster for {} into :is(...) with nested rules.", is_header),
                        selected: true,
                    });
                    i = cursor;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn extract_multi_branch_pattern(selector: &str) -> Option<(Vec<String>, Option<String>)> {
    let branches: Vec<&str> = split_top_level_comma(selector)
        .into_iter()
        .map(|s| s.trim())
        .collect();
    if branches.len() < 2 {
        return None;
    }
    if branches.iter().any(|b| b.contains("::")) {
        return None;
    }

    let first = branches[0];
    if let Some(space_pos) = first.rfind(' ') {
        let suffix = &first[space_pos..];
        if branches.iter().all(|b| b.ends_with(suffix)) {
            let prefixes: Vec<String> = branches
                .iter()
                .map(|b| b[..b.len() - suffix.len()].trim().to_string())
                .collect();
            if prefixes.iter().all(|p| is_valid_selector_token(p)) {
                return Some((prefixes, Some(suffix.trim().to_string())));
            }
        }
    }

    if branches
        .iter()
        .all(|b| is_valid_selector_token(b) && !b.contains(' '))
    {
        let prefixes: Vec<String> = branches.iter().map(|b| b.to_string()).collect();
        return Some((prefixes, None));
    }

    None
}

fn plan_merge_identical_rule_bodies(
    path: &Path,
    source: &str,
    nodes: &[SourceNode],
    enabled: &HashSet<RuleId>,
    plans: &mut Vec<PlanEntry>,
) {
    if !enabled.contains(&RuleId::MergeIdenticalRuleBodies) {
        return;
    }
    let mut i = 0;
    while i < nodes.len() {
        let first = &nodes[i];
        if matches!(&first.kind, NodeKind::Style) {
            let first_body = first.body(source).unwrap_or("").trim();
            if !first_body.is_empty() {
                let mut cluster = vec![first];
                let mut cursor = i + 1;
                let mut prev_end = first.end;

                while cursor < nodes.len() {
                    let next = &nodes[cursor];
                    if !is_whitespace_only(source, prev_end..next.start) {
                        break;
                    }
                    if matches!(&next.kind, NodeKind::Style)
                        && next.body(source).unwrap_or("").trim() == first_body
                    {
                        cluster.push(next);
                        prev_end = next.end;
                        cursor += 1;
                        continue;
                    }
                    break;
                }

                if cluster.len() > 1 {
                    let last = cluster.last().unwrap();
                    let selectors: Vec<&str> =
                        cluster.iter().map(|c| c.prelude(source).trim()).collect();
                    let parent_indent = line_indent(source, first.start);
                    let first_body_range = first.body_range.as_ref().unwrap();
                    let unit = detect_indent_unit(source, first_body_range.clone())
                        .unwrap_or_else(|| "  ".to_string());
                    let nested_indent = format!("{parent_indent}{unit}");

                    let mut decls = String::new();
                    for line in source[first_body_range.clone()].lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            decls.push_str(&nested_indent);
                            decls.push_str(trimmed);
                            decls.push('\n');
                        }
                    }

                    let joined_sel = selectors.join(&format!(",\n{parent_indent}"));
                    let proposed =
                        format!("{parent_indent}{joined_sel} {{\n{decls}{parent_indent}}}");
                    plans.push(PlanEntry {
                        id: String::new(),
                        file: path.to_path_buf(),
                        rules: vec![RuleId::MergeIdenticalRuleBodies],
                        safety: Safety::Safe,
                        source_range: SourceRange {
                            start: first.start,
                            end: last.end,
                        },
                        original: source[first.start..last.end].to_string(),
                        proposed,
                        proof: Proof::safe_local(),
                        warnings: Vec::new(),
                        reason: format!("Merge {} rules with identical declaration bodies into a single comma-separated rule.", cluster.len()),
                        selected: true,
                    });
                    i = cursor;
                    continue;
                }
            }
        }
        i += 1;
    }
}

pub fn split_top_level_comma(selector: &str) -> Vec<&str> {
    let bytes = selector.as_bytes();
    let mut parts = Vec::new();
    let mut last = 0;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b',' if parens == 0 && brackets == 0 => {
                parts.push(&selector[last..i]);
                last = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if last < selector.len() {
        parts.push(&selector[last..]);
    }
    parts
}

pub fn factor_selector_list(
    selector: &str,
    body: &str,
    indent: &str,
    unit: &str,
) -> Option<String> {
    let branches: Vec<&str> = split_top_level_comma(selector)
        .into_iter()
        .map(|s| s.trim())
        .collect();
    if branches.len() < 2 {
        return None;
    }
    let base = branches[0];
    if contains_top_level_comma(base) || base.contains("::") || base.is_empty() {
        return None;
    }

    let mut inner_selectors = Vec::new();
    for &branch in &branches {
        if branch == base {
            inner_selectors.push("&".to_string());
        } else {
            let rel = branch.strip_prefix(base)?;
            if rel.starts_with("::")
                || rel.starts_with(':')
                || rel.starts_with('[')
                || rel.starts_with('.')
                || rel.starts_with('#')
            {
                inner_selectors.push(format!("&{rel}"));
            } else {
                let trimmed = rel.strip_prefix(' ')?;
                inner_selectors.push(trimmed.trim_start().to_string());
            }
        }
    }

    let nested_indent = format!("{indent}{unit}");
    let inner_decl_indent = format!("{nested_indent}{unit}");
    let mut out = String::new();
    out.push_str(base);
    out.push_str(" {\n");
    out.push_str(&nested_indent);
    out.push_str(&inner_selectors.join(&format!(",\n{nested_indent}")));
    out.push_str(" {\n");
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            out.push_str(&inner_decl_indent);
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.push_str(&nested_indent);
    out.push_str("}\n");
    out.push_str(indent);
    out.push('}');
    Some(out)
}

pub fn factor_with_is(selector: &str) -> Option<(String, bool)> {
    let branches: Vec<&str> = split_top_level_comma(selector)
        .into_iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if branches.len() < 2 {
        return None;
    }

    if branches.iter().any(|b| b.contains("::")) {
        return None;
    }

    let specificities: Vec<Specificity> =
        branches.iter().map(|b| calculate_specificity(b)).collect();
    let uniform_specificity = specificities.windows(2).all(|w| w[0] == w[1]);

    let first = branches[0];

    // Suffix alternatives (e.g. .alpha .title, #hero .title -> :is(.alpha, #hero) .title)
    if let Some(space_pos) = first.rfind(' ') {
        let suffix = &first[space_pos..];
        if branches.iter().all(|b| b.ends_with(suffix)) {
            let prefixes: Vec<&str> = branches
                .iter()
                .map(|b| b[..b.len() - suffix.len()].trim())
                .collect();
            if prefixes.iter().all(|p| is_valid_selector_token(p)) {
                let is_inner = prefixes.join(", ");
                return Some((format!(":is({is_inner}){suffix}"), uniform_specificity));
            }
        }
    }

    // Descendant alternatives (e.g. .card .title, .card .subtitle -> .card :is(.title, .subtitle))
    if let Some(space_pos) = first.rfind(' ') {
        let prefix = &first[..=space_pos];
        if branches.iter().all(|b| b.starts_with(prefix)) {
            let suffixes: Vec<&str> = branches.iter().map(|b| b[prefix.len()..].trim()).collect();
            if suffixes.iter().all(|s| is_valid_selector_token(s)) {
                let is_inner = suffixes.join(", ");
                return Some((format!("{prefix}:is({is_inner})"), uniform_specificity));
            }
        }
    }

    // Pseudo-class alternatives (e.g. .button:hover, .button:focus -> .button:is(:hover, :focus))
    if let Some(colon_pos) = first.find(':') {
        let base = &first[..colon_pos];
        if !base.is_empty()
            && !base.contains(' ')
            && branches.iter().all(|b| {
                b.starts_with(base)
                    && b[base.len()..].starts_with(':')
                    && !b[base.len()..].starts_with("::")
            })
        {
            let pseudos: Vec<&str> = branches.iter().map(|b| b[base.len()..].trim()).collect();
            if pseudos
                .iter()
                .all(|p| p.starts_with(':') && !p.starts_with("::") && !p.contains(' '))
            {
                let is_inner = pseudos.join(", ");
                return Some((format!("{base}:is({is_inner})"), uniform_specificity));
            }
        }
    }

    // Attribute alternatives
    if let Some(bracket_pos) = first.find('[') {
        let base = &first[..bracket_pos];
        if !base.is_empty()
            && !base.contains(' ')
            && branches
                .iter()
                .all(|b| b.starts_with(base) && b[base.len()..].starts_with('['))
        {
            let attrs: Vec<&str> = branches.iter().map(|b| b[base.len()..].trim()).collect();
            if attrs.iter().all(|a| a.starts_with('[') && a.ends_with(']')) {
                let is_inner = attrs.join(", ");
                return Some((format!("{base}:is({is_inner})"), uniform_specificity));
            }
        }
    }

    None
}

fn is_valid_selector_token(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    first == '.'
        || first == '#'
        || first == '['
        || first == ':'
        || first.is_ascii_alphabetic()
        || first == '*'
        || first == '>'
        || first == '+'
        || first == '~'
}

pub fn factor_with_where(selector: &str) -> Option<String> {
    let branches: Vec<&str> = split_top_level_comma(selector)
        .into_iter()
        .map(|s| s.trim())
        .collect();
    if branches.len() < 2 {
        return None;
    }
    if branches.iter().any(|b| b.contains("::")) {
        return None;
    }
    Some(format!(":where({})", branches.join(", ")))
}

pub fn modernize_media_query_str(prelude: &str) -> Option<String> {
    let mut result = prelude.to_string();
    let mut changed = false;

    // Range: (min-width: 400px) and (max-width: 800px) -> (400px <= width <= 800px)
    if let (Some(min_idx), Some(max_idx)) = (result.find("min-width:"), result.find("max-width:")) {
        if min_idx < max_idx {
            if let (Some(min_val), Some(max_val)) = (
                extract_media_val(&result, "min-width:"),
                extract_media_val(&result, "max-width:"),
            ) {
                let pattern = format!("(min-width: {min_val}) and (max-width: {max_val})");
                let replacement = format!("({min_val} <= width <= {max_val})");
                if result.contains(&pattern) {
                    result = result.replace(&pattern, &replacement);
                    changed = true;
                }
            }
        }
    }

    // Single: (min-width: 800px) -> (width >= 800px)
    while let Some(val) = extract_media_val(&result, "min-width:") {
        let pattern = format!("(min-width: {val})");
        let replacement = format!("(width >= {val})");
        result = result.replace(&pattern, &replacement);
        changed = true;
    }

    // Single: (max-width: 800px) -> (width <= 800px)
    while let Some(val) = extract_media_val(&result, "max-width:") {
        let pattern = format!("(max-width: {val})");
        let replacement = format!("(width <= {val})");
        result = result.replace(&pattern, &replacement);
        changed = true;
    }

    // Single: (min-height: 400px) -> (height >= 400px)
    while let Some(val) = extract_media_val(&result, "min-height:") {
        let pattern = format!("(min-height: {val})");
        let replacement = format!("(height >= {val})");
        result = result.replace(&pattern, &replacement);
        changed = true;
    }

    // Single: (max-height: 400px) -> (height <= 400px)
    while let Some(val) = extract_media_val(&result, "max-height:") {
        let pattern = format!("(max-height: {val})");
        let replacement = format!("(height <= {val})");
        result = result.replace(&pattern, &replacement);
        changed = true;
    }

    if changed { Some(result) } else { None }
}

fn extract_media_val<'a>(source: &'a str, feature: &str) -> Option<&'a str> {
    let start = source.find(feature)? + feature.len();
    let end = source[start..].find(')')? + start;
    Some(source[start..end].trim())
}

fn selector_relation(parent: &str, child: &str) -> Option<(RelationKind, String)> {
    let parent = parent.trim();
    let child = child.trim();
    if parent.is_empty()
        || child.is_empty()
        || contains_top_level_comma(parent)
        || contains_top_level_comma(child)
        || parent.contains("::")
        || child == parent
        || !child.starts_with(parent)
    {
        return None;
    }

    let remainder = &child[parent.len()..];
    let trimmed = remainder.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let (relation, nested_selector) = if remainder.starts_with("::") {
        (RelationKind::PseudoElement, format!("&{remainder}"))
    } else if remainder.starts_with(':') {
        (RelationKind::PseudoClass, format!("&{remainder}"))
    } else if remainder.starts_with('[') {
        (RelationKind::Attribute, format!("&{remainder}"))
    } else if remainder.starts_with('.') || remainder.starts_with('#') {
        (RelationKind::Compound, format!("&{remainder}"))
    } else if let Some(first_char) = trimmed
        .chars()
        .next()
        .filter(|c| *c == '>' || *c == '+' || *c == '~')
    {
        let after_comb = trimmed[first_char.len_utf8()..].trim();
        if after_comb == parent {
            (RelationKind::Combinator, format!("{first_char} {parent}"))
        } else if let Some(after_parent) = after_comb.strip_prefix(parent) {
            (
                RelationKind::Combinator,
                format!("{first_char} {parent}{after_parent}"),
            )
        } else {
            (
                RelationKind::Combinator,
                format!("{first_char} {after_comb}"),
            )
        }
    } else if remainder
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_whitespace())
    {
        let descendant = remainder.trim();
        (RelationKind::Descendant, descendant.to_string())
    } else {
        return None;
    };

    Some((relation, nested_selector))
}

fn conditional_child(
    source: &str,
    parent_selector: &str,
    node: &SourceNode,
    enabled: &HashSet<RuleId>,
) -> Option<ClusterChild> {
    let (name, rule) = match &node.kind {
        NodeKind::AtBlock { name, .. } if name == "media" => (name.as_str(), RuleId::NestMedia),
        NodeKind::AtBlock { name, .. } if name == "supports" => {
            (name.as_str(), RuleId::NestSupports)
        }
        NodeKind::AtBlock { name, .. } if name == "container" => {
            (name.as_str(), RuleId::NestContainer)
        }
        NodeKind::AtBlock { name, .. } if name == "starting-style" => {
            (name.as_str(), RuleId::NestStartingStyle)
        }
        _ => return None,
    };
    if !enabled.contains(&rule) {
        return None;
    }

    let body_range = node.body_range.clone()?;
    let inner_nodes = scan_nodes(source, body_range.clone());
    if inner_nodes.is_empty() {
        return None;
    }

    let mut inners = Vec::new();
    for inner in &inner_nodes {
        if !matches!(&inner.kind, NodeKind::Style) {
            return None;
        }
        let inner_prelude = inner.prelude(source);
        let inner_body = inner.body_range.clone()?;
        if inner_prelude == parent_selector.trim() {
            inners.push(ConditionalInner::Direct {
                body_range: inner_body,
            });
        } else if let Some((_rel, nested_sel)) = selector_relation(parent_selector, inner_prelude) {
            inners.push(ConditionalInner::Nested {
                nested_selector: nested_sel,
                body_range: inner_body,
            });
        } else {
            return None;
        }
    }

    debug_assert!(
        name == "media" || name == "supports" || name == "container" || name == "starting-style"
    );
    Some(ClusterChild::Conditional {
        node: node.clone(),
        rule,
        inners,
    })
}

pub fn consolidate_not_in_selector(selector: &str) -> Option<(String, bool)> {
    if !selector.contains(":not(") {
        return None;
    }
    let mut result = String::new();
    let mut i = 0;
    let bytes = selector.as_bytes();
    let mut changed = false;
    let mut uniform_specificity = true;

    while i < bytes.len() {
        if i + 5 <= bytes.len() && &selector[i..i + 5] == ":not(" {
            let mut args = Vec::new();
            let mut current_end = i;

            while current_end + 5 <= bytes.len()
                && &selector[current_end..current_end + 5] == ":not("
            {
                let open = current_end + 4;
                if let Some(close) = find_matching_paren(selector, open) {
                    let arg = selector[open + 1..close].trim();
                    args.push(arg);
                    current_end = close + 1;
                } else {
                    break;
                }
            }

            if args.len() > 1 {
                changed = true;
                let specs: Vec<Specificity> =
                    args.iter().map(|a| calculate_specificity(a)).collect();
                if specs.windows(2).any(|w| w[0] != w[1]) {
                    uniform_specificity = false;
                }

                result.push_str(":not(");
                result.push_str(&args.join(", "));
                result.push(')');
                i = current_end;
                continue;
            }
        }
        let ch = selector[i..].chars().next().unwrap();
        result.push(ch);
        i += ch.len_utf8();
    }

    if changed {
        Some((result, uniform_specificity))
    } else {
        None
    }
}

fn find_matching_paren(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 1usize;
    let mut i = open + 1;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[derive(Debug, Clone)]
struct HierarchicalRule {
    relative_selector: String,
    body_lines: Vec<String>,
    sub_rules: Vec<HierarchicalRule>,
    conditional_header: Option<String>,
}

fn render_cluster(source: &str, parent: &SourceNode, children: &[ClusterChild]) -> String {
    let parent_body_range = parent.body_range.as_ref().expect("style rules have bodies");
    let parent_indent = line_indent(source, parent.start);
    let unit =
        detect_indent_unit(source, parent_body_range.clone()).unwrap_or_else(|| "  ".to_string());
    let nested_indent = format!("{parent_indent}{unit}");

    let mut out = String::new();
    let open = parent_body_range.start - 1;
    out.push_str(&source[parent.start..=open]);

    let parent_body = &source[parent_body_range.clone()];
    let trimmed_body = parent_body.trim();
    if !trimmed_body.is_empty() {
        out.push('\n');
        for line in parent_body.lines() {
            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                out.push_str(&nested_indent);
                out.push_str(trimmed_line);
                out.push('\n');
            }
        }
    }

    // Build hierarchical rules
    let mut root_rules: Vec<HierarchicalRule> = Vec::new();

    for child in children {
        match child {
            ClusterChild::Style {
                node,
                nested_selector,
                ..
            } => {
                let mut body_lines = Vec::new();
                if let Some(body_range) = &node.body_range {
                    for line in source[body_range.clone()].lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            body_lines.push(trimmed.to_string());
                        }
                    }
                }
                insert_hierarchical_style(&mut root_rules, nested_selector.trim(), body_lines);
            }
            ClusterChild::Conditional { node, inners, .. } => {
                let header = node.prelude(source).trim().to_string();
                let mut cond_sub_rules = Vec::new();
                for inner in inners {
                    match inner {
                        ConditionalInner::Direct { body_range } => {
                            let mut lines = Vec::new();
                            for line in source[body_range.clone()].lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    lines.push(trimmed.to_string());
                                }
                            }
                            cond_sub_rules.push(HierarchicalRule {
                                relative_selector: String::new(),
                                body_lines: lines,
                                sub_rules: Vec::new(),
                                conditional_header: None,
                            });
                        }
                        ConditionalInner::Nested {
                            nested_selector,
                            body_range,
                        } => {
                            let mut lines = Vec::new();
                            for line in source[body_range.clone()].lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    lines.push(trimmed.to_string());
                                }
                            }
                            cond_sub_rules.push(HierarchicalRule {
                                relative_selector: nested_selector.trim().to_string(),
                                body_lines: lines,
                                sub_rules: Vec::new(),
                                conditional_header: None,
                            });
                        }
                    }
                }
                root_rules.push(HierarchicalRule {
                    relative_selector: String::new(),
                    body_lines: Vec::new(),
                    sub_rules: cond_sub_rules,
                    conditional_header: Some(header),
                });
            }
        }
    }

    for rule in &root_rules {
        out.push('\n');
        render_hierarchical_rule(&mut out, rule, &nested_indent, &unit);
    }

    out.push_str(&parent_indent);
    out.push('}');
    out
}

fn insert_hierarchical_style(
    root_rules: &mut Vec<HierarchicalRule>,
    selector: &str,
    body_lines: Vec<String>,
) {
    if let Some(last_rule) = root_rules.last_mut() {
        if last_rule.conditional_header.is_none() && !last_rule.relative_selector.is_empty() {
            let parent_sel = &last_rule.relative_selector;
            if let Some(rel) = extract_relative_subselector(parent_sel, selector) {
                insert_hierarchical_style(&mut last_rule.sub_rules, &rel, body_lines);
                return;
            }
        }
    }

    root_rules.push(HierarchicalRule {
        relative_selector: selector.to_string(),
        body_lines,
        sub_rules: Vec::new(),
        conditional_header: None,
    });
}

fn extract_relative_subselector(parent: &str, child: &str) -> Option<String> {
    let parent = parent.trim();
    let child = child.trim();
    if child == parent || !child.starts_with(parent) {
        return None;
    }
    let remainder = &child[parent.len()..];
    let trimmed = remainder.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    if remainder.starts_with("::")
        || remainder.starts_with(':')
        || remainder.starts_with('[')
        || remainder.starts_with('.')
        || remainder.starts_with('#')
    {
        Some(format!("&{remainder}"))
    } else if let Some(first_char) = trimmed
        .chars()
        .next()
        .filter(|c| *c == '>' || *c == '+' || *c == '~')
    {
        let after_comb = trimmed[first_char.len_utf8()..].trim();
        Some(format!("{first_char} {after_comb}"))
    } else if remainder
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_whitespace())
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn render_hierarchical_rule(out: &mut String, rule: &HierarchicalRule, indent: &str, unit: &str) {
    let inner_indent = format!("{indent}{unit}");

    if let Some(header) = &rule.conditional_header {
        out.push_str(indent);
        out.push_str(header);
        out.push_str(" {\n");
        for (idx, sub) in rule.sub_rules.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            if sub.relative_selector.is_empty() {
                for line in &sub.body_lines {
                    out.push_str(&inner_indent);
                    out.push_str(line);
                    out.push('\n');
                }
            } else {
                render_hierarchical_rule(out, sub, &inner_indent, unit);
            }
        }
        out.push_str(indent);
        out.push_str("}\n");
    } else {
        out.push_str(indent);
        out.push_str(&rule.relative_selector);
        out.push_str(" {\n");

        for line in &rule.body_lines {
            out.push_str(&inner_indent);
            out.push_str(line);
            out.push('\n');
        }

        for sub in &rule.sub_rules {
            out.push('\n');
            render_hierarchical_rule(out, sub, &inner_indent, unit);
        }

        out.push_str(indent);
        out.push_str("}\n");
    }
}

fn line_indent(source: &str, offset: usize) -> String {
    let line_start = source[..offset].rfind('\n').map_or(0, |idx| idx + 1);
    source[line_start..offset]
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n' && *c != '\r')
        .collect()
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].lines().count()
}

fn detect_indent_unit(source: &str, body: Range<usize>) -> Option<String> {
    for line in source[body].lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if !indent.is_empty() {
            return Some(indent);
        }
    }
    None
}

fn contains_top_level_comma(selector: &str) -> bool {
    let bytes = selector.as_bytes();
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b',' if parens == 0 && brackets == 0 => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

pub fn apply_selected_plans(
    source: &str,
    plans: &[PlanEntry],
    include_review: bool,
) -> Result<String> {
    let mut selected: Vec<&PlanEntry> = plans
        .iter()
        .filter(|plan| {
            plan.selected
                && (plan.safety == Safety::Safe
                    || (include_review && plan.safety == Safety::Review))
        })
        .collect();
    selected.sort_by(|a, b| {
        a.source_range
            .start
            .cmp(&b.source_range.start)
            .then_with(|| b.source_range.end.cmp(&a.source_range.end))
    });

    let mut non_overlapping: Vec<&PlanEntry> = Vec::with_capacity(selected.len());
    let mut last_end = 0;
    for plan in selected {
        if plan.source_range.start >= last_end {
            last_end = plan.source_range.end;
            non_overlapping.push(plan);
        }
    }

    let mut output = source.to_string();
    for plan in non_overlapping.into_iter().rev() {
        if plan.source_range.start <= output.len()
            && plan.source_range.end <= output.len()
            && plan.source_range.start <= plan.source_range.end
        {
            output.replace_range(
                plan.source_range.start..plan.source_range.end,
                &plan.proposed,
            );
        }
    }
    Ok(output)
}

pub fn unified_diff(old: &str, new: &str, old_name: &str, new_name: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(old_name, new_name)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(css: &str, rules: &[RuleId]) -> Vec<PlanEntry> {
        analyze_source(PathBuf::from("test.css"), css, rules)
            .unwrap()
            .plans
    }

    #[test]
    fn nests_adjacent_pseudo_and_descendant_rules() {
        let css = ".card {\n  color: red;\n}\n.card:hover {\n  color: blue !important;\n}\n.card .title {\n  font-weight: 700;\n}\n";
        let plans = plan(css, &RuleId::ALL);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("&:hover"));
        assert!(output.contains(".title"));
        assert!(output.contains("color: blue !important;"));
    }

    #[test]
    fn nests_exact_full_modernize_example() {
        let original = r#".card {
  color: #222;
  padding: 1rem;
}
.card:hover {
  color: #111 !important;
}
.card::before {
  content: "";
}
.card[data-active] {
  border-color: currentColor;
}
.card.featured {
  box-shadow: 0 0 0 1px currentColor;
}
.card .title {
  font-weight: 700;
}
.card > .body {
  min-width: 0;
}
.card + .card {
  margin-top: 1rem;
}
@media (width >= 48rem) {
  .card {
    padding: 1.5rem;
  }
}
@supports (display: grid) {
  .card {
    display: grid;
  }
}
"#;

        let expected = r#".card {
  color: #222;
  padding: 1rem;

  &:hover {
    color: #111 !important;
  }

  &::before {
    content: "";
  }

  &[data-active] {
    border-color: currentColor;
  }

  &.featured {
    box-shadow: 0 0 0 1px currentColor;
  }

  .title {
    font-weight: 700;
  }

  > .body {
    min-width: 0;
  }

  + .card {
    margin-top: 1rem;
  }

  @media (width >= 48rem) {
    padding: 1.5rem;
  }

  @supports (display: grid) {
    display: grid;
  }
}"#;

        let plans = plan(
            original,
            &[
                RuleId::NestPseudoClass,
                RuleId::NestPseudoElement,
                RuleId::NestAttribute,
                RuleId::NestCompound,
                RuleId::NestDescendant,
                RuleId::NestCombinator,
                RuleId::NestMedia,
                RuleId::NestSupports,
            ],
        );
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(original, &plans, false).unwrap();
        assert_eq!(output.trim(), expected.trim());
    }

    #[test]
    fn factors_selector_list_sharing_base() {
        let css = ".marker,\n.marker::before,\n.marker::after {\n  box-sizing: border-box;\n}\n";
        let plans = plan(css, &[RuleId::FactorSelectorList]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".marker {"));
        assert!(output.contains("&,"));
        assert!(output.contains("&::before,"));
        assert!(output.contains("&::after {"));
        assert!(output.contains("box-sizing: border-box;"));
    }

    #[test]
    fn modernizes_is_with_uniform_specificity() {
        let css = ".button:hover, .button:focus, .button:active {\n  color: blue;\n}\n";
        let plans = plan(css, &[RuleId::ModernizeIs]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".button:is(:hover, :focus, :active)"));
    }

    #[test]
    fn modernizes_media_range_syntax() {
        let css = "@media (min-width: 800px) {\n  .card { padding: 2rem; }\n}\n";
        let plans = plan(css, &[RuleId::ModernizeMediaRange]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("@media (width >= 800px)"));
    }

    #[test]
    fn consolidates_not_selectors() {
        let css = "input:not([type=\"checkbox\"]):not([type=\"radio\"]) {\n  border: 1px solid gray;\n}\n";
        let plans = plan(css, &[RuleId::ConsolidateNot]);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].safety, Safety::Review);
        let output = apply_selected_plans(css, &plans, true).unwrap();
        assert!(output.contains("input:not([type=\"checkbox\"], [type=\"radio\"])"));
    }

    #[test]
    fn refuses_subtoken_is_factoring_false_positive() {
        let css = ".same-specificity-a,\n.same-specificity-b {\n  color: black;\n}\n";
        let plans = plan(css, &[RuleId::ModernizeIs]);
        assert!(plans.is_empty());
    }

    #[test]
    fn modernizes_descendant_is_alternatives() {
        let css = ".card .title, .card .subtitle, .card .description {\n  color: black;\n}\n";
        let plans = plan(css, &[RuleId::ModernizeIs]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".card :is(.title, .subtitle, .description)"));
    }

    #[test]
    fn modernizes_suffix_is_alternatives() {
        let css = ".alpha .title,\n#hero .title {\n  color: rebeccapurple;\n}\n";
        let plans = plan(css, &[RuleId::ModernizeIs]);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].safety, Safety::Review);
        let output = apply_selected_plans(css, &plans, true).unwrap();
        assert!(output.contains(":is(.alpha, #hero) .title"));
    }

    #[test]
    fn factors_multi_selector_cluster_with_is_and_nesting() {
        let css = r#".alpha .title,
#hero .title {
  color: rebeccapurple;
}

.alpha .subtitle,
#hero .subtitle {
  color: slateblue;
}

.alpha,
#hero {
  border-color: currentColor;
}
"#;
        let plans = plan(css, &[RuleId::ModernizeIs]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, true).unwrap();
        assert!(output.contains(":is(.alpha, #hero) {"));
        assert!(output.contains("border-color: currentColor;"));
        assert!(output.contains(".title {"));
        assert!(output.contains("color: rebeccapurple;"));
        assert!(output.contains(".subtitle {"));
        assert!(output.contains("color: slateblue;"));
    }

    #[test]
    fn refuses_bem_token_concatenation() {
        let css = ".card { color: red; }\n.card__title { font-weight: 700; }\n";
        let plans = plan(css, &RuleId::ALL);
        assert!(plans.is_empty());
    }

    #[test]
    fn merges_same_named_layer_blocks() {
        let css = "@layer overrides {\n  .layered-card {\n    color: darkgreen;\n  }\n}\n\n@layer overrides {\n  .layer-important {\n    color: orange !important;\n  }\n}\n";
        let plans = plan(css, &[RuleId::MergeSameNamedLayer]);
        assert_eq!(plans.len(), 2);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("@layer overrides {"));
        assert!(output.contains(".layered-card {"));
        assert!(output.contains(".layer-important {"));
    }

    #[test]
    fn merges_adjacent_media_queries() {
        let css = "@media (width >= 48rem) {\n  .card {\n    padding: 2rem;\n  }\n}\n\n@media (width >= 48rem) {\n  .panel {\n    padding: 2rem;\n  }\n}\n";
        let plans = plan(css, &[RuleId::MergeAdjacentMedia]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("@media (width >= 48rem) {"));
        assert!(output.contains(".card {"));
        assert!(output.contains(".panel {"));
    }

    #[test]
    fn merges_adjacent_supports_queries() {
        let css = "@supports (display: grid) {\n  .card {\n    display: grid;\n  }\n}\n\n@supports (display: grid) {\n  .panel {\n    display: grid;\n  }\n}\n";
        let plans = plan(css, &[RuleId::MergeAdjacentSupports]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("@supports (display: grid) {"));
        assert!(output.contains(".card {"));
        assert!(output.contains(".panel {"));
    }

    #[test]
    fn merges_adjacent_identical_selectors() {
        let css = ".card {\n  color: black;\n}\n\n.card {\n  padding: 1rem;\n}\n";
        let plans = plan(css, &[RuleId::MergeAdjacentIdenticalSelector]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".card {"));
        assert!(output.contains("color: black;"));
        assert!(output.contains("padding: 1rem;"));
    }

    #[test]
    fn merges_identical_rule_bodies() {
        let css = ".card:hover {\n  color: red;\n}\n\n.panel:hover {\n  color: red;\n}\n";
        let plans = plan(css, &[RuleId::MergeIdenticalRuleBodies]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".card:hover,"));
        assert!(output.contains(".panel:hover {"));
        assert!(output.contains("color: red;"));
    }

    #[test]
    fn factors_identical_states_with_is() {
        let css = ".card:hover {\n  background: silver;\n}\n\n.card:focus {\n  background: silver;\n}\n\n.card:focus-visible {\n  background: silver;\n}\n";
        let plans = plan(css, &[RuleId::FactorIdenticalStatesWithIs]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".card {"));
        assert!(output.contains("&:is(:hover, :focus, :focus-visible) {"));
        assert!(output.contains("background: silver;"));
    }

    #[test]
    fn nests_multi_level_tree_hierarchy() {
        let css = r#".tree {
  display: grid;
  gap: 0.5rem;
}
.tree .node {
  position: relative;
}
.tree .node .label {
  display: flex;
}
.tree .node .label:hover {
  color: var(--accent);
}
.tree .node > .children {
  margin-inline-start: 1.25rem;
}
.tree .node > .children > .node + .node {
  margin-block-start: 0.25rem;
}
"#;
        let plans = plan(
            css,
            &[
                RuleId::NestDescendant,
                RuleId::NestCombinator,
                RuleId::NestPseudoClass,
            ],
        );
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".node {"));
        assert!(output.contains(".label {"));
        assert!(output.contains("&:hover {"));
        assert!(output.contains("> .children {"));
        assert!(output.contains("> .node + .node {"));
    }

    #[test]
    fn nests_in_place_input_states() {
        let css = "input:user-invalid {\n  border-color: crimson;\n}\ninput:user-valid {\n  border-color: seagreen;\n}\ninput:placeholder-shown {\n  color: gray;\n}\n";
        let plans = plan(css, &[RuleId::NestPseudoClass]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains("input {"));
        assert!(output.contains("&:user-invalid {"));
        assert!(output.contains("&:user-valid {"));
        assert!(output.contains("&:placeholder-shown {"));
    }

    #[test]
    fn gathers_consecutive_conditions_by_selector() {
        let css = "@media (width >= 30rem) {\n  .responsive-grid {\n    gap: 1rem;\n  }\n}\n\n@media (width >= 80rem) {\n  .responsive-grid {\n    gap: 2rem;\n  }\n}\n";
        let plans = plan(css, &[RuleId::NestMedia]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".responsive-grid {"));
        assert!(output.contains("@media (width >= 30rem) {"));
        assert!(output.contains("@media (width >= 80rem) {"));
    }

    #[test]
    fn factors_selector_list_with_adjacent_hover() {
        let css = ".notice,\n.notice::before,\n.notice::after {\n  color: currentColor;\n}\n\n.notice:hover {\n  background: color-mix(in srgb, currentColor 8%, transparent);\n}\n";
        let plans = plan(css, &[RuleId::FactorSelectorList, RuleId::NestPseudoClass]);
        assert_eq!(plans.len(), 1);
        let output = apply_selected_plans(css, &plans, false).unwrap();
        assert!(output.contains(".notice {"));
        assert!(output.contains("&,"));
        assert!(output.contains("&::before,"));
        assert!(output.contains("&::after {"));
        assert!(output.contains("&:hover {"));
        assert!(output.contains("background: color-mix"));
    }

    #[test]
    fn gathers_non_adjacent_related_selector_rules_with_nested_blocks() {
        let css = r#".skip-link {
    position: absolute;
    inset-block-start: -48px;
    inset-inline-start: 1rem;
    z-index: 10000000000;
    background: var(--bg-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    padding: 0.55rem 0.8rem;
    text-decoration: none;
    font-weight: 700;
    transition: inset-block-start 0.2s ease;

    &:focus-visible {
        inset-block-start: 0.75rem;
    }
}

.unrelated-rule {
    color: red;
}

.skip-link {
    font: optional;

    &::after {
        content: '';
    }

    :not(*) & {
        all: unset
    }
}
"#;
        let plans = plan(css, &[RuleId::GatherRelatedSelectorRules]);
        assert_eq!(plans.len(), 2);
        let output = apply_selected_plans(css, &plans, true).unwrap();
        assert!(output.contains("font: optional;"));
    }

    #[test]
    fn gathers_non_adjacent_related_pseudo_and_combinator_rules() {
        let css = r#".skip-link {
    position: absolute;
    inset-block-start: -48px;
    inset-inline-start: 1rem;
    z-index: 10000000000;
    background: var(--bg-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    padding: 0.55rem 0.8rem;
    text-decoration: none;
    font-weight: 700;
    transition: inset-block-start 0.2s ease;

    &:focus-visible {
        inset-block-start: 0.75rem;
    }
}

.unrelated {
    color: red;
}

.skip-link {
    font: optional;

    &::after {
        content: '';
    }

    :not(*) & {
        all: unset
    }
}

.skip-link+* {
    display: block;
}

.skip-link::backdrop {
    background-color: gray;
}

.skip-link:has(*) {
    color: #27ca3f;
}
"#;
        let plans = plan(css, &[RuleId::GatherRelatedSelectorRules]);
        assert_eq!(plans.len(), 5);
        let output = apply_selected_plans(css, &plans, true).unwrap();
        assert!(output.contains("font: optional;"));
        assert!(output.contains("+ * {"));
        assert!(output.contains("&::backdrop {"));
        assert!(output.contains("&:has(*) {"));
    }
}
