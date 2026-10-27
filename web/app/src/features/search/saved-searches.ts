import { dataOf, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A saved search as the person sees it. */
export type SavedSearch = Schema<"SavedSearchItem">;

/** The two lists of *Saved searches*: the ones the person made, and the ones shared with them. */
export type SavedSet = Schema<"SearchSet">;

/** The words of the two lists. */
export const savedSet = { mine: "mine", shared: "shared" } as const satisfies Record<
  string,
  SavedSet
>;

/** How the results of a search are shown, as the server keeps it. */
export type Presentation = Schema<"PresentationRef">;

/** Something about saved searches changed: every list and every search is read again. */
export function refreshSavedSearches(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.search.savedAll() });
}

/** One saved search by its ID; `404` (not found) for one the person may not open. */
export function useSavedSearch(id: string) {
  return useQuery({
    queryKey: queryKeys.search.savedOne(id),
    placeholderData: keepPreviousData,
    retry: false,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/saved-searches/{search}", {
          params: { path: { search: id } },
        }),
      ),
  });
}

/** The saved searches of one list, a page at a time, oldest first. */
export function useSavedList(which: SavedSet, size: number = pageSize.largest) {
  return usePaged({
    queryKey: queryKeys.search.saved(which, size),
    read: (after) =>
      api.client.GET("/api/v1/saved-searches", {
        params: { query: { scope: which, limit: size, ...after } },
      }),
  });
}

/** What is needed to keep a search under a name. */
export interface NewSearch {
  readonly name: string;
  /** The canonical text of the search. */
  readonly text: string;
  /** How the results are shown; the list as the project has it when left out. */
  readonly presentation?: Presentation | undefined;
}

/** The presentation as the body of a request takes it: no part of it is read-only. */
function writable(presentation: Presentation) {
  return { ...presentation, columns: presentation.columns.map((column) => ({ ...column })) };
}

/** *Save search*: keeps the search under a name; it is private and the person's own. */
export function useSaveSearch() {
  return useMutation({
    mutationFn: async ({ name, text, presentation }: NewSearch) =>
      dataOf(
        await api.client.POST("/api/v1/saved-searches", {
          body: {
            name,
            search_text: text,
            ...(presentation === undefined
              ? {}
              : {
                  presentation: writable(presentation),
                }),
          },
        }),
      ),
    onSuccess: refreshSavedSearches,
  });
}

/** *Save*: changes the text and the view of a saved search on the version that was looked at. */
export function useChangeSearch(id: string) {
  return useMutation({
    mutationFn: async ({
      text,
      presentation,
      version,
    }: {
      readonly text: string;
      readonly presentation: Presentation;
      readonly version: number;
    }) =>
      dataOf(
        await api.client.PATCH("/api/v1/saved-searches/{search}", {
          params: { path: { search: id } },
          body: { search_text: text, presentation: writable(presentation), version },
        }),
      ),
    onSuccess: refreshSavedSearches,
  });
}

/** The groups or the projects the person may share a saved search with, a page at a time. */
export function useShareChoices(kind: Schema<"ShareKind">) {
  return usePaged({
    queryKey: queryKeys.search.shareChoices(kind),
    read: (after) =>
      api.client.GET("/api/v1/saved-searches/share-choices", {
        params: { query: { kind, limit: pageSize.largest, ...after } },
      }),
  });
}

/** The words of the two kinds a search is shared with. */
export const shareKind = { group: "group", project: "project" } as const satisfies Record<
  string,
  Schema<"ShareKind">
>;

/** What a share is about: a group or a project. */
export interface ShareTarget {
  readonly kind: Schema<"ShareKind">;
  readonly id: string;
}

/** Shares a saved search with a group or a project, or stops sharing it. */
export function useChangeShare(search: string) {
  return useMutation({
    mutationFn: async ({ on, target }: { readonly on: boolean; readonly target: ShareTarget }) => {
      if (on) {
        await api.client.POST("/api/v1/saved-searches/{search}/shares", {
          params: { path: { search } },
          body: target,
        });
      } else {
        await api.client.DELETE("/api/v1/saved-searches/{search}/shares/{kind}/{id}", {
          params: { path: { search, kind: target.kind, id: target.id } },
        });
      }
    },
    onSuccess: refreshSavedSearches,
  });
}

/** *Use as the project's list*: the saved search the project shows, or none (`null`). */
export function useSetProjectList(project: string) {
  return useMutation({
    mutationFn: async (search: string | null) =>
      dataOf(
        await api.client.PUT("/api/v1/projects/{project}/list", {
          params: { path: { project } },
          body: { search },
        }),
      ),
    onSuccess: refreshSavedSearches,
  });
}

/** The saved search a project shows as its list, if it chose one and the person may open it. */
export function useProjectListChoice(project: string) {
  return useQuery({
    queryKey: queryKeys.search.projectList(project),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/projects/{project}/list", {
          params: { path: { project } },
        }),
      ),
  });
}
