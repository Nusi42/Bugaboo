import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { skipToken, useQuery, type UseQueryResult } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { linesFrom, pageSize, useAllPages, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { refusedOf, type Refused } from "../../shared/refused.ts";
import { releaseKeys } from "../../shared/release-keys.ts";

/** A release as the server sends it: the fields can be missing in an answer. */
export type ListedRelease = Schema<"ReleaseItem">;

/** The state of a release: *Planned*, *Released* or *Retired*. */
export type StateWord = Schema<"ReleaseState">;

/** A release that has what a line of a list needs. */
export interface Release {
  readonly id: string;
  readonly name: string;
  /** The project it was made in: the one that offers it, and the ones inside it. */
  readonly project: Readonly<Schema<"ProjectRef">>;
  /** *Release date*; none while it has none. */
  readonly releaseDate: string | null;
  readonly state: StateWord;
  /** *Released on*: the day it was marked *Released*; none while it has none. */
  readonly releasedOn: string | null;
  /** Whether the person may make, change, mark and delete the releases of its project. */
  readonly mayManage: boolean;
  /** Counts up with every change: a change is made on the version that was looked at. */
  readonly version: number;
}

/** The release of an item, or nothing for an item that came without what a release needs. */
export function releaseOf(item: ListedRelease): Release | undefined {
  const { id, name, project, state, version } = item;
  if (
    id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    project === undefined ||
    project === null ||
    state === undefined ||
    state === null ||
    version === undefined ||
    version === null
  ) {
    return undefined;
  }
  return {
    id,
    name,
    project,
    state,
    version,
    releaseDate: item.release_date ?? null,
    releasedOn: item.released_on ?? null,
    mayManage: item.may_manage === true,
  };
}

/**
 * The releases offered in a project (ISS-08, ADR-0049): those made in it and in the projects above
 * it, retired ones too, every page, by *Release date* (none last), then by name. A project is an ID
 * or a short name; nothing is read without one.
 */
export function useReleases(project: string | undefined) {
  const list = usePaged(
    {
      queryKey: releaseKeys.list(project ?? ""),
      read: (after) =>
        api.client.GET("/api/v1/projects/{project}/releases", {
          params: {
            path: { project: project ?? "" },
            query: { limit: pageSize.largest, ...after },
          },
        }),
    },
    { enabled: project !== undefined },
  );
  useAllPages(list);
  return {
    isPending: list.isPending,
    isError: list.isError,
    error: list.error,
    releases: linesFrom(list.data?.pages, releaseOf),
  };
}

/** The field of an issue that names a release, by its word in the search language. */
export type ReleaseField = "planned for release" | "found in release";

/** How many issues name a release in a field, as the reader sees them. */
export interface ReleaseCount {
  /** The number of issues the reader sees. */
  readonly issues: number;
  /** The filter that selects them, as search text; none where the group has no name the reader may use. */
  readonly filter: string | undefined;
}

/**
 * The count of the issues that name each release in a field, over the issues the reader sees
 * (PRIV-13): the totals of the search grouped by that field, so no count is worked out here. A release
 * that no group names has none.
 */
export function useReleaseCounts(
  project: string | undefined,
  field: ReleaseField,
): UseQueryResult<ReadonlyMap<string, ReleaseCount>> {
  return useQuery({
    queryKey: releaseKeys.counts(project ?? "", field),
    queryFn: project === undefined ? skipToken : () => countsOf(project, field),
  });
}

/** One page of the groups of the totals, and the pages after it. */
async function groupsFrom(
  project: string,
  field: ReleaseField,
  cursor: string | null,
): Promise<readonly Schema<"SearchTotalRowBody">[]> {
  const page = dataOf(
    await api.client.POST("/api/v1/search/totals", {
      body: { within: project, by: field, ...(cursor === null ? {} : { cursor }) },
    }),
  );
  return page.next_cursor === null
    ? page.groups
    : [...page.groups, ...(await groupsFrom(project, field, page.next_cursor))];
}

/** The count of each release a field names, from the groups of the totals. */
async function countsOf(
  project: string,
  field: ReleaseField,
): Promise<ReadonlyMap<string, ReleaseCount>> {
  const counts = new Map<string, ReleaseCount>();
  for (const group of await groupsFrom(project, field, null)) {
    if (group.key.type === "release") {
      counts.set(group.key.value, { issues: group.issues, filter: group.filter ?? undefined });
    }
  }
  return counts;
}

/** Reads every list and count of releases again: one was made, changed or deleted. */
export function refreshReleases(): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: releaseKeys.all });
}

