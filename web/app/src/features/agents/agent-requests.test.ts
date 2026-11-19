import { describe, expect, it } from "vitest";

import { approvedRequests, waitingRequests, type AgentRequest } from "./agent-requests.ts";

function request(id: string, stands: AgentRequest["stands"]): AgentRequest {
  return {
    id,
    asked_by: "Script",
    by_app: false,
    reason: "A team",
    agents: [{ name: "Planner", purpose: "plans" }],
    stands,
    asked_at: "2027-01-15T08:00:00Z",
    waits_until: "2027-01-16T08:00:00Z",
  };
}

describe("requests for agents", () => {
  it("ai_30 shows the waiting requests to answer and the approved ones apart", () => {
    const items = [
      request("a", "waiting"),
      request("b", "approved"),
      request("c", "declined"),
      request("d", "ran-out"),
      request("e", "collected"),
    ];
    expect(waitingRequests(items).map((item) => item.id)).toEqual(["a"]);
    expect(approvedRequests(items).map((item) => item.id)).toEqual(["b"]);
  });
});
