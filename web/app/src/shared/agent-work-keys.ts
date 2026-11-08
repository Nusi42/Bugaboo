/**
 * The keys of what agent work keeps in the query cache. They are written here only because
 * `query-keys.ts` is as long as a file may be.
 */
export const agentWorkKeys = {
  /** The agents of the person, as the window of a hand-over offers them. */
  myAgents: () => ["agent-work", "my-agents"],
};
