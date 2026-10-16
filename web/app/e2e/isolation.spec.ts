import { dataOf } from "@bugaboo/api-client";

import { makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { makeProjectWithIssue } from "./support/issues.ts";

// A worker runs the spec files one after the other on one port (M4 WP-03). Whatever a spec made
// must be gone for the next one: the next spec starts a server on an empty data folder, so it
// can set the organization up again and finds no project and no issue.

const ANOTHER_SPEC = "another-spec.spec.ts";

test("test_06 the next spec on a worker finds none of what the spec before made", async ({
  server,
  pool,
}) => {
  const admin = await makeAdmin(server);
  await makeProjectWithIssue(admin, "ISO", "Left behind");
  const before = dataOf(await admin.client.GET("/api/v1/projects", { params: { query: {} } }));
  expect(before.items).toHaveLength(1);
  await admin.dispose();

  const next = await pool.forFile(ANOTHER_SPEC, {}, false, () => Promise.resolve());

  // The same port, a server that is not set up yet: the setup code is accepted again.
  expect(next.url).toBe(server.url);
  const again = await makeAdmin(next);
  const after = dataOf(await again.client.GET("/api/v1/projects", { params: { query: {} } }));
  expect(after.items).toHaveLength(0);
  await again.dispose();
});
