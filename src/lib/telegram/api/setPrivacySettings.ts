import { mtprotoService } from "../../../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";

export type PrivacyKey =
  | "phone_number"
  | "last_seen"
  | "profile_photo"
  | "forwards"
  | "phone_call"
  | "chat_invite";

export type PrivacyRule = "allow_all" | "allow_contacts" | "disallow_all";

const keyMap: Record<PrivacyKey, Api.TypeInputPrivacyKey> = {
  phone_number: new Api.InputPrivacyKeyPhoneNumber(),
  last_seen: new Api.InputPrivacyKeyStatusTimestamp(),
  profile_photo: new Api.InputPrivacyKeyProfilePhoto(),
  forwards: new Api.InputPrivacyKeyForwards(),
  phone_call: new Api.InputPrivacyKeyPhoneCall(),
  chat_invite: new Api.InputPrivacyKeyChatInvite(),
};

const ruleMap: Record<PrivacyRule, Api.TypeInputPrivacyRule> = {
  allow_all: new Api.InputPrivacyValueAllowAll(),
  allow_contacts: new Api.InputPrivacyValueAllowContacts(),
  disallow_all: new Api.InputPrivacyValueDisallowAll(),
};

export async function setPrivacySettings(
  setting: string,
  value: string
): Promise<ApiResult<void>> {
  if (!setting) {
    throw new Error("setting is required");
  }
  if (!value) {
    throw new Error("value is required");
  }

  if (!(setting in keyMap)) {
    throw new Error("Unknown privacy key: " + setting);
  }

  if (!(value in ruleMap)) {
    throw new Error(
      "Unknown rule: " +
        value +
        ". Valid: allow_all, allow_contacts, disallow_all"
    );
  }

  const privacyKey = setting as PrivacyKey;
  const privacyRule = value as PrivacyRule;

  const client = mtprotoService.getClient();

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.invoke(
      new Api.account.SetPrivacy({
        key: keyMap[privacyKey],
        rules: [ruleMap[privacyRule]],
      })
    );
  });

  return { data: undefined, fromCache: false };
}
