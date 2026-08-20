import amy88x31 from "./88x31/amy.png?url";
import elissa88x31 from "./88x31/elissa.png?url";
import i3vie88x31 from "./88x31/i3vie.png?url";
import mugman88x31 from "./88x31/mugman.gif?url";
import nin088x31 from "./88x31/nin0.png?url";
import paige88x31 from "./88x31/paige.gif?url";
import rushii88x31 from "./88x31/rushii.webp?url";
import vee88x31 from "./88x31/vee.gif?url";
import wing88x31 from "./88x31/wing.gif?url";
import worf88x31 from "./88x31/worf.gif?url";
import zoot88x31 from "./88x31/zoot.png?url";
import amyAvatar from "./avatars/amy.png?url";
import cookieAvatar from "./avatars/cookie.png?url";
import elissaAvatar from "./avatars/elissa.jpeg?url";
import fresAvatar from "./avatars/fres.png?url";
import i3vieAvatar from "./avatars/i3vie.jpeg?url";
import krstlskllAvatar from "./avatars/krstlskll.avif?url";
import maddieAvatar from "./avatars/maddie.png?url";
import mugmanAvatar from "./avatars/mugman.jpeg?url";
import nin0Avatar from "./avatars/nin0.png?url";
import paigeAvatar from "./avatars/paige.png?url";
import rushiiAvatar from "./avatars/rushii.png?url";
import sqaaakoiAvatar from "./avatars/sqaaakoi.png?url";
import veeAvatar from "./avatars/vee.png?url";
import wingAvatar from "./avatars/wing.jpeg?url";
import worfAvatar from "./avatars/worf.png?url";
import zootAvatar from "./avatars/zoot.jpeg?url";


export interface Friend {
    name: string;
    discordId: string | null;
    avatarUrl: string;
    url: string | null;
    _88x31url: string | null;
}

// Friends!

// If i know you, feel free to pr

export const friends: Friend[] = [
    {
        name: "nin0",
        discordId: "886685857560539176",
        url: "https://nin0.dev",
        avatarUrl: nin0Avatar,
        _88x31url: nin088x31,
    },
    {
        name: "maddie",
        discordId: "1298435571395330108",
        url: "https://maddie.lgbt",
        avatarUrl: maddieAvatar,
        _88x31url: null,
    },
    {
        name: "zoot",
        discordId: "289556910426816513",
        url: "https://zt64.dev",
        avatarUrl: zootAvatar,
        _88x31url: zoot88x31,
    },
    {
        name: "elissa",
        discordId: "381494697073573899",
        url: "https://elissa.moe",
        avatarUrl: elissaAvatar,
        _88x31url: elissa88x31,
    },
    {
        name: "sqaaakoi",
        discordId: "259558259491340288",
        url: "https://sqaaakoi.xyz",
        avatarUrl: sqaaakoiAvatar,
        _88x31url: null,
    },
    {
        name: "i3vie",
        discordId: "1215424013443272736",
        url: "https://i3vie.dev",
        avatarUrl: i3vieAvatar,
        _88x31url: i3vie88x31,
    },
    {
        name: "cookie",
        discordId: "721853658941227088",
        avatarUrl: cookieAvatar,
        url: null,
        _88x31url: null,
    },
    {
        name: "fres",
        discordId: "843448897737064448",
        avatarUrl: fresAvatar,
        url: "https://slug.cat",
        _88x31url: null,
    },
    {
        name: "wing",
        discordId: "298295889720770563",
        url: "https://wingio.xyz/",
        avatarUrl: wingAvatar,
        _88x31url: wing88x31,
    },
    {
        name: "krstlskll",
        discordId: "929208515883569182",
        url: "https://krstlskll69.github.io/",
        avatarUrl: krstlskllAvatar,
        _88x31url: null,
    },
    {
        name: "mugman",
        discordId: "601836455006044163",
        url: "https://mugman.tech",
        avatarUrl: mugmanAvatar,
        _88x31url: mugman88x31,
    },
    {
        name: "worf",
        discordId: "262786101037498369",
        url: "https://worf.win/",
        avatarUrl: worfAvatar,
        _88x31url: worf88x31,
    },
    {
        name: "vee",
        discordId: "343383572805058560",
        url: "https://vendicated.dev",
        avatarUrl: veeAvatar,
        _88x31url: vee88x31,
    },
    {
        name: "rushii",
        discordId: null,
        url: "https://rushii.dev",
        avatarUrl: rushiiAvatar,
        _88x31url: rushii88x31,
    },
    {
        name: "paige",
        discordId: "1375697625864601650",
        url: "https://paige.moe",
        avatarUrl: paigeAvatar,
        _88x31url: paige88x31,
    },
    {
        name: "amy",
        discordId: "603229858612510720",
        url: "https://amy.rip",
        avatarUrl: amyAvatar,
        _88x31url: amy88x31,
    },
];
