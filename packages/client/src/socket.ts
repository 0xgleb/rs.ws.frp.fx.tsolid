/**
 * @module socket
 *
 * WebSocket connection layer for the sync protocol.
 *
 * Wraps Socket.IO with Effect schema validation to provide type-safe
 * message decoding. Incoming `"sync"` events are validated against the
 * `InboundMessageSchema` before being passed to registered handlers.
 *
 * ## Architecture
 *
 * ```
 * Server ──"sync"──► Socket.IO ──► Effect.Schema.decode ──► SyncMessage
 *                                         │
 *                                    (validation failure → console.warn)
 * ```
 *
 * ## Usage
 *
 * ```ts
 * import { createConnection, sendCommand, sendHandshake } from "./socket";
 *
 * const conn = createConnection();
 *
 * // Listen for validated messages
 * conn.onMessage((msg) => {
 *   if (msg.kind === "full") {
 *     // Full entity snapshot
 *   } else {
 *     // Incremental patch
 *   }
 * });
 *
 * // Send a command
 * sendCommand(conn, "PlaceOrder", { symbol: "BTC", side: "buy" });
 *
 * // Send a handshake (on reconnect)
 * sendHandshake(conn, "Order", entityId, currentState);
 * ```
 */

import { io, type Socket } from "socket.io-client";
import { Schema, Either } from "effect";

// --- Schema decode boundary ---

/**
 * Effect schema for validating incoming OrderFull payloads.
 *
 * All numeric fields (quantities, prices) are strings because Rust's
 * `Decimal` serializes via `serde-str`. The client displays them as-is
 * or parses them for arithmetic.
 */
const OrderFullSchema = Schema.Struct({
  symbol: Schema.String,
  side: Schema.String,
  status: Schema.String,
  totalQuantity: Schema.String,
  filledQuantity: Schema.String,
  averagePrice: Schema.String,
  fills: Schema.Record({ key: Schema.String, value: Schema.Struct({
    quantity: Schema.String,
    price: Schema.String,
  })}),
  notes: Schema.Record({ key: Schema.String, value: Schema.String }),
});

/**
 * Effect schema for validating incoming OrderPatch payloads.
 *
 * All fields are optional (patches are sparse). Map fields use
 * `NullOr` to support tombstone deletions (`null` = remove key).
 */
const OrderPatchSchema = Schema.Struct({
  symbol: Schema.optional(Schema.String),
  side: Schema.optional(Schema.String),
  status: Schema.optional(Schema.String),
  totalQuantity: Schema.optional(Schema.String),
  filledQuantity: Schema.optional(Schema.String),
  averagePrice: Schema.optional(Schema.String),
  fills: Schema.optional(Schema.Record({ key: Schema.String, value: Schema.NullOr(Schema.Struct({
    quantity: Schema.optional(Schema.String),
    price: Schema.optional(Schema.String),
  }))})),
  notes: Schema.optional(Schema.Record({ key: Schema.String, value: Schema.NullOr(Schema.String) })),
});

/**
 * Discriminated union schema for server-to-client sync messages.
 *
 * Discriminated on `kind`:
 * - `"full"` -- Complete entity state (OrderFull payload)
 * - `"patch"` -- Incremental update (OrderPatch payload)
 */
const InboundMessageSchema = Schema.Union(
  Schema.Struct({
    entityTag: Schema.Literal("Order"),
    entityId: Schema.String,
    kind: Schema.Literal("full"),
    payload: OrderFullSchema,
  }),
  Schema.Struct({
    entityTag: Schema.Literal("Order"),
    entityId: Schema.String,
    kind: Schema.Literal("patch"),
    payload: OrderPatchSchema,
  }),
);

/** Decoder function that validates unknown data against InboundMessageSchema. */
const decodeMessage = Schema.decodeUnknownEither(InboundMessageSchema);

/** Type-safe sync message as decoded by the InboundMessageSchema. */
export type SyncMessage = typeof InboundMessageSchema.Type;

