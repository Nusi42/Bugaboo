import { type Schema, dataOf } from "@bugaboo/api-client";
import { keepPreviousData, skipToken, useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A project in a list, as the server sends it: the fields can be missing. */
export type ListItem = Schema<"ProjectItem">;
/** One project with where it is and what the person may do there. */
export type OneProject = Schema<"ProjectDetailBody">;
/** A project as a name in a sentence. */
export type Near = Schema<"ProjectRef">;
/** What a move would do. */
export type Preview = Schema<"MovePreviewBody">;

/** The fields of a project in a list that the screens need. */
const LIST_FIELDS = fieldList<ListItem>(["id", "short_name", "name", "has_sub_projects", "parent"]);

/** What narrows a list of projects. */
export interface ProjectFilter {
  /** The projects directly inside this one (an ID); the top of the tree without it. */
  readonly inside?: string;
  /** Words to find in names and short names, anywhere; an empty text lists them all. */
  readonly search?: string;
  /** Leave out this project (an ID) and everything inside it. */
  readonly outside?: string;
  /** Only the places this project (an ID) may be moved into by the person. */
  readonly movable?: string;
}

/** A project that has what a line of a list needs (the fields can be missing in an answer). */
export interface ProjectLine {
  readonly id: string;
  readonly shortName: string;
  readonly name: string;
  readonly hasSubProjects: boolean;
  readonly parent: Near | null;
}

/** The line of a project, or nothing for an item that came without what it needs. */
export function lineOf(item: ListItem): ProjectLine | undefined {
  const { id, short_name: shortName, name } = item;
  if (id === undefined || id === null || shortName === undefined || shortName === null) {
    return undefined;
  }
  return {
    id,
    shortName,
    name: name ?? shortName,
    hasSubProjects: item.has_sub_projects === true,
    parent: item.parent ?? null,
  };
}

/**
 * The projects the person sees that the filter names, a page at a time: *Show more* asks for the
 * next. A search for a place to choose asks for a short page.
 */
export function useProjectList(filter: ProjectFilter, size: number = pageSize.largest) {
  return usePaged({
    queryKey: queryKeys.projects.list(filter, size),
    read: (after) =>
      api.client.GET("/api/v1/projects", {
        params: { query: { ...filter, fields: LIST_FIELDS, limit: size, ...after } },
      }),
  });
}

/** The lines of the pages read so far. */
export function linesOf(pages: readonly Schema<"Page_ProjectItem">[] | undefined): ProjectLine[] {
  return linesFrom(pages, lineOf);
}

/** One project by its address (a short name, also an old one, or an ID); nothing is asked without one. */
export function useProject(address: string | undefined) {
  return useQuery({
    queryKey: queryKeys.projects.one(address ?? ""),
    // While another address loads (a project got a new short name), the page keeps showing the
    // project: nothing blinks away and what was typed stays.
    placeholderData: keepPreviousData,
    queryFn:
      address === undefined
        ? skipToken
        : async () =>
            dataOf(
              await api.client.GET("/api/v1/projects/{project}", {
                params: { path: { project: address } },
              }),
            ),
  });
}

/** Where a project would go: another project (an ID), or the top level (`to: null`). */
export interface MoveTarget {
  readonly to: string | null;
}

/** What a move of a project to `target` would do. With no target picked yet nothing is asked. */
export function useMovePreview(project: string, target: MoveTarget | null) {
  return useQuery({
    queryKey: queryKeys.projects.movePreview(project, target?.to ?? null),
    // A preview is a question about now: it is never kept for later.
    gcTime: 0,
    queryFn:
      target === null
        ? skipToken
        : async () =>
            dataOf(
              await api.client.POST("/api/v1/projects/{project}/move-preview", {
                params: { path: { project } },
                body: { to: target.to },
              }),
            ),
  });
}

/** A project was made, changed or moved: every list and every project is read again. */
export function refreshProjects(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.projects.all });
}

/** What is needed to make a project. */
export interface NewProject {
  readonly shortName: string;
  readonly name: string;
}

/** Makes a project at the top level, or inside `parent` (an ID). */
export function useCreateProject(parent: string | undefined) {
  return useMutation({
    mutationFn: async ({ shortName, name }: NewProject) => {
      const body = { short_name: shortName, name };
      return dataOf(
        parent === undefined
          ? await api.client.POST("/api/v1/projects", { body })
          : await api.client.POST("/api/v1/projects/{project}/sub-projects", {
              params: { path: { project: parent } },
              body,
            }),
      );
    },
    onSuccess: refreshProjects,
  });
}

/** What is changed about a project: what is left out stays. */
export interface ProjectChanges {
  readonly id: string;
  /** The version that was looked at. */
  readonly version: number;
  readonly name?: string;
  readonly shortName?: string;
}

/** Changes the name or the short name of a project. */
export function useChangeProject() {
  return useMutation({
    mutationFn: async ({ id, version, name, shortName }: ProjectChanges) => {
      return dataOf(
        await api.client.PATCH("/api/v1/projects/{project}", {
          params: { path: { project: id } },
          body: {
            version,
            ...(name === undefined ? {} : { name }),
            ...(shortName === undefined ? {} : { short_name: shortName }),
          },
        }),
      );
    },
    onSuccess: refreshProjects,
  });
}

/** What is needed to move a project. */
export interface ProjectMove {
  readonly id: string;
  /** Where it goes: another project (an ID), or the top level. */
  readonly to: string | null;
  /** The version of the preview that was looked at. */
  readonly preview: string;
}

/** Moves a project, after its preview. */
export function useMoveProject() {
  return useMutation({
    mutationFn: async ({ id, to, preview }: ProjectMove) => {
      return dataOf(
        await api.client.POST("/api/v1/projects/{project}/move", {
          params: { path: { project: id } },
          body: { to, preview },
        }),
      );
    },
    onSuccess: refreshProjects,
  });
}
