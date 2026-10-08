import stylelint from "stylelint";

// The rules of design/theme.kdl, compiled, are read with these rules (LINT-CSS, ADR-0026, coding-
// standards section 4). A rule holds no value of its own: not a length (not even a 1px line, which
// is the token `border-width`), not a color, a duration, a weight, a line height, a layer or an
// opacity. A zero, and an opacity of 0 or 1, say what they say without a token.

const lengthUnits =
  "px|rem|em|ex|ch|cap|lh|rlh|vw|vh|vmin|vmax|svw|svh|lvw|lvh|dvw|dvh|pt|pc|cm|mm|in|Q";
// A number with a length unit that is not part of a longer word or a variable name.
const anyLength = String.raw`/(?<![\w.#-])-?\d*\.?\d+(?:${lengthUnits})\b/i`;
const anyNumber = String.raw`/^-?\d*\.?\d+$/`;
const anyTime = String.raw`/(?<![\w.#-])-?\d*\.?\d+(?:ms|s)\b/i`;
const anyOpacity = String.raw`/^(?!(?:0|1)$)-?\d*\.?\d+$/`;

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
    // Sizes, weights, line heights, layers, opacities and times: tokens only.
    "declaration-property-value-disallowed-list": {
      "/.+/": [anyLength],
      "/^(?:line-height|font-weight|z-index)$/": [anyNumber],
      "/^opacity$/": [anyOpacity],
      "/^(?:transition|animation)/": [anyTime],
    },
    // Fonts: tokens only (a control may take the family of its parent, which comes from a token),
    // and no `font` shorthand that could hide a literal family or size.
    "declaration-property-value-allowed-list": {
      "font-family": [String.raw`/^(?:var\(--[\w-]+\)|inherit)$/`],
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
    "selector-pseudo-class-no-unknown": true,
    "selector-pseudo-element-no-unknown": true,
    "selector-type-no-unknown": true,
    "unit-no-unknown": true,
    "function-no-unknown": true,
    "at-rule-no-unknown": true,
  },
};
