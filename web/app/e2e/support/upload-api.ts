import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { type Pictured } from "./files.ts";

/**
 * Adds a file to an issue through the API, as the page does: starts the upload, sends the file in
 * one part, finishes it. Gives the attachment. A refusal (`limit.reached`, a type that is blocked)
 * is the error of the call that was refused.
 */
export async function uploadThroughApi(
  client: ApiSession["client"],
  issue: string,
  file: Pictured,
): Promise<{ readonly id: string }> {
  const started = dataOf(
    await client.POST("/api/v1/issues/{issue}/uploads", {
      params: { path: { issue } },
      body: { name: file.name, size: file.buffer.length },
    }),
  );
  dataOf(
    await client.PUT("/api/v1/uploads/{upload}", {
      params: { path: { upload: started.id }, query: { offset: 0 } },
      // The route takes the bytes as they are: the serializer hands over the part itself.
      body: "",
      bodySerializer: () => file.buffer,
      headers: { "content-type": "application/octet-stream" },
    }),
  );
  return dataOf(
    await client.POST("/api/v1/uploads/{upload}/finish", {
      params: { path: { upload: started.id } },
    }),
  );
}
