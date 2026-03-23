//! Template file scanning and rendering.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// An unresolved template expression found after rendering.
pub struct UnresolvedExpr {
    /// Repo-relative file path.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// The expression text, e.g. `{{ project_name }}`.
    pub expr: String,
}

/// Return repo-relative paths of all template files under `workdir`.
///
/// When `patterns` is `Some`, only files matching at least one glob are
/// returned.  When `None`, all files are returned.  `.git`, `.gittemplate`,
/// `.gitvendors`, and `.gitattributes` are always excluded.
pub fn collect_files(
    workdir: &Path,
    patterns: Option<&[String]>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let globset = if let Some(pats) = patterns {
        let mut b = globset::GlobSetBuilder::new();
        for p in pats {
            b.add(globset::Glob::new(p)?);
        }
        Some(b.build()?)
    } else {
        None
    };

    let mut files = Vec::new();
    walk(workdir, workdir, &globset, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk(
    root: &Path,
    dir: &Path,
    globset: &Option<globset::GlobSet>,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(root)?;

        // Skip the .git directory entirely.
        let top = rel
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("");
        if top == ".git" {
            continue;
        }

        if path.is_dir() {
            walk(root, &path, globset, out)?;
        } else {
            let rel_str = rel.to_str().ok_or("non-UTF-8 path")?;
            if matches!(
                rel_str,
                ".gittemplate" | ".gitvendors" | ".gitattributes"
            ) {
                continue;
            }
            let include = globset.as_ref().map_or(true, |gs| gs.is_match(rel));
            if include {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

/// Extract `{{ identifier }}` variable names from a string without full parsing.
fn extract_var_names(s: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut rest = s;
    while let Some(pos) = rest.find("{{") {
        rest = &rest[pos + 2..];
        let inner = rest.trim_start_matches(|c: char| c == ' ' || c == '\t');
        let end = inner
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(inner.len());
        if end > 0 {
            vars.push(inner[..end].to_string());
        }
    }
    vars
}

/// Scan `files` for `{{ identifier }}` expressions and return unique variable
/// names in first-seen order.
pub fn scan_vars(
    workdir: &Path,
    files: &[PathBuf],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut seen = HashSet::new();
    let mut vars = Vec::new();
    for rel in files {
        let Ok(content) = std::fs::read_to_string(workdir.join(rel)) else {
            continue; // skip binary files
        };
        for name in extract_var_names(&content) {
            if seen.insert(name.clone()) {
                vars.push(name);
            }
        }
    }
    Ok(vars)
}

/// Render each file in `files` in-place using minijinja and stage the result.
///
/// Returns a list of expressions that were left unresolved because the
/// corresponding variable had an empty value.
pub fn render_files(
    workdir: &Path,
    files: &[PathBuf],
    values: &HashMap<String, String>,
    repo: &git2::Repository,
) -> Result<Vec<UnresolvedExpr>, Box<dyn std::error::Error>> {
    let empty_vars: HashSet<&str> = values
        .iter()
        .filter(|(_, v)| v.trim().is_empty())
        .map(|(k, _)| k.as_str())
        .collect();

    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);

    let mut unresolved = Vec::new();
    let mut index = repo.index()?;

    for rel in files {
        let abs = workdir.join(rel);
        let Ok(content) = std::fs::read_to_string(&abs) else {
            continue; // skip binary files
        };

        let rel_str = rel.to_str().ok_or("non-UTF-8 path")?;

        if !empty_vars.is_empty() {
            for (line_no, line) in content.lines().enumerate() {
                for var in extract_var_names(line) {
                    if empty_vars.contains(var.as_str()) {
                        unresolved.push(UnresolvedExpr {
                            file: rel_str.to_string(),
                            line: line_no + 1,
                            expr: format!("{{{{ {} }}}}", var),
                        });
                    }
                }
            }
        }

        let rendered = env
            .render_str(&content, values)
            .map_err(|e| format!("{}: {}", rel_str, e))?;

        std::fs::write(&abs, rendered)?;
        index.add_path(rel)?;
    }

    index.write()?;
    Ok(unresolved)
}
