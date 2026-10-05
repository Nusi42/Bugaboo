import { t } from "@bugaboo/messages";
import { cx } from "@bugaboo/ui";
import { useRouter } from "@tanstack/react-router";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import styles from "./QuickSearch.module.css";
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
      className={cx(styles.results)}
      renderEmptyState={() => <p className={styles.empty}>{t("search.nothing")}</p>}
      onAction={open}
    >
      {groups.map((group) => (
        <ListBoxSection key={group.id} className={cx(styles.group)}>
          <Header className={styles.heading}>{group.heading}</Header>
          {group.results.map((result) => (
            <ListBoxItem
              key={resultKey(group, result)}
              id={resultKey(group, result)}
              textValue={result.title}
              className={cx(styles.item)}
            >
              <span className={styles.title}>{result.title}</span>
              {result.hint === undefined ? null : (
                <Text slot={descriptionSlot} className={styles.hint}>
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
