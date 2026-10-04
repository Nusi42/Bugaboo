// Builds docs/03-architecture.html from docs/architecture/*.md, docs/adr/*.md and tools/checks/catalog.toml.
// The Markdown and the catalogue are the single sources; never edit the generated HTML.
// Usage: npm run build  (from tools/docs-html)
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { Marked } from "marked";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const docs = join(root, "docs");
const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

const marked = new Marked({
  gfm: true,
  renderer: {
    code({ text, lang }) {
      return lang === "mermaid"
        ? `<pre class="mermaid">${esc(text)}</pre>\n`
        : `<pre><code>${esc(text)}</code></pre>\n`;
    },
    table(token) {
      return `<div class="tbl-wrap">${this.constructor.prototype.table.call(this, token).replace("<table>", '<table class="dtable">')}</div>\n`;
    },
  },
});

const chapters = readdirSync(join(docs, "architecture"))
  .filter((f) => /^\d\d-.*\.md$/.test(f))
  .sort();
const adrs = readdirSync(join(docs, "adr"))
  .filter((f) => /^\d{4}-.*\.md$/.test(f))
  .sort();

// Map links between the Markdown files to anchors inside the one HTML page.
function fixLinks(html) {
  return html.replace(/<a href="([^"]+)">(.*?)<\/a>/g, (m, href, text) => {
    const ch = href.match(/(?:^|\/)(\d\d)-[\w-]+\.md$/);
    if (ch && !href.includes("adr/")) return `<a href="#s${+ch[1]}">${text}</a>`;
    const adr = href.match(/adr\/(\d{4})-[\w-]+\.md$/) || href.match(/^(\d{4})-[\w-]+\.md$/);
    if (adr) return `<a href="#adr-${adr[1]}">${text}</a>`;
    if (/^https?:/.test(href))
      return `<a href="${href}" target="_blank" rel="noopener">${text}</a>`;
    return `<span title="${esc(href)} in the repository">${text}</span>`;
  });
}
const badge = (n) => `<button class="sec" data-sec>§${n}</button>`;

// The check catalogue is rendered as a table, so it stays the single source.
// Supports the catalogue's simple format only: [[check]] blocks with key = "string" or key = ["a", "b"].
function catalogTable() {
  const toml = readFileSync(join(root, "tools", "checks", "catalog.toml"), "utf8");
  const checks = [];
  for (const line of toml.split(/\r?\n/)) {
    if (line.startsWith("[[check]]")) {
      checks.push({});
      continue;
    }
    const m = line.match(/^(\w+)\s*=\s*(.+)$/);
    if (!m || !checks.length) continue;
    checks[checks.length - 1][m[1]] = JSON.parse(m[2].trim()); // strings and string arrays are valid JSON
  }
  const rows = checks
    .map(
      (c) =>
        `<tr><td><code>${esc(c.id)}</code></td><td>${esc(c.title)}</td><td>${esc(c.rule)}</td><td>${esc(c.tool)}</td><td>${esc(c.when.join(" · "))}</td></tr>`,
    )
    .join("\n");
  return `<div class="tbl-wrap"><table class="dtable"><thead><tr><th>ID</th><th>Catches</th><th>Rule</th><th>How</th><th>When</th></tr></thead><tbody>\n${rows}\n</tbody></table></div>\n<p>${checks.length} checks, generated from <code>tools/checks/catalog.toml</code>.</p>\n`;
}

let body = "";
chapters.forEach((file, i) => {
  const n = i + 1;
  let sub = 0;
  let html = marked.parse(readFileSync(join(docs, "architecture", file), "utf8"));
  html = html
    .replace(/<h4>/g, "<h5>")
    .replace(/<\/h4>/g, "</h5>")
    .replace(/<h3>/g, "<h4>")
    .replace(/<\/h3>/g, "</h4>")
    .replace(/<h2>(.*?)<\/h2>/g, (m, t) => {
      sub++;
      return `<h3 id="s${n}-${sub}">${badge(`${n}.${sub}`)}${t}</h3>`;
    })
    .replace(/<h1>(.*?)<\/h1>/, (m, t) => `<h2 id="s${n}">${badge(n)}${t}</h2>`);
  if (file.startsWith("10-")) {
    sub++;
    html += `<h3 id="s${n}-${sub}">${badge(`${n}.${sub}`)}All checks</h3>\n${catalogTable()}`;
  }
  body += `<!-- ${file} -->\n${fixLinks(html)}\n`;
});

