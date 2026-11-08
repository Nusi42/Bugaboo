import { api } from "./api.ts";
import { usePermissions } from "./use-me.ts";

/**
 * Whether *My agents* is in the sidebar and the sheet *More*: only for a person who has an agent or may create
 * one (UX §4.17). The question is one small request (the first agent, its ID only), asked once and kept.
 */
export function useMyAgentsEntry(): boolean {
  const mayCreate = usePermissions().includes("create-agents");
  const first = api.query.useQuery("get", "/api/v1/agents", {
    params: { query: { limit: 1, fields: "id" } },
  });
  return mayCreate || (first.data?.items.length ?? 0) > 0;
}
