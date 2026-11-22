import type { Schema } from "@bugaboo/api-client";
import { useMutation } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The person is shown again, on their page and in the list, after a change of their standing. */
function refresh(id: string): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.people.one(id) });
  void queryClient.invalidateQueries({ queryKey: queryKeys.people.all });
}

/** *Switch off* and *Switch on* of a person (ADR-0044, decision 5); the page shows the new standing once it is done. */
export function useSwitch(id: string) {
  const path = { params: { path: { person: id } } };
  const off = useMutation({
    mutationFn: async () => {
      // A refusal rejects in the client; the answer of a switch has no body.
      await api.client.POST("/api/v1/people/{person}/switch-off", path);
    },
    onSuccess: () => {
      refresh(id);
    },
  });
  const on = useMutation({
    mutationFn: async () => {
      await api.client.POST("/api/v1/people/{person}/switch-on", path);
    },
    onSuccess: () => {
      refresh(id);
    },
  });
  return {
    off: () => {
      off.mutate();
    },
    on: () => {
      on.mutate();
    },
    busy: off.isPending || on.isPending,
    error: off.error ?? on.error,
  };
}

/** What *Remove a person* would delete now, what stays shown as *Former user*, and what the law keeps until when. */
export function removalPlanOptions(id: string) {
  return api.query.queryOptions("get", "/api/v1/people/{person}/removal-plan", {
    params: { path: { person: id } },
  });
}

/** *Remove a person*, with the choice about their comments and the password again; the page then shows *Former user* in place. */
export function useRemove(id: string) {
  return useMutation({
    mutationFn: async ({
      comments,
      password,
    }: {
      readonly comments: Schema<"RemovedTexts">;
      readonly password: string;
    }) => {
      await api.client.POST("/api/v1/people/{person}/remove", {
        params: { path: { person: id } },
        body: { comments, password },
      });
    },
    onSuccess: () => {
      refresh(id);
    },
  });
}
