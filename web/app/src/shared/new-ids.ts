let counter = 0;

/** An invented ID for something new in a draft (a status, a status change, a condition, a section); never sent. */
export function newId(kind: string): string {
  counter += 1;
  return `new-${kind}-${counter}`;
}
