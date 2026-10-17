/** The things a person can be editing under a lock, as the note about the lock names them. */
export const editedThing = {
  description: "editing.the-description",
  comment: "editing.this-comment",
  layout: "editing.this-layout",
  workflow: "editing.this-workflow",
} as const;

export type EditedThing = (typeof editedThing)[keyof typeof editedThing];
