import { Text } from "@/components/Text";

import { PersonalButtonLink, SourceLink, ThemeLink } from "./Links";
import { joinWithKey } from "../utils/array";
import {NBSP} from "@/utils/constants";

import {
    Fragment, type PropsWithChildren, type ReactNode,
} from "react";

export interface FooterProps extends PropsWithChildren {
    className?: string;
}

component FooterSeperator() {
    <Text tag="span">{`${NBSP}|${NBSP}`}</Text>
}

export component BaseFooter({
    className,
    children: _children,
}: FooterProps) {
    const children: ReactNode[] = Array.isArray(_children)
        ? [..._children]
        : [_children];

    <div className={className}>
        for (const child of children; index i) {
            <Fragment key={`footer-separator-${i}`}>
                if (i) {
                    <FooterSeperator />
                }
                {child}
            </Fragment>
        }
    </div>
}

export component DefaultFooter() {
    <BaseFooter className="mb-1">
        <ThemeLink key="footer-theme-link" />
        <SourceLink key="footer-source-link" />
        <PersonalButtonLink key="footer-button-link" />
    </BaseFooter>
}
