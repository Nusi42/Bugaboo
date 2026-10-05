import { createApi } from "@bugaboo/api-client";

/**
 * The one connection of the browser app to the server (ARCH-WEB-API). Every screen reads and
 * changes server data through it, and the results live in the query cache and nowhere else.
 */
export const api = createApi();
