import { cx } from "./cx.ts";
import styles from "./Icon.module.css";
import { glyphs, type IconName } from "./icons.ts";

export interface IconProps {
  readonly name: IconName;
  /** 16 px (in lists) instead of 20 px (in navigation). */
  readonly small?: boolean;
}

/**
 * An icon. It only decorates: the text next to it says what it means (principle 2), so screen
 * readers skip it. A button that has only an icon is an `IconButton`, which has a name.
 */
export function Icon({ name, small = false }: IconProps) {
  const Glyph = glyphs[name];
  return <Glyph className={cx(styles.icon, small && styles.small)} />;
}
