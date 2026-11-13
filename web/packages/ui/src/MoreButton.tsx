import type { ActionMenuTriggerProps } from "./ActionMenu.tsx";
import { doNothing } from "./do-nothing.ts";
import { IconButton } from "./IconButton.tsx";
import { iconName } from "./icons.ts";

/** The square button of *More* with its icon; it is named by the row's words. */
export function MoreButton({ label, onOpen }: ActionMenuTriggerProps) {
  return <IconButton icon={iconName.more} label={label} onPress={onOpen ?? doNothing} />;
}
