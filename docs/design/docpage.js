/* Bugaboo document pages — theme picker, contents and copyable section numbers.
   Expects: <select id="look">, #toc, #toc-phone, .toc-phone, headings .doc h2[id]/h3[id] with a .sec[data-sec] button. */
"use strict";
(function () {
/* ---------- theme ---------- */
const root = document.documentElement;
let look = "system";
try { look = localStorage.getItem("bugaboo-doc-look") || "system"; } catch (e) {}
function hostIsDark() {
  const t = root.getAttribute("data-theme");
  if (t === "dark") return true;
  if (t === "light") return false;
  return matchMedia("(prefers-color-scheme: dark)").matches;
}
function applyLook() {
  root.setAttribute("data-look", look === "system" ? (hostIsDark() ? "dusk" : "daylight") : look);
}
const sel = document.getElementById("look");
sel.value = look;
sel.addEventListener("change", () => {
  look = sel.value;
  try { localStorage.setItem("bugaboo-doc-look", look); } catch (e) {}
  applyLook();
});
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyLook);
new MutationObserver(applyLook).observe(root, { attributes: true, attributeFilter: ["data-theme"] });
applyLook();

/* ---------- contents (built from the headings, so it never drifts) ---------- */
const heads = [...document.querySelectorAll(".doc h2[id], .doc h3[id]")];
function tocHtml() {
  return heads.map(h => {
    const n = h.querySelector(".sec").textContent;
    const text = [...h.childNodes].filter(x => !(x.classList && (x.classList.contains("sec") || x.classList.contains("ids")))).map(x => x.textContent).join("").trim();
    return '<a href="#' + h.id + '" class="' + (h.tagName === "H3" ? "l3" : "") + '" data-for="' + h.id + '"><span class="n">' + n + "</span><span>" + text + "</span></a>";
  }).join("");
}
document.getElementById("toc").innerHTML = tocHtml();
document.getElementById("toc-phone").innerHTML = tocHtml();
document.getElementById("toc-phone").addEventListener("click", e => { if (e.target.closest("a")) document.querySelector(".toc-phone").open = false; });
const tocLinks = [...document.querySelectorAll("#toc a")];
const spy = new IntersectionObserver(entries => {
  entries.forEach(en => {
    if (en.isIntersecting) {
      tocLinks.forEach(a => a.classList.toggle("on", a.dataset.for === en.target.id));
      const on = document.querySelector("#toc a.on");
      if (on) on.scrollIntoView({ block: "nearest" });
    }
  });
}, { rootMargin: "-70px 0px -70% 0px" });
heads.forEach(h => spy.observe(h));

/* ---------- section numbers: copy "§4.4 Issue page" for comments ---------- */
let tip;
function say(msg) {
  if (tip) tip.remove();
  tip = document.createElement("div");
  tip.className = "copied"; tip.setAttribute("role", "status"); tip.textContent = msg;
  document.body.appendChild(tip);
  setTimeout(() => tip && tip.remove(), 3200);
}
document.querySelectorAll("[data-sec]").forEach(b => {
  const h = b.parentElement;
  b.setAttribute("aria-label", "Copy section number " + b.textContent);
  b.addEventListener("click", () => {
    const text = [...h.childNodes].filter(x => !(x.classList && x.classList.contains("ids"))).map(x => x.textContent).join("").replace(/\s+/g, " ").trim().replace(/^(§[\d.]+)/, "$1 ");
    const done = () => say('Copied "' + text + '". Paste it into your comment.');
    try {
      navigator.clipboard.writeText(text).then(done, () => say(text + " (select and copy this)"));
    } catch (e) { say(text + " (select and copy this)"); }
  });
});


})();
