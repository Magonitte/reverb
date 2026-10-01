type Handler = (payload: unknown) => void;

const handlers = new Map<string, Set<Handler>>();

/** Barramento de eventos do backend falso (equivalente ao `listen`/`emit` do Tauri). */
export const mockBus = {
  on(event: string, handler: Handler): () => void {
    let set = handlers.get(event);
    if (!set) {
      set = new Set();
      handlers.set(event, set);
    }
    const target = set;
    target.add(handler);
    return () => {
      target.delete(handler);
    };
  },
  emit(event: string, payload: unknown): void {
    handlers.get(event)?.forEach((handler) => handler(payload));
  },
  clear(): void {
    handlers.clear();
  },
};
