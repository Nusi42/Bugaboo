import { dataOf } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import type { Transport } from "./transfer.ts";

/** The type of the bytes of a part, as the route of a part says it. */
const bytesType = "application/octet-stream";

/** The four calls of an upload, through the one connection of the app (ARCH-WEB-API). */
export const connection: Transport = {
  start: async ({ issue, comment, name, size }) =>
    dataOf(
      await api.client.POST("/api/v1/issues/{issue}/uploads", {
        params: { path: { issue } },
        body: { name, size, comment },
      }),
    ),
  state: async (upload) =>
    dataOf(
      await api.client.GET("/api/v1/uploads/{upload}", {
        params: { path: { upload } },
      }),
    ),
  send: async (upload, offset, bytes) =>
    dataOf(
      await api.client.PUT("/api/v1/uploads/{upload}", {
        params: { path: { upload }, query: { offset } },
        // The route takes the bytes as they are: the serializer hands over the part itself.
        body: "",
        bodySerializer: () => bytes,
        headers: { "content-type": bytesType },
      }),
    ),
  finish: async (upload) =>
    dataOf(
      await api.client.POST("/api/v1/uploads/{upload}/finish", {
        params: { path: { upload } },
      }),
    ),
};
