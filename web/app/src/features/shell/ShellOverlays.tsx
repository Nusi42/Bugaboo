import { t } from "@bugaboo/messages";
import {
  IconButton,
  iconName,
  type LayoutMode,
  Overlay,
  overlayEdge,
  partOf,
  part,
} from "@bugaboo/ui";

import { Fragment } from "react";

import { MoreSheet } from "./MoreSheet.tsx";
import { closeOverlay, useOverlay } from "./overlay-state.ts";
import { useOverlayHosts } from "./overlay-hosts.ts";
import { QuickSearch } from "./QuickSearch.tsx";
import { ShortcutHelp } from "./ShortcutHelp.tsx";
import { Sidebar, type SidebarProps } from "./Sidebar.tsx";

export interface ShellOverlaysProps extends Pick<
  SidebarProps,
  "organizationSettings" | "name" | "onSignOut" | "signingOut"
> {
  readonly layout: LayoutMode;
}

/** The menu a tablet slides in: the sidebar, in a layer over the page. */
function Drawer(props: Omit<ShellOverlaysProps, "layout">) {
  const overlay = useOverlay("menu");
  return (
    <Overlay {...overlay} label={t("shell.menu")} from={overlayEdge.start}>
      <div {...partOf(part.shellOverlays)}>
        <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        <Sidebar narrow={false} inDrawer onNavigate={closeOverlay} {...props} />
      </div>
    </Overlay>
  );
}

/**
 * Everything that opens over the page: the menu of a tablet, the *More* sheet of a phone, quick
 * search and the list of shortcuts.
 */
export function ShellOverlays({ layout, ...person }: ShellOverlaysProps) {
  const tablet = layout === "tablet";
  const phone = layout === "phone";
  const hosts = useOverlayHosts();
  return (
    <>
      {tablet ? <Drawer {...person} /> : null}
      {phone ? (
        <MoreSheet
          organizationSettings={person.organizationSettings}
          name={person.name}
          onSignOut={person.onSignOut}
          signingOut={person.signingOut}
        />
      ) : null}
      {hosts.map((host) => (
        <Fragment key={host.id}>{host.render()}</Fragment>
      ))}
      <QuickSearch />
      <ShortcutHelp />
    </>
  );
}
