import { useQuery } from "@tanstack/react-query";

import { api } from "./api.ts";

/** No image from outside: what the preview has until the hosts are read. */
const noHosts: readonly string[] = [];

/**
 * The hosts that images in text may come from (the setting `content.image-hosts`, SEC-13), already
 * split by the server: the preview of the editor shows the same images as the server does. Until
 * they are read, no image from outside is shown.
 */
export function useImageHosts(): readonly string[] {
  const found = useQuery(api.query.queryOptions("get", "/api/v1/content/image-hosts"));
  return found.data?.hosts ?? noHosts;
}
