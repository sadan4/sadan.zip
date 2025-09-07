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
    {
        name: "i3vie",
        discordId: "1215424013443272736",
        url: new URL("https://i3vie.dev"),
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/180745311?v=4"),
    },
    {
        name: "cookie",
        discordId: "721853658941227088",
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/52550063?v=4"),
    },
    {
        name: "fres",
        discordId: "843448897737064448",
        avatarUrl: new URL("https://slug.cat/avatar.png"),
        url: new URL("https://slug.cat"),
    },
    {
        name: "wing",
        discordId: "298295889720770563",
        url: new URL("https://wingio.xyz/"),
        avatarUrl: new URL("https://avatars.githubusercontent.com/u/44992537?v=4"),
    },
];
