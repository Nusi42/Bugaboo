import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import type { VectorFile } from "@bugaboo/wasm-search";

const vectorsLocation = new URL("../../tools/testdata/search-vectors.json", import.meta.url);

function isVectorFile(value: unknown): value is VectorFile {
  return (
    typeof value === "object" &&
    value !== null &&
    "readable_fields" in value &&
    "cases" in value &&
    Array.isArray(value.cases) &&
    "places" in value &&
    Array.isArray(value.places)
  );
}

/** The vectors that `cargo xtask gen search-vectors` writes from the Rust code of the server. */
export function readVectors(): VectorFile {
  const vectors: unknown = JSON.parse(readFileSync(fileURLToPath(vectorsLocation), "utf8"));
  if (!isVectorFile(vectors)) {
    throw new Error("The search vectors are not what the generator writes.");
  }
  return vectors;
}
