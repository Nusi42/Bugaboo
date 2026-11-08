/** The keys of what agents keep in the query cache, so one prefix reads them all again. */
export const agentKeys = {
  all: ["agents"],
  /** Every agent of the organization, a page at a time. */
  organization: () => ["agents", "organization"],
};
