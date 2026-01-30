import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { PrivacyResult } from "./apiResultTypes";
import { narrow } from "./apiCastHelpers";

export type PrivacyKey =
  | "phone_number"
  | "last_seen"
  | "profile_photo"
  | "forwards"
  | "phone_call"
  | "chat_invite";

export interface PrivacySettings {
  key: PrivacyKey;
  rules: string[];
}

const keyMap: Record<PrivacyKey, Api.TypeInputPrivacyKey> = {
  phone_number: new Api.InputPrivacyKeyPhoneNumber(),
  last_seen: new Api.InputPrivacyKeyStatusTimestamp(),
  profile_photo: new Api.InputPrivacyKeyProfilePhoto(),
  forwards: new Api.InputPrivacyKeyForwards(),
  phone_call: new Api.InputPrivacyKeyPhoneCall(),
  chat_invite: new Api.InputPrivacyKeyChatInvite(),
};

export async function getPrivacySettings(
  key: string = "last_seen"
): Promise<ApiResult<PrivacySettings>> {
  if (!(key in keyMap)) {
    throw new Error(
      "Unknown privacy key: " +
        key +
        ". Valid keys: " +
        Object.keys(keyMap).join(", ")
    );
  }

  const privacyKey = key as PrivacyKey;
  const client = mtprotoService.getClient();

  const result = await mtprotoService.withFloodWaitHandling(async () => {
    return client.invoke(
      new Api.account.GetPrivacy({ key: keyMap[privacyKey] })
    );
  });

  const rules = narrow<PrivacyResult>(result)?.rules;
  if (!rules || !Array.isArray(rules)) {
    return {
      data: { key: privacyKey, rules: [] },
      fromCache: false,
    };
  }

  const ruleNames = rules.map((r) => r.className ?? "Unknown rule");

  return {
    data: { key: privacyKey, rules: ruleNames },
    fromCache: false,
  };
}
