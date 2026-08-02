export function safeReturnPath(value: unknown) {
  if (typeof value !== "string") return "/overview";
  try {
    const target = new URL(value, window.location.origin);
    if (target.origin !== window.location.origin) return "/overview";
    const path = `${target.pathname}${target.search}${target.hash}`;
    if (path === "/login" || path === "/setup") return "/overview";
    return path;
  } catch {
    return "/overview";
  }
}
