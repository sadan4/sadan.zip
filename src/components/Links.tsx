import Discord from "./icons/Discord";
import LinkIcon from "./icons/FriendLink";
import Github from "./icons/Github";
import LastFM from "./icons/LastFM";
import NameMC from "./icons/NameMC";
import SaveTheWorld from "./icons/SaveTheWorld";
import Steam from "./icons/Steam";
import { Clickable } from "./Clickable";

import type { ComponentProps, PropsWithChildren } from "react";

export interface LinkProps extends PropsWithChildren {
    href: HTMLAnchorElement["href"];
    target?:
      | `_${"blank" | "self" | "parent" | "top"}`
      | (HTMLAnchorElement["target"] & Record<never, never>);
}
export default function Link({ target = "_blank", href, children }: LinkProps) {
    return (
        <Clickable
            tag="a"
            href={href}
            target={target}
        >
            {children}
        </Clickable>
    );
}

export function ThemeLink() {
    return (
        <Link href="https://github.com/enkia/tokyo-night-vscode-theme/tree/master">
            Color Scheme
        </Link>
    );
}

export function SourceLink() {
    return <Link href="https://github.com/sadan4/sadan.zip">Source Code</Link>;
}

export interface IconLinkProps extends ComponentProps<"svg"> {
}

export interface DiscordIconLinkProps extends IconLinkProps {
    userId: string;
}

export function DiscordIconLink({ userId, ...props }: DiscordIconLinkProps) {
    return (
        <Link href={`https://discord.com/users/${userId}`}>
            <Discord {...props} />
        </Link>
    );
}

export interface NameMCIconLinkProps extends IconLinkProps {
    UUID: string;
}

export function NameMCIconLink({ UUID, ...props }: NameMCIconLinkProps) {
    return (
        <Link href={`https://namemc.com/profile/${UUID}`}>
            <NameMC {...props} />
        </Link>
    );
}

export interface LastFMIconLinkProps extends IconLinkProps {
    username: string;
}

export function LastFMIconLink({ username, ...props }: LastFMIconLinkProps) {
    return (
        <Link href={`https://last.fm/user/${username}`}>
            <LastFM {...props} />
        </Link>
    );
}

export interface SteamIconLinkProps extends IconLinkProps {
    userId: string;
}
export function SteamIconLink({ userId, ...props }: SteamIconLinkProps) {
    return (
        <Link href={`https://steamcommunity.com/id/${userId}`}>
            <Steam {...props} />
        </Link>
    );
}

export interface FortniteDBIconLinkProps extends IconLinkProps {
    username: string;
}

export function FortniteDBIconLink({ username, ...props }: FortniteDBIconLinkProps) {
    return (
        <Link href={`https://fortnitedb.com/profile/${username}`}>
            <SaveTheWorld {...props} />
        </Link>
    );
}

export interface GithubIconLinkProps extends IconLinkProps {
    username: string;
}

export function GithubIconLink({ username, ...props }: GithubIconLinkProps) {
    return (
        <Link href={`https://github.com/${username}`}>
            <Github {...props} />
        </Link>
    );
}

export interface FriendWebsiteLinkProps extends IconLinkProps {
    href: string;
}
export function FriendWebsiteLink({ href, ...props }: FriendWebsiteLinkProps) {
    return (
        <Link href={href} >
            <LinkIcon {...props} />
        </Link>
    );
}
