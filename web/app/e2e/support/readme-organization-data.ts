import { type Schema } from "@bugaboo/api-client";

// The made-up organization the pictures of the front page (README.md) show: Quillmoor Cycle Works,
// a small bicycle workshop that builds frames and repairs bikes. It is an example on purpose, far
// from software, so nobody who skims the front page takes an issue of it for a part of Bugaboo.
// The people, their addresses (example.org) and the company are made up. This file holds what the
// organization is; `readme-organization.ts` makes it through the API.

/** Somebody of the workshop, by the key the issues name them with. */
export interface Member {
  readonly name: string;
  readonly email: string;
}

/** The people of the workshop. The first one makes the organization and is its administrator. */
export const members = {
  marta: { name: "Marta Lindqvist", email: "marta.lindqvist@example.org" },
  jonas: { name: "Jonas Falk", email: "jonas.falk@example.org" },
  priya: { name: "Priya Raman", email: "priya.raman@example.org" },
  tomas: { name: "Tomás Varga", email: "tomas.varga@example.org" },
  irene: { name: "Irene Okafor", email: "irene.okafor@example.org" },
  hana: { name: "Hana Sato", email: "hana.sato@example.org" },
  emre: { name: "Emre Kaya", email: "emre.kaya@example.org" },
  ruth: { name: "Ruth Calloway", email: "ruth.calloway@example.org" },
  malik: { name: "Malik Haddad", email: "malik.haddad@example.org" },
} as const satisfies Record<string, Member>;

/** A key of `members`. */
export type Who = keyof typeof members;

/** The project the whole workshop works in, and the projects inside it. */
export const company = { shortName: "QCW", name: "Quillmoor Cycle Works" } as const;

export const projects = {
  repairs: { shortName: "REP", name: "Repairs" },
  frames: { shortName: "FRM", name: "Frame building" },
  orders: { shortName: "ORD", name: "Parts and orders" },
  openDay: { shortName: "DAY", name: "Spring open day" },
} as const;

/** A project inside the company, by its key. */
export type Place = keyof typeof projects;

/** The own roles of the workshop: each a copy of a starting role, by the key of `roleName`. */
export const ownRoles = [
  { name: "Mechanic", copyOf: "teamMember" },
  { name: "Bookkeeper", copyOf: "viewer" },
] as const;

/** The groups of the workshop, their people and the role the group has in the whole company. */
export const groups: readonly {
  readonly name: string;
  readonly people: readonly Who[];
  readonly role: string;
}[] = [
  { name: "Workshop", people: ["priya", "tomas"], role: "Mechanic" },
  { name: "Frame shop", people: ["irene", "hana"], role: "teamMember" },
  { name: "Front desk", people: ["emre"], role: "teamMember" },
];

/** People with a role of their own in the company, beside their groups. */
export const roles: readonly {
  readonly who: Who;
  readonly role: string;
  readonly place: Place | "company";
}[] = [
  { who: "jonas", role: "projectLead", place: "company" },
  { who: "malik", role: "Bookkeeper", place: "company" },
  { who: "ruth", role: "guest", place: "repairs" },
];

/** The statuses of the repairs, and the ways from one to the next with the words of their buttons. */
export const repairWorkflow: {
  readonly name: string;
  readonly statuses: readonly Schema<"StatusRef">[];
  readonly changes: readonly {
    readonly from: string;
    readonly to: string;
    readonly button: string;
    readonly askForWho?: true;
  }[];
} = {
  name: "Repairs",
  statuses: [
    { key: "booked-in", name: "Booked in", status_group: "not-started" },
    { key: "checked", name: "Checked", status_group: "in-progress" },
    { key: "waiting-for-parts", name: "Waiting for parts", status_group: "in-progress" },
    { key: "repairing", name: "Repairing", status_group: "in-progress" },
    { key: "ready", name: "Ready for pickup", status_group: "in-progress" },
    { key: "picked-up", name: "Picked up", status_group: "done" },
  ],
  changes: [
    { from: "booked-in", to: "checked", button: "Check the bike" },
    { from: "checked", to: "waiting-for-parts", button: "Order parts" },
    { from: "checked", to: "repairing", button: "Start the repair", askForWho: true },
    { from: "waiting-for-parts", to: "repairing", button: "Parts are here", askForWho: true },
    { from: "repairing", to: "waiting-for-parts", button: "Needs more parts" },
    { from: "repairing", to: "ready", button: "Repair done" },
    { from: "ready", to: "picked-up", button: "Picked up" },
  ],
};

/** The own fields of the workshop's issues. */
export const fields = {
  customer: { name: "Customer", kind: "text", choices: [] },
  frameSize: { name: "Frame size", kind: "pick-one", choices: ["XS", "S", "M", "L", "XL"] },
} as const;

/** The labels of the workshop: a group of which an issue has one, and labels on their own. */
export const labels = {
  group: { name: "Part of the bike", pickOne: true },
  inGroup: ["Brakes", "Gears", "Wheels", "Frame", "Paint", "Lights and power"],
  alone: ["Warranty", "Waiting for the customer"],
} as const;

/** The searches Marta saved, which the sidebar lists. */
export const savedSearches = [
  { name: "My open work", text: "statusGroup is not Done and assignedTo is me" },
  { name: "Urgent, not done", text: "statusGroup is not Done and priority is high" },
  { name: "Changed this week", text: "changed in last 7 days" },
] as const;

/** One issue of the workshop. A status left out is the first of its workflow. */
export interface Job {
  readonly place: Place;
  readonly title: string;
  readonly type: "Task" | "Bug" | "Feature";
  readonly reporter: Who;
  readonly status?: string;
  readonly priority?: Schema<"Priority">;
  readonly assigned?: Who;
  readonly label?: string;
  readonly customer?: string;
  readonly frameSize?: string;
  readonly tags?: readonly string[];
}

/** The repair the pictures open. */
export const cargoBike: Job & { readonly description: string } = {
  place: "repairs",
  title: "Cargo bike: the rear hub grinds under load",
  type: "Task",
  reporter: "ruth",
  status: "repairing",
  priority: "high",
  assigned: "priya",
  label: "Wheels",
  customer: "Ruth Calloway",
  tags: ["cargo-bike"],
  description: [
    "## What the customer says",
    "",
    "The rear wheel grinds when the box is full, mostly uphill. It started after the winter.",
    "",
    "## What we found",
    "",
    "1. The bearings of the rear hub are pitted.",
    "2. The brake rotor is worn close to its limit.",
    "3. The chain is fine.",
    "",
    "## Plan",
    "",
    "A new hub is cheaper than a rebuild. Ruth agreed on the phone; **ready by Friday**.",
  ].join("\n"),
};

/** What the workshop wrote under the cargo bike, in order. */
export const cargoBikeComments: readonly { readonly who: Who; readonly text: string }[] = [
  { who: "priya", text: "The bearings are pitted on both sides. A new hub is the better deal." },
  { who: "emre", text: "Ruth is fine with a new hub. It is ordered and should come on Thursday." },
  { who: "jonas", text: "Please check the brake rotor too while the wheel is out." },
];

/** The sub-issue of the cargo bike, and the order it waits for. */
export const cargoBikeRotor: Job = {
  place: "repairs",
  title: "Replace the rear brake rotor of the cargo bike",
  type: "Task",
  reporter: "priya",
  status: "checked",
  assigned: "tomas",
  label: "Brakes",
};

export const cargoBikeHub: Job = {
  place: "orders",
  title: "Order a rear hub for the cargo bike",
  type: "Task",
  reporter: "emre",
  status: "in-progress",
  priority: "high",
  assigned: "emre",
};
