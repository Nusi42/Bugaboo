import { partOf, part, UnbrokenWords } from "@bugaboo/ui";

/** The name of a file, in bold, whose words never break at a hyphen. */
export function FileName({ name }: { readonly name: string }) {
  return (
    <span {...partOf(part.filesName)}>
      <UnbrokenWords text={name} />
    </span>
  );
}
