import type { RuleCatalogItem } from "./types";

export async function resolveRuleCatalog(
  fetchRemote: () => Promise<RuleCatalogItem[]>,
  fallback: RuleCatalogItem[],
): Promise<{ source: "remote" | "fallback"; items: RuleCatalogItem[] }> {
  try {
    const remote = await fetchRemote();
    if (remote.length > 0) {
      return { source: "remote", items: remote };
    }

    return { source: "fallback", items: fallback };
  } catch {
    return { source: "fallback", items: fallback };
  }
}
