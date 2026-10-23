import { type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, useAllPages, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A group of labels, as the list of the organization shows it. */
export type LabelGroup = Schema<"LabelGroupBody">;
/** A label of the organization's list. */
export type Label = Schema<"LabelBody">;

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
        ...(item.label_group === undefined || item.label_group === null
          ? {}
          : { group: item.label_group }),
        name,
        color,
        retired,
        version,
      };
}
