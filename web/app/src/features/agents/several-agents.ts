import type { Schema } from "@bugaboo/api-client";

/** An agent a list created, with its key, as the server answered it once. */
export type CreatedAgent = Schema<"CreatedAgentBody">;

/** The names of the agents of the list: one a line, without the empty lines and the spaces around. */
export function namesOf(text: string): readonly string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** The line of one key to copy: the name of its agent, then the key. */
export function keyLine(made: CreatedAgent): string {
  return `${made.agent.name ?? ""}: ${made.key.secret}`;
}

/** Every key of the list to copy at once, one line each, in the order of the list. */
export function allKeysText(made: readonly CreatedAgent[]): string {
  return made.map((item) => keyLine(item)).join("\n");
}
