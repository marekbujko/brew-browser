/**
 * Host-OS detection for the renderer — the single source of truth for
 * platform-aware user-facing copy (e.g. "Reveal in Finder" on macOS vs
 * "Show in file manager" on Linux).
 *
 * Why navigator-based and not `@tauri-apps/plugin-os`: that plugin is not a
 * dependency, and adding one for a handful of label swaps isn't worth the
 * weight. The WebView's `navigator.userAgent` faithfully reflects the host OS
 * — macOS reports "Macintosh"/"Mac OS X", Linux reports "Linux"/"X11". That's
 * more than precise enough for choosing a noun in a button label.
 *
 * Evaluated once at module load: the host OS does not change mid-session.
 */
const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";

/** True when running on macOS. Defaults to true under SSR/no-navigator so the
 *  static build (and any non-WebView render) keeps the macOS wording. */
export const isMac = ua === "" || /Mac|Macintosh|Mac OS X/i.test(ua);

/** True when running on Linux (X11/Wayland WebView). */
export const isLinux = /Linux|X11/i.test(ua) && !isMac;

/** The OS file manager's name, for interpolation into copy. */
export const fileManagerName = isMac ? "Finder" : "file manager";

/** The OS credential store's name, for mid-sentence interpolation. */
export const keyringName = isMac ? "macOS Keychain" : "system keyring";

/** Sentence-leading form of {@link keyringName} ("macOS Keychain" already
 *  starts uppercase; "system keyring" becomes "System keyring"). */
export const keyringNameCapitalized = isMac ? "macOS Keychain" : "System keyring";
