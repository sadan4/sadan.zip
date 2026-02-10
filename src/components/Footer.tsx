import { Text } from "@/components/Text";

import { PersonalButtonLink, SourceLink, ThemeLink } from "./Links";
import { joinWithKey } from "../utils/array";

import {
    memo,
    type PropsWithChildren,
    type ReactNode,
} from "react";

export interface FooterProps extends PropsWithChildren {
    className?: string;
}

function FooterSeperator() {
    return <Text tag="span"> | </Text>;
}

export function BaseFooter({
    className,
    children: _children,
}: FooterProps) {
    const children: ReactNode[] = Array.isArray(_children)
        ? [..._children]
        : [_children];

    return (
        <div className={className}>{joinWithKey(children, (i) => <FooterSeperator key={`footer-seperator-${i}`} />)}</div>
    );
}

export const DefaultFooter = memo(function DefaultFooter() {
    return (
        <BaseFooter className="mb-1">
            <ThemeLink key="footer-theme-link" />
            <SourceLink key="footer-source-link" />
            <PersonalButtonLink key="footer-button-link" />
        </BaseFooter>
    );
});
