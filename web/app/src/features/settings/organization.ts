import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import {
  isMySettings,
  type MySettings,
  type Setting,
  type SettingValue,
} from "../../shared/my-settings.ts";
import { type OptimisticMutation, useOptimisticMutation } from "../../shared/use-optimistic.ts";

/** What an administrator does with one setting of the organization. */
export type Edit =
  | { readonly kind: "change"; readonly key: string; readonly value: SettingValue }
  | { readonly kind: "reset"; readonly key: string }
  | { readonly kind: "lock"; readonly key: string; readonly locked: boolean };

/** The settings of the organization for the keys that start with `topics`, separated by commas. */
export function organizationOptions(topics: string) {
  return api.query.queryOptions("get", "/api/v1/settings", {
    params: { query: { topic: topics } },
  });
}

/** One setting as it looks after an edit, before the server has answered. */
function edited(setting: Setting, edit: Edit): Setting {
  if (edit.kind === "change") {
    return { ...setting, value: edit.value, own: true, source: { kind: "organization" } };
  }
  if (edit.kind === "reset") {
    return {
      ...setting,
      value: setting.default,
      own: false,
      locked: false,
      source: { kind: "default" },
    };
  }
  return { ...setting, locked: edit.locked };
}

/** The list as it looks after an edit, before the server has answered. */
export function after(before: MySettings | undefined, edit: Edit): MySettings | undefined {
  return before === undefined
    ? undefined
    : {
        settings: before.settings.map((setting) =>
          setting.key === edit.key ? edited(setting, edit) : setting,
        ),
      };
}

/** Sends an edit to the server. */
async function send(edit: Edit): Promise<void> {
  const path = { params: { path: { key: edit.key } } };
  switch (edit.kind) {
    case "change":
      await api.client.PUT("/api/v1/settings/{key}", { ...path, body: { value: edit.value } });
      return;
    case "reset":
      await api.client.DELETE("/api/v1/settings/{key}", path);
      return;
    case "lock":
      await (edit.locked
        ? api.client.PUT("/api/v1/settings/{key}/lock", path)
        : api.client.DELETE("/api/v1/settings/{key}/lock", path));
  }
}

/** The settings of the organization of one page, and the edits that show at once (UX-06). */
export function useOrganizationSettings(topics: string): {
  readonly settings: MySettings | undefined;
  readonly failed: unknown;
  readonly edits: OptimisticMutation<Edit>;
} {
  const query = useQuery(organizationOptions(topics));
  const edits = useOptimisticMutation({
    queryKey: organizationOptions(topics).queryKey,
    isCached: isMySettings,
    apply: after,
    send,
  });
  return { settings: query.data, failed: query.error, edits };
}
