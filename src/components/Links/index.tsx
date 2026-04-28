import { type StandardTextProps, Text } from "@/components/Text";
import cn from "@/utils/cn";
import { discordUrl } from "@/utils/constants";
import type { ComponentPropsWithRef } from "@react-spring/web";
import { createLink } from "@tanstack/react-router";

import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import Discord from "../icons/Discord";
import Github from "../icons/Github";
import LastFM from "../icons/LastFM";
import { LinkIcon } from "../icons/Link";
import NameMC from "../icons/NameMC";
import SaveTheWorld from "../icons/SaveTheWorld";
import Steam from "../icons/Steam";

import type { ComponentProps, PropsWithChildren } from "react";

export interface ExternalLinkProps extends PropsWithChildren {
    to: HTMLAnchorElement["href"];
    target?:
      | `_${"blank" | "self" | "parent" | "top"}`
      | (HTMLAnchorElement["target"] & Record<never, never>);
}

export default function ExternalLink({ target = "_blank", to: href, children }: ExternalLinkProps) {
    return (
        <Clickable
            tag="a"
            href={href}
            target={target}
            className={styles.link}
        >
            {children}
        </Clickable>
    );
}

export interface LinkProps extends ComponentPropsWithRef<"a"> {

}

function LinkComponent({ className, ...props }: LinkProps) {
    return (
        <Clickable
            tag="a"
            {...props}
            className={cn(className, styles.link)}
        />
    );
}

export const Link = createLink(LinkComponent);

export interface TextLinkProps extends StandardTextProps, Omit<ComponentPropsWithRef<"a">, "color"> {
    textClassName?: string;
}

function TextLinkComponent({
    className,
    textClassName,
    noselect,
    nowrap,
    center,
    color,
    size,
    weight,
    children,
    ...props
}: TextLinkProps) {
    return (
        <Clickable
            tag="a"
            className={cn(className, styles.textLink)}
            {...props}
        >
            <Text
                className={cn(textClassName)}
                noselect={noselect}
                nowrap={nowrap}
                center={center}
                color={color}
                size={size}
                weight={weight}
            >
                {children}
            </Text>
        </Clickable>
    );
}

export const TextLink = createLink(TextLinkComponent);

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

export function PersonalButtonLink() {
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
        <ExternalLink to={discordUrl(userId)}>
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