/** What a change of a release names: what is left out stays as it is. */
export type ReleaseChange = Omit<Schema<"ChangeReleaseBody">, "version">;

/** The answer of a change of a release: the release as it is now, or why not. */
export type Changed = { readonly release: Release } | { readonly refused: Refused };

/** The answer of a route that gave a release back, as a release or as the sentence that says it was not read. */
function changedOf(item: ListedRelease): Changed {
  const release = releaseOf(item);
  return release === undefined ? { refused: { reason: t("releases.not-read") } } : { release };
}

/** Makes a release in a project, in the state *Planned*. */
export async function createRelease(
  project: string,
  body: Schema<"NewReleaseBody">,
): Promise<Changed> {
  try {
    const made = dataOf(
      await api.client.POST("/api/v1/projects/{project}/releases", {
        params: { path: { project } },
        body,
      }),
    );
    await refreshReleases();
    return changedOf(made);
  } catch (error) {
    return { refused: refusedOf(error) };
  }
}

/** Changes a release on the version that was looked at. */
export async function changeRelease(release: Release, change: ReleaseChange): Promise<Changed> {
  try {
    const changed = dataOf(
      await api.client.PATCH("/api/v1/projects/{project}/releases/{release}", {
        params: { path: { project: release.project.id, release: release.id } },
        body: { version: release.version, ...change },
      }),
    );
    await refreshReleases();
    return changedOf(changed);
  } catch (error) {
    return { refused: refusedOf(error) };
  }
}

/**
 * Puts a release that no issue names into the trash, with *Undo*. A release that issues name is
 * refused by the server with a sentence that says to retire it instead; it comes back as the reason.
 */
export async function deleteRelease(release: Release): Promise<Refused | null> {
  const where = { project: release.project.id, release: release.id };
  try {
    dataOf(
      await api.client.DELETE("/api/v1/projects/{project}/releases/{release}", {
        params: { path: where },
      }),
    );
  } catch (error) {
    return refusedOf(error);
  }
  undoStore.offer({
    message: t("releases.deleted", { name: release.name }),
    undo: () => {
      void api.client
        .POST("/api/v1/projects/{project}/releases/{release}/restore", {
          params: { path: where },
        })
        .then(refreshReleases, (error: unknown) => {
          undoStore.offer({ message: messageOf(error) });
        });
    },
  });
  await refreshReleases();
  return null;
}

/** The releases a person may choose for an issue: not retired ones, and those the issue names already. */
export function offeredFor(
  releases: readonly Release[],
  held: readonly { readonly id: string }[],
): readonly Release[] {
  return releases.filter(
    (release) => release.state !== "retired" || held.some((one) => one.id === release.id),
  );
}

/** A release an issue names, as the server shows it. */
export type ReleaseRef = Readonly<Schema<"ReleaseRefBody">>;

/** What an issue names of a release once it is chosen: what the page shows until the server answers. */
export function refOf(release: Release): ReleaseRef {
  return {
    id: release.id,
    name: release.name,
    state: release.state,
    ...(release.releaseDate === null ? {} : { release_date: release.releaseDate }),
    offered: true,
  };
}

/**
 * Gives a release another *Release date* (`null` takes it away) on the version that was looked at, and reads
 * the releases again. Answers the version the change made, which an *Undo* of it is made on; rejects with the
 * refusal of the server. The one way a screen outside the page *Releases* changes a release (the timeline).
 */
export async function setReleaseDate(
  release: { readonly project: string; readonly id: string },
  version: number,
  day: string | null,
): Promise<number | undefined> {
  const changed = dataOf(
    await api.client.PATCH("/api/v1/projects/{project}/releases/{release}", {
      params: { path: { project: release.project, release: release.id } },
      body: { version, release_date: day },
    }),
  );
  await refreshReleases();
  return changed.version ?? undefined;
}
