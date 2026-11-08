/** The keys of what agents keep in the query cache, so one prefix reads them all again. */
export const agentKeys = {
  all: ["agents"],
  /** Every agent of the organization, a page at a time. */
  organization: () => ["agents", "organization"],
  /** The standing rules of one agent, a page at a time. */
  rules: (agent: string) => ["agents", "rules", agent],
};
