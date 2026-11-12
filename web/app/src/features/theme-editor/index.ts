import "@bugaboo/messages/parts/theme-editor";

export { OwnThemes } from "./OwnThemes.tsx";
export { ownThemeChoices, themeKinds, type OwnThemeChoice } from "./choices.ts";
export { loadThemeEditor } from "./load.ts";
export { useOwnThemes, type ThemeValues } from "./themes.ts";
// What the editor makes of the rating of `bb-core`, which its test asks the module about.
export { hardPairsOf, pairKey, ratingOf, withColor, withMore, withoutColor } from "./model.ts";
