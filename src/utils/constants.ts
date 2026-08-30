import type { TBundleHash } from "./types";

export function discordUrl(userId: string): string {
    return `https://discord.com/users/${userId}`;
}

export function nameMCUrl(UUID: string): string {
    return `https://namemc.com/profile/${UUID}`;
}
export function githubProfileUrl(username: string): string {
    return `https://github.com/${username}`;
}

export function steamProfileUrl(userId: string): string {
    return `https://steamcommunity.com/id/${userId}`;
}

export function lastFMProfileUrl(username: string): string {
    return `https://last.fm/user/${username}`;
}

export function fndbProfileUrl(username: string): string {
    return `https://fortnitedb.com/profile/${username}`;
}

export const NBSP = "\u00A0";

export const EM_DASH = "\u2014";

export const REPLACEMENT_CHARACTER = "\uFFFD";

export const DISCORD_ID = "521819891141967883";
export const MC_UUID = "b7c4f5b1-762f-41ea-b6b4-45aba74198e5";
export const GITHUB_USERNAME = "sadan4";
export const STEAM_USERNAME = "sadan4";
export const LASTFM_USERNAME = "sadan4";
export const EPIC_USERNAME = "sadan4";

export const GITHUB_PROFILE_URL = /* @__PURE__ */ githubProfileUrl(GITHUB_USERNAME);
export const GITHUB_REPO_URL = /* @__PURE__ */ `${GITHUB_PROFILE_URL}/sadan.zip`;
export const GITHUB_REPO_CREATE_ISSUE_URL = /* @__PURE__ */ `${GITHUB_REPO_URL}/issues/new`;

const IS_SERVER_LOCAL = false;
// oxlint-disable-next-line typescript/no-unnecessary-condition
const SERVER_BASE_URL = IS_SERVER_LOCAL ? "http://localhost:8484" : "https://s-d-br.sadan.zip";

export function BUNDLE_TARBALL_URL(buildHash: TBundleHash): string {
    return `${SERVER_BASE_URL}/build/archive/${buildHash}.tar.zst`;
}

export function BUNDLE_TARBALL_FILENAME(buildHash: TBundleHash): string {
    return `${buildHash}.tar.zst`;
}

// cspell:disable
export const lorem = `
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum ut dui est. Cras commodo, erat eget finibus varius, augue est dignissim turpis, nec bibendum nisl justo vitae sem. Aenean sit amet vulputate tortor. Nullam eu vestibulum nisi. Phasellus hendrerit sollicitudin malesuada. Nullam est tellus, convallis in justo quis, efficitur laoreet erat. Duis nulla elit, sodales sed vulputate faucibus, commodo quis sapien. Donec in ligula non risus sagittis fermentum nec ac diam. Phasellus vel dictum nisi, sed pharetra justo.

In viverra eleifend tortor ultricies molestie. Duis ullamcorper, lacus ac vehicula malesuada, tortor leo rhoncus enim, eu tempor ipsum purus non ipsum. Integer rutrum ipsum sit amet ante laoreet malesuada. Morbi hendrerit vestibulum neque in dignissim. Pellentesque aliquet tempor sem non ultricies. Nulla ac imperdiet erat, sit amet finibus lacus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam ornare accumsan tellus, ac aliquet turpis imperdiet et.
`;

export const GNU_LINUX_COPYPASTA = "I'd just like to interject for a moment. What you're referring to as Linux, is in fact, GNU/Linux, or as I've recently taken to calling it, GNU plus Linux. Linux is not an operating system unto itself, but rather another free component of a fully functioning GNU system made useful by the GNU corelibs, shell utilities and vital system components comprising a full OS as defined by POSIX. Many computer users run a modified version of the GNU system every day, without realizing it. Through a peculiar turn of events, the version of GNU which is widely used today is often called “Linux,” and many of its users are not aware that it is basically the GNU system, developed by the GNU Project. There really is a Linux, and these people are using it, but it is just a part of the system they use.";

// cspell:enable

export const EMPTY_OBJECT = /* @__PURE__ */ Object.freeze({});
export const EMPTY_SET = /* @__PURE__ */ Object.freeze(new Set<never>());
export const EMPTY_MAP = /* @__PURE__ */ Object.freeze(new Map<never, never>());
export const EMPTY_ARRAY = /* @__PURE__ */ Object.freeze([]);
export const EMPTY_NULL_OBJECT = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.create(null));
export const NOOP = /* @__PURE__ */ Object.freeze(() => { });

/**
 * do nothing
 * 
 * @see {@link https://doc.rust-lang.org/std/hint/fn.black_box.html|std::hint::black_box}
 */
export function blackBox(_value: unknown): void {}
