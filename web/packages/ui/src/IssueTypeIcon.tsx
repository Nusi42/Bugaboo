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
}

/**
 * The icon of an issue type. It only decorates: the name of the type is always next to it
 * (principle 2), so screen readers skip it.
 */
export function IssueTypeIcon({ name, size = iconSize.small }: IssueTypeIconProps) {
  const Glyph = glyphs[name];
  return <Glyph {...partOf(part.icon, { variants: [size === iconSize.small && "small"] })} />;
}
