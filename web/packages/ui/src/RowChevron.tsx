import { Icon } from "./Icon.tsx";
import { iconName } from "./icons.ts";
import { iconSize } from "./props.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** The sign that a row opens something: the one chevron, at the end of the row (M5 exit, O16). */
export function RowChevron() {
  return (
    <span {...partOf(part.rowChevron)}>
      <Icon name={iconName.next} size={iconSize.small} />
    </span>
  );
}
