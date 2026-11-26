import { dataOf, type Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";
import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { refreshPins } from "../../shared/refresh-pins.ts";
import type { ShareTarget } from "../../shared/share.ts";

/** A dashboard as the person sees it, with the boxes that check for them (ADR-0050). */
export type Dashboard = Schema<"DashboardBody">;

/** A box as the person sees it: its place packed for them. */
export type Box = Schema<"BoxItem">;

/** A box as a change or a preview asks for it. */
export type AskedBox = Schema<"BoxBody">;

/** What a box answers, by the read of its kind. */
export type BoxAnswer = Schema<"BoxAnswerBody">;

/** A dashboard in a list. */
export type DashboardInList = Schema<"DashboardSummary">;

/** The two lists of *Dashboards*: the ones the person made, and the ones shared with them. */
export type DashboardSet = Schema<"SearchSet">;

/** The words of the two lists. */
export const dashboardSet = { mine: "mine", shared: "shared" } as const satisfies Record<
  string,
  DashboardSet
>;

/** The kinds of box, as the server names them. */
export const boxKind = {
  searchResults: "search-results",
  numbersAndChart: "numbers-and-chart",
  timeTotals: "time-totals",
  smallTimeline: "small-timeline",
  myTimer: "my-timer",
  recentChanges: "recent-changes",
} as const satisfies Record<string, Schema<"BoxKind">>;

/** The name of each kind of box, as *Add box* and the form of a box say it. */
export const kindName = {
  "search-results": "dashboards.kind.search-results",
  "numbers-and-chart": "dashboards.kind.numbers-and-chart",
  "time-totals": "dashboards.kind.time-totals",
  "small-timeline": "dashboards.kind.small-timeline",
  "my-timer": "dashboards.kind.my-timer",
  "recent-changes": "dashboards.kind.recent-changes",
} as const satisfies Record<Schema<"BoxKind">, MessageKey>;

/** The shapes of a box of *Numbers and chart*. */
export const chartShape = { bars: "bars", number: "number" } as const satisfies Record<
  string,
  Schema<"ChartShape">
>;

/** Whose time a box of *Time totals* adds up. */
export const timeSpan = { search: "search", myWeek: "my-week" } as const satisfies Record<
  string,
  Schema<"TimeSpan">
>;

/** The heading of *Dashboards*, in the sidebar and in the sheet *More*. */
export const dashboardsHeading = "dashboards.heading" satisfies MessageKey;

/** Something about dashboards changed: every list, every dashboard and the pins are read again. */
export function refreshDashboards(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.dashboards.all });
  refreshPins();
}

/** The dashboards of one list, a page at a time. */
export function useDashboardList(which: DashboardSet, size: number) {
  return usePaged({
    queryKey: queryKeys.dashboards.list(which, size),
    read: (after) =>
      api.client.GET("/api/v1/dashboards", {
        params: { query: { scope: which, limit: size, ...after } },
      }),
  });
}

/** One dashboard by its ID; `404` (not found) for one the person may not open. */
export function useDashboard(id: string) {
  return useQuery({
    queryKey: queryKeys.dashboards.one(id),
    placeholderData: keepPreviousData,
    retry: false,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/dashboards/{dashboard}", {
          params: { path: { dashboard: id } },
        }),
      ),
  });
}

/** The person's *Home*: their own dashboard, made at the first visit (ADR-0050, decision 6). */
export function useHomeDashboard() {
  return useQuery({
    queryKey: queryKeys.dashboards.home(),
    queryFn: async () => dataOf(await api.client.GET("/api/v1/dashboards/home")),
  });
}

/** What one box answers for the person who looks at it. */
export function useBoxAnswer(dashboard: string, box: string) {
  return useQuery({
    queryKey: queryKeys.dashboards.box(dashboard, box),
    placeholderData: keepPreviousData,
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/dashboards/{dashboard}/boxes/{box}", {
          params: { path: { dashboard, box } },
        }),
      ),
  });
}

/** What a box that is not on a dashboard yet would answer: the preview of *Add box*. */
export function useBoxPreview(box: AskedBox, enabled: boolean) {
  // Where a box would stand changes nothing of what it answers, so the place is not part of the key.
  const { column: _column, row: _row, width: _width, height: _height, ...asked } = box;
  return useQuery({
    queryKey: queryKeys.dashboards.preview(JSON.stringify(asked)),
    enabled,
    retry: false,
    queryFn: async () =>
      dataOf(await api.client.POST("/api/v1/dashboards/box-preview", { body: box })),
  });
}

