import { CircleCheck, CircleDashed, Contrast, type LucideIcon } from "lucide-react";

import { partOf } from "./parts.ts";
import { iconSize, type IconSize } from "./props.ts";
import type { StatusGroup } from "./status.ts";
import { part } from "./tokens.ts";

/** The drawing of each group: a dashed ring, a half-filled ring, a check. The compiler checks all three. */
const glyphs: Readonly<Record<StatusGroup, LucideIcon>> = {
  "not-started": CircleDashed,
  "in-progress": Contrast,
  done: CircleCheck,
};

export interface StatusIconProps {
  readonly group: StatusGroup;
  /** `small` is 16 px (in lists), `normal` 20 px. */
  readonly size?: IconSize;
  /**
   * The word of the status, where the icon stands alone and nothing next to it names the status
   * (a row of a list): screen readers then read it. Without it the icon only decorates (R5).
   */
  readonly label?: string;
}

/** The icon of a status group, in the group's color. */
export function StatusIcon({ group, size = iconSize.normal, label }: StatusIconProps) {
  const Glyph = glyphs[group];
  return (
    <Glyph
      {...partOf(part.statusIcon, {
        variants: [group, size === iconSize.small && "small"],
      })}
      {...(label === undefined ? {} : { role: "img", "aria-label": label })}
    />
  );
}
