// The app version and changelog, set at build time by vite.config.ts (`define`) from the
// workspace Cargo.toml and CHANGELOG.md: the UI keeps no copy of either.

import { parseChangelog, type Release } from "./changelog";

declare const __APP_VERSION__: string;
declare const __APP_CHANGELOG__: string;

/** The app version, `[workspace.package] version` of Cargo.toml. */
export const APP_VERSION: string = __APP_VERSION__;

/** The releases of CHANGELOG.md, newest first. */
export const RELEASES: Release[] = parseChangelog(__APP_CHANGELOG__);
