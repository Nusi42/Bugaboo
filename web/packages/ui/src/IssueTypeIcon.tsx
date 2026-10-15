import type { issueIconWords } from "@bugaboo/api-client";
import {
  Bookmark,
  BookOpen,
  Bug,
  CircleHelp,
  Flag,
  Flame,
  Hammer,
  Lightbulb,
  Rocket,
  Shield,
  Sparkles,
  SquareCheck,
  Star,
  Target,
  Wrench,
  Zap,
  type LucideIcon,
} from "lucide-react";

import { partOf } from "./parts.ts";
import { iconSize, type IconSize } from "./props.ts";
import { part } from "./tokens.ts";

/** The icon an issue type can have: a word of the server's fixed list. */
export type IssueTypeIconName = (typeof issueIconWords)[number];

/** The drawing of each icon; the compiler makes sure no word of the server's list is left out. */
const glyphs: Readonly<Record<IssueTypeIconName, LucideIcon>> = {
  bug: Bug,
  "square-check": SquareCheck,
  sparkles: Sparkles,
  flag: Flag,
  target: Target,
  lightbulb: Lightbulb,
  rocket: Rocket,
  wrench: Wrench,
  "book-open": BookOpen,
  shield: Shield,
  zap: Zap,
  "circle-help": CircleHelp,
  star: Star,
  bookmark: Bookmark,
  hammer: Hammer,
  flame: Flame,
};

export interface IssueTypeIconProps {
  readonly name: IssueTypeIconName;
  /** `small` is 16 px (in lists), `normal` 20 px. */
  readonly size?: IconSize;
  /**
   * The name of the type, where the icon stands alone and nothing next to it names the type (a
   * row of a list): screen readers then read it. Without it the icon only decorates.
   */
  readonly label?: string;
}

/**
 * The icon of an issue type. Where the name of the type is next to it (principle 2) it only
 * decorates and screen readers skip it; where it stands alone it carries the name as `label`.
 */
export function IssueTypeIcon({ name, size = iconSize.small, label }: IssueTypeIconProps) {
  const Glyph = glyphs[name];
  return (
    <Glyph
      {...partOf(part.icon, { variants: [size === iconSize.small && "small"] })}
      {...(label === undefined ? {} : { role: "img", "aria-label": label })}
    />
  );
}
