import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";
import { useId, useState } from "react";
import { Tab, TabList, TabPanel, Tabs } from "react-aria-components";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { ChangeEntry, NothingYet } from "./ChangeEntry.tsx";
import { CommentComposer } from "./CommentComposer.tsx";
import { CommentEntry } from "./CommentEntry.tsx";
import { groupByEvent, historyTabs, useHistory, type HistoryTabName } from "./history.ts";

const tabLabel = {
  everything: "history.tab-everything",
  comments: "history.tab-comments",
  changes: "history.tab-changes",
} as const;

/** The entries of one tab, newest first, with *Show earlier* under them. */
function Entries({ issue, tab }: { readonly issue: string; readonly tab: HistoryTabName }) {
  const list = useHistory(issue, tab);
  const groups = groupByEvent(itemsOf(list.data?.pages));
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {groups.length === 0 && list.isSuccess && !list.hasNextPage ? <NothingYet /> : null}
      {groups.map((group) =>
        group[0]?.kind === "comment" ? (
          <CommentEntry key={group[0].event} entry={group[0]} />
        ) : (
          <ChangeEntry key={group[0]?.event} items={group} />
        ),
      )}
      {list.hasNextPage ? (
        <Button
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("history.more")}
        </Button>
      ) : null}
    </div>
  );
}

/**
 * *History* (ISS-06): the comments and the changes of the issue in time order, in three tabs, with
 * the box to write a comment. A change to a field the reader may not see is not in it.
 */
export function HistoryBlock({ readableId }: { readonly readableId: string }) {
  const heading = useId();
  const [tab, setTab] = useState<HistoryTabName>("everything");
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("history.title")}
      </h2>
      <CommentComposer issue={readableId} />
      <Tabs
        selectedKey={tab}
        onSelectionChange={(key) => {
          setTab(historyTabs.find((each) => each === key) ?? "everything");
        }}
      >
        <TabList aria-label={t("history.tabs")} {...partOf(part.historyTabs)}>
          {historyTabs.map((each) => (
            <Tab key={each} id={each} {...partOf(part.historyTab)}>
              {t(tabLabel[each])}
            </Tab>
          ))}
        </TabList>
        {historyTabs.map((each) => (
          <TabPanel key={each} id={each}>
            <Entries issue={readableId} tab={each} />
          </TabPanel>
        ))}
      </Tabs>
    </section>
  );
}
