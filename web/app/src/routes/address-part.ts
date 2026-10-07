import { useParams } from "@tanstack/react-router";

/** One word of the address of the page that is open, such as the ID of a person or of a role. */
export function useAddressPart(name: string): string | undefined {
  const params: unknown = useParams({ strict: false });
  const part: unknown =
    typeof params === "object" && params !== null ? Reflect.get(params, name) : undefined;
  return typeof part === "string" ? part : undefined;
}
