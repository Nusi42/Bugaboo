// The browser app's HTML (web/app/index.html) gets the strict presets with no exceptions.
// The hand-written documentation pages have their own, looser config: htmlvalidate.docs.config.mjs.
export default {
  extends: ["html-validate:recommended", "html-validate:a11y"],
};
