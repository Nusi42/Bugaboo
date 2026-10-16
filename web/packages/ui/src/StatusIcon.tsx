import { CircleCheck, CircleDashed, createLucideIcon, type LucideIcon } from "lucide-react";

import { partOf } from "./parts.ts";
import { iconSize, type IconSize } from "./props.ts";
import type { StatusGroup } from "./status.ts";
import { part } from "./tokens.ts";

/** A ring with its right half filled: the group *In progress*. Lucide's own half circle is only outlined. */
const HalfFilledRing = createLucideIcon("circle-half-filled", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "ring" }],
  ["path", { d: "M12 18a6 6 0 0 0 0-12v12z", fill: "currentColor", key: "half" }],
]);

/** The drawing of each group: a dashed ring, a half-filled ring, a check. The compiler checks all three. */
const glyphs: Readonly<Record<StatusGroup, LucideIcon>> = {
  "not-started": CircleDashed,
  "in-progress": HalfFilledRing,
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
