<script lang="ts">
  /**
   * DeviceFlowModal.svelte — Phase 12e
   *
   * Renders the user-facing half of the GitHub OAuth Device Flow:
   *
   *   ┌─────────────────────────────────────┐
   *   │  Sign in to GitHub                  │
   *   │                                     │
   *   │  1. Open github.com/login/device    │
   *   │  2. Enter this code:                │
   *   │                                     │
   *   │           [ WDJB-MJHT ]             │
   *   │                                     │
   *   │  ⟳ Waiting for authorization…       │
   *   │  [Cancel]                           │
   *   └─────────────────────────────────────┘
   *
   * The polling loop itself lives in `github.svelte.ts` — this
   * component is pure presentation. It mounts when `signinState.kind`
   * is anything other than "idle" and unmounts when it returns to idle.
   *
   * Cancellation: the [Cancel] button calls `github.cancelSignin()`,
   * which aborts the poll loop and flips state back to idle.
   */

  import { onDestroy, untrack } from "svelte";
  import X from "@lucide/svelte/icons/x";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import Loader from "@lucide/svelte/icons/loader-2";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

  import { github } from "$lib/stores/github.svelte";
  import { safeOpenUrl } from "$lib/util/url";
  import { toast } from "$lib/stores/toast.svelte";

  /** True when the modal should be visible. Idle state hides it. */
  let isOpen = $derived(github.signinState.kind !== "idle");

  /** Remaining seconds before the device_code expires (waiting state only). */
  let remainingSeconds = $state<number | null>(null);

  /** Tick once a second to refresh the "expires in" countdown.
      Cleared on unmount. */
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    const s = github.signinState;
    if (s.kind === "waiting") {
      tickHandle = setInterval(() => {
        if (github.signinState.kind === "waiting") {
          const ms = github.signinState.expiresAt - Date.now();
          remainingSeconds = Math.max(0, Math.floor(ms / 1000));
        } else {
          remainingSeconds = null;
        }
      }, 1000);
      // Prime immediately.
      const ms = s.expiresAt - Date.now();
      remainingSeconds = Math.max(0, Math.floor(ms / 1000));
    } else {
      if (tickHandle) clearInterval(tickHandle);
      tickHandle = null;
      remainingSeconds = null;
    }
    return () => {
      if (tickHandle) clearInterval(tickHandle);
      tickHandle = null;
    };
  });

  /** Auto-dismiss the terminal states after a beat so the user sees
      the outcome but the modal doesn't linger.

      `github.status?.username` is wrapped in `untrack` so this effect's
      ONLY reactive dependency is `signinState`. Without it, every
      post-sign-in status hydration (or any later status refresh while
      still in "approved" state) would re-run the effect and queue
      another toast — the symptom was a stack of "Signed in to GitHub"
      toasts up the right edge of the window. */
  $effect(() => {
    const s = github.signinState;
    if (s.kind === "approved") {
      const name = untrack(() => github.status?.username);
      toast.success(name ? `Signed in as @${name}` : "Signed in to GitHub");
      const t = setTimeout(() => github.cancelSignin(), 1500);
      return () => clearTimeout(t);
    }
    if (s.kind === "denied") {
      toast.error("GitHub sign-in denied");
      const t = setTimeout(() => github.cancelSignin(), 2000);
      return () => clearTimeout(t);
    }
    if (s.kind === "expired") {
      toast.error("Sign-in code expired", "Try again.");
      const t = setTimeout(() => github.cancelSignin(), 2000);
      return () => clearTimeout(t);
    }
    // NOTE: `error` state intentionally does NOT toast — the modal renders
    // the message inline (see {:else if signinState.kind === "error"} below),
    // and clicking Sign in again would otherwise spawn a new toast per
    // attempt, stacking them up the right edge of the window.
  });

  function onCancel() {
    github.cancelSignin();
  }

  function copyCode() {
    if (github.signinState.kind !== "waiting") return;
    const code = github.signinState.userCode;
    void navigator.clipboard.writeText(code).then(
      () => toast.success("Code copied to clipboard"),
      () => toast.error("Couldn't copy code"),
    );
  }

  async function openVerification() {
    if (github.signinState.kind !== "waiting") return;
    await safeOpenUrl(github.signinState.verificationUri);
  }

  // Esc to cancel.
  function onKey(e: KeyboardEvent) {
    if (!isOpen) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    }
  }
  $effect(() => {
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
  });
</script>

