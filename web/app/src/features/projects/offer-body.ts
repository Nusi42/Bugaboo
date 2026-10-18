/**
 * The body that changes one line of *Offered in*: the version of the issue type or group the line
 * belongs to, and the line: a project (none for the whole organization) and whether it is offered
 * (none takes the line away). One shape for the issue types and the groups of labels.
 */
export function offerChangeBody(
  version: number,
  project: string | null,
  offered: boolean | null,
): {
  version: number;
  changes: { project?: string; offered?: boolean }[];
} {
  return {
    version,
    changes: [
      {
        ...(project === null ? {} : { project }),
        ...(offered === null ? {} : { offered }),
      },
    ],
  };
}
