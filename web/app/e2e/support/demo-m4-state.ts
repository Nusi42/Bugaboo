import { type Mailbox } from "./mailbox.ts";
import { sampleGuestIn, type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M4 share (`m4-exit-demo.spec.ts`).

/** The project whose workflow asks for who does the work (*Ask for*): the sample's Operations. */
export const PROJECT = "OPS";

/** The title of the bug the Guest reports. */
export const BUG = "The nightly backup stops at the second database";

/** The customer the Team member writes: a field the Guest may not see. */
export const CUSTOMER = "Alder Street Bakery";

/** The own field of the sample that the demo hides from Guests. */
export const customerField = "Customer";

/** The folder of the pictures of the demo inside the folder of the screenshots. */
export const DEMO_M4 = "M4/demo";

/** The picture the Team member pastes onto the bug. */
export const PASTED = "backup-error.png";

/** The Guest who reported the bug and follows it (a Guest of the Website, given a role in Operations). */
export const guest: SamplePerson = sampleGuestIn("WEB");

/** What the steps need to know of each other: the bug, and the mail server. */
export const made: { key: string; mailbox: Mailbox | undefined } = {
  key: "",
  mailbox: undefined,
};