// --- Connection ---

/**
 * Abstraction over a Socket.IO connection with typed message handling.
 *
 * Provides a clean interface for the store layer to receive validated
 * messages without dealing with raw Socket.IO events or schema
 * decoding directly.
 */
export interface SyncConnection {
  /** The underlying Socket.IO socket instance. */
  readonly socket: Socket;
  /** Register a handler for validated sync messages. */
  readonly onMessage: (handler: (msg: SyncMessage) => void) => void;
  /** Emit a raw event to the server. */
  readonly send: (event: string, data: unknown) => void;
  /** Disconnect the socket. */
  readonly disconnect: () => void;
}

/**
 * Create a new Socket.IO connection to the sync server.
 *
 * Connects via WebSocket transport with automatic reconnection.
 * Incoming `"sync"` events are decoded with Effect Schema and only
 * valid messages are forwarded to registered handlers.
 *
 * @param url - Server URL. Defaults to `"/"` (same origin, proxied by Vite).
 * @returns A {@link SyncConnection} instance.
 *
 * @example
 * ```ts
 * const conn = createConnection("http://localhost:3000");
 * conn.onMessage((msg) => console.log(msg.entityTag, msg.kind));
 * ```
 */
export function createConnection(url?: string): SyncConnection {
  const socket = io(url ?? "/", {
    transports: ["websocket"],
    reconnection: true,
    reconnectionDelay: 1000,
    reconnectionDelayMax: 5000,
  });

  const handlers: Array<(msg: SyncMessage) => void> = [];

  socket.on("sync", (data: unknown) => {
    const result = decodeMessage(data);
    if (Either.isRight(result)) {
      for (const handler of handlers) {
        handler(result.right);
      }
    } else {
      console.warn("Failed to decode sync message:", result.left);
    }
  });

  return {
    socket,
    onMessage: (handler) => { handlers.push(handler); },
    send: (event, data) => { socket.emit(event, data); },
    disconnect: () => { socket.disconnect(); },
  };
}

// --- Handshake ---

/**
 * Send a handshake message to the server declaring the client's
 * current state for a specific entity.
 *
 * Called on connect/reconnect for each entity the client holds.
 * The server will respond with either a Full (if payload is empty)
 * or a Patch (diff from client state to server state).
 *
 * @param conn - The active sync connection.
 * @param entityTag - Entity type name (e.g., `"Order"`).
 * @param entityId - Entity UUID string.
 * @param payload - Client's current state as a patch-shaped object.
 *
 * @example
 * ```ts
 * sendHandshake(conn, "Order", orderId, { status: "open", symbol: "BTC" });
 * ```
 */
export function sendHandshake(
  conn: SyncConnection,
  entityTag: string,
  entityId: string,
  payload: Record<string, unknown>,
): void {
  conn.send("handshake", {
    entityTag,
    entityId,
    kind: "patch",
    payload,
  });
}

// --- Commands ---

/**
 * Send a command to the server.
 *
 * Commands are application-level actions that mutate server state.
 * A unique `requestId` is generated for each command to enable
 * acknowledgment tracking.
 *
 * @param conn - The active sync connection.
 * @param command - Command name (e.g., `"PlaceOrder"`, `"UpdateOrder"`).
 * @param payload - Command-specific payload data.
 *
 * @example
 * ```ts
 * // Place a new order
 * sendCommand(conn, "PlaceOrder", { symbol: "ETH", side: "sell" });
 *
 * // Update an existing order
 * sendCommand(conn, "UpdateOrder", {
 *   entityId: "550e8400-...",
 *   patch: { status: "filled" },
 * });
 * ```
 */
export function sendCommand(
  conn: SyncConnection,
  command: string,
  payload: Record<string, unknown>,
): void {
  const requestId = crypto.randomUUID();
  conn.send("command", {
    requestId,
    command,
    payload,
  });
}
