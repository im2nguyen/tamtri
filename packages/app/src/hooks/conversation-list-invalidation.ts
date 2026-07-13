type Listener = () => void;

const listeners = new Set<Listener>();

export function subscribeConversationListInvalidation(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function invalidateConversationList(): void {
  for (const listener of listeners) {
    listener();
  }
}
