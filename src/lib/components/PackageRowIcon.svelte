<script lang="ts">
  // Shared package icon for every list view + the detail header — the Tauri
  // analog of native's `PackageIcon`/`DetailIcon`. Renders, in priority order:
  //   1. the resolved app icon (casks: Appcasks → homepage-favicon cascade)
  //   2. a kind glyph fallback — `terminal` for formulae (CLI tools, no app
  //      icon), `square-dashed` for casks we tried but couldn't resolve.
  // Formulae never hit the backend (they have no app icon); the terminal glyph
  // is the identity. Casks resolve via the shared `iconCache` (token-keyed,
  // session-memoized, backend disk-cached). `homepage`/`resolveCask` only
  // matter for the favicon fallback when the caller doesn't already carry a
  // resolved `iconSource` (Library does; Discover/Trending don't).
  import Terminal from "@lucide/svelte/icons/terminal";
  import SquareDashed from "@lucide/svelte/icons/square-dashed";
  import { iconCache } from "$lib/stores/iconCache.svelte";
  import { catalog } from "$lib/stores/catalog.svelte";
  import type { IconSource, Package, PackageKind } from "$lib/types";

  interface Props {
    token: string;
    kind: PackageKind;
    /** Known icon source (Library rows carry this from the full Package). */
    iconSource?: IconSource;
    /** Known homepage for the favicon fallback (when iconSource is absent). */
    homepage?: string | null;
    /** When true and neither iconSource nor homepage is given, look the cask
        up in the catalog to recover its homepage (Discover search hits). */
    resolveCask?: boolean;
    size?: number;
  }

  let { token, kind, iconSource, homepage, resolveCask = false, size = 24 }: Props =
    $props();

  // Larger icons (detail header) get a macOS-app-flavoured rounding; small
  // list icons stay subtly rounded. Mirrors native's cornerRadius 14 @ 64pt.
  const radius = $derived(size >= 48 ? 14 : 4);
  const glyphSize = $derived(Math.max(12, Math.round(size * (size >= 48 ? 0.7 : 0.72))));

  let dataUrl = $state<string | null>(null);
  let loaded = $state(false);

  $effect(() => {
    // Formulae: no app icon — the terminal glyph IS the icon. No backend hop.
    if (kind === "formula") {
      dataUrl = null;
      loaded = true;
      return;
    }

    // Cask — peek the session cache synchronously to avoid a microtask.
    const peeked = iconCache.peek(token);
    if (peeked !== undefined) {
      dataUrl = peeked;
      loaded = true;
      return;
    }

    loaded = false;
    dataUrl = null;
    let canceled = false;

    (async () => {
      let src: IconSource | null = iconSource ?? null;
      if (!src) {
        if (homepage) {
          src = { kind: "homepage", homepage };
        } else if (resolveCask) {
          const c = await catalog.lookupCask(token);
          src = c?.homepage ? { kind: "homepage", homepage: c.homepage } : { kind: "none" };
        } else {
          src = { kind: "none" };
        }
      }
      if (canceled) return;
      // getIcon only reads `.name` + `.iconSource`; a minimal object is enough.
      const result = await iconCache.getIcon({ name: token, iconSource: src } as unknown as Package);
      if (canceled) return;
      dataUrl = result;
      loaded = true;
    })();

    return () => {
      canceled = true;
    };
  });
</script>

<span class="pri-slot" style="width: {size}px; height: {size}px" aria-hidden="true">
  {#if dataUrl}
    <img src={dataUrl} alt="" width={size} height={size} class="pri-icon" style="border-radius: {radius}px" />
  {:else if kind === "formula"}
    <span class="pri-glyph"><Terminal size={glyphSize} /></span>
  {:else if loaded}
    <span class="pri-glyph"><SquareDashed size={glyphSize} /></span>
  {/if}
  <!-- cask still resolving: empty slot keeps the name column aligned -->
</span>

<style>
  .pri-slot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    /* designSystem §6: list icons render instant, no fade-in */
  }
  .pri-icon {
    object-fit: contain;
    display: block;
  }
  /* Glyph inherits the row's currentColor (so it tints with selection) and
     just dims via opacity — no hard-coded color to fight the selected state. */
  .pri-glyph {
    display: inline-flex;
    opacity: 0.55;
  }
</style>
