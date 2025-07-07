export interface Friend {
    discordId?: string;
    name: string;
    avatarUrl: URL;
    url?: URL;
}

export const friends: Friend[] = [
    {
        name: "nin0",
        discordId: "886685857560539176",
        url: new URL("https://nin0.dev"),
        avatarUrl: new URL("https://nin0.dev/logo.png"),
    },
    {
        name: "s4mi",
        discordId: "1298435571395330108",
        url: new URL("https://s4mi.dev"),
        avatarUrl: new URL("https://s4mi.dev/pfp.png"),
    },
    {
        name: "zoot",
        discordId: "289556910426816513",
        url: new URL("https://zt64.dev"),
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/31907977?v=4"),
    },
    {
        name: "darwinx64",
        discordId: "1375697625864601650",
        url: new URL("https://maize.moe"),
        avatarUrl: new URL("https://maize.moe/avatar.png"),
    },
    {
        name: "elissa",
        discordId: "381494697073573899",
        // url: new URL("https://elissa.moe"),
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/140089641?v=4"),
    },
    {
        name: "sqaaakoi",
        discordId: "259558259491340288",
        url: new URL("https://sqaaakoi.xyz"),
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/37475903?v=4"),
    },
];
