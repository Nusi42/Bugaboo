/* Bugaboo document pages — theme picker, contents and copyable section numbers.
   Expects: <select id="look">, #toc, #toc-phone, .toc-phone, headings .doc h2[id]/h3[id] with a .sec[data-sec] button. */
"use strict";
(function () {
  /* ---------- theme ---------- */
  const root = document.documentElement;
  let look = "system";
  try {
    look = localStorage.getItem("bugaboo-doc-look") ?? "system";
  } catch {
    /* Storage is blocked: the choice lasts until the page closes. */
  }
  /* "Match my device" sets no data-look: the generated token CSS then follows the device (and the
     host's data-theme), so no theme name is repeated here. */
  function applyLook() {
    if (look === "system") delete root.dataset.look;
    else root.dataset.look = look;
  }
  const sel = document.querySelector("#look");
  sel.value = look;
  sel.addEventListener("change", () => {
    look = sel.value;
    try {
      localStorage.setItem("bugaboo-doc-look", look);
    } catch {
      /* Storage is blocked: the choice lasts until the page closes. */
    }
    applyLook();
  });
  applyLook();

  /* ---------- contents (built from the headings, so it never drifts) ---------- */
  const heads = [...document.querySelectorAll(".doc h2[id], .doc h3[id]")];
  function tocHtml() {
    return heads
      .map((h) => {
        const n = h.querySelector(".sec").textContent;
        const text = [...h.childNodes]
          .filter(
            (x) => !(x.classList && (x.classList.contains("sec") || x.classList.contains("ids"))),
          )
          .map((x) => x.textContent)
          .join("")
          .trim();
        return (
          '<a href="#' +
          h.id +
          '" class="' +
          (h.tagName === "H3" ? "l3" : "") +
          '" data-for="' +
          h.id +
          '"><span class="n">' +
          n +
          "</span><span>" +
          text +
          "</span></a>"
        );
      })
      .join("");
  }
  document.querySelector("#toc").innerHTML = tocHtml();
  document.querySelector("#toc-phone").innerHTML = tocHtml();
  document.querySelector("#toc-phone").addEventListener("click", (e) => {
    if (e.target.closest("a")) document.querySelector(".toc-phone").open = false;
  });
  const tocLinks = [...document.querySelectorAll("#toc a")];
  const spy = new IntersectionObserver(
    (entries) => {
      entries.forEach((en) => {
        if (en.isIntersecting) {
          tocLinks.forEach((a) => a.classList.toggle("on", a.dataset.for === en.target.id));
          const on = document.querySelector("#toc a.on");
          if (on) on.scrollIntoView({ block: "nearest" });
        }
      });
    },
    { rootMargin: "-70px 0px -70% 0px" },
  );
  heads.forEach((h) => spy.observe(h));

  /* ---------- section numbers: copy "§4.4 Issue page" for comments ---------- */
  let tip;
  function say(msg) {
    if (tip) tip.remove();
    tip = document.createElement("div");
    tip.className = "copied";
    tip.setAttribute("role", "status");
    tip.textContent = msg;
    document.body.append(tip);
    setTimeout(() => tip && tip.remove(), 3200);
  }
  document.querySelectorAll("[data-sec]").forEach((b) => {
    const h = b.parentElement;
    b.setAttribute("aria-label", "Copy section number " + b.textContent);
    b.addEventListener("click", () => {
      const text = [...h.childNodes]
        .filter((x) => !(x.classList && x.classList.contains("ids")))
        .map((x) => x.textContent)
        .join("")
        .replaceAll(/\s+/gu, " ")
        .trim()
        .replace(/^(§[\d.]+)/u, "$1 ");
      const done = () => say('Copied "' + text + '". Paste it into your comment.');
      try {
        navigator.clipboard.writeText(text).then(done, () => say(text + " (select and copy this)"));
      } catch {
        say(text + " (select and copy this)");
      }
    });
  });
})();
