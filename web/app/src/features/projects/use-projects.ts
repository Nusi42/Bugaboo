import { ApiError, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useInfiniteQuery, useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** A project in a list, as the server sends it: the fields can be missing. */
export type ListItem = Schema<"ProjectItem">;
/** One project with where it is and what the person may do there. */
export type OneProject = Schema<"ProjectDetailBody">;
/** A project as a name in a sentence. */
export type Near = Schema<"ProjectRefBody">;
/** What a move would do. */
export type Preview = Schema<"MovePreviewBody">;

/** How many projects one page holds. */
const PAGE_SIZE = 100;
/** How many a search asks for: quick search and the places a project can go show a few. */
export const SEARCH_SIZE = 20;

/** The fields of a project in a list that the screens need. */
const LIST_FIELDS = "id,short_name,name,has_sub_projects,parent";

/** The key of everything about projects in the query cache. */
const projectsKey = ["projects"] as const;

/** What narrows a list of projects. */
export interface ProjectFilter {
  /** The projects directly inside this one (an ID); the top of the tree without it. */
  readonly inside?: string;
  /** Words to find in names and short names, anywhere; an empty text lists them all. */
  readonly search?: string;
  /** Leave out this project (an ID) and everything inside it. */
  readonly outside?: string;
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

/** The projects the person sees that the filter names, a page at a time: *Show more* asks for the next. */
export function useProjectList(filter: ProjectFilter, enabled = true, size = PAGE_SIZE) {
  return useInfiniteQuery({
    queryKey: [...projectsKey, "list", filter, size],
    enabled,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/projects", {
        params: {
          query: {
            ...filter,
            fields: LIST_FIELDS,
            limit: size,
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_ProjectItem">) => last.next_cursor ?? null,
  });
}

/** The lines of the pages read so far. */
export function linesOf(pages: readonly Schema<"Page_ProjectItem">[] | undefined): ProjectLine[] {
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => lineOf(item) ?? []));
}

/** One project by its address (a short name, also an old one, or an ID). */
export function useProject(address: string) {
  return useQuery({
    queryKey: [...projectsKey, "one", address],
    // While another address loads (a project got a new short name), the page keeps showing the
    // project: nothing blinks away and what was typed stays.
    placeholderData: keepPreviousData,
    queryFn: async () => {
      const { data } = await api.client.GET("/api/v1/projects/{project}", {
        params: { path: { project: address } },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

/** What a move of a project to `to` (an ID, or the top level for `null`) would do. */
export function useMovePreview(project: string, to: string | null, enabled: boolean) {
  return useQuery({
    queryKey: [...projectsKey, "move-preview", project, to],
    enabled,
    // A preview is a question about now: it is never kept for later.
    gcTime: 0,
    queryFn: async () => {
      const { data } = await api.client.POST("/api/v1/projects/{project}/move-preview", {
        params: { path: { project } },
        body: { to },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

/** A project was made, changed or moved: every list and every project is read again. */
export function refreshProjects(): void {
  void queryClient.invalidateQueries({ queryKey: projectsKey });
}

/** What is needed to make a project. */
export interface NewProject {
  readonly shortName: string;
  readonly name: string;
}

/** Makes a project at the top level, or inside `parent` (an ID). */
export function useMakeProject(parent: string | undefined) {
  return useMutation({
    mutationFn: async ({ shortName, name }: NewProject) => {
      const body = { short_name: shortName, name };
      const { data } =
        parent === undefined
          ? await api.client.POST("/api/v1/projects", { body })
          : await api.client.POST("/api/v1/projects/{project}/sub-projects", {
              params: { path: { project: parent } },
              body,
            });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
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
      const { data } = await api.client.PATCH("/api/v1/projects/{project}", {
        params: { path: { project: id } },
        body: {
          version,
          ...(name === undefined ? {} : { name }),
          ...(shortName === undefined ? {} : { short_name: shortName }),
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
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
      const { data } = await api.client.POST("/api/v1/projects/{project}/move", {
        params: { path: { project: id } },
        body: { to, preview },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    onSuccess: refreshProjects,
  });
}
