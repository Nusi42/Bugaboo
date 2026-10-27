/**
 * The calls of the search language: the module speaks JSON and counts bytes of the UTF-8 text; here
 * the places are turned into places in the string (`offsets.ts`), and an input the module refuses
 * is a defect of the build, which throws: the browser and the Rust types differ.
 */
import {
  check,
  duration_minutes_of,
  issue_id_of,
  open_last_word,
  operand_source,
  own_field_word,
  parse,
  print,
  suggest,
} from "../generated/bb_wasm_search.js";
import { inStringUnits, offsetsOf } from "./offsets.ts";
import type { Answer, Operand, ReadableFields, Search, Suggestions } from "./types.generated.ts";

/** The search language, as the server has it. Places are in units of the string. */
export interface SearchLanguage {
  /** What a text comes to without the fields: its tree and canonical text, or its first mistake. */
  readonly parse: (text: string) => Answer;
  /** The same with the fields the person may read: all mistakes of the checker. */
  readonly check: (text: string, fields: ReadableFields) => Answer;
  /** The canonical text of a tree, which Simple search edits. */
  readonly print: (search: Search) => string;
  /** What may come next at the cursor, a place in the string. */
  readonly suggest: (text: string, cursor: number, fields: ReadableFields) => Suggestions;
  /** The text of a person who is still typing, with a `*` after the last word when it can take one. */
  readonly openLastWord: (text: string) => string;
  /** How a value stands in the text of a search, quoted where the language needs it. */
  readonly operandSource: (operand: Operand) => string;
  /** The Issue ID that the words are, in capitals; none when they are not exactly one. */
  readonly issueIdOf: (words: string) => string | undefined;
  /** The minutes of a text that is a length of time and nothing else; none otherwise. */
  readonly durationMinutesOf: (text: string) => number | undefined;
  /** The word of an own field, made from its name by the one rule of the language. */
  readonly ownFieldWord: (name: string) => string;
}

function isAnswer(value: unknown): value is Answer {
  return typeof value === "object" && value !== null && "mistakes" in value && "search" in value;
}

function isSuggestions(value: unknown): value is Suggestions {
  return typeof value === "object" && value !== null && "items" in value && "replace" in value;
}

/** The answer of the module for `text`, with places in units of the string. */
function answerFor(text: string, json: string): Answer {
  const answer: unknown = inStringUnits(JSON.parse(json), offsetsOf(text));
  if (!isAnswer(answer)) {
    throw new Error("The search language gave something that is no answer.");
  }
  return answer;
}

export const language: SearchLanguage = {
  openLastWord: (text) => open_last_word(text),
  parse: (text) => answerFor(text, parse(text)),
  check: (text, fields) => answerFor(text, check(text, JSON.stringify(fields))),
  print: (search) => print(JSON.stringify(search)),
  suggest: (text, cursor, fields) => {
    const offsets = offsetsOf(text);
    const found: unknown = inStringUnits(
      JSON.parse(suggest(text, offsets.inBytes(cursor), JSON.stringify(fields))),
      offsets,
    );
    if (!isSuggestions(found)) {
      throw new Error("The search language gave something that is no suggestions.");
    }
    return found;
  },
  operandSource: (operand) => operand_source(JSON.stringify(operand)),
  issueIdOf: (words) => issue_id_of(words) ?? undefined,
  durationMinutesOf: (text) => duration_minutes_of(text) ?? undefined,
  ownFieldWord: own_field_word,
};
