import { mtprotoService } from "../services/mtprotoService";
import type { ApiResult } from "./types";
import { Api } from "telegram";
import type { BotCommandInput } from "./apiResultTypes";

export async function setBotCommands(
  commands: BotCommandInput[]
): Promise<ApiResult<number>> {
  if (!Array.isArray(commands) || commands.length === 0) {
    throw new Error("commands array is required and must not be empty");
  }

  const client = mtprotoService.getClient();

  const botCommands = commands.map(
    (c: BotCommandInput) =>
      new Api.BotCommand({
        command: String(c.command ?? ""),
        description: String(c.description ?? ""),
      })
  );

  await mtprotoService.withFloodWaitHandling(async () => {
    await client.invoke(
      new Api.bots.SetBotCommands({
        scope: new Api.BotCommandScopeDefault(),
        langCode: "",
        commands: botCommands,
      })
    );
  });

  return { data: commands.length, fromCache: false };
}
