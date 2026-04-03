/**
 * @module store
 *
 * Reactive state store for synchronized entities.
 *
 * Uses SolidJS `createStore` for fine-grained reactivity. Incoming sync
 * messages (Full or Patch) are applied to the store, triggering reactive
 * updates in any UI components that read the affected data.
 *
 * ## Merge semantics
 *
 * Follows RFC 7396 JSON Merge Patch:
 * - **Full messages** replace the entire entity at the given ID.
 * - **Patch messages** merge field-by-field into the existing entity.
 * - **Scalar fields**: patch value replaces original (if present).
 * - **Nested maps** (fills): recursive merge with `null` as tombstone.
 * - **Leaf maps** (notes): `null` removes the key, non-null overwrites.
 *
 * ## Usage
 *
 * ```ts
 * import { store, applyMessage, setConnected } from "./store";
 *
 * // Read reactive state
 * console.log(store.orders);      // Record<string, OrderFull>
 * console.log(store.connected);   // boolean
 *
 * // Apply an incoming message
 * applyMessage(msg);
 *
 * // Get snapshot for handshake
 * const entries = getStoreSnapshot();
 * ```
 */

import { createStore, reconcile, produce } from "solid-js/store";
import type { SyncMessage } from "./socket.ts";
import type { OrderFull, OrderPatch, OrderLineFull, OrderLinePatch } from "../generated/sync.ts";

// --- Store shape ---

/**
 * Shape of the global sync store.
 *
 * - `orders`: Record of all known orders keyed by entity UUID string.
 * - `connected`: Whether the Socket.IO connection is currently active.
 */
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

/**
 * Update the connection status in the store.
 *
 * Called by the App component when Socket.IO connects/disconnects.
 *
 * @param connected - Whether the socket is currently connected.
 */
export function setConnected(connected: boolean): void {
  setStore("connected", connected);
}

// --- Merge-patch helpers ---

/**
 * Merge an OrderLinePatch into an OrderLineFull.
 *
 * Each field in the patch replaces the corresponding field in the
 * full value if present; otherwise the original is kept.
 *
 * @param full - The current fill state.
 * @param patch - The sparse update.
 * @returns A new OrderLineFull with the patch applied.
 */
function mergeOrderLinePatch(full: OrderLineFull, patch: OrderLinePatch): OrderLineFull {
  return {
    quantity: patch.quantity ?? full.quantity,
    price: patch.price ?? full.price,
  };
}

/**
 * Merge an OrderPatch into an OrderFull following RFC 7396 semantics.
 *
 * - **Scalar fields**: replaced if present in patch.
 * - **Nested fills map**: each entry is either merged (if it exists),
 *   created (if new), or deleted (if `null` tombstone).
 * - **Leaf notes map**: each entry is overwritten or deleted.
 *
 * @param full - The current order state.
 * @param patch - The sparse update.
 * @returns A new OrderFull with the patch applied.
 */
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

/**
 * Apply an incoming sync message to the reactive store.
 *
 * Dispatches based on `entityTag` and `kind`:
 * - `kind: "full"` -- Replaces the entire entity at the given ID.
 * - `kind: "patch"` -- Merges into existing state. If the entity
 *   doesn't exist, the patch is dropped (server should send Full
 *   for new entities).
 *
 * @param msg - A validated {@link SyncMessage} from the connection layer.
 *
 * @example
 * ```ts
 * connection.onMessage((msg) => {
 *   applyMessage(msg);
 *   // UI components reading store.orders will update reactively
 * });
 * ```
 */
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

/**
 * Extract the current store state as handshake entries.
 *
 * Returns one entry per known entity, suitable for sending to the
 * server on reconnect so it can compute the minimal diff.
 *
 * @returns Array of handshake entries with entityTag, entityId, and payload.
 *
 * @example
 * ```ts
 * const snapshot = getStoreSnapshot();
 * for (const entry of snapshot) {
 *   sendHandshake(conn, entry.entityTag, entry.entityId, entry.payload);
 * }
 * ```
 */
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
