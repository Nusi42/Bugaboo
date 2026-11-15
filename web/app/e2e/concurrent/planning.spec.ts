import { preparePlanCrowd } from "../support/crowd-plan.ts";
import { aMoveAfterWhileAnIssueChanges } from "../support/crowd-plan-move.ts";
import {
  aStaleSaveOffersKeepTheirsOrUseMine,
  tenPeopleUndoTheirRuns,
} from "../support/crowd-plan-runs.ts";
import {
  anAwayTimeWhileThePlanIsRead,
  aWarningAcceptedWhileItsBarMoves,
  tenPeopleMoveBars,
} from "../support/crowd-plan-steps.ts";
import { test } from "../support/fixtures.ts";

// Ten people planning at the same time (architecture §11, M8 WP-12, PLAN-01, PLAN-11, PLAN-13,
// TEST-CONCURRENT): the people move bars at the same moments, one accepts a warning while its bar
// moves, one enters an away time while the plan is read, a move after something is sent after an
// issue it moves was changed, a date saved on a version somebody moved since offers *Keep Ben's
// change* or *Use mine*, and the runs of ten people are each undone whole. The rules of the run (no date is lost, an accepted warning holds
// the dates the person saw, an away time shows on the next answer, a move after is kept whole) are
// judged by `bb-load-check` (crates/bb-load-check/src/plan.rs) and the test of the server
// `plan_03_ten_people_plan_at_the_same_moments_and_the_rules_hold`; the browsers here show what a
// person sees of it.

test.use({ preparation: { run: preparePlanCrowd } });
test.describe.configure({ mode: "serial" });

test("plan_03_ten_people_move_bars_at_the_same_moments_and_no_date_is_lost", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  await tenPeopleMoveBars(browser, server, { viewport, hasTouch });
});

test("plan_13_a_warning_accepted_while_its_bar_moves_comes_back", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  await aWarningAcceptedWhileItsBarMoves(browser, server, { viewport, hasTouch });
});

test("plan_11_an_away_time_entered_while_the_plan_is_read_shows_on_the_next_answer", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  await anAwayTimeWhileThePlanIsRead(browser, server, { viewport, hasTouch });
});

test("plan_13_move_it_after_while_someone_changes_an_issue_it_moves_is_refused_whole", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  await aMoveAfterWhileAnIssueChanges(browser, server, { viewport, hasTouch });
});

test("plan_03_a_date_saved_on_a_version_somebody_moved_offers_keep_theirs_or_use_mine", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  await aStaleSaveOffersKeepTheirsOrUseMine(browser, server, { viewport, hasTouch });
});

test("plan_13_ten_people_undo_their_runs_at_the_same_moments_and_each_is_undone_whole", async ({
  server,
}) => {
  await tenPeopleUndoTheirRuns(server);
});
