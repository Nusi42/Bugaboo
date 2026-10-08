import type { MessageKey } from "@bugaboo/messages";
import type { IconName } from "@bugaboo/ui/icons";

import type { Path } from "./paths.ts";
import type { PermissionKey } from "./permissions.ts";

/**
 * One screen of the signed-in app, as the menus, the top bar and quick search know it. The list
 * below is the one place that says which screens exist, where they sit and what they are called
 * (SSOT): a screen that is not in it has no menu entry, and one that is has it everywhere.
 */
export interface Place {
  readonly path: Path;
  readonly name: MessageKey;
  readonly icon: IconName;
  /** The screen this one belongs under: *Back* on a phone goes there. `null` for the start page. */
  readonly parent: Path | null;
  /** Only people who hold one of these permissions for the whole organization see it. */
  readonly anyOf?: readonly PermissionKey[];
  /** For a topic of a settings frame: the heading it is listed under in the left menu. */
  readonly group?: MessageKey;
  /** Whether menus and quick search list it. A screen reached only from another one is not. */
  readonly listed: boolean;
}
