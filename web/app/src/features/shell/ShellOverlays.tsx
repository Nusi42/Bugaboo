import { t } from "@bugaboo/messages";
import { IconButton, iconName, Overlay, type LayoutMode } from "@bugaboo/ui";

import { MoreSheet } from "./MoreSheet.tsx";
import { closeOverlay, useOverlay } from "./overlay-state.ts";
import { QuickSearch } from "./QuickSearch.tsx";
import { ShortcutHelp } from "./ShortcutHelp.tsx";
import styles from "./ShellOverlays.module.css";
import { Sidebar, type SidebarProps } from "./Sidebar.tsx";
import { UndoHost } from "./UndoHost.tsx";

export interface ShellOverlaysProps extends Pick<
  SidebarProps,
  "administrator" | "name" | "onSignOut" | "signingOut"
> {
  readonly layout: LayoutMode;
}

/** The menu a tablet slides in: the sidebar, in a layer over the page. */
function Drawer(props: Omit<ShellOverlaysProps, "layout">) {
  const overlay = useOverlay("menu");
  return (
    <Overlay {...overlay} label={t("shell.menu")} start>
      <div className={styles.drawer}>
        <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        <Sidebar narrow={false} inDrawer onNavigate={closeOverlay} {...props} />
      </div>
    </Overlay>
  );
}

/**
 * Everything that opens over the page: the menu of a tablet, the *More* sheet of a phone, quick
 * search, the list of shortcuts, and the offers to undo what was just done.
 */
export function ShellOverlays({ layout, ...person }: ShellOverlaysProps) {
  const tablet = layout === "tablet";
  const phone = layout === "phone";
  return (
    <>
      {tablet ? <Drawer {...person} /> : null}
      {phone ? (
        <MoreSheet
          administrator={person.administrator}
          name={person.name}
          onSignOut={person.onSignOut}
          signingOut={person.signingOut}
        />
      ) : null}
      <QuickSearch />
      <ShortcutHelp />
      <UndoHost aboveBar={phone} />
    </>
  );
}