/** The ready-made boxes, in the order *Add box* offers them (ADR-0050, decision 4). */
export function useReadyMadeBoxes() {
  return useQuery({
    queryKey: queryKeys.dashboards.readyMade(),
    queryFn: async () => dataOf(await api.client.GET("/api/v1/dashboards/ready-made-boxes")),
  });
}

/** A box as a change asks for it: what the person sees of it, with its ID so the change keeps it. */
export function bodyOf(box: Box): AskedBox {
  return {
    id: box.id,
    kind: box.kind,
    column: box.column,
    row: box.row,
    width: box.width,
    height: box.height,
    title: box.title,
    by: box.by,
    shape: box.shape,
    span: box.span,
    saved_search: box.saved_search,
    search_text: box.search_text,
  };
}

/** *Create a dashboard*: private and the person's own, with no boxes yet. */
export function useCreateDashboard() {
  return useMutation({
    mutationFn: async (name: string) =>
      dataOf(await api.client.POST("/api/v1/dashboards", { body: { name } })),
    onSuccess: refreshDashboards,
  });
}

/** What a change of a dashboard sends: a new name, or the boxes the person sees, on the version they looked at. */
export interface DashboardChange {
  readonly name?: string;
  readonly boxes?: readonly AskedBox[];
  readonly version: number;
}

/** Changes the name or the boxes of a dashboard (*Change*; *Customize* on *Home*). */
export function useChangeDashboard(dashboard: Dashboard) {
  return useMutation({
    mutationFn: async (change: DashboardChange) =>
      dataOf(
        await api.client.PATCH("/api/v1/dashboards/{dashboard}", {
          params: { path: { dashboard: dashboard.id } },
          body: {
            version: change.version,
            ...(change.name === undefined ? {} : { name: change.name }),
            ...(change.boxes === undefined ? {} : { boxes: [...change.boxes] }),
          },
        }),
      ),
    onSuccess: (changed: Dashboard) => {
      queryClient.setQueryData(
        dashboard.home ? queryKeys.dashboards.home() : queryKeys.dashboards.one(changed.id),
        changed,
      );
      // A new name shows in the lists and the pins; what the boxes answer does not change with their places.
      void queryClient.invalidateQueries({ queryKey: queryKeys.dashboards.lists() });
      refreshPins();
    },
  });
}

/** Puts a dashboard into the trash; *Undo* takes it out again. */
export function useDeleteDashboard() {
  return useMutation({
    mutationFn: async (dashboard: string) => {
      await api.client.DELETE("/api/v1/dashboards/{dashboard}", {
        params: { path: { dashboard } },
      });
    },
    onSuccess: refreshDashboards,
  });
}

/** Takes a dashboard out of the trash (*Undo* of a delete). */
export async function restoreDashboard(dashboard: string): Promise<void> {
  await api.client.POST("/api/v1/dashboards/{dashboard}/restore", {
    params: { path: { dashboard } },
  });
  refreshDashboards();
}

/** *Save a copy*: a private dashboard of the person, with the boxes they see. */
export function useCopyDashboard(dashboard: string) {
  return useMutation({
    mutationFn: async (name: string) =>
      dataOf(
        await api.client.POST("/api/v1/dashboards/{dashboard}/copy", {
          params: { path: { dashboard } },
          body: { name },
        }),
      ),
    onSuccess: refreshDashboards,
  });
}

/** The groups or the projects the person may share a dashboard with, a page at a time, read while asked for. */
export function useDashboardShareChoices(kind: Schema<"ShareKind">, enabled: boolean) {
  return usePaged(
    {
      queryKey: queryKeys.dashboards.shareChoices(kind),
      read: (after) =>
        api.client.GET("/api/v1/dashboards/share-choices", {
          params: { query: { kind, limit: pageSize.largest, ...after } },
        }),
    },
    { enabled },
  );
}

/** Shares a dashboard with a group or a project, or stops sharing it. */
export function useChangeDashboardShare(dashboard: string) {
  return useMutation({
    mutationFn: async ({ on, target }: { readonly on: boolean; readonly target: ShareTarget }) => {
      if (on) {
        dataOf(
          await api.client.POST("/api/v1/dashboards/{dashboard}/shares", {
            params: { path: { dashboard } },
            body: target,
          }),
        );
      } else {
        await api.client.DELETE("/api/v1/dashboards/{dashboard}/shares/{kind}/{id}", {
          params: { path: { dashboard, kind: target.kind, id: target.id } },
        });
      }
    },
    onSuccess: refreshDashboards,
  });
}
