//! `puml init-genai` — drop AI-author instruction templates into a project.
//!
//! Downstream AI tools (Codex, Cursor, Copilot, Claude, Windsurf, …) each
//! read a different file for project-specific guidance. This subcommand
//! writes the same `puml` authoring rules in each tool's expected location
//! so all of them share consistent guidance.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// One template-to-destination mapping. Destinations are relative to the
/// target directory passed to [`run`].
struct Template {
    /// Where the file lands (relative to the target directory).
    dest: &'static str,
    /// Human-friendly tool name for the summary report.
    tool: &'static str,
    /// Embedded template body.
    body: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        dest: "CLAUDE.md",
        tool: "Claude Code",
        body: include_str!("init_genai_templates/CLAUDE.md"),
    },
    Template {
        dest: "AGENTS.md",
        tool: "Codex / Aider / OpenCode",
        body: include_str!("init_genai_templates/AGENTS.md"),
    },
    Template {
        dest: ".cursor/rules/puml.mdc",
        tool: "Cursor",
        body: include_str!("init_genai_templates/cursor.mdc"),
    },
    Template {
        dest: ".github/copilot-instructions.md",
        tool: "GitHub Copilot",
        body: include_str!("init_genai_templates/copilot-instructions.md"),
    },
    Template {
        dest: ".windsurfrules",
        tool: "Windsurf",
        body: include_str!("init_genai_templates/windsurfrules.md"),
    },
];

/// Outcome of a single template write — used for the summary report.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Created(PathBuf),
    Overwritten(PathBuf),
    Skipped(PathBuf),
}

/// Drop the AI-author templates into `dir`. With `force = false`, files that
/// already exist are skipped; with `force = true`, they're overwritten.
pub fn run(dir: &Path, force: bool) -> Result<Vec<Outcome>> {
    let mut report = Vec::with_capacity(TEMPLATES.len());
    for tpl in TEMPLATES {
        let dest = dir.join(tpl.dest);
        let exists = dest.exists();
        if exists && !force {
            report.push(Outcome::Skipped(dest));
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {:?}", parent))?;
        }
        std::fs::write(&dest, tpl.body).with_context(|| format!("writing {:?}", dest))?;
        report.push(if exists {
            Outcome::Overwritten(dest)
        } else {
            Outcome::Created(dest)
        });
    }
    Ok(report)
}

/// Print a human-readable summary of the [`run`] outcomes.
pub fn print_report(report: &[Outcome], force: bool) {
    let mut created = 0usize;
    let mut overwritten = 0usize;
    let mut skipped = 0usize;
    for (outcome, tpl) in report.iter().zip(TEMPLATES.iter()) {
        match outcome {
            Outcome::Created(p) => {
                created += 1;
                println!("  created    {}  ({})", p.display(), tpl.tool);
            }
            Outcome::Overwritten(p) => {
                overwritten += 1;
                println!("  overwrote  {}  ({})", p.display(), tpl.tool);
            }
            Outcome::Skipped(p) => {
                skipped += 1;
                println!(
                    "  skipped    {}  ({}) — already exists",
                    p.display(),
                    tpl.tool
                );
            }
        }
    }
    print!("\n{} created", created);
    if force {
        print!(", {} overwritten", overwritten);
    }
    println!(", {} skipped", skipped);
    if skipped > 0 && !force {
        println!("Re-run with --force to overwrite existing files.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "puml-init-genai-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn fresh_run_creates_every_template() {
        let dir = tempdir();
        let report = run(&dir, false).unwrap();
        assert_eq!(report.len(), TEMPLATES.len());
        for outcome in &report {
            assert!(matches!(outcome, Outcome::Created(_)));
        }
        for tpl in TEMPLATES {
            let path = dir.join(tpl.dest);
            assert!(path.exists(), "{:?} should exist", path);
            let body = std::fs::read_to_string(&path).unwrap();
            assert!(body.contains("puml"));
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rerun_without_force_skips_existing() {
        let dir = tempdir();
        run(&dir, false).unwrap();
        let report = run(&dir, false).unwrap();
        for outcome in &report {
            assert!(matches!(outcome, Outcome::Skipped(_)));
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn force_overwrites_existing() {
        let dir = tempdir();
        let claude = dir.join("CLAUDE.md");
        std::fs::write(&claude, "old content").unwrap();
        let report = run(&dir, true).unwrap();
        let claude_outcome = report
            .iter()
            .find(|o| matches!(o, Outcome::Overwritten(p) if p == &claude))
            .expect("CLAUDE.md should be overwritten");
        assert!(matches!(claude_outcome, Outcome::Overwritten(_)));
        let body = std::fs::read_to_string(&claude).unwrap();
        assert_ne!(body, "old content");
        assert!(body.contains("puml"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nested_destinations_create_parent_directories() {
        let dir = tempdir();
        run(&dir, false).unwrap();
        assert!(dir.join(".cursor/rules/puml.mdc").exists());
        assert!(dir.join(".github/copilot-instructions.md").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