{#if isOpen}
  <div class="scrim" role="presentation" onclick={onCancel}></div>
  <div class="wrap" role="dialog" aria-modal="true" aria-label="Sign in to GitHub">
    <div class="modal">
      <header>
        <h2>Sign in to GitHub</h2>
        <button class="close" aria-label="Cancel sign in" onclick={onCancel}>
          <X size={16} />
        </button>
      </header>

      <div class="body">
        {#if github.signinState.kind === "starting"}
          <div class="status">
            <Loader size={18} class="spin" />
            <span>Contacting GitHub…</span>
          </div>
        {:else if github.signinState.kind === "waiting"}
          <ol class="steps">
            <li>
              Open
              <button class="link" type="button" onclick={openVerification}>
                {github.signinState.verificationUri}
                <ExternalLink size={12} />
              </button>
              in your browser.
            </li>
            <li>Enter this code:</li>
          </ol>
          <button class="code" type="button" onclick={copyCode} title="Click to copy">
            {github.signinState.userCode}
          </button>
          <p class="hint">Click the code to copy it.</p>

          <div class="status">
            <Loader size={16} class="spin" />
            <span>Waiting for authorization…</span>
          </div>

          {#if remainingSeconds !== null}
            <p class="expires">
              Code expires in {Math.floor(remainingSeconds / 60)}m {remainingSeconds % 60}s.
            </p>
          {/if}
        {:else if github.signinState.kind === "approved"}
          <div class="status status-ok">
            <CircleCheck size={20} />
            <span>Signed in as @{github.status?.username ?? "github user"}.</span>
          </div>
        {:else if github.signinState.kind === "denied"}
          <div class="status status-bad">
            <CircleX size={20} />
            <span>Sign-in denied.</span>
          </div>
        {:else if github.signinState.kind === "expired"}
          <div class="status status-bad">
            <TriangleAlert size={20} />
            <span>Code expired. Please try again.</span>
          </div>
        {:else if github.signinState.kind === "error"}
          <div class="status status-bad">
            <TriangleAlert size={20} />
            <span>{github.signinState.message}</span>
          </div>
        {/if}
      </div>

      <footer>
        <button type="button" class="btn-secondary" onclick={onCancel}>
          {github.signinState.kind === "waiting" || github.signinState.kind === "starting"
            ? "Cancel"
            : "Close"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.4);
    z-index: 95;
    animation: fadeIn var(--motion-duration-base) var(--motion-ease-out);
  }
  .wrap {
    position: fixed;
    inset: 0;
    z-index: 96;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    pointer-events: none;
  }
  .modal {
    pointer-events: auto;
    width: 100%;
    max-width: 440px;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: pop var(--motion-duration-base) var(--motion-ease-out);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  header h2 {
    font-size: var(--text-h2);
    font-weight: var(--fw-semibold);
    color: var(--color-text-primary);
  }
  .close {
    color: var(--color-text-muted);
    padding: 4px;
    border-radius: var(--radius-sm);
  }
  .close:hover {
    background: var(--color-surface-sunken);
    color: var(--color-text-primary);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
  }
  .steps {
    margin: 0;
    padding-left: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    color: var(--color-text-primary);
    font-size: var(--text-body);
    line-height: var(--lh-snug);
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--color-text-link);
    font-size: inherit;
  }
  .link:hover { text-decoration: underline; }

  .code {
    align-self: center;
    font-family: var(--font-mono);
    font-size: 24px;
    letter-spacing: 0.1em;
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    color: var(--color-text-primary);
    cursor: pointer;
    user-select: all;
  }
  .code:hover {
    background: var(--color-surface);
    border-color: var(--color-accent, #b8542a);
  }

  .hint {
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--text-body-sm);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-secondary);
    font-size: var(--text-body);
  }
  .status-ok { color: #58a55c; }
  .status-bad { color: #d24a4a; }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }

  .expires {
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--text-body-sm);
  }

  footer {
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  .btn-secondary {
    padding: 6px 12px;
    border-radius: var(--radius-md);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    color: var(--color-text-primary);
    font-size: var(--text-body-sm);
    font-weight: var(--fw-medium);
    cursor: pointer;
  }
  .btn-secondary:hover {
    background: var(--color-surface-sunken);
  }

  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes pop {
    from { opacity: 0; transform: scale(0.96); }
    to   { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .scrim, .modal { animation: none; }
    :global(.spin) { animation: none; }
  }
</style>
