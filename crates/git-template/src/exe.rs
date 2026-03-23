//! Orchestration functions for `git template` subcommands.

use std::collections::HashMap;
use std::path::Path;

use git2::Repository;
use git_vendor::cli::name_from_url;
use git_vendor::exe::MergeOutcome;
use git_vendor::{exe as vendor_exe, vendor_ref};

use crate::cli::HistoryArg;
use crate::config;
use crate::editor;
use crate::render;

pub use git_vendor::exe::open_repo;

/// Initialize the current repository from a template repository.
///
/// Vendors the upstream, renders template files, cleans up, and commits.
pub fn init(
    repo: &Repository,
    url: &str,
    name: Option<&str>,
    branch: Option<&str>,
    history: HistoryArg,
    keep_vendor: bool,
    file_favor: Option<git2::FileFavor>,
) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(history, HistoryArg::Replay) {
        return Err("replay history is not supported for `git template init`".into());
    }

    let workdir = repo
        .workdir()
        .ok_or("repository has no working directory")?;

    let name = name.unwrap_or_else(|| name_from_url(url));

    // 1. Vendor the template repo (all files, repo root).
    let outcome = vendor_exe::add(repo, name, url, branch, &["**"], None, file_favor)?;
    match &outcome {
        MergeOutcome::Conflict { .. } => {
            return Err(
                "template vendoring produced conflicts; resolve them before proceeding".into(),
            );
        }
        MergeOutcome::UpToDate { .. } => unreachable!("add never produces UpToDate"),
        MergeOutcome::Clean { .. } => {}
    }

    // Capture vendor tip OID now — we may delete the ref below.
    let vendor_tip = repo
        .find_reference(&vendor_ref(name))
        .and_then(|r| r.peel_to_commit())
        .map(|c| c.id())
        .ok();

    // 2. Read .gittemplate.
    let cfg = config::load(workdir)?;

    // 3. Collect template files.
    let files = {
        let patterns = cfg.as_ref().and_then(|c| c.template.files.as_deref());
        render::collect_files(workdir, patterns)?
    };

    // 4. Determine variables: from config, or by scanning files.
    let vars: Vec<editor::Var> = match &cfg {
        Some(c) if !c.template.vars.is_empty() => c
            .template
            .vars
            .iter()
            .map(|v| editor::Var {
                name: v.name.clone(),
                description: Some(v.description.clone()),
                default: v.default.clone(),
            })
            .collect(),
        _ => render::scan_vars(workdir, &files)?
            .into_iter()
            .map(|name| editor::Var {
                name,
                description: None,
                default: String::new(),
            })
            .collect(),
    };

    // 5. Prompt for values if there are any variables.
    let values: HashMap<String, String> = if vars.is_empty() {
        HashMap::new()
    } else {
        editor::prompt(&vars, repo.path())?
    };

    // 6. Render template files in-place and stage them.
    let unresolved = render::render_files(workdir, &files, &values, repo)?;

    // 7. Remove .gittemplate (and optionally vendor tracking files).
    let mut to_remove = vec![".gittemplate"];
    if !keep_vendor {
        to_remove.push(".gitvendors");
        to_remove.push(".gitattributes");
    }
    {
        for file in &to_remove {
            let p = workdir.join(file);
            if p.exists() {
                std::fs::remove_file(p)?;
            }
        }
        let mut index = repo.index()?;
        for file in &to_remove {
            let _ = index.remove_path(Path::new(file));
        }
        index.write()?;
    }
    if !keep_vendor {
        if let Ok(mut r) = repo.find_reference(&vendor_ref(name)) {
            r.delete()?;
        }
    }

    // 8. Commit.
    commit(repo, url, history, vendor_tip)?;

    // 9. Warn about unresolved expressions.
    if !unresolved.is_empty() {
        eprintln!(
            "warning: {} unresolved template expression(s) — variables were left empty:",
            unresolved.len()
        );
        for u in &unresolved {
            eprintln!("  {}:{}  {}", u.file, u.line, u.expr);
        }
    }

    eprintln!("Initialized from template '{}'.", name);
    Ok(())
}

fn local_sig(repo: &Repository) -> Result<git2::Signature<'static>, git2::Error> {
    let cfg = repo.config()?;
    let name = cfg
        .get_string("user.name")
        .unwrap_or_else(|_| "git-template".to_string());
    let email = cfg
        .get_string("user.email")
        .unwrap_or_else(|_| "git-template@localhost".to_string());
    git2::Signature::now(&name, &email)
}

fn commit(
    repo: &Repository,
    url: &str,
    history: HistoryArg,
    vendor_tip: Option<git2::Oid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sig = local_sig(repo)?;
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let msg = format!("chore: initialize from template {}", url);

    let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    match history {
        HistoryArg::Squash => {
            let mut parents: Vec<git2::Commit<'_>> = Vec::new();
            if let Some(h) = head_commit {
                parents.push(h);
            }
            if let Some(tip) = vendor_tip {
                if let Ok(c) = repo.find_commit(tip) {
                    parents.push(c);
                }
            }
            let refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
            repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &refs)?;
        }
        HistoryArg::Linear => {
            let parents: Vec<git2::Commit<'_>> = head_commit.into_iter().collect();
            let refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
            repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &refs)?;
        }
        HistoryArg::Replay => unreachable!("checked at top of init"),
    }

    Ok(())
}
