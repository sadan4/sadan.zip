import { Text } from "@/components/Text";
import { discordUrl } from "@/utils/constants";
import { Link } from "@tanstack/react-router";

import Discord from "./icons/Discord";
import Github from "./icons/Github";
import LastFM from "./icons/LastFM";
import { LinkIcon } from "./icons/Link";
import NameMC from "./icons/NameMC";
import SaveTheWorld from "./icons/SaveTheWorld";
import Steam from "./icons/Steam";
import { Clickable } from "./Clickable";

import type { ComponentProps, PropsWithChildren } from "react";

export interface ExternalLinkProps extends PropsWithChildren {
    to: string;
    target?:
      | `_${"blank" | "self" | "parent" | "top"}`
      | (HTMLAnchorElement["target"] & Record<never, never>);
}


export function ExternalLink({ to, target = "_blank", children }: ExternalLinkProps) {
    return (
        <Clickable
            tag="a"
            href={to}
            target={target}
        >
            {children}
        </Clickable>
    );
}


export function ThemeLink() {
    return (
        <ExternalLink to="https://github.com/enkia/tokyo-night-vscode-theme/tree/master">
            <Text tag="span">Color Scheme</Text>
        </ExternalLink>
    );
}

export function SourceLink() {
    return (
        <ExternalLink to="https://github.com/sadan4/sadan.zip">
            <Text tag="span">Source Code</Text>
        </ExternalLink>
    );
}

export function ButtonLink() {
    return (
        <Link to="/88x31">
            <Text tag="span">88x31</Text>
        </Link>
    );
}

export interface IconLinkProps extends ComponentProps<"svg"> {
}

export interface DiscordIconLinkProps extends IconLinkProps {
    userId: string;
}

export function DiscordIconLink({ userId, ...props }: DiscordIconLinkProps) {
    return (
        <ExternalLink to={discordUrl(userId).toString()}>
            <Discord {...props} />
        </ExternalLink>
    );
}

export interface NameMCIconLinkProps extends IconLinkProps {
    UUID: string;
}

export function NameMCIconLink({ UUID, ...props }: NameMCIconLinkProps) {
    return (
        <ExternalLink to={`https://namemc.com/profile/${UUID}`}>
            <NameMC {...props} />
        </ExternalLink>
    );
}

export interface LastFMIconLinkProps extends IconLinkProps {
    username: string;
}

export function LastFMIconLink({ username, ...props }: LastFMIconLinkProps) {
    return (
        <ExternalLink to={`https://last.fm/user/${username}`}>
            <LastFM {...props} />
        </ExternalLink>
    );
}

export interface SteamIconLinkProps extends IconLinkProps {
    userId: string;
}
export function SteamIconLink({ userId, ...props }: SteamIconLinkProps) {
    return (
        <ExternalLink to={`https://steamcommunity.com/id/${userId}`}>
            <Steam {...props} />
        </ExternalLink>
    );
}

export interface FortniteDBIconLinkProps extends IconLinkProps {
    username: string;
}

export function FortniteDBIconLink({ username, ...props }: FortniteDBIconLinkProps) {
    return (
        <ExternalLink to={`https://fortnitedb.com/profile/${username}`}>
            <SaveTheWorld {...props} />
        </ExternalLink>
    );
}

export interface GithubIconLinkProps extends IconLinkProps {
    username: string;
}

export function GithubIconLink({ username, ...props }: GithubIconLinkProps) {
    return (
        <ExternalLink to={`https://github.com/${username}`}>
            <Github {...props} />
        </ExternalLink>
    );
}

export interface FriendWebsiteLinkProps extends IconLinkProps {
    href: string;
}
export function FriendWebsiteLink({ href, width = 24, height = 24, ...props }: FriendWebsiteLinkProps) {
    return (
        <ExternalLink to={href} >
            <LinkIcon
                width={width}
                height={height}
                {...props}
            />
        </ExternalLink>
    );
}
