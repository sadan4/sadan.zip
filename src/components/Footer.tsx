import { namedContext } from "@/utils/devtools";
import { error } from "@/utils/error";
import { Text } from "@components/Text";

import { ScrollArea } from "./layout/ScrollArea";
import { ButtonLink, SourceLink, ThemeLink } from "./Links";
import { joinWithKey } from "../utils/array";

import {
    type ComponentPropsWithoutRef,
    memo,
    type PropsWithChildren,
    type ReactNode,
    use,
    useMemo,
    useState,
} from "react";
import { createPortal } from "react-dom";

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
            <ButtonLink key="footer-button-link" />
        </BaseFooter>
    );
});

interface FooterContainerProps extends ComponentPropsWithoutRef<"div"> {
}

interface FooterContext {
    footer: HTMLDivElement | null;
    content: HTMLDivElement | null;
}

const FooterContext = namedContext<FooterContext | null>(null, "FooterContext");

function useFooterContext() {
    const ctx = use(FooterContext);

    if (ctx == null) {
        error("useFooterContext must be used within a FooterContainer");
    }

    return ctx;
}

export function FooterContainer({
    children,
    ...props
}: FooterContainerProps) {
    const [footer, setFooter] = useState<HTMLDivElement | null>(null);
    const [content, setContent] = useState<HTMLDivElement | null>(null);

    const value = useMemo<FooterContext>(() => ({
        footer,
        content,
    }), [content, footer]);

    return (
        <FooterContext value={value}>
            <ScrollArea className="h-screen max-h-screen">
                <div className="grid h-full w-full grid-rows-[1fr_min-content]">
                    {children}
                    <div
                        {...props}
                        ref={setContent}
                    />
                    <div
                        ref={setFooter}
                        className="flex justify-center"
                    />
                </div>
            </ScrollArea>
        </FooterContext>
    );
}

export interface FooterContentProps extends PropsWithChildren {

}

export function FooterContent({ children }: FooterContentProps) {
    const { content } = useFooterContext();

    return content && createPortal(children, content);
}

export interface FooterFooterProps extends PropsWithChildren {
}

export function FooterFooter({ children }: FooterFooterProps) {
    const { footer } = useFooterContext();

    return footer && createPortal(children, footer);
}

