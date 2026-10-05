// Turns the OpenAPI document on standard input into TypeScript types on standard output.
// `cargo xtask gen api` runs this; nobody calls it by hand.
import openapiTS, { astToString } from "openapi-typescript";
import ts from "typescript";

const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
const document = JSON.parse(Buffer.concat(chunks).toString("utf8"));

// An object that lists no properties is an open object, whose form is somebody else's standard:
// the challenge and the answer of a passkey belong to WebAuthn. openapi-typescript would make it
// `Record<string, never>`, an object that must stay empty.
const openObject = ts.factory.createTypeReferenceNode("Record", [
  ts.factory.createKeywordTypeNode(ts.SyntaxKind.StringKeyword),
  ts.factory.createKeywordTypeNode(ts.SyntaxKind.UnknownKeyword),
]);

const syntaxTree = await openapiTS(document, {
  // Not `immutable`: that would make every list a `readonly T[]`, which the `Readable` helper of
  // openapi-fetch cannot read (it turns such a list into an object without `map`). Screens take
  // answers as read-only parameters instead (lint: prefer-readonly-parameter-types).
  immutable: false,
  transform(schemaObject) {
    const open =
      schemaObject.type === "object" &&
      schemaObject.properties === undefined &&
      schemaObject.additionalProperties === undefined;
    return open ? openObject : undefined;
  },
});
process.stdout.write(astToString(syntaxTree));
