use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn discover_css_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        if root
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
        {
            return Ok(vec![root.to_path_buf()]);
        }
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .build();

    for entry in walker {
        let entry = entry.with_context(|| format!("failed while walking {}", root.display()))?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.into_path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
        {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
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
