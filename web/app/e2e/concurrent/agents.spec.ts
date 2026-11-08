import { prepareAgents, workWithAgents } from "../../../tooling/load/agents-run.ts";
import { newBook } from "../../../tooling/load/book.ts";
import { prepare } from "../../../tooling/load/organization.ts";
import {
  administratorOf,
  expectTheWorkOfTheAgents,
  expectTheWorkToHaveStopped,
  openIssuesOfCrews,
} from "../support/crowd-agents.ts";
import { expect, test } from "../support/fixtures.ts";

// Ten people and their agents at the same time (architecture §11, M7 WP-13, AI-30 to AI-38): the
// eight Team members of the project each have an agent and an issue handed over to it, one more
// person loses access while their agent works, and two agents take one issue at the same moment.
// The rules of the run (one taker, nothing beyond the person, every change with its run, every
// undone run complete) are judged by `bb-load-check` in the load run and the invariants test; the
// browsers here show what each person sees of it.

test("ai_30_ten_people_and_their_agents_work_at_the_same_moments_and_each_sees_the_result", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const { sessions } = await prepare(server, 10);
  const admin = administratorOf(sessions);
  const book = newBook();
  const agents = await prepareAgents(server, admin, sessions.slice(1, -1), book);
  // Each person has a browser of their own, open on the issue they handed over.
  const pages = await openIssuesOfCrews(browser, server, look, sessions, agents);
  await workWithAgents(server, admin, agents, book);
  // Two agents took one issue at the same moment and exactly one got it.
  const written = book.file([]);
  expect(written.races).toHaveLength(1);
  expect(written.races?.[0]?.winners).toHaveLength(1);
  // The person who lost access was refused after the loss, and every person sees the work of their
  // agent: the block of agent work and the mark of the short comment the agent wrote.
  expect(written.lost?.[0]?.kept_after).toBe(0);
  await expectTheWorkOfTheAgents(pages);
  const stopped = await expectTheWorkToHaveStopped(browser, server, look, admin, agents);
  await Promise.all([...pages, stopped].map((page) => page.context().close()));
});
