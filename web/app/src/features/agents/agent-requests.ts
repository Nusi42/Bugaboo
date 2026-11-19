import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { refreshRoutes } from "../../shared/query-keys.ts";
import { refreshAgents } from "./agents.ts";

/** A request of a program for agents, as its person sees it. */
export type AgentRequest = Schema<"AgentRequestBody">;

/** The requests asked of the person in the last week, newest first. */
export function useAgentRequests() {
  return api.query.useQuery("get", "/api/v1/agents/requests");
}

/** Reads the requests and the agents again after an answer. */
function refreshRequests(): void {
  refreshRoutes(["/api/v1/agents/requests"]);
  refreshAgents();
}

/** *Approve*: every agent of the ticked requests, with the password once. */
export function useApproveAgentRequests() {
  return api.query.useMutation("post", "/api/v1/agents/requests/approve", {
    onSuccess: refreshRequests,
  });
}

/** *Decline* one request. */
export function useDeclineAgentRequest() {
  return api.query.useMutation("post", "/api/v1/agents/requests/{request}/decline", {
    onSuccess: refreshRequests,
  });
}

/** The requests that wait for the person's answer. */
export function waitingRequests(items: readonly AgentRequest[]): readonly AgentRequest[] {
  return items.filter((item) => item.stands === "waiting");
}

/** The requests approved whose keys wait for the program that asked. */
export function approvedRequests(items: readonly AgentRequest[]): readonly AgentRequest[] {
  return items.filter((item) => item.stands === "approved");
}
