import { Text } from "@/components/Text";

import { SourceLink, ThemeLink } from "./Links";
import { joinWith } from "../utils/array";

import {
    type ComponentProps,
    type PropsWithChildren,
    type ReactNode,
} from "react";

export interface FooterProps extends PropsWithChildren {
    className?: string;
}

function FooterSeperator() {
    return <Text tag="span"> | </Text>;
}

export default function Footer({
    className,
    children: _children,
}: FooterProps) {
    const children: ReactNode[] = Array.isArray(_children)
        ? [..._children]
        : [_children];

    return (
        <div className={className}>{joinWith(children, <FooterSeperator key="footer-seperator" />)}</div>
    );
}

export const DefaultFooter = function DefaultFooter() {
    return (
        <Footer className="mb-1">
            <ThemeLink key="footer-theme-link" />
            <SourceLink key="footer-source-link" />
        </Footer>
    );
};

interface FooterContainerProps extends ComponentProps<"div"> {
    footer: () => ReactNode;
}

export function FooterContainer({
    children,
    footer: Footer,
    ...props
}: FooterContainerProps) {
    return (
        <div className="grid grid-rows-[1fr_min-content] w-full h-full">
            <div {...props}>{children}</div>
            <div className="flex justify-center items-center content-center">
                <Footer />
            </div>
        </div>
    );
}
