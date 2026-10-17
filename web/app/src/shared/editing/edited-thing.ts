/** The things a person can be editing under a lock, as the note about the lock names them. */
export const editedThing = {
  description: "editing.the-description",
  comment: "editing.this-comment",
  layout: "editing.this-layout",
  workflow: "editing.this-workflow",
} as const;

/** The kinds of things that are edited as a whole and can be taken away, as a word in a sentence. */
export const editedKind = {
  layout: "editing.kind-layout",
  workflow: "editing.kind-workflow",
} as const;

export type EditedKind = (typeof editedKind)[keyof typeof editedKind];

export type EditedThing = (typeof editedThing)[keyof typeof editedThing];
