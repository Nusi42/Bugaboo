import { glyphs, type IconName } from "./icons.ts";
import { partOf } from "./parts.ts";
import { iconSize, type IconSize } from "./props.ts";
import { part } from "./tokens.ts";

export interface IconProps {
  readonly name: IconName;
  /** `small` is 16 px (in lists), `normal` 20 px (in navigation). */
  readonly size?: IconSize;
}

/**
 * An icon. It only decorates: the text next to it says what it means (principle 2), so screen
 * readers skip it. A button that has only an icon is an `IconButton`, which has a name.
 */
export function Icon({ name, size = iconSize.normal }: IconProps) {
  const Glyph = glyphs[name];
  return <Glyph {...partOf(part.icon, { variants: [size === iconSize.small && "small"] })} />;
}
