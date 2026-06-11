<script lang="ts">
  import { onMount } from "svelte";
  import Plus from "@lucide/svelte/icons/plus";
  import Archive from "@lucide/svelte/icons/archive";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Upload from "@lucide/svelte/icons/upload";
  import Download from "@lucide/svelte/icons/download";

  import Button from "./Button.svelte";
  import Modal from "./Modal.svelte";
  import DestructiveConfirm from "./DestructiveConfirm.svelte";
  import Input from "./Input.svelte";
  import LoadingState from "./LoadingState.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { brewfiles } from "$lib/stores/brewfiles.svelte";
  import { activity } from "$lib/stores/activity.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { brewfileDump, brewfileInstall, brewfileDelete, brewfileExport, brewfileImport } from "$lib/api";
  import type { BrewfileSummary } from "$lib/types";
  import { isLinux } from "$lib/util/platform";
  import { reportableToastError } from "$lib/util/reportIssue";

  let newLabel = $state("");
  let creating = $state(false);
  let showNewModal = $state(false);
  let toDelete: BrewfileSummary | null = $state(null);
  let toRestore: BrewfileSummary | null = $state(null);

  onMount(() => { brewfiles.load(); });

  function defaultLabel(): string {
    const now = new Date();
    const y = now.getFullYear();
    const m = String(now.getMonth() + 1).padStart(2, "0");
    const d = String(now.getDate()).padStart(2, "0");
    return `snapshot-${y}-${m}-${d}`;
  }

  function openNew() {
    newLabel = defaultLabel();
    showNewModal = true;
  }

  async function doCreate() {
    if (!newLabel.trim()) return;
    creating = true;
    const tmpId = crypto.randomUUID();
    activity.startJob(`Dumping Brewfile: ${newLabel}`, tmpId, `brew bundle dump`);
    ui.openDrawer();
    try {
      const summary = await brewfileDump(newLabel.trim(), (evt) => {
        if (evt.kind === "started" && evt.jobId !== tmpId) {
          const j = activity.jobs.find((j) => j.jobId === tmpId);
          if (j) j.jobId = evt.jobId;
        }
        activity.handleEvent(evt);
      });
      toast.success(`Snapshot saved`, `${summary.counts.formulae + summary.counts.casks} packages`);
      showNewModal = false;
      newLabel = "";
      brewfiles.load();
    } catch (e) {
      reportableToastError("Snapshot failed", e);
    } finally {
      creating = false;
    }
  }

  async function doRestore(b: BrewfileSummary) {
    toRestore = null;
    const tmpId = crypto.randomUUID();
    activity.startJob(`Restoring ${b.label}`, tmpId, `brew bundle install`);
    ui.openDrawer();
    try {
      const result = await brewfileInstall(b.id, (evt) => {
        if (evt.kind === "started" && evt.jobId !== tmpId) {
          const j = activity.jobs.find((j) => j.jobId === tmpId);
          if (j) j.jobId = evt.jobId;
        }
        activity.handleEvent(evt);
      });
      if (result.success) toast.success("Restore complete");
      else toast.error("Restore failed");
    } catch (e) {
      reportableToastError("Restore failed", e);
    }
  }

  async function doDelete(b: BrewfileSummary) {
    toDelete = null;
    try {
      await brewfileDelete(b.id);
      toast.success(`Deleted snapshot "${b.label}"`);
      brewfiles.load();
    } catch (e) {
      reportableToastError("Delete failed", e);
    }
  }

  async function doExport(b: BrewfileSummary) {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const target = await save({
        defaultPath: `${b.label}.Brewfile`,
        filters: [{ name: "Brewfile", extensions: ["Brewfile", "txt", ""] }],
      });
      if (!target) return;
      await brewfileExport(b.id, target);
      toast.success(`Exported "${b.label}"`);
    } catch (e) {
      reportableToastError("Export failed", e);
    }
  }

  async function doImport() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        filters: [{ name: "Brewfile", extensions: ["Brewfile", "txt", ""] }],
      });
      if (!picked || typeof picked !== "string") return;
      const label = picked.split("/").pop() ?? "imported";
      await brewfileImport(picked, label);
      toast.success(`Imported "${label}"`);
      brewfiles.load();
    } catch (e) {
      reportableToastError("Import failed", e);
    }
  }

  function formatDate(s: string): string {
    try { return new Date(s).toLocaleString(); } catch { return s; }
  }
</script>

