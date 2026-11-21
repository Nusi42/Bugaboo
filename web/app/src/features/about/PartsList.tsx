import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part, ShowMoreOf, useHoldWhileReading } from "@bugaboo/ui";
import { Collection, ListBox, ListBoxItem, ListLayout, Virtualizer } from "react-aria-components";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { fieldList, itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";

type Part = Schema<"PartBody">;

/** What the list shows of a part: all of it. */
const SHOWN_FIELDS = fieldList<Part>(["name", "version", "license"]);

/** The version and the license of a part; a file has no version, a part may declare no license. */
function factsOf(each: Readonly<Part>): string {
  const license = each.license === "" ? t("about.part-no-license") : each.license;
  return each.version === "" ? license : t("about.part-facts", { version: each.version, license });
}

/** The parts of the release, a page at a time. */
function useParts() {
  return usePaged({
    queryKey: queryKeys.about.parts(),
    read: (after) =>
      api.client.GET("/api/v1/about/parts", {
        params: { query: { fields: SHOWN_FIELDS, limit: pageSize.usual, ...after } },
      }),
  });
}

/** A part with the identity the list gives its row: the name and the version tell two versions of a part apart. */
interface ListedPart extends Part {
  readonly id: string;
}

function rowOf(each: Readonly<Part>, at: number): ListedPart {
  return { ...each, id: `${at}:${each.name}:${each.version}` };
}

/**
 * The parts of this release by name, version and license, in a list that draws only the rows that are in the window.
 * The next page is read with *Show more parts* only: a list that read it by itself as its end came into view pushed
 * the button away from the keyboard focus, page after page (WCAG 2.4.11).
 */
export function PartsList() {
  const parts = useParts();
  useHoldWhileReading(parts.isLoading);
  const rows = itemsOf(parts.data?.pages).map((each, at) => rowOf(each, at));
  return (
    <>
      {parts.isError ? <Alert tone={alertTone.danger}>{messageOf(parts.error)}</Alert> : null}
      <Virtualizer layout={ListLayout}>
        <ListBox
          aria-label={t("about.parts")}
          renderEmptyState={() =>
            parts.isError ? null : <p {...partOf(part.pickListFacts)}>{t("about.parts-none")}</p>
          }
          {...partOf(part.pickList)}
        >
          <Collection items={rows}>
            {(each) => (
              <ListBoxItem id={each.id} textValue={each.name} {...partOf(part.pickListRow)}>
                <span {...partOf(part.pickListName)}>{each.name}</span>
                <span {...partOf(part.pickListFacts)}>{factsOf(each)}</span>
              </ListBoxItem>
            )}
          </Collection>
        </ListBox>
      </Virtualizer>
      <ShowMoreOf label={t("about.parts-more")} list={parts} />
    </>
  );
}
