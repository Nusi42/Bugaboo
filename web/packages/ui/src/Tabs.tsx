import type { ReactNode } from "react";
import { Tab, TabList, TabPanel, Tabs as AriaTabs } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** One tab: what it is called, and the word that names it. */
export interface TabItem<Id extends string> {
  readonly id: Id;
  readonly label: string;
}

export interface TabsProps<Id extends string> {
  /** The name of the list of tabs for screen readers. */
  readonly label: string;
  readonly tabs: readonly TabItem<Id>[];
  /** The tab that is open. */
  readonly selected: Id;
  readonly onSelect: (id: Id) => void;
  /** What a tab shows when it is open. */
  readonly children: (id: Id) => ReactNode;
}

/**
 * Tabs (the history, the notifications, the sections of a form): a row of tab buttons and the open
 * tab's panel. Written once, with the keyboard and the reading of a tab list from React Aria.
 */
export function Tabs<Id extends string>({
  label,
  tabs,
  selected,
  onSelect,
  children,
}: TabsProps<Id>) {
  return (
    <AriaTabs
      selectedKey={selected}
      onSelectionChange={(key) => {
        const found = tabs.find((each) => each.id === key);
        if (found !== undefined) {
          onSelect(found.id);
        }
      }}
    >
      <TabList aria-label={label} {...partOf(part.tabs)}>
        {tabs.map((each) => (
          <Tab key={each.id} id={each.id} {...partOf(part.tab)}>
            {each.label}
          </Tab>
        ))}
      </TabList>
      {tabs.map((each) => (
        <TabPanel key={each.id} id={each.id} {...partOf(part.tabPanel)}>
          {children(each.id)}
        </TabPanel>
      ))}
    </AriaTabs>
  );
}
