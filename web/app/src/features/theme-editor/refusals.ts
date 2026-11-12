import { ApiError, type ErrorCode, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { colorName, type NamedPair } from "./model.ts";

/** A member of the answer of a refusal with `code` that is a list; nothing for any other error. */
function listIn(error: unknown, code: ErrorCode, member: string): readonly unknown[] {
  if (!(error instanceof ApiError) || error.problem?.code !== code) {
    return [];
  }
  const found: unknown = Reflect.get(error.problem, member);
  return Array.isArray(found) ? found : [];
}

/** Whether a value is a pair of colors as the server names it. */
function isPair(value: unknown): value is NamedPair {
  return (
    typeof value === "object" &&
    value !== null &&
    "first" in value &&
    typeof value.first === "string" &&
    "second" in value &&
    typeof value.second === "string" &&
    "place" in value &&
    typeof value.place === "string" &&
    "more_contrast" in value &&
    typeof value.more_contrast === "boolean"
  );
}

/** The pairs the server named when it refused a theme for everyone as hard to read (`theme.hard-to-read`). */
export function refusedPairsOf(error: unknown): readonly NamedPair[] {
  return listIn(error, "theme.hard-to-read", "hard_to_read").filter((each) => isPair(each));
}

/** The words of why the server refused a value of a theme. */
function reasonWords(): Readonly<Record<Schema<"ValueProblemKind">, string>> {
  return {
    "unknown-name": t("theme-editor.refused-value.unknown-name"),
    "not-a-color": t("theme-editor.refused-value.not-a-color"),
    "more-is-the-same": t("theme-editor.refused-value.more-is-the-same"),
    "bad-shadow": t("theme-editor.refused-value.bad-shadow"),
    "radius-out-of-bounds": t("theme-editor.refused-value.radius-out-of-bounds"),
    "not-a-stack": t("theme-editor.refused-value.not-a-stack"),
    "bad-shape": t("theme-editor.refused-value.bad-shape"),
  };
}

/** Whether a text is one of the reasons the words know. */
function isReason(
  words: Readonly<Record<Schema<"ValueProblemKind">, string>>,
  reason: string,
): reason is Schema<"ValueProblemKind"> {
  return Object.hasOwn(words, reason);
}

/** One refused value in words: its name and why, as the server named them (`theme.value-refused`). */
function refusedWords(value: unknown): string | undefined {
  if (typeof value !== "object" || value === null || !("name" in value) || !("reason" in value)) {
    return undefined;
  }
  const { name, reason } = value;
  const words = reasonWords();
  if (typeof name !== "string" || typeof reason !== "string" || !isReason(words, reason)) {
    return undefined;
  }
  return t("theme-editor.refused-value", { name: colorName(name), reason: words[reason] });
}

/** The values of an uploaded theme that the server refused, each by its name and why. */
export function refusedValuesOf(error: unknown): readonly string[] {
  return listIn(error, "theme.value-refused", "refused_values").flatMap((each) => {
    const words = refusedWords(each);
    return words === undefined ? [] : [words];
  });
}
