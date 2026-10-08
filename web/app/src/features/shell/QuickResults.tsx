import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useRouter } from "@tanstack/react-router";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { resultKey, type Collected } from "./quick-search-collect.ts";

/** The words of React Aria Components for the slots of an item, not text for people. */
const descriptionSlot = "description";

export interface QuickResultsProps {
  readonly found: Collected;
}

/** What quick search found, grouped by where it was found. */
export function QuickResults({ found }: QuickResultsProps) {
  const { groups } = found;
  const router = useRouter();
  // Going to a result closes the dialog, also when the result is the page that is open already.
  const open = (id: string | number): void => {
    for (const group of groups) {
      const hit = group.results.find((result) => resultKey(group, result) === id);
      if (hit !== undefined) {
        router.history.push(hit.path);
      }
    }
    closeOverlay();
  };
  return (
    <ListBox
      aria-label={t("search.results")}
      {...partOf(part.quickSearchResults)}
      renderEmptyState={() => <p {...partOf(part.quickSearchEmpty)}>{t("search.nothing")}</p>}
      onAction={open}
    >
      {groups.map((group) => (
        <ListBoxSection key={group.id} {...partOf(part.quickSearchGroup)}>
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
                <Text slot={descriptionSlot} {...partOf(part.quickSearchHint)}>
                  {result.hint}
                </Text>
              )}
            </ListBoxItem>
          ))}
        </ListBoxSection>
      ))}
    </ListBox>
  );
}
