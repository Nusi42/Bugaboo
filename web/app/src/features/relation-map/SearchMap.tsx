import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  DelayedSkeleton,
  part,
  partOf,
  selectLook,
  useLayout,
} from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { messageOf } from "../../shared/errors.ts";
import { issueMapAddress } from "../../shared/paths.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { ViewChoices, type ListViewControl, type SearchFieldsAnswer } from "../search/index.ts";
import { useSearchMap } from "./map-data.ts";
import { cutWords } from "./map-view.ts";
import { RelationMap } from "./RelationMap.tsx";

export interface SearchMapProps extends ListViewControl {
  /** The canonical text of the search. */
  readonly text: string;
  /** The project it looks in, and the projects inside it; none for every project. */
  readonly within: string | undefined;
  /** The fields the person may read: *Show as* names them. */
  readonly fields: SearchFieldsAnswer;
}

/**
 * *Show as › Map* (UX §4.15): the results of a search (at most the issues a map holds) and the
 * relations between them, in the same kind of layout as the map of an issue. *Start from here*
 * opens the map of that issue.
 */
export function SearchMap({ text, within, view, onView, fields }: SearchMapProps) {
  const phone = useLayout() === "phone";
  const navigate = useNavigate();
  const kindsRead = useRelationKinds();
  const kinds = relationKindLinesOf(kindsRead.data?.pages).map((kind) => kind.id);
  const shown = view.map;
  const map = useSearchMap({
    text,
    within,
    shown,
    kinds: kindsRead.data === undefined ? undefined : kinds,
  });
  // *Show as* stands first whatever the map is doing, so it never goes from under the cursor and a
  // map that could not be read can go back to the list. On a phone it is in the sheet of the filters.
  const start = phone ? null : (
    <ViewChoices view={view} fields={fields} onView={onView} look={selectLook.inline} />
  );
  if (map.data === undefined) {
    return (
      <>
        <div {...partOf(part.mapTools)}>{start}</div>
        {map.isError ? (
          <Alert tone={alertTone.danger}>{messageOf(map.error)}</Alert>
        ) : (
          <DelayedSkeleton lines={4} />
        )}
      </>
    );
  }
  return (
    <RelationMap
      answer={map.data}
      kindPages={kindsRead.data?.pages}
      label={(count) => t("relation-map.search-picture-label", { count })}
      cut={cutWords.search()}
      nothing={t("relation-map.nothing-found")}
      start={start}
      shown={shown}
      onShown={(next) => {
        onView({ ...view, map: next });
      }}
      onStartFrom={(key) => {
        void navigate({ href: issueMapAddress(key) });
      }}
    />
  );
}
