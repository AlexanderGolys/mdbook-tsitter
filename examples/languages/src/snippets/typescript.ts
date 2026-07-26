type Loading = { state: "loading"; id: string };
type Ready<T> = { state: "ready"; value: T };
type Failed = { state: "failed"; error: Error };
type Result<T> = Loading | Ready<T> | Failed;

interface Cache<T extends { id: string }> {
  get(id: string): Promise<T | undefined>;
  put(value: T): Promise<void>;
}

class MemoryCache<T extends { id: string }>
  implements Cache<T> {
  readonly #items = new Map<string, T>();

  async get(id: string): Promise<T | undefined> {
    return this.#items.get(id);
  }

  async put(value: T): Promise<void> {
    this.#items.set(value.id, value);
  }
}

async function* pages<T>(
  url: URL,
  decode: (raw: unknown) => T[],
): AsyncGenerator<T> {
  let next: URL | undefined = url;

  while (next) {
    const response = await fetch(next);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const body: unknown = await response.json();
    for (const item of decode(body)) {
      yield item;
    }

    const link = response.headers.get("x-next");
    next = link ? new URL(link, next) : undefined;
  }
}

function describe<T>(result: Result<T>): string {
  switch (result.state) {
    case "loading":
      return `loading ${result.id}`;
    case "ready":
      return JSON.stringify(result.value);
    case "failed":
      return result.error.message;
  }
}

const config = {
  retries: 3,
  mode: "eager",
} as const satisfies Record<string, unknown>;