<section class="snapshots">
  <!-- Pane title ("Snapshots") moved to the window title bar; head
       keeps the Import + New Snapshot primary actions. -->
  <header class="panel-head" data-tauri-drag-region>
    <div class="head-right" data-tauri-drag-region="false">
      <Button size="md" variant="secondary" onclick={doImport}>
        {#snippet icon()}<Upload size={14} />{/snippet}
        Import…
      </Button>
      <Button size="md" variant="primary" onclick={openNew}>
        {#snippet icon()}<Plus size={14} />{/snippet}
        New Snapshot
      </Button>
    </div>
  </header>

  <div class="list-wrap">
    {#if brewfiles.loading}
      <LoadingState rows={4} label="Loading snapshots…" />
    {:else if brewfiles.error}
      <EmptyState title="Couldn't load snapshots" body={brewfiles.error}>
        {#snippet icon()}<Archive size={48} />{/snippet}
        {#snippet cta()}<Button variant="secondary" onclick={() => brewfiles.load()}>Retry</Button>{/snippet}
      </EmptyState>
    {:else if brewfiles.list.length === 0}
      <!-- Inline CTAs intentionally omitted: the same actions live in
           the panel-head's top-right (Import… + New Snapshot), so the
           empty state stays purely informational. The storage path mirrors
           the backend's `resolve_brewfiles_dir` (dirs::data_dir() +
           brew-browser/brewfiles/): ~/Library/Application Support on macOS,
           XDG data home (~/.local/share by default) on Linux. -->
      <EmptyState
        title="No snapshots yet."
        body={isLinux
          ? "Save your current setup so you can restore it on another machine. Snapshots live in ~/.local/share/brew-browser/brewfiles/ — findable outside the app too."
          : "Save your current setup so you can restore it on another Mac. Snapshots live in ~/Library/Application Support/brew-browser/brewfiles/ — findable outside the app too."}
      >
        {#snippet icon()}<Archive size={48} />{/snippet}
      </EmptyState>
    {:else}
      <ul class="cards">
        {#each brewfiles.list as b (b.id)}
          <li class="card">
            <header class="card-head">
              <div>
                <h2>{b.label}</h2>
                <!-- Brewfiles legitimately carry cask lines (a snapshot may
                     come from a Mac) — real cask counts always show. Only
                     the decorative "0 casks" is suppressed on Linux. -->
                <p class="meta">{formatDate(b.createdAt)} · {b.counts.formulae} formulae{#if !isLinux || b.counts.casks > 0} · {b.counts.casks} casks{/if}{#if b.counts.masApps > 0} · {b.counts.masApps} MAS apps{/if}</p>
              </div>
              <div class="actions">
                <Button size="sm" variant="primary" onclick={() => (toRestore = b)}>
                  {#snippet icon()}<RotateCcw size={14} />{/snippet}
                  Restore
                </Button>
                <Button size="sm" variant="secondary" onclick={() => doExport(b)}>
                  {#snippet icon()}<Download size={14} />{/snippet}
                  Export…
                </Button>
                <Button size="sm" variant="ghost" onclick={() => (toDelete = b)} ariaLabel={`Delete ${b.label}`} title="Delete">
                  {#snippet icon()}<Trash2 size={14} />{/snippet}
                  Delete
                </Button>
              </div>
            </header>
            <p class="path text-muted truncate" title={b.path}>{b.path}</p>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<Modal open={showNewModal} title="New Snapshot" defaultFocus="first" onClose={() => (showNewModal = false)}>
  <div class="modal-body">
    <label>
      <span class="lbl">Name</span>
      <Input bind:value={newLabel} placeholder="snapshot-name" />
    </label>
    <p class="hint text-muted">Stored in <code>~/Library/Application Support/brew-browser/brewfiles/</code></p>
  </div>
  {#snippet actions()}
    <Button variant="secondary" onclick={() => (showNewModal = false)}>Cancel</Button>
    <Button variant="primary" loading={creating} onclick={doCreate}>Create</Button>
  {/snippet}
</Modal>

<DestructiveConfirm
  open={!!toDelete}
  title={toDelete ? `Delete snapshot "${toDelete.label}"?` : ""}
  confirmLabel="Delete"
  onCancel={() => (toDelete = null)}
  onConfirm={() => toDelete && doDelete(toDelete)}
>
  <p>The Brewfile will be removed from disk. This cannot be undone.</p>
</DestructiveConfirm>

<DestructiveConfirm
  open={!!toRestore}
  title={toRestore ? `Restore from "${toRestore.label}"?` : ""}
  confirmLabel="Restore"
  confirmVariant="primary"
  onCancel={() => (toRestore = null)}
  onConfirm={() => toRestore && doRestore(toRestore)}
>
  <p>This will install packages from the snapshot. Existing packages are skipped.</p>
</DestructiveConfirm>

<style>
  .snapshots { display: flex; flex-direction: column; min-height: 0; height: 100%; }
  .panel-head {
    display: flex; justify-content: flex-end; align-items: center;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
    gap: var(--space-3);
  }
  .head-right { display: flex; align-items: center; gap: var(--space-2); margin-left: auto; }
  .list-wrap { flex: 1; overflow-y: auto; min-height: 0; padding: var(--space-4); }
  .cards { display: flex; flex-direction: column; gap: var(--space-3); }
  .card {
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    box-shadow: var(--shadow-xs);
    transition: box-shadow var(--motion-duration-fast) var(--motion-ease-out);
  }
  .card:hover { box-shadow: var(--shadow-sm); }
  .card-head { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--space-3); }
  .card h2 { font-size: var(--text-h2); margin-bottom: 2px; }
  .meta { font-size: var(--text-body-sm); color: var(--color-text-secondary); }
  .actions { display: flex; gap: var(--space-2); flex-wrap: wrap; }
  .path { font-size: var(--text-caption); margin-top: var(--space-2); font-family: var(--font-mono); }

  .modal-body { display: flex; flex-direction: column; gap: var(--space-3); }
  .modal-body label { display: flex; flex-direction: column; gap: var(--space-1); }
  .lbl { font-size: var(--text-body-sm); color: var(--color-text-secondary); font-weight: var(--fw-medium); }
  .hint { font-size: var(--text-caption); }
  .hint code { background: var(--color-surface-sunken); padding: 1px 4px; border-radius: var(--radius-sm); }
</style>
