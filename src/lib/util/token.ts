/**
 * Token normalization for Homebrew package identifiers.
 *
 * Homebrew install-analytics (the Trending data source) report tap formulae
 * **fully-qualified** as `user/tap/name`, but the bundled catalog + enrichment
 * are keyed by the **bare** name (`name`). Lookups that receive a qualified
 * token must fall back to the bare name. Bare tokens pass through unchanged.
 *
 * Mirrors the native build's `AppModel.bareToken(_:)`.
 */
export function bareToken(token: string): string {
  const slash = token.lastIndexOf("/");
  return slash >= 0 ? token.slice(slash + 1) : token;
}
