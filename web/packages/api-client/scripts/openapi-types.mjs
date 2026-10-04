// Turns the OpenAPI document on standard input into TypeScript types on standard output.
// `cargo xtask gen api` runs this; nobody calls it by hand.
import openapiTS, { astToString } from "openapi-typescript";

const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
const document = JSON.parse(Buffer.concat(chunks).toString("utf8"));
// `immutable` makes every property readonly: answers from the server are never edited in place.
const syntaxTree = await openapiTS(document, { immutable: true });
process.stdout.write(astToString(syntaxTree));
