class EventBus {
  #listeners = new Map();

  on(type, listener) {
    const listeners = this.#listeners.get(type) ?? [];
    listeners.push(listener);
    this.#listeners.set(type, listeners);
    return () => this.off(type, listener);
  }

  off(type, listener) {
    const listeners = this.#listeners.get(type);
    this.#listeners.set(
      type,
      listeners?.filter(item => item !== listener) ?? [],
    );
  }

  async emit(type, detail) {
    const listeners = this.#listeners.get(type) ?? [];
    const event = { type, detail, time: Date.now() };
    return Promise.all(
      listeners.map(listener => listener(event)),
    );
  }
}

async function* readPages(start, { signal } = {}) {
  let url = new URL(start);

  while (url) {
    const response = await fetch(url, { signal });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const { items, next } = await response.json();
    yield* items;
    url = next ? new URL(next, url) : null;
  }
}

const bus = new EventBus();
const stop = bus.on("item", ({ detail }) => {
  console.log(detail?.name ?? "unnamed");
});

for await (const item of readPages("/api/items")) {
  await bus.emit("item", item);
}
stop();
