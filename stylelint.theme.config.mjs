import modules, { anyLength } from "./stylelint.config.mjs";

// The rules of design/theme.kdl, compiled, are read with the rules of the CSS Modules and a few more
// (LINT-CSS, ADR-0026). A rule holds no value of its own: not a length (not even a 1px line, which
// is the token `border-width`), not a duration, not a weight, a line height, a layer or an opacity.
// A zero, and an opacity of 0 or 1, say what they say without a token.
const anyNumber = String.raw`/^-?\d*\.?\d+$/`;
const anyTime = String.raw`/(?<![\w.#-])-?\d*\.?\d+(?:ms|s)\b/i`;
const anyOpacity = String.raw`/^(?!(?:0|1)$)-?\d*\.?\d+$/`;

/** @type {import("stylelint").Config} */
export default {
  ...modules,
  // Nothing is ignored here: the compiled rules hold no raw value.
  ignoreFiles: [],
  rules: {
    ...modules.rules,
    "declaration-property-value-disallowed-list": {
      "/.+/": [anyLength],
      "/^(?:line-height|font-weight|z-index)$/": [anyNumber],
      "/^opacity$/": [anyOpacity],
      "/^(?:transition|animation)/": [anyTime],
    },
  },
};
