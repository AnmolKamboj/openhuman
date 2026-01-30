/**
 * Type bridge utilities for Telegram MTProto API.
 *
 * The `telegram` library's `client.invoke()` returns complex union types,
 * and `getInputEntity()` returns `TypeInputPeer` which doesn't directly
 * narrow to specific entity types like `TypeInputChannel`.
 *
 * These helpers centralize the necessary type casts in one documented
 * location rather than scattering casts throughout tool files.
 */
import type { Api } from "telegram";

/** Cast getInputEntity() result to Api.TypeInputChannel */
export function toInputChannel(entity: object): Api.TypeInputChannel {
  return entity as Api.TypeInputChannel;
}

/** Cast getInputEntity() result to Api.TypeInputUser */
export function toInputUser(entity: object): Api.TypeInputUser {
  return entity as Api.TypeInputUser;
}

/** Cast getInputEntity() result to Api.TypeInputPeer */
export function toInputPeer(entity: object): Api.TypeInputPeer {
  return entity as Api.TypeInputPeer;
}

/**
 * Narrow an invoke() result to the expected shape.
 * Telegram's client.invoke() returns complex union types that can't
 * be narrowed purely via control flow. This provides a typed
 * alternative to inline casts in tool files.
 */
export function narrow<T extends object>(result: object): T {
  return result as T;
}
