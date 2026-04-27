export function discordUrl(userId: string): string {
    return `https://discord.com/users/${userId}`;
}

export const NBSP = "\u00A0";

export const EM_DASH = "\u2014";

export const REPLACEMENT_CHARACTER = "\uFFFD";

export const GITHUB_REPO_URL = "https://github.com/sadan4/sadan.zip";
export const GITHUB_REPO_CREATE_ISSUE_URL = "https://github.com/sadan4/sadan.zip/issues/new";

// cspell:disable
export const lorem = `
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum ut dui est. Cras commodo, erat eget finibus varius, augue est dignissim turpis, nec bibendum nisl justo vitae sem. Aenean sit amet vulputate tortor. Nullam eu vestibulum nisi. Phasellus hendrerit sollicitudin malesuada. Nullam est tellus, convallis in justo quis, efficitur laoreet erat. Duis nulla elit, sodales sed vulputate faucibus, commodo quis sapien. Donec in ligula non risus sagittis fermentum nec ac diam. Phasellus vel dictum nisi, sed pharetra justo.

In viverra eleifend tortor ultricies molestie. Duis ullamcorper, lacus ac vehicula malesuada, tortor leo rhoncus enim, eu tempor ipsum purus non ipsum. Integer rutrum ipsum sit amet ante laoreet malesuada. Morbi hendrerit vestibulum neque in dignissim. Pellentesque aliquet tempor sem non ultricies. Nulla ac imperdiet erat, sit amet finibus lacus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam ornare accumsan tellus, ac aliquet turpis imperdiet et.
`;

export const GNU_LINUX_COPYPASTA = "I'd just like to interject for a moment. What you're referring to as Linux, is in fact, GNU/Linux, or as I've recently taken to calling it, GNU plus Linux. Linux is not an operating system unto itself, but rather another free component of a fully functioning GNU system made useful by the GNU corelibs, shell utilities and vital system components comprising a full OS as defined by POSIX. Many computer users run a modified version of the GNU system every day, without realizing it. Through a peculiar turn of events, the version of GNU which is widely used today is often called “Linux,” and many of its users are not aware that it is basically the GNU system, developed by the GNU Project. There really is a Linux, and these people are using it, but it is just a part of the system they use.";

// cspell:enable

export const EMPTY_OBJECT = Object.freeze({});
export const EMPTY_SET = Object.freeze(new Set<never>());
export const EMPTY_MAP = Object.freeze(new Map<never, never>());
export const EMPTY_ARRAY = Object.freeze([]);
export const EMPTY_NULL_OBJECT = Object.freeze(Object.create(null));
export const NOOP = Object.freeze(() => { });
