const KEY = "spain-helper-session-id";

export function sessionId() {
  const existing = localStorage.getItem(KEY);
  if (existing) return existing;
  const value = crypto.randomUUID();
  localStorage.setItem(KEY, value);
  return value;
}
