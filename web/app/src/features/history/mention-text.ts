/** A name that is being typed after an `@`: letters, digits and the marks a handle may hold. */
const partial = /(?:^|\s)@([\p{L}\p{N}_.-]*)$/u;

/** The start of the name the person is typing right now after an `@`, or nothing. */
export function mentionBeingTyped(text: string): string | undefined {
  return partial.exec(text)?.[1];
}

/** The text with the name that is being typed replaced by the whole handle and a space. */
export function withMention(text: string, handle: string): string {
  return text.replace(partial, (match) => {
    const space = match.startsWith("@") ? "" : match.slice(0, 1);
    return `${space}@${handle} `;
  });
}
