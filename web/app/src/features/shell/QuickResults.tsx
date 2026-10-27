import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { busyKeys, resultKey } from "./quick-search-collect.ts";
import type { QuickFound } from "./use-quick-results.ts";

/** The words of React Aria Components for the slots of an item, not text for people. */
const descriptionSlot = "description";
/** The way a list that appears gives its first result the virtual focus. */
const focusFirst = "first";

export interface QuickResultsProps {
  readonly found: QuickFound;
  /** Changes when the list is to be made new, so that its first result gets the focus. */
  readonly listKey: number;
  /** Opens a result by its name. */
  readonly onOpen: (key: string | number) => void;
}

/** What quick search found, grouped by where it was found. */
export function QuickResults({ found, listKey, onOpen }: QuickResultsProps) {
  const { groups, waiting } = found;
  return (
    // The list is busy while the server has yet to answer for the words now typed. React Aria
    // drops `aria-busy` from the section of a group, so it is set on the element around the whole
    // list, the smallest one that renders it; the rows of busy groups are disabled besides.
    <div aria-busy={waiting}>
      <ListBox
        key={listKey}
        // oxlint-disable-next-line jsx-a11y/no-autofocus -- UX §8 (keyboard and screen reader) and the WP-09 acceptance line (the results are a list box, the arrow keys move, Enter opens): the rule guards against taking the focus of the page when it loads; this gives only the virtual focus inside the list box of a dialog that is open already, the focus of the page stays in the box, and it is the first result that Enter opens
        autoFocus={focusFirst}
        disabledKeys={busyKeys(groups)}
        aria-label={t("search.results")}
        {...partOf(part.quickSearchResults)}
        renderEmptyState={() =>
          // Not "nothing found" while the server has yet to answer.
          waiting ? null : <p {...partOf(part.quickSearchEmpty)}>{t("search.nothing")}</p>
        }
        onAction={onOpen}
      >
        {groups.map((group) => (
          <ListBoxSection key={group.id} {...partOf(part.column)}>
            <Header {...partOf(part.quickSearchHeading)}>{group.heading}</Header>
            {group.results.map((result) => (
              <ListBoxItem
                key={resultKey(group, result)}
                id={resultKey(group, result)}
                textValue={result.title}
                {...partOf(part.quickSearchItem)}
              >
                <span {...partOf(part.quickSearchTitle)}>{result.title}</span>
                {result.hint === undefined ? null : (
                  <Text slot={descriptionSlot} {...partOf(part.hint)}>
                    {result.hint}
                  </Text>
                )}
              </ListBoxItem>
            ))}
          </ListBoxSection>
        ))}
      </ListBox>
    </div>
  );
}
