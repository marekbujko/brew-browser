<script lang="ts">
  import Terminal from "@lucide/svelte/icons/terminal";
  import Copy from "@lucide/svelte/icons/copy";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import Button from "$lib/components/Button.svelte";
  import { openTerminalInstall } from "$lib/api";
  import { env } from "$lib/stores/env.svelte";
  import { toast } from "$lib/stores/toast.svelte";
  import { isMac } from "$lib/util/platform";

  /**
   * Official Homebrew install one-liner from brew.sh — the SAME script
   * installs Linuxbrew (to /home/linuxbrew/.linuxbrew or ~/.linuxbrew).
   * Mirrors the fixed TERMINAL_INSTALL_SCRIPT constant in
   * `src-tauri/src/commands/env.rs` — keep the two in sync if upstream
   * ever changes the incantation.
   */
  const INSTALL_COMMAND =
    '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"';

  let openingTerminal = $state(false);

  // Same clipboard pattern as DeviceFlowModal.copyCode().
  function copyCommand() {
    void navigator.clipboard.writeText(INSTALL_COMMAND).then(
      () => toast.success("Install command copied to clipboard"),
      () => toast.error("Couldn't copy command"),
    );
  }

  async function openTerminal() {
    openingTerminal = true;
    try {
      await openTerminalInstall();
    } catch {
      // osascript failed (e.g. Automation permission denied) — fall back
      // to the clipboard, which is the primary affordance anyway.
      void navigator.clipboard.writeText(INSTALL_COMMAND).then(
        () =>
          toast.info(
            "Couldn't open Terminal",
            "The install command was copied instead — paste it into any terminal.",
          ),
        () => toast.error("Couldn't open Terminal or copy the command"),
      );
    } finally {
      openingTerminal = false;
    }
  }
</script>

<div class="onboarding" role="region" aria-label="Homebrew setup">
  <EmptyState
    title="Let's get Homebrew set up"
    body={isMac
      ? "brew-browser is a friendly face for Homebrew, the package manager for your Mac. Install it once and your library appears here automatically — no relaunch needed."
      : "brew-browser is a friendly face for Homebrew, which runs on Linux too. Install it once and your library appears here automatically — no relaunch needed."}
  >
    {#snippet icon()}<Terminal size={48} />{/snippet}
    {#snippet cta()}
      <div class="steps">
        <ol>
          {#if env.status?.cltFound === false}
            <li>
              Install the Xcode Command Line Tools first: run
              <code class="inline">xcode-select --install</code> in Terminal and follow the prompts.
            </li>
          {/if}
          {#if !isMac}
            <li>
              Install Homebrew's build dependencies first — on Debian/Ubuntu:
              <code class="inline">sudo apt-get install build-essential procps curl file git</code>
              (other distros: see the "Homebrew on Linux" page at docs.brew.sh).
            </li>
          {/if}
          <li>
            Install Homebrew by running this in {isMac ? "Terminal" : "your terminal"}:
            <div class="cmd"><code>{INSTALL_COMMAND}</code></div>
          </li>
        </ol>
        <div class="actions">
          <Button variant="primary" onclick={copyCommand} ariaLabel="Copy install command">
            {#snippet icon()}<Copy size={14} />{/snippet}
            Copy command
          </Button>
          {#if isMac}
            <!-- macOS-only: `open_terminal_install` drives Terminal.app via
                 osascript and is gated server-side too. On Linux there is no
                 single terminal emulator to target — the copy button is the
                 whole affordance. -->
            <Button
              variant="secondary"
              loading={openingTerminal}
              onclick={openTerminal}
              title="Opens Terminal with the install command pre-typed"
            >
              Open Terminal
            </Button>
          {/if}
        </div>
        <p class="waiting" role="status">
          <span class="dot" aria-hidden="true"></span>
          Waiting for Homebrew… the app refreshes itself the moment the install finishes.
        </p>
      </div>
    {/snippet}
  </EmptyState>
</div>

<style>
  /* Fill the main pane so EmptyState's min-height:100% centering works. */
  .onboarding {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    background: var(--color-surface);
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    text-align: left;
    min-width: 0;
  }
  ol {
    margin: 0;
    padding-left: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  li {
    font-size: var(--text-body-sm);
    color: var(--color-text-secondary);
    line-height: var(--lh-normal);
  }
  code.inline {
    font-family: var(--font-mono);
    font-size: var(--text-body-sm);
    color: var(--color-text-primary);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
  }
  .cmd {
    margin-top: var(--space-2);
    padding: var(--space-3);
    background: var(--color-surface-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .cmd code {
    font-family: var(--font-mono);
    font-size: var(--text-body-sm);
    color: var(--color-text-primary);
    white-space: pre-wrap;
    word-break: break-all;
    user-select: all;
  }
  .actions {
    display: flex;
    gap: var(--space-3);
    justify-content: center;
  }
  .waiting {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-size: var(--text-body-sm);
    color: var(--color-text-muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-muted);
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50%      { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot { animation: none; }
  }
</style>
