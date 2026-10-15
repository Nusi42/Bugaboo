import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, SearchField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";
import {
  Collection,
  ListBox,
  ListBoxItem,
  ListBoxLoadMoreItem,
  ListLayout,
  Virtualizer,
} from "react-aria-components";

import { heldText } from "../../shared/access-words.ts";
import { detailsText, listText } from "../../shared/list-text.ts";
import { messageOf } from "../../shared/errors.ts";
import { personAddress } from "../../shared/paths.ts";
import { type PersonLine, linesOf, usePeople } from "./people.ts";

/**
 * What a row is expected to measure, before it is drawn: the list measures the real rows as they
 * appear, so a row of another height only makes the first guess of the scroll bar a little off.
 */
const ROW_GUESS = 72;

/** The address, groups and roles of a person in one line, as far as the viewer sees them. */
function factsOf(person: PersonLine): string {
  const parts: string[] = [person.email];
  if (person.groups !== null) {
    parts.push(
      person.groups.length === 0
        ? t("people.no-groups")
        : t("people.in-groups", { groups: listText(person.groups.map((group) => group.name)) }),
    );
  }
  if (person.roles.length > 0) {
    parts.push(listText(person.roles.map(heldText)));
  }
  return detailsText(parts);
}

interface RowsProps {
  readonly lines: readonly PersonLine[];
  readonly onLoadMore: () => void;
}

/** The rows, drawn only where they are on screen. A row opens the page of the person. */
function Rows({ lines, onLoadMore }: RowsProps) {
  return (
    <Virtualizer layout={ListLayout} layoutOptions={{ estimatedRowSize: ROW_GUESS }}>
      <ListBox aria-label={t("people.list")} {...partOf(part.people)}>
        <Collection items={lines}>
          {(person) => (
            <ListBoxItem
              id={person.id}
              href={personAddress(person.id)}
              textValue={person.name}
              {...partOf(part.peoplePerson)}
            >
              <span {...partOf(part.peoplePersonName)}>{person.name}</span>
              <span {...partOf(part.peoplePersonFacts)}>{factsOf(person)}</span>
            </ListBoxItem>
          )}
        </Collection>
        <ListBoxLoadMoreItem onLoadMore={onLoadMore} />
      </ListBox>
    </Virtualizer>
  );
}

/**
 * The people (SEC-02): a searchable list that draws only the rows that are on screen, a page at a
 * time. Whoever sees this holds *Manage people*, so names and addresses are shown; groups and
 * roles only as far as the viewer may see them.
 */
export function PeopleList() {
  const [query, setQuery] = useState("");
  const people = usePeople(query.trim());
  const lines = linesOf(people.data?.pages);
  const more = (): void => {
    if (people.hasNextPage && !people.isFetchingNextPage) {
      void people.fetchNextPage();
    }
  };
  return (
    <div {...partOf(part.stack)}>
      <SearchField
        label={t("people.search")}
        clearLabel={t("search.clear")}
        value={query}
        onChange={setQuery}
      />
      {people.isError ? <Alert tone={alertTone.danger}>{messageOf(people.error)}</Alert> : null}
      {people.isSuccess && lines.length === 0 ? (
        <p {...partOf(part.muted)}>{t("people.nothing")}</p>
      ) : (
        <Rows lines={lines} onLoadMore={more} />
      )}
      {people.hasNextPage ? (
        <div {...partOf(part.row)}>
          <Button isPending={people.isFetchingNextPage} onPress={more}>
            {t("people.more")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
