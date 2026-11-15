import { Card, PaneWithHead, paneLook, useLayout, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { keptScroll, scrollBox } from "../../shared/scroll-boxes.ts";
import { usePageTitle } from "../../shared/use-page.ts";

/** The screen of the menu: only it names the tab; a topic names its own. */
function MenuScreen({
  heading,
  label,
  menu,
}: {
  readonly heading: string;
  readonly label: string;
  readonly menu: ReactNode;
}) {
  usePageTitle(heading);
  return (
    <Card heading={heading} level={1}>
      <nav aria-label={label}>{menu}</nav>
    </Card>
  );
}

export interface SettingsScreensProps {
  /** What the settings are called: the heading of the menu screen and the name of the menu. */
  readonly heading: string;
  /** What names the menu for screen readers, where it is another than the heading. */
  readonly label?: string;
  /** The menu of the topics; none where there is only one topic, and no menu is drawn. */
  readonly menu?: ReactNode;
  /** This is the screen of the menu itself (a phone shows the menu and a topic one at a time). */
  readonly onMenu: boolean;
  /** The open topic. */
  readonly children: ReactNode;
}

/**
 * The one rule of every settings screen: the menu is a screen of its own (`onMenu`), or a topic is
 * shown, with the menu at its side on a computer and with *Back* on a phone, where one screen shows
 * at a time.
 */
export function SettingsScreens({ heading, label, menu, onMenu, children }: SettingsScreensProps) {
  const layout = useLayout();
  if (onMenu) {
    return <MenuScreen heading={heading} label={label ?? heading} menu={menu} />;
  }
  // The heading and the introduction of the topic stand above the rest of it, and on a computer fold over it as it
  // scrolls (UX §6.5). A tablet and a phone keep the same tree, so nothing typed is lost when the layout changes; a
  // phone keeps them in the card.
  const topic = (
    <PaneWithHead look={layout === "computer" ? paneLook.folds : paneLook.still} head={null}>
      <div {...partOf(part.settingsTopic)} {...keptScroll(scrollBox.topic)}>
        {children}
      </div>
    </PaneWithHead>
  );
  if (layout === "phone" || menu === undefined) {
    return topic;
  }
  return (
    <div {...partOf(part.settingsFrame)}>
      <nav aria-label={label ?? heading} {...partOf(part.settingsFrameMenu)}>
        {menu}
      </nav>
      <div {...partOf(part.shellContent)}>{topic}</div>
    </div>
  );
}
