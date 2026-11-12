import { t } from "@bugaboo/messages";
import { type ChoiceColorName, type ColorToken } from "@bugaboo/ui";

/**
 * The groups of the theme editor (UX §5.6), in the order the editor shows them. *Shadow, corners
 * and fonts* is the last one and holds no colors.
 */
export const colorGroups = ["backgrounds", "text", "lines", "highlight", "status"] as const;
export type ColorGroup = (typeof colorGroups)[number];

/** The names of the groups. */
export const groupLabels: Readonly<Record<ColorGroup, string>> = {
  backgrounds: t("theme-editor.group.backgrounds"),
  text: t("theme-editor.group.text"),
  lines: t("theme-editor.group.lines"),
  highlight: t("theme-editor.group.highlight"),
  status: t("theme-editor.group.status"),
};

/** Where a color stands in the editor, and its plain name. */
export interface ColorWord {
  readonly group: ColorGroup;
  readonly label: string;
}

/** The side of a choice color that a token is. */
type ChoiceSide = "bg" | "fg" | "mark";

/** The plain name of a side of a choice color: *Pink: background*. */
function choice(color: ChoiceColorName, side: ChoiceSide): string {
  const words = { color: t(`choice-color.${color}`) };
  const sides: Readonly<Record<ChoiceSide, string>> = {
    bg: t("theme-color.choice-bg", words),
    fg: t("theme-color.choice-fg", words),
    mark: t("theme-color.choice-mark", words),
  };
  return sides[side];
}

/** The areas of the menu, each with a color of its own. */
type Area = "issues" | "time" | "agents" | "settings";

/** What a color of an area is used for. */
type AreaSide = "main" | "open" | "bar" | "line" | "soft";

/** The plain name of a color of an area: *Issues area, bar*. */
function area(which: Area, side: AreaSide): string {
  const names: Readonly<Record<Area, string>> = {
    issues: t("theme-color.area-name.issues"),
    time: t("theme-color.area-name.time"),
    agents: t("theme-color.area-name.agents"),
    settings: t("theme-color.area-name.settings"),
  };
  const words = { area: names[which] };
  const sides: Readonly<Record<AreaSide, string>> = {
    main: t("theme-color.area-main", words),
    open: t("theme-color.area-open", words),
    bar: t("theme-color.area-bar", words),
    line: t("theme-color.area-line", words),
    soft: t("theme-color.area-soft", words),
  };
  return sides[side];
}

/**
 * Every color an own theme may set (the generated list of the theme file), with its group and its
 * plain name. The type makes the list whole: a color the theme file adds needs a line here.
 */
