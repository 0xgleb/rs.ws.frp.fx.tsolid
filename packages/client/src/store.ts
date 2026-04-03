import { createStore, reconcile, produce } from "solid-js/store";
import type { SyncMessage } from "./socket.ts";
import type { OrderFull, OrderPatch, OrderLineFull, OrderLinePatch } from "../generated/sync.ts";

// --- Store shape ---

export interface SyncStore {
  orders: Record<string, OrderFull>;
  connected: boolean;
}

const [store, setStore] = createStore<SyncStore>({
  orders: {},
  connected: false,
});

export { store };

// --- Connection status ---

export function setConnected(connected: boolean): void {
  setStore("connected", connected);
}

// --- Merge-patch helpers ---

function mergeOrderLinePatch(full: OrderLineFull, patch: OrderLinePatch): OrderLineFull {
  return {
    quantity: patch.quantity ?? full.quantity,
    price: patch.price ?? full.price,
  };
}

function mergeOrderPatch(full: OrderFull, patch: OrderPatch): OrderFull {
  const result: OrderFull = {
    symbol: patch.symbol ?? full.symbol,
    side: patch.side ?? full.side,
    status: patch.status ?? full.status,
    totalQuantity: patch.totalQuantity ?? full.totalQuantity,
    filledQuantity: patch.filledQuantity ?? full.filledQuantity,
    averagePrice: patch.averagePrice ?? full.averagePrice,
    fills: { ...full.fills },
    notes: { ...full.notes },
  };

  // Merge nested fills map
  if (patch.fills !== undefined) {
    for (const [key, value] of Object.entries(patch.fills)) {
      if (value === null) {
        // Tombstone — delete the key
        delete result.fills[key];
      } else {
        const existing = result.fills[key];
        if (existing !== undefined) {
          result.fills[key] = mergeOrderLinePatch(existing, value);
        } else {
          // New entry — treat patch as full (all fields must be present for new entries)
          result.fills[key] = {
            quantity: value.quantity ?? "0",
            price: value.price ?? "0",
          };
        }
      }
    }
  }

  // Merge leaf notes map
  if (patch.notes !== undefined) {
    for (const [key, value] of Object.entries(patch.notes)) {
      if (value === null) {
        delete result.notes[key];
      } else {
        result.notes[key] = value;
      }
    }
  }

  return result;
}

// --- Apply incoming messages ---

export function applyMessage(msg: SyncMessage): void {
  switch (msg.entityTag) {
    case "Order": {
      const id = msg.entityId;
      if (msg.kind === "full") {
        // Full replacement — set the entire order at this key
        setStore("orders", { [id]: msg.payload as OrderFull });
      } else {
        // Patch — merge into existing state
        const existing = store.orders[id];
        if (existing !== undefined) {
          const merged = mergeOrderPatch(existing, msg.payload as OrderPatch);
          setStore("orders", { [id]: merged });
        }
        // If entity doesn't exist and we get a patch, ignore it
        // (server should send Full for new entities)
      }
      break;
    }
  }
}

// --- Get current store snapshot for handshake ---

export function getStoreSnapshot(): Array<{
  entityTag: string;
  entityId: string;
  payload: Record<string, unknown>;
}> {
  const entries: Array<{
    entityTag: string;
    entityId: string;
    payload: Record<string, unknown>;
  }> = [];

  for (const [id, order] of Object.entries(store.orders)) {
    entries.push({
      entityTag: "Order",
      entityId: id,
      payload: order as unknown as Record<string, unknown>,
    });
  }

  return entries;
}
