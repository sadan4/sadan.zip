import { type StandardTextProps, Text } from "@/components/Text";
import cn from "@/utils/cn";
import type { ComponentPropsWithRef } from "@react-spring/web";
import { createLink } from "@tanstack/react-router";

import * as styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import { LinkIcon } from "../icons/Link";

import type { ComponentProps, PropsWithChildren } from "react";

export interface ExternalLinkProps extends PropsWithChildren {
    to: HTMLAnchorElement["href"];
    target?:
      | `_${"blank" | "self" | "parent" | "top"}`
      | (HTMLAnchorElement["target"] & Record<never, never>);
}

export function ExternalLink({ target = "_blank", to: href, children }: ExternalLinkProps) {
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

export function DownloadRamLink() {
    return (
        <Link to="/download-ram">
            <Text tag="span">Download RAM</Text>
        </Link>
    );
}

export interface IconLinkProps extends ComponentProps<"svg"> {
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
