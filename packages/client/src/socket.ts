import { io, type Socket } from "socket.io-client";
import { Schema, Either } from "effect";

// --- Schema decode boundary ---

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

const decodeMessage = Schema.decodeUnknownEither(InboundMessageSchema);

export type SyncMessage = typeof InboundMessageSchema.Type;

// --- Connection ---

export interface SyncConnection {
  readonly socket: Socket;
  readonly onMessage: (handler: (msg: SyncMessage) => void) => void;
  readonly send: (event: string, data: unknown) => void;
  readonly disconnect: () => void;
}

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
