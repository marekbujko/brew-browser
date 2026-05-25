<script lang="ts">
  import { onMount } from "svelte";
  import PackageIcon from "@lucide/svelte/icons/package";
  import ArrowUpCircle from "@lucide/svelte/icons/arrow-up-circle";
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import CheckCircle2 from "@lucide/svelte/icons/check-circle-2";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";

  import Button from "./Button.svelte";
  import Pill from "./Pill.svelte";
  import LoadingState from "./LoadingState.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Star from "@lucide/svelte/icons/star";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import Loader from "@lucide/svelte/icons/loader-2";
  import { packages } from "$lib/stores/packages.svelte";
  import { env } from "$lib/stores/env.svelte";
  import { categories } from "$lib/stores/categories.svelte";
  import { catalog } from "$lib/stores/catalog.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { discover } from "$lib/stores/discover.svelte";
  import { library } from "$lib/stores/library.svelte";
  import { activity } from "$lib/stores/activity.svelte";
  import { github } from "$lib/stores/github.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { brewUpdate, brewUpgrade, diskUsage, diskUsageClearCache, openInFinder } from "$lib/api";
  import UpgradeModal from "./UpgradeModal.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { resolveCategoryIcon } from "$lib/util/categoryIcon";
  import { brewErrorMessage, isBrewError, type DiskUsageReport } from "$lib/types";
  import { reportableToastError } from "$lib/util/reportIssue";

  let disk = $state<DiskUsageReport | null>(null);
  let diskLoading = $state(false);
  let diskError = $state<string | null>(null);

  async function loadDisk(force = false) {
    if (diskLoading) return;
    diskLoading = true;
    diskError = null;
    try {
      if (force) {
        try { await diskUsageClearCache(); } catch { /* best-effort */ }
      }
      disk = await diskUsage();
    } catch (e) {
      diskError = `Disk probe failed: ${isBrewError(e) ? brewErrorMessage(e) : String(e)}`;
    } finally {
      diskLoading = false;
    }
  }

  async function reveal(path: string) {
    try {
      await openInFinder(path);
    } catch (e) {
      reportableToastError("Couldn't reveal in Finder", e);
    }
  }

  onMount(() => {
    if (!packages.list) packages.load();
    categories.ensureLoaded();
    catalog.ensureLoaded();
    loadDisk();
  });

  // ────────────────────────────────────────────────────────────────
  // Refresh handler — full three-step sync:
  //
  //   1. `brew update` (git-fetch on every tap in $HOMEBREW_PREFIX/
  //      Library/Taps/) — makes brew itself aware of new upstream
  //      versions. Streams to the Activity drawer so the user can see
  //      what brew is doing (typical output: "Already up-to-date" or
  //      "Updated 2 taps: homebrew/core, homebrew/cask").
  //   2. Catalog refresh — writes a fresh formula.json + cask.json to
  //      ~/Library/Application Support/brew-browser/catalog/ for our
  //      own Discover / Library / Search views. Doesn't touch brew.
  //   3. Installed list reload (force=true) — re-runs brew info
  //      --installed --json=v2 with the now-up-to-date version
  //      knowledge from step 1, so the outdated flag is accurate.
  //
  // All three are needed for "Refresh" to mean what the user expects:
  // "make every count and version on this page reflect reality right
  // now." Without step 1, brew doesn't know about new releases. Without
  // step 2, the Discover index lags. Without step 3, the Updates card
  // shows stale outdated flags.
  //
  // Each step's failure surfaces a typed toast and short-circuits the
  // rest — a failed brew update typically means a flaky network or a
  // tap permissions problem, both worth fixing before continuing.
  let refreshing = $state(false);
  async function refreshCatalog() {
    if (refreshing) return;
    refreshing = true;

    const tmpId = crypto.randomUUID();
    activity.startJob("Updating Homebrew taps", tmpId, "brew update");
    ui.openDrawer();

    try {
      // Step 1: brew update. Stream into Activity so the user sees
      // each tap getting refreshed (or a clean "Already up-to-date").
      const updateResult = await brewUpdate((evt) => {
        if (evt.kind === "started" && evt.jobId !== tmpId) {
          const j = activity.jobs.find((j) => j.jobId === tmpId);
          if (j) j.jobId = evt.jobId;
        }
        activity.handleEvent(evt);
      });
      if (!updateResult.success) {
        toast.error("brew update finished with errors", "See the Activity drawer.");
        return;
      }

      // Step 2: catalog refresh (our own index from formulae.brew.sh).
      const ok = await catalog.refresh();
      if (!ok) {
        if (catalog.refreshError) {
          toast.error("Catalog refresh failed", catalog.refreshError);
        }
        return;
      }

      // Step 3: reload installed list with force=true so brew_list
      // re-runs `brew info --installed` and the outdated flags reflect
      // the fresh upstream knowledge from step 1. Best-effort — the
      // success toast below covers the user-visible outcome.
      await packages.load(true).catch(() => {});

      toast.success("Refreshed", "brew taps + catalog + installed list all current");
    } catch (e) {
      reportableToastError("Refresh failed", e);
    } finally {
      refreshing = false;
    }
  }

  // ────────────────────────────────────────────────────────────────
  // Phase 12f — personal-stats card
  //
  // Only meaningful when (a) the user is signed in to GitHub, (b) the
  // Settings → GitHub toggle is on (the IPC short-circuits otherwise),
  // and (c) paranoid mode is off. The store's `batchIsStarred` does
  // the bounded fan-out via a 50-permit semaphore; we kick it off
  // once the package list is loaded so we have homepages to probe.

  /** Installed packages that resolve to a GitHub repo via any of their
      URL fields. Reads the backend-pre-resolved `githubHomepage` so the
      count includes packages with non-GitHub homepages but GitHub-hosted
      `urls.stable.url` (formula) or `url` (cask). Canonical
      `https://github.com/<o>/<r>` form, ready for `batchIsStarred`. */
  let installedGithubHomepages = $derived.by<string[]>(() => {
    if (!packages.all || packages.all.length === 0) return [];
    return packages.all
      .map((p) => p.githubHomepage)
      .filter((hp): hp is string => hp !== null && hp !== undefined);
  });

  let personalStatsEligible = $derived(
    !!github.status?.signedIn &&
      settings.effective.githubEnabled &&
      !settings.effective.paranoidMode,
  );

  let personalStatsLoading = $derived(github.starredBatchLoading);

  /** Trigger the batch probe once we have homepages + the user is signed
      in. Re-runs whenever the eligible homepage list grows. The store
      caches per homepage so subsequent re-renders don't re-fetch. */
  $effect(() => {
    if (!personalStatsEligible) return;
    if (installedGithubHomepages.length === 0) return;
    void github.batchIsStarred(installedGithubHomepages);
  });

  /** Count of homepages whose cached outcome is exactly `true`. */
  let personalStarredCount = $derived.by(() => {
    let n = 0;
    for (const hp of installedGithubHomepages) {
      if (github.starredCache.get(hp) === true) n += 1;
    }
    return n;
  });

  let personalGithubTotal = $derived(installedGithubHomepages.length);

  function fmtBytes(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB`;
    if (b < 1024 ** 3) return `${(b / 1024 ** 2).toFixed(1)} MB`;
    return `${(b / 1024 ** 3).toFixed(2)} GB`;
  }

  function fmt(n: number): string {
    return n.toLocaleString();
  }

  function openPackage(name: string, kind: "formula" | "cask") {
    ui.selectPackage(name, kind);
  }

  /** Jump to Library, pre-select the Outdated filter. Used by the Updates card. */
  function goToOutdatedLibrary() {
    library.setFilter("outdated");
    ui.setSection("library");
  }

  /** Counts derived from the loaded package list. All zero until `load` completes. */
  let counts = $derived.by(() => {
    const all = packages.all;
    const outdated = packages.outdated.length;
    const formulae = packages.formulae.length;
    const casks = packages.casks.length;
    const pinned = all.filter((p) => p.pinned).length;
    const onRequest = all.filter((p) => p.installedOnRequest).length;
    const asDependency = all.filter((p) => p.installedAsDependency && !p.installedOnRequest).length;
    return {
      total: all.length,
      formulae,
      casks,
      outdated,
      pinned,
      onRequest,
      asDependency,
    };
  });

  /** Bar-chart split: percentage of formulae vs casks (display only). */
  let split = $derived.by(() => {
    if (counts.total === 0) return { formulaPct: 0, caskPct: 0 };
    const fp = Math.round((counts.formulae / counts.total) * 100);
    return { formulaPct: fp, caskPct: 100 - fp };
  });

  /**
   * Top-N categories among installed packages, with counts. We weight by single
   * membership (each package contributes 1 to each category it's tagged with;
   * a package in 3 categories adds to all 3). Uncategorized is collapsed into
   * the "Other" remainder so the bar chart focuses on signal.
   */
  let topCategories = $derived.by<
    Array<{ slug: string; label: string; icon: string; count: number }>
  >(() => {
    if (!categories.data || packages.all.length === 0) return [];
    const counts: Record<string, number> = {};
    for (const p of packages.all) {
      const cats = categories.categoriesOf(p.name, p.kind);
      for (const slug of cats) {
        if (slug === "uncategorized") continue;
        counts[slug] = (counts[slug] ?? 0) + 1;
      }
    }
    const arr = Object.entries(counts)
      .map(([slug, count]) => ({
        slug,
        label: categories.labelOf(slug),
        icon: categories.data?.categories[slug]?.icon ?? "HelpCircle",
        count,
      }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 8);
    return arr;
  });

  let topCategoryMax = $derived(topCategories[0]?.count ?? 1);

  /** Donut palette — 9 visually distinct hues that work on both light and dark
      surfaces. Last slot is the muted "Other" color. */
  const DONUT_PALETTE = [
    "#5b8def", // blue
    "#e07a5f", // coral
    "#81b29a", // sage
    "#f2cc8f", // amber
    "#a78bfa", // violet
    "#ec4899", // pink
    "#10b981", // emerald
    "#f97316", // orange
    "#64748b", // slate (used for "Other")
  ];
  const DONUT_RADIUS = 46;
  const DONUT_CIRC = 2 * Math.PI * DONUT_RADIUS;

  /**
   * Donut segments derived from category memberships. Top 8 categories shown
   * individually; everything else collapses into a single "Other" slice. The
   * percentages add to 100% across all category memberships (a single package
   * in multiple categories contributes to each — same model as the legend).
   */
  let categorySegments = $derived.by<
    Array<{
      slug: string;
      label: string;
      icon: string;
      count: number;
      pct: number;
      startPct: number;
      color: string;
    }>
  >(() => {
    if (!categories.data || packages.all.length === 0) return [];
    const tally: Record<string, number> = {};
    for (const p of packages.all) {
      const cats = categories.categoriesOf(p.name, p.kind);
      for (const slug of cats) {
        tally[slug] = (tally[slug] ?? 0) + 1;
      }
    }
    const sortedAll = Object.entries(tally).sort((a, b) => b[1] - a[1]);
    const top = sortedAll.slice(0, 8);
    const otherCount = sortedAll.slice(8).reduce((sum, [, c]) => sum + c, 0);
    const total = sortedAll.reduce((sum, [, c]) => sum + c, 0);
    if (total === 0) return [];

    const base = top.map(([slug, count], idx) => ({
      slug,
      label: categories.labelOf(slug),
      icon: categories.data?.categories[slug]?.icon ?? "HelpCircle",
      count,
      color: DONUT_PALETTE[idx % DONUT_PALETTE.length],
    }));
    if (otherCount > 0) {
      base.push({
        slug: "__other__",
        label: "Other",
        icon: "HelpCircle",
        count: otherCount,
        color: DONUT_PALETTE[8],
      });
    }
    let cum = 0;
    return base.map((s) => {
      const pct = (s.count / total) * 100;
      const seg = { ...s, pct, startPct: cum };
      cum += pct;
      return seg;
    });
  });

  function jumpToCategory(slug: string) {
    if (slug === "__other__") {
      ui.setSection("discover");
      return;
    }
    // setSection FIRST: it clears chip filters on any real section change,
    // so selectOnly must come after. See ui.setSection().
    ui.setSection("discover");
    discover.selectOnly(slug);
  }

  /** First 5 outdated packages, sorted alphabetically for stable display. */
  let outdatedPreview = $derived.by(() =>
    [...packages.outdated].sort((a, b) => a.name.localeCompare(b.name)).slice(0, 5),
  );

  let upgradeAllRunning = $state(false);
  let upgradeModalOpen = $state(false);

  async function upgradeAll() {
    if (upgradeAllRunning) return;
    upgradeAllRunning = true;
    const tmpId = crypto.randomUUID();
    activity.startJob(`Upgrading all packages`, tmpId, "brew upgrade");
    ui.openDrawer();
    try {
      const result = await brewUpgrade(null, (evt) => {
        if (evt.kind === "started" && evt.jobId !== tmpId) {
          const j = activity.jobs.find((j) => j.jobId === tmpId);
          if (j) j.jobId = evt.jobId;
        }
        activity.handleEvent(evt);
      });
      if (result.success) {
        toast.success(`Upgraded ${counts.outdated} packages`);
        packages.load(true);
      } else {
        toast.error("Upgrade-all finished with errors");
      }
    } catch (e) {
      reportableToastError("Upgrade-all failed", e);
    } finally {
      upgradeAllRunning = false;
    }
  }
</script>

<section class="dashboard">
  <!-- Pane title moved to the window title bar (+page.svelte).
       Dashboard has no secondary header actions, so the panel-head
       is dropped entirely. -->

  <div class="body">
    {#if packages.loading && !packages.list}
      <LoadingState rows={6} label="Loading your packages…" />
    {:else if packages.error}
      <div class="error-card">
        <AlertCircle size={20} />
        <div>
          <strong>Couldn't load packages.</strong>
          <p class="text-muted">{packages.error}</p>
        </div>
        <Button variant="secondary" size="sm" onclick={() => packages.load(true)}>Retry</Button>
      </div>
    {:else}
      <!-- Hero stats row -->
      <div class="hero">
        <button class="stat" onclick={() => ui.setSection("library")} title="Open Library">
          <span class="stat-icon"><PackageIcon size={20} /></span>
          <span class="stat-value">{fmt(counts.total)}</span>
          <span class="stat-label">installed</span>
        </button>

        <button
          class="stat stat--accent"
          class:dim={counts.outdated === 0}
          onclick={goToOutdatedLibrary}
          title={counts.outdated > 0 ? "Open Library, filter outdated" : "Everything is up to date"}
          disabled={counts.outdated === 0}
        >
          <span class="stat-icon">
            {#if counts.outdated > 0}
              <ArrowUpCircle size={20} />
            {:else}
              <CheckCircle2 size={20} />
            {/if}
          </span>
          <span class="stat-value">{counts.outdated === 0 ? "All current" : fmt(counts.outdated)}</span>
          <span class="stat-label">{counts.outdated === 0 ? "" : "updates available"}</span>
        </button>

        <div class="stat" title={env.summary}>
          <span class="stat-icon"><HardDrive size={20} /></span>
          <span class="stat-value">{env.report?.version ?? "—"}</span>
          <span class="stat-label">{env.report?.prefix ?? "Homebrew"}</span>
        </div>
      </div>

      <!-- Phase 12a — catalog freshness strip. Inline below the hero so
           the relationship to "version of brew" stays visible. Goes amber
           when the active catalog is older than the user's stale-banner
           threshold (default 14 days). -->
      <div
        class="catalog-line"
        class:catalog-line--stale={catalog.summary && catalog.isStale}
        aria-live="polite"
      >
        <span class="catalog-text">
          Catalog: <strong>{catalog.daysOldLabel}</strong>
          {#if catalog.summary}
            <span class="text-muted catalog-source">({catalog.summary.source})</span>
          {/if}
        </span>
        <button
          type="button"
          class="catalog-refresh"
          onclick={refreshCatalog}
          disabled={refreshing}
          title="Run brew update, refresh the catalog from formulae.brew.sh, and reload your installed list"
        >
          {#if refreshing}
            <Loader size={12} class="spin-slow" />
            <span>Refreshing…</span>
          {:else if catalog.summary && catalog.isStale}
            <RefreshCw size={12} />
            <span>Refresh from brew.sh →</span>
          {:else}
            <RefreshCw size={12} />
            <span>Refresh</span>
          {/if}
        </button>
      </div>

      <!-- Updates panel -->
      {#if counts.outdated > 0}
        <section class="card">
          <div class="card-head">
            <button
              type="button"
              class="card-title-link"
              onclick={goToOutdatedLibrary}
              title="Open Library, filter outdated"
            >
              <h2>Updates available</h2>
              <span class="card-title-chevron" aria-hidden="true">→</span>
            </button>
            <div class="updates-actions">
              <Button
                variant="secondary"
                size="sm"
                onclick={() => (upgradeModalOpen = true)}
                disabled={upgradeAllRunning}
                title="Pick which packages to upgrade"
              >
                Choose…
              </Button>
              <Button variant="primary" size="sm" onclick={upgradeAll} disabled={upgradeAllRunning}>
                {#snippet icon()}<ArrowUpCircle size={14} />{/snippet}
                {upgradeAllRunning ? "Upgrading…" : `Upgrade all (${counts.outdated})`}
              </Button>
            </div>
          </div>
          <ul class="outdated-list">
            {#each outdatedPreview as p (p.fullName + p.kind)}
              {@const isSelected = ui.selectedPackage?.name === p.name && ui.selectedPackage?.kind === p.kind}
              <li>
                <button
                  class="outdated-row"
                  class:selected={isSelected}
                  aria-current={isSelected ? "true" : undefined}
                  onclick={() => openPackage(p.name, p.kind)}
                >
                  <span class="o-name truncate">{p.name}</span>
                  <span class="o-kind"><Pill tone={p.kind === "formula" ? "formula" : "cask"}>{p.kind}</Pill></span>
                  <span class="o-version mono text-muted">
                    {p.installedVersion ?? "?"} → {p.stableVersion ?? "?"}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
          {#if counts.outdated > outdatedPreview.length}
            <div class="card-foot">
              <button class="link" onclick={goToOutdatedLibrary}>
                + {counts.outdated - outdatedPreview.length} more in Library →
              </button>
            </div>
          {/if}
        </section>
      {/if}

      <!-- Composition -->
      <section class="card">
        <div class="card-head">
          <h2>Composition</h2>
        </div>
        <div class="split">
          <div class="split-bar" role="img" aria-label={`${counts.formulae} formulae and ${counts.casks} casks`}>
            <span class="seg seg--formula" style="width: {split.formulaPct}%"></span>
            <span class="seg seg--cask" style="width: {split.caskPct}%"></span>
          </div>
          <div class="split-legend">
            <span class="legend">
              <span class="swatch swatch--formula"></span>
              <strong>{fmt(counts.formulae)}</strong> formulae
            </span>
            <span class="legend">
              <span class="swatch swatch--cask"></span>
              <strong>{fmt(counts.casks)}</strong> casks
            </span>
          </div>
          <div class="meta-row">
            {#if counts.onRequest > 0}
              <span class="meta-pill">{fmt(counts.onRequest)} on request</span>
            {/if}
            {#if counts.asDependency > 0}
              <span class="meta-pill">{fmt(counts.asDependency)} as dependency</span>
            {/if}
            {#if counts.pinned > 0}
              <span class="meta-pill">{fmt(counts.pinned)} pinned</span>
            {/if}
          </div>
        </div>
      </section>

      <!-- Phase 12f — GitHub personal-stats card. Only when signed in,
           toggle enabled, and paranoid mode off. Hidden entirely
           otherwise so signed-out users see nothing about it. -->
      {#if personalStatsEligible}
        <section class="card">
          <div class="card-head">
            <h2><span class="gh-card-icon"><GitBranch size={16} /></span> GitHub</h2>
            {#if github.status?.username}
              <span class="text-muted gh-handle">@{github.status.username}</span>
            {/if}
          </div>
          <div class="gh-card-body">
            {#if personalGithubTotal === 0}
              <p class="text-muted">
                None of your installed packages have a GitHub homepage.
              </p>
            {:else if personalStatsLoading && personalStarredCount === 0}
              <div class="gh-loading">
                <Loader size={14} class="spin-slow" />
                <span>Checking which of your {personalGithubTotal} packages you've starred…</span>
              </div>
            {:else}
              <p class="gh-line">
                <Star size={14} class="gh-line-icon" fill="currentColor" />
                <span>
                  You've starred
                  <strong>{personalStarredCount}</strong>
                  of
                  <strong>{personalGithubTotal}</strong>
                  installed packages with GitHub homepages.
                </span>
              </p>
              {#if personalStatsLoading}
                <p class="gh-line-sub">
                  <Loader size={12} class="spin-slow" />
                  <span>Refreshing…</span>
                </p>
              {/if}
            {/if}
          </div>
        </section>
      {/if}

      <!-- Top categories — donut. Phase 13: hidden when the AI Features
           toggle is off (categories are LLM-generated). -->
      {#if categories.visible && categorySegments.length > 0}
        <section class="card">
          <div class="card-head">
            <h2>Top categories in your library</h2>
          </div>
          <div class="donut-wrap">
            <svg viewBox="0 0 120 120" class="donut" role="img" aria-label="Category breakdown">
              <circle cx="60" cy="60" r={DONUT_RADIUS} class="donut-track" />
              {#each categorySegments as s (s.slug)}
                <circle
                  cx="60"
                  cy="60"
                  r={DONUT_RADIUS}
                  fill="none"
                  stroke={s.color}
                  stroke-width="20"
                  stroke-dasharray="{(s.pct / 100) * DONUT_CIRC} {DONUT_CIRC}"
                  stroke-dashoffset="{-(s.startPct / 100) * DONUT_CIRC}"
                  transform="rotate(-90 60 60)"
                />
              {/each}
              <text x="60" y="58" text-anchor="middle" class="donut-total">{fmt(counts.total)}</text>
              <text x="60" y="74" text-anchor="middle" class="donut-label">installed</text>
            </svg>
            <ul class="donut-legend">
              {#each categorySegments as s (s.slug)}
                {@const Icon = resolveCategoryIcon(s.icon)}
                <li>
                  <button
                    class="legend-row"
                    onclick={() => jumpToCategory(s.slug)}
                    title={s.slug === "__other__" ? "Browse all in Discover" : `Browse ${s.label} in Discover`}
                  >
                    <span class="legend-dot" style="background: {s.color}"></span>
                    <span class="legend-icon"><Icon size={12} /></span>
                    <span class="legend-label truncate">{s.label}</span>
                    <span class="legend-count">{fmt(s.count)}</span>
                    <span class="legend-pct text-muted">{s.pct.toFixed(1)}%</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>
        </section>
      {/if}

      <!-- Storage -->
      <section class="card">
        <div class="card-head">
          <h2>Storage</h2>
          <div class="head-right">
            {#if disk}
              <span class="text-muted total">{fmtBytes(disk.totalBytes)} total</span>
            {/if}
            <Button
              size="sm"
              variant="ghost"
              onclick={() => loadDisk(true)}
              ariaLabel="Refresh disk usage"
              title="Refresh"
              disabled={diskLoading}
            >
              {#snippet icon()}<RefreshCw size={14} />{/snippet}
              Refresh
            </Button>
          </div>
        </div>
        {#if diskLoading && !disk}
          <LoadingState rows={4} label={`Measuring disk usage… (du -sk on ${env.report?.prefix ?? "Homebrew"})`} />
        {:else if diskError && !disk}
          <div class="storage-error">
            <AlertCircle size={16} />
            <span>{diskError}</span>
          </div>
        {:else if disk}
          <ul class="storage-list">
            {#each disk.entries as e (e.path)}
              <li>
                <div class="storage-row">
                  <span class="s-label">{e.label}</span>
                  <span class="s-path text-muted truncate" title={e.path}>{e.path}</span>
                  <span class="s-bytes mono">
                    {#if e.error}
                      <span class="s-bytes-err" title={e.error}>—</span>
                    {:else if !e.exists}
                      <span class="text-muted">not present</span>
                    {:else}
                      {fmtBytes(e.bytes)}
                    {/if}
                  </span>
                  <button
                    class="s-open"
                    onclick={() => reveal(e.path)}
                    disabled={!e.exists}
                    title={e.exists ? `Reveal ${e.path} in Finder` : "Path doesn't exist"}
                    aria-label={`Reveal ${e.label} in Finder`}
                  >
                    <FolderOpen size={14} />
                  </button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>
</section>

<UpgradeModal
  open={upgradeModalOpen}
  onClose={() => (upgradeModalOpen = false)}
/>

<style>
  .dashboard { display: flex; flex-direction: column; min-height: 0; height: 100%; }

  .body {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  /* Flex children default to flex-shrink: 1, which causes the cards to be
     squashed to fit the viewport instead of overflowing the scroll container.
     Combined with `.card { overflow: hidden }`, that turned into vertical
     clipping. Pin shrink to 0 so cards keep their natural height and the
     overflow becomes a real scroll. */
  .body > * { flex-shrink: 0; }

  /* ─── Hero stats ──────────────────────────────────────── */
  .hero {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-3);
  }
  .stat {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    gap: 4px var(--space-3);
    align-items: center;
    padding: var(--space-4);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    text-align: left;
    color: var(--color-text-primary);
    cursor: pointer;
    transition: background 0.12s ease, border-color 0.12s ease, transform 0.12s ease;
  }
  .stat:hover:not(:disabled) {
    background: var(--color-surface);
    border-color: var(--color-accent);
    transform: translateY(-1px);
  }
  .stat:disabled { cursor: default; }
  .stat.dim { opacity: 0.75; }
  .stat-icon {
    grid-row: 1 / span 2;
    color: var(--color-accent);
    display: inline-flex;
  }
  .stat--accent .stat-icon { color: var(--color-brand); }
  .stat-value {
    font-size: 1.4rem;
    font-weight: var(--fw-semibold);
    line-height: 1.1;
  }
  .stat-label {
    font-size: var(--text-body-sm);
    color: var(--color-text-secondary);
  }

  /* ─── Catalog freshness line (Phase 12a) ──────────────── */
  .catalog-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    margin-top: calc(-1 * var(--space-2));
    font-size: var(--text-body-sm);
    color: var(--color-text-secondary);
  }
  .catalog-line--stale {
    color: var(--color-warning-strong);
  }
  .catalog-text strong {
    color: var(--color-text-primary);
    font-weight: var(--fw-medium);
  }
  .catalog-line--stale .catalog-text strong {
    color: var(--color-warning-strong);
  }
  .catalog-source {
    font-size: var(--text-caption);
    margin-left: 4px;
  }
  .catalog-refresh {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-link);
    font-size: var(--text-body-sm);
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }
  .catalog-refresh:hover:not(:disabled) {
    background: var(--color-surface-sunken);
    text-decoration: underline;
  }
  .catalog-refresh:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .catalog-line--stale .catalog-refresh {
    color: var(--color-warning-strong);
    font-weight: var(--fw-medium);
  }

  /* ─── Card ────────────────────────────────────────────── */
  .card {
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  .card-head h2 {
    font-size: var(--text-h3, 1rem);
    font-weight: var(--fw-semibold);
    margin: 0;
  }
  /* Right-side cluster for the Updates card's two upgrade actions
     ("Choose…" + "Upgrade all"). Tight gap keeps them paired visually. */
  .updates-actions {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .card-title-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--color-text-primary);
    padding: 0;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: color 0.12s ease;
  }
  .card-title-link:hover { color: var(--color-text-link); }
  .card-title-link:hover .card-title-chevron { transform: translateX(2px); }
  .card-title-chevron {
    color: var(--color-text-muted);
    font-size: var(--text-body);
    transition: transform 0.12s ease, color 0.12s ease;
  }
  .card-title-link:hover .card-title-chevron { color: var(--color-text-link); }
  .card-foot {
    padding: var(--space-2) var(--space-4);
    border-top: 1px solid var(--color-border);
  }
  .link {
    background: transparent;
    color: var(--color-text-link);
    font-size: var(--text-body-sm);
    cursor: pointer;
  }
  .link:hover { text-decoration: underline; }

  /* ─── Updates list ────────────────────────────────────── */
  .outdated-list { display: flex; flex-direction: column; }
  .outdated-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 80px 1fr;
    gap: var(--space-3);
    align-items: center;
    width: 100%;
    padding: var(--space-2) var(--space-4);
    text-align: left;
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    cursor: pointer;
    transition: background 0.12s ease;
  }
  .outdated-row:hover { background: var(--color-surface-sunken); }
  .outdated-row.selected {
    background: var(--color-selection-strong);
    color: var(--color-text-inverse);
  }
  .outdated-row.selected .o-version { color: inherit; }
  .outdated-list li:last-child .outdated-row { border-bottom: none; }
  .o-name { font-weight: var(--fw-medium); }
  .o-version { font-size: var(--text-body-sm); }

  /* ─── Split ───────────────────────────────────────────── */
  .split { padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-3); }
  .split-bar {
    display: flex;
    height: 12px;
    border-radius: var(--radius-full);
    overflow: hidden;
    background: var(--color-surface-sunken);
  }
  .seg { display: block; height: 100%; }
  .seg--formula { background: var(--color-info, #4a90e2); }
  .seg--cask { background: var(--color-brand, #f59e0b); }
  .split-legend { display: flex; gap: var(--space-4); flex-wrap: wrap; font-size: var(--text-body-sm); color: var(--color-text-secondary); }
  .legend { display: inline-flex; align-items: center; gap: 6px; }
  .legend strong { color: var(--color-text-primary); }
  .swatch { width: 10px; height: 10px; border-radius: 2px; }
  .swatch--formula { background: var(--color-info, #4a90e2); }
  .swatch--cask { background: var(--color-brand, #f59e0b); }
  .meta-row { display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .meta-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-2);
    height: 20px;
    border-radius: var(--radius-full);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    color: var(--color-text-secondary);
    font-size: var(--text-caption);
    font-weight: var(--fw-medium);
  }

  /* ─── Donut chart + legend ────────────────────────────── */
  .donut-wrap {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-4);
    align-items: center;
    padding: var(--space-4);
  }
  .donut { width: 180px; height: 180px; flex-shrink: 0; }
  .donut-track {
    fill: none;
    stroke: var(--color-surface-sunken);
    stroke-width: 20;
  }
  .donut-total {
    font-size: 18px;
    font-weight: var(--fw-semibold);
    fill: var(--color-text-primary);
  }
  .donut-label {
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    fill: var(--color-text-muted);
  }

  .donut-legend { display: flex; flex-direction: column; gap: 4px; }
  .legend-row {
    display: grid;
    grid-template-columns: 8px 14px minmax(0, 1fr) 50px 56px;
    gap: var(--space-2);
    align-items: center;
    width: 100%;
    padding: 4px var(--space-2);
    border-radius: var(--radius-sm);
    text-align: left;
    color: var(--color-text-primary);
    cursor: pointer;
    font-size: var(--text-body-sm);
    transition: background 0.12s ease;
  }
  .legend-row:hover { background: var(--color-surface-sunken); }
  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .legend-icon { color: var(--color-text-secondary); display: inline-flex; }
  .legend-count {
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .legend-pct {
    font-variant-numeric: tabular-nums;
    font-size: var(--text-caption);
    text-align: right;
  }
  @media (max-width: 720px) {
    .donut-wrap { grid-template-columns: 1fr; justify-items: center; }
  }

  /* ─── Storage card ────────────────────────────────────── */
  .head-right { display: flex; align-items: center; gap: var(--space-3); }
  .total { font-size: var(--text-body-sm); }
  .storage-list { display: flex; flex-direction: column; }
  .storage-row {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr) 100px 32px;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  .storage-list li:last-child .storage-row { border-bottom: none; }
  .s-label { font-weight: var(--fw-medium); font-size: var(--text-body-sm); }
  .s-path { font-family: var(--font-mono); font-size: var(--text-caption); }
  .s-bytes { font-size: var(--text-body-sm); text-align: right; }
  .s-bytes-err { color: var(--color-warning-strong); }
  .s-open {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }
  .s-open:not(:disabled):hover {
    background: var(--color-surface-sunken);
    color: var(--color-text-primary);
  }
  .s-open:disabled { opacity: 0.35; cursor: default; }
  .storage-error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    color: var(--color-text-secondary);
    font-size: var(--text-body-sm);
  }

  /* ─── GitHub personal-stats card (Phase 12f) ──────────── */
  .gh-card-icon {
    display: inline-flex;
    align-items: center;
    vertical-align: middle;
    margin-right: 4px;
    color: var(--color-text-secondary);
  }
  .gh-handle {
    font-family: var(--font-mono);
    font-size: var(--text-mono);
  }
  .gh-card-body {
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .gh-line {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-primary);
    font-size: var(--text-body);
  }
  .gh-line strong {
    font-variant-numeric: tabular-nums;
    color: var(--color-brand);
  }
  .gh-line :global(.gh-line-icon) {
    color: var(--color-brand);
    flex: none;
  }
  .gh-line-sub {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--color-text-muted);
    font-size: var(--text-body-sm);
  }
  .gh-loading {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-muted);
    font-size: var(--text-body);
  }

  /* ─── Error card ──────────────────────────────────────── */
  .error-card {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-3) var(--space-4);
    background: var(--color-danger-subtle, rgba(239, 68, 68, 0.08));
    border: 1px solid var(--color-danger, #ef4444);
    border-radius: var(--radius-md);
    color: var(--color-text-primary);
  }
</style>
