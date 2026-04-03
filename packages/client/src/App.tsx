import { Component, For, Show, createSignal, onMount, onCleanup } from "solid-js";
import { store, applyMessage, setConnected, getStoreSnapshot } from "./store.ts";
import { createConnection, sendCommand, sendHandshake, type SyncConnection } from "./socket.ts";
import type { OrderFull } from "../generated/sync.ts";

const OrderCard: Component<{ id: string; order: OrderFull }> = (props) => {
  const fillEntries = () => Object.entries(props.order.fills);
  const noteEntries = () => Object.entries(props.order.notes);

  return (
    <div class="rounded-lg border border-border bg-card p-4 space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-semibold text-foreground">{props.order.symbol}</h3>
        <span
          class="px-2 py-1 rounded text-xs font-medium"
          classList={{
            "bg-green-900/50 text-green-400": props.order.status === "open",
            "bg-yellow-900/50 text-yellow-400": props.order.status === "partially_filled",
            "bg-blue-900/50 text-blue-400": props.order.status === "filled",
            "bg-red-900/50 text-red-400": props.order.status === "closed",
          }}
        >
          {props.order.status}
        </span>
      </div>
      <div class="grid grid-cols-2 gap-2 text-sm text-muted-foreground">
        <div>Side: <span class="text-foreground">{props.order.side}</span></div>
        <div>Total Qty: <span class="text-foreground">{props.order.totalQuantity}</span></div>
        <div>Filled Qty: <span class="text-foreground">{props.order.filledQuantity}</span></div>
        <div>Avg Price: <span class="text-foreground">{props.order.averagePrice}</span></div>
      </div>
      <Show when={fillEntries().length > 0}>
        <div class="border-t border-border pt-2">
          <h4 class="text-sm font-medium text-muted-foreground mb-1">Fills</h4>
          <For each={fillEntries()}>
            {([fillId, fill]) => (
              <div class="text-xs text-muted-foreground flex justify-between">
                <span>{fillId}</span>
                <span>qty: {fill.quantity} @ {fill.price}</span>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={noteEntries().length > 0}>
        <div class="border-t border-border pt-2">
          <h4 class="text-sm font-medium text-muted-foreground mb-1">Notes</h4>
          <For each={noteEntries()}>
            {([key, value]) => (
              <div class="text-xs text-muted-foreground">
                <span class="text-foreground">{key}:</span> {value}
              </div>
            )}
          </For>
        </div>
      </Show>
      <div class="text-xs text-muted-foreground/50 truncate">ID: {props.id}</div>
    </div>
  );
};

const App: Component = () => {
  const [conn, setConn] = createSignal<SyncConnection | null>(null);
  const [symbol, setSymbol] = createSignal("BTC");
  const [side, setSide] = createSignal("buy");

  const orderEntries = () => Object.entries(store.orders);

  onMount(() => {
    const connection = createConnection();
    setConn(connection);

    connection.socket.on("connect", () => {
      setConnected(true);
      // Handshake: send current state for each known entity
      const snapshot = getStoreSnapshot();
      for (const entry of snapshot) {
        sendHandshake(connection, entry.entityTag, entry.entityId, entry.payload);
      }
    });

    connection.socket.on("disconnect", () => {
      setConnected(false);
    });

    // Process incoming messages via callback
    connection.onMessage((msg) => {
      applyMessage(msg);
    });

    onCleanup(() => {
      connection.disconnect();
    });
  });

  const handlePlaceOrder = () => {
    const c = conn();
    if (c) {
      sendCommand(c, "PlaceOrder", {
        symbol: symbol(),
        side: side(),
      });
    }
  };

  const handleUpdateFirst = () => {
    const c = conn();
    const entries = orderEntries();
    if (c && entries.length > 0) {
      const [firstId] = entries[0]!;
      sendCommand(c, "UpdateOrder", {
        entityId: firstId,
        patch: {
          status: "partially_filled",
          filledQuantity: "500",
        },
      });
    }
  };

  return (
    <div class="min-h-screen bg-background p-6">
      <div class="max-w-4xl mx-auto space-y-6">
        {/* Header */}
        <div class="flex items-center justify-between">
          <h1 class="text-2xl font-bold text-foreground">Sync Protocol PoC</h1>
          <div class="flex items-center gap-2">
            <div
              class="w-2 h-2 rounded-full"
              classList={{
                "bg-green-500": store.connected,
                "bg-red-500": !store.connected,
              }}
            />
            <span class="text-sm text-muted-foreground">
              {store.connected ? "Connected" : "Disconnected"}
            </span>
          </div>
        </div>

        {/* Controls */}
        <div class="flex gap-3 items-end">
          <div class="space-y-1">
            <label class="text-sm text-muted-foreground">Symbol</label>
            <input
              type="text"
              value={symbol()}
              onInput={(e) => setSymbol(e.currentTarget.value)}
              class="block w-32 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          <div class="space-y-1">
            <label class="text-sm text-muted-foreground">Side</label>
            <select
              value={side()}
              onChange={(e) => setSide(e.currentTarget.value)}
              class="block w-32 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="buy">Buy</option>
              <option value="sell">Sell</option>
            </select>
          </div>
          <button
            onClick={handlePlaceOrder}
            class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            Place Order
          </button>
          <button
            onClick={handleUpdateFirst}
            disabled={orderEntries().length === 0}
            class="rounded-md bg-secondary px-4 py-2 text-sm font-medium text-secondary-foreground hover:bg-secondary/80 transition-colors disabled:opacity-50"
          >
            Update First Order
          </button>
        </div>

        {/* Orders grid */}
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <For each={orderEntries()}>
            {([id, order]) => <OrderCard id={id} order={order} />}
          </For>
        </div>

        <Show when={orderEntries().length === 0}>
          <div class="text-center text-muted-foreground py-12">
            No orders yet. Place an order to get started.
          </div>
        </Show>
      </div>
    </div>
  );
};

export default App;
