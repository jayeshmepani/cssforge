use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".turbo",
    ".svelte-kit",
    "vendor",
    ".git",
    ".hg",
    ".svn",
    ".cache",
];

const IGNORED_FILE_SUFFIXES: &[&str] = &[
    ".modern.css",
    ".min.css",
    ".bak.css",
    ".backup.css",
    ".bundle.css",
    ".chunk.css",
    ".map.css",
    ".modern.php",
    ".modern.html",
    ".modern.vue",
    ".modern.svelte",
];

const EMBEDDED_STYLE_EXTENSIONS: &[&str] = &[
    "php", "html", "htm", "vue", "svelte", "astro", "twig", "html", "erb", "hbs", "blade",
];

pub fn discover_css_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        let is_css = root
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("css"));
        if is_css || is_eligible_source_file(root)? {
            return Ok(vec![root.to_path_buf()]);
        }
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .filter_entry(|entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
                && IGNORED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(name))
            {
                return false;
            }
            true
        })
        .build();

    for entry in walker {
        let entry = entry.with_context(|| format!("failed while walking {}", root.display()))?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if is_eligible_source_file(&path)? {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn is_eligible_source_file(path: &Path) -> Result<bool> {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return Ok(false);
    };
    if !ext.eq_ignore_ascii_case("css")
        && !EMBEDDED_STYLE_EXTENSIONS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(ext))
    {
        return Ok(false);
    }

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();

    for suffix in IGNORED_FILE_SUFFIXES {
        if file_name.ends_with(suffix) {
            return Ok(false);
        }
    }

    // Ensure no path component is an ignored dir
    for comp in path.components() {
        if let std::path::Component::Normal(os_str) = comp
            && let Some(s) = os_str.to_str()
            && IGNORED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(s))
        {
            return Ok(false);
        }
    }

    if ext.eq_ignore_ascii_case("css") {
        return Ok(true);
    }

    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read candidate template {}", path.display()))?;
    Ok(contains_style_tag(&source))
}

fn contains_style_tag(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.contains("<style") && lower.contains("</style")
}

pub fn is_git_dirty(path: &Path) -> bool {
    let cwd = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("."))
    };
    Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_some_and(|out| !out.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_discover_css_files_filters_generated_and_ignored() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("cssforge_disc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir)?;

        let regular_css = temp_dir.join("style.css");
        let blade_view = temp_dir.join("view.blade.php");
        let php_without_style = temp_dir.join("plain.php");
        let sub_css = temp_dir.join("sub").join("app.css");
        let modern_css = temp_dir.join("demo.modern.css");
        let min_css = temp_dir.join("bundle.min.css");
        let bak_css = temp_dir.join("legacy.bak.css");
        let node_modules_css = temp_dir.join("node_modules").join("pkg.css");
        let target_css = temp_dir.join("target").join("out.css");

        fs::create_dir_all(temp_dir.join("sub"))?;
        fs::create_dir_all(temp_dir.join("node_modules"))?;
        fs::create_dir_all(temp_dir.join("target"))?;

        fs::write(&regular_css, "body { margin: 0; }")?;
        fs::write(
            &blade_view,
            "<div><style>.btn { color: red; }</style></div>",
        )?;
        fs::write(&php_without_style, "<?php echo 'no css'; ?>")?;
        fs::write(&sub_css, ".btn { color: red; }")?;
        fs::write(&modern_css, ".modern { color: green; }")?;
        fs::write(&min_css, ".min{color:blue;}")?;
        fs::write(&bak_css, ".bak { color: purple; }")?;
        fs::write(&node_modules_css, ".lib { color: pink; }")?;
        fs::write(&target_css, ".target { color: orange; }")?;

        let discovered = discover_css_files(&temp_dir)?;
        let file_names: Vec<String> = discovered
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(file_names.len(), 3);
        assert!(file_names.contains(&"style.css".to_string()));
        assert!(file_names.contains(&"app.css".to_string()));
        assert!(file_names.contains(&"view.blade.php".to_string()));
        assert!(!file_names.contains(&"plain.php".to_string()));
        assert!(!file_names.contains(&"demo.modern.css".to_string()));
        assert!(!file_names.contains(&"bundle.min.css".to_string()));
        assert!(!file_names.contains(&"legacy.bak.css".to_string()));
        assert!(!file_names.contains(&"pkg.css".to_string()));
        assert!(!file_names.contains(&"out.css".to_string()));

        // Explicit file targeting still works
        let explicit_modern = discover_css_files(&modern_css)?;
        assert_eq!(explicit_modern.len(), 1);
        assert_eq!(explicit_modern[0], modern_css);

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