const adrNo = chapters.length + 1;
body += `<h2 id="s${adrNo}">${badge(adrNo)}Decision records</h2>\n<p>Each record explains one decision: the situation, what we decided, what follows from it, what else we considered, and which checks enforce it.</p>\n`;
adrs.forEach((file, k) => {
  const num = file.slice(0, 4);
  let html = marked.parse(readFileSync(join(docs, "adr", file), "utf8"));
  html = html
    .replace(/<h2>/g, "<h4>")
    .replace(/<\/h2>/g, "</h4>")
    .replace(
      /<h1>(.*?)<\/h1>/,
      (m, t) => `<h3 id="adr-${num}">${badge(`${adrNo}.${k + 1}`)}${t}</h3>`,
    );
  body += `<!-- ${file} -->\n${fixLinks(html)}\n`;
});

function page({ title, small, status, phase, lede, example, body }) {
  return `<title>${title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Mono:wght@400;600&family=Atkinson+Hyperlegible+Next:wght@400;500;600;700&display=swap">
<link rel="stylesheet" href="tokens.css">
<link rel="stylesheet" href="components.css">
<link rel="stylesheet" href="docpage.css">
<!-- GENERATED by tools/docs-html/build.mjs. Edit the Markdown sources, never this file. -->

<header class="top"><div class="top-in">
  <div class="brand"><svg viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="17" r="11" style="fill:var(--accent)"/><path d="M16 6v22" style="stroke:var(--bg-canvas)" stroke-width="2"/><circle cx="11.5" cy="14" r="2.2" style="fill:var(--bg-canvas)"/><circle cx="20.5" cy="14" r="2.2" style="fill:var(--bg-canvas)"/><circle cx="12" cy="21.5" r="1.8" style="fill:var(--bg-canvas)"/><circle cx="20" cy="21.5" r="1.8" style="fill:var(--bg-canvas)"/><path d="M12 6.5 9 3M20 6.5 23 3" style="stroke:var(--accent)" stroke-width="2" stroke-linecap="round" fill="none"/></svg>Bugaboo <small>${small}</small></div>
  <label class="look-pick" for="look">Theme
    <select id="look"><option value="system">Match my device</option><option value="daylight">Daylight</option><option value="paper">Paper</option><option value="dusk">Dusk</option><option value="midnight">Midnight</option></select></label>
</div></header>

<div class="layout">
  <nav class="toc" aria-label="Contents"><div class="toc-h">Contents</div><div id="toc"></div></nav>
  <main class="doc">
    <details class="toc-phone"><summary>Contents</summary><nav id="toc-phone" class="toc" style="display:block;position:static;max-height:none;padding:0"></nav></details>
    <h1>${title}</h1>
    <div class="status-line">${status}<span>${phase}</span><span>·</span><span>${new Date().toISOString().slice(0, 10)}</span></div>
    <p class="lede">${lede}</p>
    <div class="note feedback"><b>How to give feedback</b>Use this page's comment button. Mention the section number shown next to each heading (for example <span class="sec" style="vertical-align:0">${example}</span>), or select text and comment on it. Tapping a section number copies it, ready to paste into your comment.</div>
${body}
  </main>
</div>
<script src="docpage.js"></script>
`;
}

writeFileSync(
  join(docs, "03-architecture.html"),
  page({
    title: "Bugaboo architecture",
    small: "Architecture",
    phase: "Phase 3 of 5",
    example: "§7.2",
    status: '<span class="pill st-done">Accepted</span>',
    lede: "How Bugaboo is built: the big decisions, the parts and how they work together, how it runs when sold or served, the technology underneath, and the checks that keep the code true to this design. Feature IDs (like SEC-04) refer to the feature list.",
    body,
  }),
);
console.log(
  `Wrote docs/03-architecture.html (${chapters.length} chapters, ${adrs.length} decision records)`,
);

// Milestones: one Markdown file; its first heading and status line become the page header.
{
  const md = readFileSync(join(docs, "04-milestones.md"), "utf8").replace(
    /^# .*\n+Status: .*\n+/,
    "",
  );
  // Number each h2 section and its h3 subsections in order.
  let n = 0,
    sub = 0;
  const html = marked
    .parse(md)
    .replace(/<h4>/g, "<h5>")
    .replace(/<\/h4>/g, "</h5>")
    .replace(/<h([23])>(.*?)<\/h\1>/g, (m, level, t) => {
      if (level === "2") {
        n++;
        sub = 0;
        return `<h2 id="s${n}">${badge(n)}${t}</h2>`;
      }
      sub++;
      return `<h3 id="s${n}-${sub}">${badge(`${n}.${sub}`)}${t}</h3>`;
    });
  writeFileSync(
    join(docs, "04-milestones.html"),
    page({
      title: "Bugaboo milestones",
      small: "Milestones",
      phase: "Phase 4 of 5",
      example: "§5",
      status: '<span class="pill st-ip">Draft for review</span>',
      lede: "The plan to build Bugaboo: 16 milestones, how each is built, tested and reviewed, and how you check and approve it.",
      body: fixLinks(html),
    }),
  );
  console.log("Wrote docs/04-milestones.html");
}
