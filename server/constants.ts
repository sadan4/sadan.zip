export const BUILDS_PATH = "./builds";

export enum Channels {
    STABLE = "stable",
    // PTB = "ptb",
    CANARY = "canary",
}

export const assetUrl = {
    [Channels.STABLE]: "https://discord.com/assets/",
    // [Channels.PTB]: "https://ptb.discord.com/assets/",
    [Channels.CANARY]: "https://canary.discord.com/assets/",
} as const;

export const appUrl = {
    [Channels.STABLE]: "https://discord.com/app",
    // [Channels.PTB]: "https://ptb.discord.com/app",
    [Channels.CANARY]: "https://canary.discord.com/app",
} as const;

export const SYM_CJS_DEFAULT_PLACEHOLDER = "SYMBOL(SYM_CJS_DEFAULT)";
