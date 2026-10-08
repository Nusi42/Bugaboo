//! The two generated pages: the architecture (chapters, decision records, check table) and the
//! milestones.

use std::fmt::Write as _;

use anyhow::{Context as _, bail};

use super::GENERATED_MARKER;
use super::markdown::{self, Heading, Links};
use super::source::{ARCHITECTURE_README, MILESTONES, chapter_files, read_adrs, read_chapters};
use super::text::escape_text;
use crate::catalog::Catalog;
use crate::repo::Repo;
use crate::source::read_text;

pub(super) const ARCHITECTURE_PATH: &str = "docs/03-architecture.html";
pub(super) const MILESTONES_PATH: &str = "docs/04-milestones.html";

const CHECKS_CHAPTER_PREFIX: &str = "10-";

/// Everything that differs between the pages.
struct Page<'a> {
    title: &'a str,
    small: &'a str,
    status: &'a str,
    phase: &'a str,
    lede: &'a str,
    example: &'a str,
    date: &'a str,
    body: &'a str,
}

fn section_badge(number: &str) -> String {
    format!("<button class=\"sec\" data-sec>§{number}</button>")
}

fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

/// The date in a document's `Status:` line (`Status: **Accepted** · Phase 3 of 5 · 2026-10-04`).
///
/// The pages carry the date of their sources, not today's date, so regenerating them never
/// changes anything by itself (DOC-GENERATED).
fn status_date(md: &str, file: &str) -> anyhow::Result<String> {
    let line = md
        .lines()
        .find(|line| line.starts_with("Status:"))
        .with_context(|| format!("{file} has no `Status:` line"))?;
    let bytes = line.as_bytes();
    let found = (0..bytes.len().saturating_sub(9)).find(|&at| {
        let window = &bytes[at..at + 10];
        window.iter().enumerate().all(|(i, byte)| match i {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
    });
    match found {
        Some(at) => Ok(line[at..at + 10].to_owned()),
        None => bail!("The `Status:` line of {file} has no date like 2026-10-04."),
    }
}

pub(super) fn architecture(repo: &Repo, catalog: &Catalog) -> anyhow::Result<String> {
    let chapters = read_chapters(repo)?;
    let adrs = read_adrs(repo)?;
    let links = Links {
        page: String::new(),
        chapters: chapters.iter().map(|(path, _)| file_name(path)).collect(),
    };
    let date = status_date(&read_text(repo, ARCHITECTURE_README)?, ARCHITECTURE_README)?;

    let mut body = String::new();
    for (index, (path, md)) in chapters.iter().enumerate() {
        let number = index + 1;
        let file = file_name(path);
        let mut sub = 0_usize;
        let mut seen_title = false;
        let mut html = markdown::render(md, &links, |level| {
            chapter_heading(level, number, &mut sub, &mut seen_title)
        });
        if file.starts_with(CHECKS_CHAPTER_PREFIX) {
            sub += 1;
            let _ = write!(
                html,
                "<h3 id=\"s{number}-{sub}\">{}All checks</h3>\n{}",
                section_badge(&format!("{number}.{sub}")),
                catalog_table(catalog)
            );
        }
        let _ = writeln!(body, "<!-- {file} -->\n{html}");
    }

    let adr_chapter = chapters.len() + 1;
    let _ = write!(
        body,
        "<h2 id=\"s{adr_chapter}\">{}Decision records</h2>\n<p>Each record explains one decision: the situation, what we decided, what follows from it, what else we considered, and which checks enforce it.</p>\n",
        section_badge(&adr_chapter.to_string())
    );
    for (index, (path, md)) in adrs.iter().enumerate() {
        let file = file_name(path);
        let number = &file[..4];
        let mut seen_title = false;
        let html = markdown::render(md, &links, |level| {
            adr_heading(level, number, adr_chapter, index + 1, &mut seen_title)
        });
        let _ = writeln!(body, "<!-- {file} -->\n{html}");
    }

    Ok(render_page(&Page {
        title: "Bugaboo architecture",
        small: "Architecture",
        status: "<span class=\"pill st-done\">Accepted</span>",
        phase: "Phase 3 of 5",
        lede: "How Bugaboo is built: the big decisions, the parts and how they work together, how it runs when sold or served, the technology underneath, and the checks that keep the code true to this design. Feature IDs (like SEC-04) refer to the feature list.",
        example: "§7.2",
        date: &date,
        body: &body,
    }))
}

/// Chapter headings move one level down; the chapter title and its sections are numbered.
fn chapter_heading(level: u8, chapter: usize, sub: &mut usize, seen_title: &mut bool) -> Heading {
    match level {
        1 if !*seen_title => {
            *seen_title = true;
            (
                format!(
                    "<h2 id=\"s{chapter}\">{}",
                    section_badge(&chapter.to_string())
                ),
                "</h2>".to_owned(),
            )
        }
        2 => {
            *sub += 1;
            (
                format!(
                    "<h3 id=\"s{chapter}-{sub}\">{}",
                    section_badge(&format!("{chapter}.{sub}"))
                ),
                "</h3>".to_owned(),
            )
        }
        3..=5 => (format!("<h{}>", level + 1), format!("</h{}>", level + 1)),
        other => (format!("<h{other}>"), format!("</h{other}>")),
    }
}

/// Decision-record headings move two levels down; the title is numbered.
fn adr_heading(
    level: u8,
    number: &str,
    adr_chapter: usize,
    position: usize,
    seen_title: &mut bool,
) -> Heading {
    if level == 1 && !*seen_title {
        *seen_title = true;
        return (
            format!(
                "<h3 id=\"adr-{number}\">{}",
                section_badge(&format!("{adr_chapter}.{position}"))
            ),
            "</h3>".to_owned(),
        );
    }
    let shifted = (level + 2).min(6);
    (format!("<h{shifted}>"), format!("</h{shifted}>"))
}

pub(super) fn milestones(repo: &Repo) -> anyhow::Result<String> {
    let source = read_text(repo, MILESTONES)?;
    let date = status_date(&source, MILESTONES)?;
    let md = strip_title_and_status(&source);
    let links = Links {
        page: file_name(ARCHITECTURE_PATH),
        chapters: chapter_files(repo)?,
    };
    let lede = format!(
        "The plan to build Bugaboo: {} milestones, how each is built, tested and reviewed, and how you check and approve it.",
        count_milestones(&source)
    );
    let mut section = 0_usize;
    let mut sub = 0_usize;
    let body = markdown::render(md, &links, |level| match level {
        2 => {
            section += 1;
            sub = 0;
            (
                format!(
                    "<h2 id=\"s{section}\">{}",
                    section_badge(&section.to_string())
                ),
                "</h2>".to_owned(),
            )
        }
        3 => {
            sub += 1;
            (
                format!(
                    "<h3 id=\"s{section}-{sub}\">{}",
                    section_badge(&format!("{section}.{sub}"))
                ),
                "</h3>".to_owned(),
            )
        }
        4 => ("<h5>".to_owned(), "</h5>".to_owned()),
        other => (format!("<h{other}>"), format!("</h{other}>")),
    });
    Ok(render_page(&Page {
        title: "Bugaboo milestones",
        small: "Milestones",
        status: "<span class=\"pill st-ip\">Draft for review</span>",
        phase: "Phase 4 of 5",
        lede: &lede,
        example: "§5",
        date: &date,
        body: &body,
    }))
}

/// The number of milestones: the `## M<number> …` headings.
fn count_milestones(md: &str) -> usize {
    md.lines()
        .filter(|line| {
            line.strip_prefix("## M")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .count()
}

/// Drops the document's first heading and its `Status:` line; the page header shows them.
fn strip_title_and_status(md: &str) -> &str {
    let Some(rest) = md.strip_prefix("# ") else {
        return md;
    };
    let after_title = rest.split_once('\n').map_or("", |(_, tail)| tail);
    let trimmed = after_title.trim_start_matches('\n');
    match trimmed.strip_prefix("Status:") {
        Some(status) => status
            .split_once('\n')
            .map_or("", |(_, tail)| tail.trim_start_matches('\n')),
        None => md,
    }
}

/// The check catalogue as a table, so the catalogue stays the single source.
fn catalog_table(catalog: &Catalog) -> String {
    let mut rows = String::new();
    for entry in catalog.entries() {
        let _ = writeln!(
            rows,
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_text(&entry.id),
            escape_text(&entry.title),
            escape_text(&entry.rule),
            escape_text(&entry.tool),
            escape_text(&entry.when.join(" · ")),
        );
    }
    format!(
        "<div class=\"tbl-wrap\"><table class=\"dtable\"><thead><tr><th>ID</th><th>Catches</th><th>Rule</th><th>How</th><th>When</th></tr></thead><tbody>\n{rows}</tbody></table></div>\n<p>{} checks, generated from <code>tools/checks/catalog.toml</code>.</p>\n",
        catalog.entries().len()
    )
}

const TEMPLATE: &str = r#"<!-- {{marker}} `cargo xtask docs`. Edit the Markdown sources, never this file. -->
<title>{{title}}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Mono:wght@400;600&family=Atkinson+Hyperlegible+Next:wght@400;500;600;700&display=swap">
<link rel="stylesheet" href="theme.css">
<link rel="stylesheet" href="components.css">
<link rel="stylesheet" href="docpage.css">

<header class="top"><div class="top-in">
  <div class="brand"><svg viewBox="0 0 32 32" aria-hidden="true"><circle class="logo-fill-accent" cx="16" cy="17" r="11"/><path class="logo-stroke-canvas" d="M16 6v22" stroke-width="2"/><circle class="logo-fill-canvas" cx="11.5" cy="14" r="2.2"/><circle class="logo-fill-canvas" cx="20.5" cy="14" r="2.2"/><circle class="logo-fill-canvas" cx="12" cy="21.5" r="1.8"/><circle class="logo-fill-canvas" cx="20" cy="21.5" r="1.8"/><path class="logo-stroke-accent" d="M12 6.5 9 3M20 6.5 23 3" stroke-width="2" stroke-linecap="round" fill="none"/></svg>Bugaboo <small>{{small}}</small></div>
  <label class="look-pick" for="look">Theme
    <select id="look"><option value="system">Match my device</option><option value="daylight">Daylight</option><option value="paper">Paper</option><option value="dusk">Dusk</option><option value="midnight">Midnight</option></select></label>
</div></header>

<div class="layout">
  <nav class="toc" aria-label="Contents"><div class="toc-h">Contents</div><div id="toc"></div></nav>
  <main class="doc">
    <details class="toc-phone"><summary>Contents</summary><nav id="toc-phone" class="toc"></nav></details>
    <h1>{{title}}</h1>
    <div class="status-line">{{status}}<span>{{phase}}</span><span>·</span><span>{{date}}</span></div>
    <p class="lede">{{lede}}</p>
    <div class="note feedback"><b>How to give feedback</b>Use this page's comment button. Mention the section number shown next to each heading (for example <span class="sec">{{example}}</span>), or select text and comment on it. Tapping a section number copies it, ready to paste into your comment.</div>
{{body}}
  </main>
</div>
<script src="frame.js"></script>
<script src="docpage.js"></script>
"#;

fn render_page(page: &Page<'_>) -> String {
    // The body goes in last, so text inside it that looks like a placeholder stays untouched.
    TEMPLATE
        .replace("{{marker}}", GENERATED_MARKER)
        .replace("{{title}}", page.title)
        .replace("{{small}}", page.small)
        .replace("{{status}}", page.status)
        .replace("{{phase}}", page.phase)
        .replace("{{lede}}", page.lede)
        .replace("{{example}}", page.example)
        .replace("{{date}}", page.date)
        .replace("{{body}}", page.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_comes_from_the_status_line() {
        let md = "# T\n\nStatus: **Accepted** · Phase 3 of 5 · 2026-10-04\n";
        assert_eq!(status_date(md, "x.md").unwrap(), "2026-10-04");
        assert!(status_date("# T\n", "x.md").is_err());
        assert!(status_date("Status: draft\n", "x.md").is_err());
    }

    #[test]
    fn title_and_status_are_left_out_of_the_milestone_body() {
        let md = "# 04 — Milestones\n\nStatus: **Draft** · 2026-10-04\n\nIntro.\n\n## One\n";
        assert_eq!(strip_title_and_status(md), "Intro.\n\n## One\n");
        assert_eq!(strip_title_and_status("Intro.\n"), "Intro.\n");
    }

    #[test]
    fn chapter_headings_are_numbered_and_shifted() {
        let mut sub = 0;
        let mut seen = false;
        let (open, close) = chapter_heading(1, 4, &mut sub, &mut seen);
        assert_eq!(close, "</h2>");
        assert!(open.starts_with("<h2 id=\"s4\">") && open.contains("§4<"));
        let (open, _) = chapter_heading(2, 4, &mut sub, &mut seen);
        assert!(open.starts_with("<h3 id=\"s4-1\">") && open.contains("§4.1<"));
        assert_eq!(chapter_heading(3, 4, &mut sub, &mut seen).0, "<h4>");
    }

    #[test]
    fn catalogue_table_lists_every_check() {
        let catalog = Catalog::parse(
            r#"
[[check]]
id = "A-1"
title = "Fish & chips"
rule = "r"
tool = "t"
when = ["edit", "ci"]
files = ["*"]
status = "active"
"#,
        )
        .unwrap();
        let html = catalog_table(&catalog);
        assert!(html.contains("<code>A-1</code></td><td>Fish &amp; chips</td>"));
        assert!(html.contains("edit · ci"));
        assert!(html.contains("<p>1 checks, generated from"));
    }
}
