import { fetchRuleCatalog } from "@/lib/api/annotation";
import { resolveRuleCatalog } from "./rule-catalog";
import { fallbackRules } from "./rules-fallback";
import type { RuleCatalogItem } from "./types";

export async function loadRuleCatalog(
  remoteFetcher: () => Promise<RuleCatalogItem[]> = fetchRuleCatalog,
) {
  return resolveRuleCatalog(remoteFetcher, fallbackRules);
}
