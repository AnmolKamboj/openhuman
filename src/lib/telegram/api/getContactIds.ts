import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import bigInt from "big-integer";
import type { ContactIdEntry } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

type ContactIdResult = number | ContactIdEntry;

export async function getContactIds(): Promise<ApiResult<string[]>> {
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(new Api.contacts.GetContactIDs({ hash: bigInt(0) }));
  });

  if (!result || !Array.isArray(result) || result.length === 0) {
    return { data: [], fromCache: false };
  }

  const ids = narrow<ContactIdResult[]>(result).map((c) =>
    String(typeof c === "number" ? c : c.userId ?? c)
  );

  return { data: ids, fromCache: false };
}