export const colorWords: Readonly<Record<ColorToken, ColorWord>> = {
  "bg-canvas": { group: "backgrounds", label: t("theme-color.bg-canvas") },
  "bg-surface": { group: "backgrounds", label: t("theme-color.bg-surface") },
  "bg-raised": { group: "backgrounds", label: t("theme-color.bg-raised") },
  "bg-sunken": { group: "backgrounds", label: t("theme-color.bg-sunken") },
  "bg-hover": { group: "backgrounds", label: t("theme-color.bg-hover") },
  "bg-selected": { group: "backgrounds", label: t("theme-color.bg-selected") },
  border: { group: "lines", label: t("theme-color.border") },
  "border-control": { group: "lines", label: t("theme-color.border-control") },
  cue: { group: "lines", label: t("theme-color.cue") },
  "border-subtle": { group: "lines", label: t("theme-color.border-subtle") },
  divider: { group: "lines", label: t("theme-color.divider") },
  fg: { group: "text", label: t("theme-color.fg") },
  "fg-muted": { group: "text", label: t("theme-color.fg-muted") },
  "fg-subtle": { group: "text", label: t("theme-color.fg-subtle") },
  accent: { group: "highlight", label: t("theme-color.accent") },
  "accent-soft": { group: "highlight", label: t("theme-color.accent-soft") },
  "accent-fg": { group: "highlight", label: t("theme-color.accent-fg") },
  "disabled-fill": { group: "backgrounds", label: t("theme-color.disabled-fill") },
  "on-disabled": { group: "text", label: t("theme-color.on-disabled") },
  "on-accent": { group: "highlight", label: t("theme-color.on-accent") },
  danger: { group: "status", label: t("theme-color.danger") },
  "danger-soft": { group: "status", label: t("theme-color.danger-soft") },
  warning: { group: "status", label: t("theme-color.warning") },
  "warning-soft": { group: "status", label: t("theme-color.warning-soft") },
  success: { group: "status", label: t("theme-color.success") },
  "success-soft": { group: "status", label: t("theme-color.success-soft") },
  "st-ns": { group: "status", label: t("theme-color.st-ns") },
  "st-ns-soft": { group: "status", label: t("theme-color.st-ns-soft") },
  "st-ip": { group: "status", label: t("theme-color.st-ip") },
  "st-ip-soft": { group: "status", label: t("theme-color.st-ip-soft") },
  "st-done": { group: "status", label: t("theme-color.st-done") },
  "st-done-soft": { group: "status", label: t("theme-color.st-done-soft") },
  "tint-ns": { group: "status", label: t("theme-color.tint-ns") },
  "tint-ip": { group: "status", label: t("theme-color.tint-ip") },
  "tint-done": { group: "status", label: t("theme-color.tint-done") },
  "edge-high": { group: "status", label: t("theme-color.edge-high") },
  "hover-line": { group: "lines", label: t("theme-color.hover-line") },
  scrim: { group: "backgrounds", label: t("theme-color.scrim") },
  "cat1-bg": { group: "status", label: choice("cat1", "bg") },
  "cat1-fg": { group: "status", label: choice("cat1", "fg") },
  "cat2-bg": { group: "status", label: choice("cat2", "bg") },
  "cat2-fg": { group: "status", label: choice("cat2", "fg") },
  "cat3-bg": { group: "status", label: choice("cat3", "bg") },
  "cat3-fg": { group: "status", label: choice("cat3", "fg") },
  "cat4-bg": { group: "status", label: choice("cat4", "bg") },
  "cat4-fg": { group: "status", label: choice("cat4", "fg") },
  "cat5-bg": { group: "status", label: choice("cat5", "bg") },
  "cat5-fg": { group: "status", label: choice("cat5", "fg") },
  "cat6-bg": { group: "status", label: choice("cat6", "bg") },
  "cat6-fg": { group: "status", label: choice("cat6", "fg") },
  "cat7-bg": { group: "status", label: choice("cat7", "bg") },
  "cat7-fg": { group: "status", label: choice("cat7", "fg") },
  "cat8-bg": { group: "status", label: choice("cat8", "bg") },
  "cat8-fg": { group: "status", label: choice("cat8", "fg") },
  "cat9-bg": { group: "status", label: choice("cat9", "bg") },
  "cat9-fg": { group: "status", label: choice("cat9", "fg") },
  "cat10-bg": { group: "status", label: choice("cat10", "bg") },
  "cat10-fg": { group: "status", label: choice("cat10", "fg") },
  "cat11-bg": { group: "status", label: choice("cat11", "bg") },
  "cat11-fg": { group: "status", label: choice("cat11", "fg") },
  "cat12-bg": { group: "status", label: choice("cat12", "bg") },
  "cat12-fg": { group: "status", label: choice("cat12", "fg") },
  "cat1-mark": { group: "status", label: choice("cat1", "mark") },
  "cat2-mark": { group: "status", label: choice("cat2", "mark") },
  "cat3-mark": { group: "status", label: choice("cat3", "mark") },
  "cat4-mark": { group: "status", label: choice("cat4", "mark") },
  "cat5-mark": { group: "status", label: choice("cat5", "mark") },
  "cat6-mark": { group: "status", label: choice("cat6", "mark") },
  "cat7-mark": { group: "status", label: choice("cat7", "mark") },
  "cat8-mark": { group: "status", label: choice("cat8", "mark") },
  "cat9-mark": { group: "status", label: choice("cat9", "mark") },
  "cat10-mark": { group: "status", label: choice("cat10", "mark") },
  "cat11-mark": { group: "status", label: choice("cat11", "mark") },
  "cat12-mark": { group: "status", label: choice("cat12", "mark") },
  "area-ink": { group: "highlight", label: t("theme-color.area-ink") },
  "area-issues": { group: "highlight", label: area("issues", "main") },
  "area-issues-open": { group: "highlight", label: area("issues", "open") },
  "area-issues-bar": { group: "highlight", label: area("issues", "bar") },
  "area-issues-line": { group: "highlight", label: area("issues", "line") },
  "area-issues-soft": { group: "highlight", label: area("issues", "soft") },
  "area-time": { group: "highlight", label: area("time", "main") },
  "area-time-open": { group: "highlight", label: area("time", "open") },
  "area-time-bar": { group: "highlight", label: area("time", "bar") },
  "area-time-line": { group: "highlight", label: area("time", "line") },
  "area-time-soft": { group: "highlight", label: area("time", "soft") },
  "area-agents": { group: "highlight", label: area("agents", "main") },
  "area-agents-open": { group: "highlight", label: area("agents", "open") },
  "area-agents-bar": { group: "highlight", label: area("agents", "bar") },
  "area-agents-line": { group: "highlight", label: area("agents", "line") },
  "area-agents-soft": { group: "highlight", label: area("agents", "soft") },
  "area-settings": { group: "highlight", label: area("settings", "main") },
  "area-settings-open": { group: "highlight", label: area("settings", "open") },
  "area-settings-bar": { group: "highlight", label: area("settings", "bar") },
  "area-settings-line": { group: "highlight", label: area("settings", "line") },
  "area-settings-soft": { group: "highlight", label: area("settings", "soft") },
};
