import { t } from "@bugaboo/messages";
import { PickerChoice, SearchPicker } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { timeKeys } from "../../shared/time-keys.ts";

export interface PeoplePickerProps {
  /** A person was chosen: their ID. */
  readonly onPick: (person: string) => void;
}

/** The people the reader may open, a page at a time, narrowed by the words typed. */
function usePeople(search: string) {
  const list = usePaged({
    queryKey: timeKeys.people(search),
    read: (after) =>
      api.client.GET("/api/v1/work-days/people", {
        params: {
          query: { limit: pageSize.short, ...(search === "" ? {} : { search }), ...after },
        },
      }),
  });
  return { list, people: itemsOf(list.data?.pages) };
}

/**
 * Choosing one person by name for *Working hours of others*: the search box and the people the
 * reader may open, a page at a time. It offers names and nothing else: no hours, no counts, and no
 * order by any number (ADR-0035, decision 8).
 */
export function PeoplePicker({ onPick }: PeoplePickerProps) {
  const [words, setWords] = useState("");
  const search = words.trim();
  const { list, people } = usePeople(search);
  return (
    <SearchPicker
      legend={t("time.others.pick-legend")}
      searchLabel={t("time.others.pick-search")}
      clearLabel={t("time.others.pick-clear")}
      listLabel={t("time.others.pick-list")}
      query={words}
      onQuery={setWords}
      failed={list.error === null ? undefined : messageOf(list.error)}
      nothing={
        list.data !== undefined && people.length === 0 ? t("time.others.pick-none") : undefined
      }
      more={
        list.hasNextPage
          ? {
              label: t("time.others.pick-more"),
              pending: list.isFetchingNextPage,
              press: () => void list.fetchNextPage(),
            }
          : undefined
      }
    >
      {people.flatMap((each) =>
        each.id === undefined || each.id === null || each.name === undefined || each.name === null
          ? []
          : [
              <PickerChoice
                key={each.id}
                label={each.name}
                accessibleName={t("time.others.pick-take", { person: each.name })}
                onPick={() => {
                  onPick(each.id ?? "");
                }}
              />,
            ],
      )}
    </SearchPicker>
  );
}
