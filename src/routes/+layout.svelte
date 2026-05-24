<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { ui, watchSystemTheme } from "$lib/stores/ui.svelte";
  import { startEnvProbe } from "$lib/stores/env.svelte";
  import { activity } from "$lib/stores/activity.svelte";
  import { services } from "$lib/stores/services.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { github } from "$lib/stores/github.svelte";

  let { children } = $props();

  onMount(() => {
    ui.loadThemeFromStorage();
    // Settings (Phase 12b) — all read with enum/numeric validation so a
    // corrupt or hostile localStorage entry can't poison runtime state.
    ui.loadDefaultSectionFromStorage();
    ui.loadVibrancyMaterialFromStorage();
    ui.loadConfirmDestructiveFromStorage();
    ui.loadActivitySettingsFromStorage();
    ui.loadSidebarCollapsedFromStorage();
    activity.hydrate();
    // Phase 12d — hydrate the persisted settings.json into the renderer
    // so the Network section, the Catalog stale banner, and the cask
    // icon mode all read from one source of truth.
    void settings.load();
    // Prime the services list so the sidebar's "Services" badge can show a
    // count from first paint; the Services tab refreshes again on mount.
    void services.load();
    // Hydrate GitHub sign-in status on app start so the intercept guard
    // in PackageDetail (requireGithubSignIn) sees the real signed-in
    // state from the first paint. Without this, the first click on
    // Star/Watch/File-issue would bounce the user to Settings → GitHub
    // even when they're already authenticated, because the status only
    // hydrated when Settings → GitHub mounted.
    void github.loadStatus();

    // Native macOS menu bridge — Rust emits `menu:about` / `menu:settings`
    // when the user picks those items from the App menu in the system menu
    // bar; we just open the corresponding modal. The Cmd+, accelerator is
    // also bound on the Settings menu item so both surfaces stay in sync
    // with the in-app shortcut already handled in `+page.svelte`.
    let unlistenAbout: UnlistenFn | undefined;
    let unlistenSettings: UnlistenFn | undefined;
    void listen("menu:about", () => { ui.openAbout(); }).then((u) => { unlistenAbout = u; });
    void listen("menu:settings", () => { ui.openSettings(); }).then((u) => { unlistenSettings = u; });

    const unwatch = watchSystemTheme(() => ui.theme);
    const stopProbe = startEnvProbe();
    return () => {
      unwatch();
      stopProbe();
      unlistenAbout?.();
      unlistenSettings?.();
    };
  });
</script>

<!--
  Window dragging in Tauri 2 with titleBarStyle: "Overlay" is wired via the
  `data-tauri-drag-region` attribute on regular DOM elements (Sidebar brand
  area + each panel-head). Tauri's WebView handles click-vs-drag detection
  natively, so interactive children inside drag regions still receive their
  clicks. Avoids the fixed-overlay pattern (which intercepts scroll-wheel
  events at the top of the window).
-->

{@render children()}
