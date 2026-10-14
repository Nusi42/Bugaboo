import { t } from "@bugaboo/messages";

import { roleName } from "./access.ts";
import { sampleEverywhere, type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M3 share (`m3-exit-demo.spec.ts`).

/** The title of the bug the Team member makes, and of the one the Guest makes. */
export const GUEST_BUG = "The cart forgets my coupon";
export const MEMBER_BUG = "The price on the shop page is one cent off";
/** The cost center the Team member writes. */
export const COST_CENTER = "CC-4711";

/** The keys of the bug the Team member made and of the one the Guest made, kept for the steps after. */
export const made = { key: "", guestKey: "" };

export const costCenter = t("issue-field.cost-center");
/** The key of the field *Assigned to*, which the form of the Website is given. */
export const assignedTo = "assigned-to";
export const assignedToName = t("issue-field.assigned-to");

/** The two Team members of the whole organization who work on the Website in the demo. */
export function theTwo(): { readonly first: SamplePerson; readonly second: SamplePerson } {
  const [first, second] = sampleEverywhere(roleName.teamMember, 2);
  if (first === undefined || second === undefined) {
    throw new Error("The sample has too few Team members.");
  }
  return { first, second };
}
