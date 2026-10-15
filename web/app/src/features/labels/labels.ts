import { type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { FieldLook } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { versionCounted } from "../issues/index.ts";

/** A group of labels, as the list of the organization shows it. */
export type LabelGroup = Schema<"LabelGroupBody">;
/** A label of the organization's list. */
export type Label = Schema<"LabelBody">;
/** A tag or a label on an issue, as the chip needs it. */
export type Chip = Pick<Schema<"TagRefBody">, "id" | "name" | "color">;

/** The labels of the organization with their groups: anybody who is signed in may read them. */
export function useLabels() {
  return api.query.useQuery("get", "/api/v1/labels");
}

/** A tag that has what a line of the list and its editor need. */
export interface TagLine {
  readonly id: string;
  readonly name: string;
  readonly color: Schema<"TagRefBody">["color"];
  readonly wholeOrganization: boolean;
  readonly projects: readonly Schema<"ProjectRef">[];
  /** The version the editor looked at; a change is made on it. */
  readonly version: number;
}

/** The line of a tag, or nothing for an item that came without what it needs. */
export function tagLineOf(item: Schema<"TagItem">): TagLine | undefined {
  const { id, name, color, whole_organization: whole, projects, version } = item;
  return id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    color === undefined ||
    color === null ||
    whole === undefined ||
    whole === null ||
    version === undefined ||
    version === null
    ? undefined
    : { id, name, color, wholeOrganization: whole, projects: projects ?? [], version };
}

/** The tags of the organization, by name, a page at a time (for the people who look after them). */
export function useTags() {
  return usePaged({
    queryKey: queryKeys.tags.list(),
    read: (after) =>
      api.client.GET("/api/v1/tags", {
        params: {
          query: {
            fields: fieldList<Schema<"TagItem">>([
              "id",
              "name",
              "color",
              "whole_organization",
              "projects",
              "version",
            ]),
            limit: pageSize.largest,
            ...after,
          },
        },
      }),
  });
}

/** The tags in the pages read so far. */
export function tagLinesOf(pages: readonly Schema<"Page_TagItem">[] | undefined): TagLine[] {
  return linesFrom(pages, tagLineOf);
}

/** The tags and labels of one issue, and what the person may do with them. */
export function useIssueSorting(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/sorting", {
    params: { path: { issue } },
  });
}

/** The tags to offer in a project, for an issue of this type, while a person types. */
export function useTagSuggestions(project: string, issueType: string, text: string) {
  return api.query.useQuery(
    "get",
    "/api/v1/projects/{project}/tag-suggestions",
    { params: { path: { project }, query: { text, issue_type: issueType } } },
    { enabled: text.trim() !== "" },
  );
}

/**
 * A label or tag was made, changed, combined or put on an issue: what shows them is read again,
 * and so are the issues, because putting one on or taking one off counts the version of an issue.
 */
export function refreshLabels(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.tags.all });
  void queryClient.invalidateQueries({ queryKey: queryKeys.issues.all });
  for (const route of [
    "/api/v1/labels",
    "/api/v1/issues/{issue}/sorting",
    "/api/v1/projects/{project}/tag-suggestions",
    "/api/v1/issues/{issue}/history",
  ]) {
    void queryClient.invalidateQueries({ queryKey: ["get", route] });
  }
  void queryClient.invalidateQueries({ queryKey: queryKeys.history.all });
}

/** The answer of putting a tag or a label on an issue or taking one off: the version it counted to. */
export interface Counted {
  readonly version: number;
}

/**
 * A tag or label was put on an issue or taken off: the page writes the version the issue counted
 * up to into what it knows, so a change made right after does not conflict, and shows the rest again.
 */
export function afterSorting(issue: string, { version }: Counted): void {
  versionCounted(issue, version);
  refreshLabels();
}

/** The choice that stands for a label in no group. */
export const NO_GROUP = "";

/** The groups to choose from in a label's form: *No group*, then the groups that are not retired. */
export function groupChoices(
  groups: readonly LabelGroup[],
): readonly { readonly value: string; readonly label: string }[] {
  return [
    { value: NO_GROUP, label: t("labels.no-group") },
    ...groups
      .filter((group) => !group.retired)
      .map((group) => ({ value: group.id, label: group.name })),
  ];
}

/** The look of a tag or label on an issue: a small colored label. */
export const chipLook = "badge" satisfies FieldLook;
