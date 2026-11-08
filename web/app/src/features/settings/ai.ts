import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { refreshRoutes } from "../../shared/query-keys.ts";

/** The setup of AI of the organization; the key is never in it. */
export type AiSetup = Schema<"AiSetupBody">;
/** Where the data goes, which provider and which country, and what may go. */
export type WhatGoesWhere = Schema<"WhatGoesWhereBody">;
/** Where the data of a call goes. */
export type DataDestination = Schema<"Destination">;

/** The routes that tell the screens how AI stands; a change of any of them reads them all again. */
const AI_ROUTES = [
  "/api/v1/ai",
  "/api/v1/assistant",
  "/api/v1/projects/{project}/ai",
  "/api/v1/never-sent-to-ai",
] as const;

/** AI was set up or switched: the setup, *What goes where* and the places that ask whether the assistant is there are read again. */
export function refreshAi(): void {
  refreshRoutes(AI_ROUTES);
}

/** What a screen needs of a read: the answer, read only, and why there is none. */
export interface Read<Data> {
  readonly data: Data | undefined;
  readonly error: unknown;
}

/** The setup of AI of the organization. */
export function useAiSetup(): Read<AiSetup> {
  const { data, error } = api.query.useQuery("get", "/api/v1/ai");
  return { data, error };
}

/** What goes where: the provider, its country, the model and which data may go. */
export function useWhatGoesWhere(): Read<WhatGoesWhere> {
  const { data, error } = api.query.useQuery("get", "/api/v1/ai/what-goes-where");
  return { data, error };
}

/** Changes the setup; a part that is left out stays as it is. */
export function useChangeAi() {
  return api.query.useMutation("put", "/api/v1/ai", { onSuccess: refreshAi });
}

/** Enters, changes or removes the key of the provider. */
export function useSetAiKey() {
  return api.query.useMutation("put", "/api/v1/ai/key", { onSuccess: refreshAi });
}

/** Confirms that a data processing agreement with the provider is in place. */
export function useConfirmAgreement() {
  return api.query.useMutation("post", "/api/v1/ai/agreement", { onSuccess: refreshAi });
}

/** *Try the provider*: one short call that holds no data of the organization. */
export function useTryProvider() {
  return api.query.useMutation("post", "/api/v1/ai/try");
}

/** The switch of AI of one project. */
export function useProjectAi(project: string): Read<Schema<"ProjectAiBody">> {
  const { data, error } = api.query.useQuery("get", "/api/v1/projects/{project}/ai", {
    params: { path: { project } },
  });
  return { data, error };
}

/** Switches AI on or off for a project, or lets it follow the one above again. */
export function useChangeProjectAi() {
  return api.query.useMutation("put", "/api/v1/projects/{project}/ai", { onSuccess: refreshAi });
}

/** Which built-in fields and projects are kept from AI. */
export function useNeverSentMarks(): Read<Schema<"NeverSentBody">> {
  const { data, error } = api.query.useQuery("get", "/api/v1/never-sent-to-ai");
  return { data, error };
}

/** Marks a built-in field *Never sent to AI*, or takes the mark away. */
export function useMarkField() {
  return api.query.useMutation("put", "/api/v1/never-sent-to-ai/fields/{field}", {
    onSuccess: refreshAi,
  });
}

/** Marks a project *Never sent to AI*, or takes the mark away. */
export function useMarkProject() {
  return api.query.useMutation("put", "/api/v1/never-sent-to-ai/projects/{project}", {
    onSuccess: refreshAi,
  });
}

/** The web addresses an administrator allowed for AI apps. */
export function useAppAddresses() {
  return api.query.useQuery("get", "/api/v1/oauth/web-addresses");
}

/** Allows a web address for AI apps. */
export function useAddAppAddress() {
  return api.query.useMutation("post", "/api/v1/oauth/web-addresses", {
    onSuccess: () => {
      refreshRoutes(["/api/v1/oauth/web-addresses"]);
    },
  });
}

/** Takes a web address away: apps that registered with it are refused at their next sign-in. */
export function useRemoveAppAddress() {
  return api.query.useMutation("post", "/api/v1/oauth/web-addresses/remove", {
    onSuccess: () => {
      refreshRoutes(["/api/v1/oauth/web-addresses"]);
    },
  });
}

/** The name of the provider a destination leads to, as the catalog writes it. */
export function providerName(destination: DataDestination): string {
  const names: Readonly<Record<DataDestination, string>> = {
    anthropic: t("ai.provider.anthropic"),
    "local-model": t("ai.provider.local-model"),
    "other-provider": t("ai.provider.other"),
  };
  return names[destination];
}

/** The country the data goes to for a destination, as the catalog writes it. */
export function countryName(destination: DataDestination): string {
  const names: Readonly<Record<DataDestination, string>> = {
    anthropic: t("ai.country.anthropic"),
    "local-model": t("ai.country.local-model"),
    "other-provider": t("ai.country.other"),
  };
  return names[destination];
}
