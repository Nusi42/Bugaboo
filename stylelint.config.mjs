import stylelint from "stylelint";

// CSS Modules use design tokens only (ADR-0010, coding-standards section 4).
// Lengths and colors come from `var(--token)`; the only literal sizes allowed are 0 and a 1px border.

const lengthUnits =
  "px|rem|em|ex|ch|cap|lh|rlh|vw|vh|vmin|vmax|svw|svh|lvw|lvh|dvw|dvh|pt|pc|cm|mm|in|Q";
// A number with a length unit that is not part of a longer word or a variable name.
const anyLength = String.raw`/(?<![\w.#-])-?\d*\.?\d+(?:${lengthUnits})\b/i`;
// The same, except exactly 1px, which borders and outlines may use.
const lengthExceptHairline = String.raw`/(?<![\w.#-])(?!1px\b)-?\d*\.?\d+(?:${lengthUnits})\b/i`;

// The only properties that may use a literal 1px: the border and outline line widths (also the
// per-side border shorthands).
const hairlineProperties = String.raw`(?:border|outline|border(?:-[a-z]+)*-width|outline-width|border-(?:top|right|bottom|left|block|inline)(?:-(?:start|end))?)`;

// System colors (Highlight, CanvasText ...) are real colors chosen by the person's device. They are
// allowed only inside @media (forced-colors: active), where tokens no longer apply.
const ruleName = "bugaboo/system-colors-only-in-forced-colors";
const messages = stylelint.utils.ruleMessages(ruleName, {
  rejected: (word) =>
    `Unexpected system color "${word}". Use a token, or move it into @media (forced-colors: active).`,
});
const systemColor =
  /(?<![\w-])(?:AccentColor|AccentColorText|ActiveText|ButtonBorder|ButtonFace|ButtonText|Canvas|CanvasText|Field|FieldText|GrayText|Highlight|HighlightText|LinkText|Mark|MarkText|SelectedItem|SelectedItemText|VisitedText)(?![\w-])/iu;
const systemColorsPlugin = stylelint.createPlugin(ruleName, (enabled) => (root, result) => {
  if (!enabled) {
    return;
  }
  root.walkDecls((declaration) => {
    const word = systemColor.exec(declaration.value)?.[0];
    if (word === undefined) {
      return;
    }
    for (let node = declaration.parent; node; node = node.parent) {
      if (
        node.type === "atrule" &&
        node.name === "media" &&
        /forced-colors\s*:\s*active/iu.test(node.params)
      ) {
        return;
      }
    }
    stylelint.utils.report({
      ruleName,
      result,
      node: declaration,
      word,
      message: messages.rejected(word),
    });
  });
});
systemColorsPlugin.ruleName = ruleName;
systemColorsPlugin.messages = messages;

/** @type {import("stylelint").Config} */
export default {
  // The generated token sheet is the one place that holds raw values (gen tokens, ADR-0010).
  ignoreFiles: ["web/packages/ui/src/tokens.css"],
  plugins: [systemColorsPlugin],
  rules: {
    [ruleName]: true,
    // Colors: tokens only.
    "color-no-hex": true,
    "color-named": "never",
    "function-disallowed-list": [
      "rgb",
      "rgba",
      "hsl",
      "hsla",
      "hwb",
      "lab",
      "lch",
      "oklab",
      "oklch",
      "color",
    ],
    // Sizes: tokens only. Borders and outlines may be exactly 1px.
    "declaration-property-value-disallowed-list": {
      [`/^(?!${hairlineProperties}$)/`]: [anyLength],
      [`/^${hairlineProperties}$/`]: [lengthExceptHairline],
    },
    // Fonts: tokens only, and no `font` shorthand that could hide a literal family or size.
    "declaration-property-value-allowed-list": {
      "font-family": [String.raw`/^var\(--[\w-]+\)$/`],
    },
    "property-disallowed-list": ["font"],
    "declaration-no-important": true,
    // Correctness.
    "block-no-empty": true,
    "color-no-invalid-hex": true,
    "declaration-block-no-duplicate-properties": true,
    "declaration-block-no-shorthand-property-overrides": true,
    "font-family-no-missing-generic-family-keyword": true,
    "no-duplicate-selectors": true,
    "no-descending-specificity": true,
    "property-no-unknown": true,
    "selector-pseudo-class-no-unknown": [true, { ignorePseudoClasses: ["global", "local"] }],
    "selector-pseudo-element-no-unknown": true,
    "selector-type-no-unknown": true,
    "unit-no-unknown": true,
    "function-no-unknown": true,
    "at-rule-no-unknown": true,
    // CSS Modules: class names are camelCase so they read as `styles.className`.
    "selector-class-pattern": ["^[a-z][a-zA-Z0-9]*$", { resolveNestedSelectors: true }],
  },
};
