import type { PropsWithChildren } from "react";
import Discord from "./icons/Discord";
import NameMC from "./icons/NameMC";
import LastFM from "./icons/LastFM";
import Steam from "./icons/Steam";
import SaveTheWorld from "./icons/SaveTheWorld";
import Github from "./icons/Github";

export interface LinkProps extends PropsWithChildren {
  href: HTMLAnchorElement["href"];
  target?:
  | `_${"blank" | "self" | "parent" | "top"}`
  | (HTMLAnchorElement["target"] & Record<never, never>);
}
export default function Link({ target = "_blank", href, children }: LinkProps) {
  return (
    <a href={href} target={target}>
      {children}
    </a>
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

export interface DiscordIconLinkProps {
  userId: string;
}

export function DiscordIconLink({ userId }: DiscordIconLinkProps) {
  return <Link href={`https://discord.com/users/${userId}`}>
    <Discord />
  </Link>;
}

export interface NameMCIconLinkProps {
  UUID: string;
}

export function NameMCIconLink({ UUID }: NameMCIconLinkProps) {
  return <Link href={`https://namemc.com/profile/${UUID}`}>
    <NameMC />
  </Link>;
}

export interface LastFMIconLinkProps {
  username: string;
}

export function LastFMIconLink({ username }: LastFMIconLinkProps) {
  return <Link href={`https://last.fm/user/${username}`}>
    <LastFM />
  </Link>;
}

export interface SteamIconLinkProps {
  userId: string;
}
export function SteamIconLink({ userId }: SteamIconLinkProps) {
  return <Link href={`https://steamcommunity.com/id/${userId}`}>
    <Steam />
  </Link>;
}

export interface FortniteDBIconLinkProps {
  username: string;
}

export function FortniteDBIconLink({ username }: FortniteDBIconLinkProps) {
  return <Link href={`https://fortnitedb.com/profile/${username}`}>
    <SaveTheWorld />
  </Link>;
}

export interface GithubIconLinkProps {
  username: string;
}

export function GithubIconLink({ username }: GithubIconLinkProps) {
  return <Link href={`https://github.com/${username}`}>
    <Github />
  </Link>;
}
