import { type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { FieldLook } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, useAllPages, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys, refreshLabelReads } from "../../shared/query-keys.ts";
import { versionCounted } from "../issues/index.ts";

/** A group of labels, as the list of the organization shows it. */
export type LabelGroup = Schema<"LabelGroupBody">;
/** A label of the organization's list. */
export type Label = Schema<"LabelBody">;
/** A tag or a label on an issue, as the chip needs it. */
export type Chip = Pick<Schema<"TagRefBody">, "id" | "name" | "color">;

/**
 * The labels of the organization with their groups: anybody who is signed in may read them. With a
 * project (a short name or an ID), each group says whether the project offers it.
 */
export function useLabels(project?: string) {
  const projectQuery = project === undefined ? {} : { project };
  const groupsRead = usePaged({
    queryKey: queryKeys.labels.groups(project),
    read: (after) =>
      api.client.GET("/api/v1/label-groups", {
        params: {
          query: {
            ...projectQuery,
            fields: fieldList<Schema<"LabelGroupItem">>([
              "id",
              "name",
              "pick_one",
              "retired",
              "created_in",
              "may_manage",
              "version",
              "offered_here",
              "own_offer",
              "may_change_offer",
            ]),
            limit: pageSize.largest,
            ...after,
          },
        },
      }),
  });
  const labelsRead = usePaged({
    queryKey: queryKeys.labels.list(project),
    read: (after) =>
      api.client.GET("/api/v1/labels", {
        params: { query: { ...projectQuery, limit: pageSize.largest, ...after } },
      }),
  });
  // The groups and the labels are choices in forms: every page is read, not the first.
  useAllPages(groupsRead);
  useAllPages(labelsRead);
  return {
    groups: linesFrom(groupsRead.data?.pages, groupOf),
    labels: linesFrom(labelsRead.data?.pages, labelOf),
    isError: groupsRead.isError || labelsRead.isError,
    error: groupsRead.error ?? labelsRead.error,
  };
}

/** The group of an item of the list, or nothing for an item that came without what it needs. */
export function groupOf(item: Schema<"LabelGroupItem">): LabelGroup | undefined {
  const { id, name, pick_one: pickOne, retired, created_in: createdIn, may_manage, version } = item;
  return id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    pickOne === undefined ||
    pickOne === null ||
    retired === undefined ||
    retired === null ||
    createdIn === undefined ||
    createdIn === null ||
    may_manage === undefined ||
    may_manage === null ||
    version === undefined ||
    version === null
    ? undefined
    : {
        id,
        name,
        pick_one: pickOne,
        retired,
        created_in: createdIn,
        may_manage,
        version,
        ...(item.offered_here === undefined || item.offered_here === null
          ? {}
          : { offered_here: item.offered_here }),
        ...(item.own_offer === undefined || item.own_offer === null
          ? {}
          : { own_offer: item.own_offer }),
        ...(item.may_change_offer === undefined || item.may_change_offer === null
          ? {}
          : { may_change_offer: item.may_change_offer }),
      };
}

/** The label of an item of the list, or nothing for an item that came without what it needs. */
export function labelOf(item: Schema<"LabelItem">): Label | undefined {
  const { id, name, color, retired, version } = item;
  return id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    color === undefined ||
    color === null ||
    retired === undefined ||
    retired === null ||
    version === undefined ||
    version === null
    ? undefined
    : {
        id,
        ...(item.group === undefined || item.group === null ? {} : { group: item.group }),
        name,
        color,
        retired,
        version,
      };
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
export function useIssueLabelsAndTags(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/labels-and-tags", {
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
  refreshLabelReads();
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
export function afterLabelsAndTags(issue: string, { version }: Counted): void {
  versionCounted(issue, version);
  refreshLabels();
}

/** The choice that stands for a label in no group. */
export const NO_GROUP = "";

/** The groups the person looks after and may put a label in: those not retired. */
export function managedGroupChoices(
  groups: readonly LabelGroup[],
): readonly { readonly value: string; readonly label: string }[] {
  return groups
    .filter((group) => !group.retired && group.may_manage)
    .map((group) => ({ value: group.id, label: group.name }));
}

/** The groups to choose from in a label's form: *No group*, then the groups that are not retired. */
export function groupChoices(
  groups: readonly LabelGroup[],
): readonly { readonly value: string; readonly label: string }[] {
  return [
    { value: NO_GROUP, label: t("labels.no-label-group") },
    ...groups
      .filter((group) => !group.retired)
      .map((group) => ({ value: group.id, label: group.name })),
  ];
}

/**
 * The labels to choose from for an issue of a project: not retired, not on the issue, and not in a
 * group the project switched off. The labels the issue has stay on it either way.
 */
export function useLabelsToAdd(
  project: string | undefined,
  has: readonly Chip[],
): readonly Label[] {
  const labels = useLabels(project);
  const closed = new Set(
    labels.groups.filter((group) => group.offered_here === false).map((group) => group.id),
  );
  const held = new Set(has.map((each) => each.id));
  return labels.labels.filter(
    (label) =>
      !label.retired &&
      !held.has(label.id) &&
      (label.group === undefined || label.group === null || !closed.has(label.group)),
  );
}

/** The look of a tag or label on an issue: a small colored label. */
export const chipLook = "badge" satisfies FieldLook;
