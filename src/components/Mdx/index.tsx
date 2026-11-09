import { Codeblock } from "@/components/Codeblock";
import { ExternalLink, Link } from "@/components/Links";
import { Language } from "@/utils/textmate";
import { Link as RouterLink } from "@tanstack/react-router";

import * as styles from "./styles.module.scss";

import type { MDXComponents, MDXContent } from "mdx/types";
import {
    Children,
    cloneElement,
    type ComponentProps,
    isValidElement,
    type PropsWithChildren,
    type ReactNode,
} from "react";

const tagToLanguageMap: Partial<Record<string, Language>> = Object.freeze({
    json: Language.JSON,
    ts: Language.TYPESCRIPT,
    typescript: Language.TYPESCRIPT,
    js: Language.JAVASCRIPT,
    javascript: Language.JAVASCRIPT,
    tsx: Language.TYPESCRIPT_REACT,
    jsx: Language.JAVASCRIPT_REACT,
    html: Language.HTML,
    css: Language.CSS,
});

function MdxLink({ href = "", children, ...props }: ComponentProps<"a">) {
    // footnotes link to each other by hash, and need their id and aria attributes to work
    if (href.startsWith("#")) {
        return (
            <Link
                to="."
                hash={href.slice(1)}
                {...props}
            >
                {children}
            </Link>
        );
    }

    return href.startsWith("/")
        ? <Link to={href}>{children}</Link>
        : <ExternalLink to={href}>{children}</ExternalLink>;
}

function linkText(children: ReactNode, fragment: string | undefined): ReactNode {
    return Children.map(children, (child) => {
        if (!isValidElement<PropsWithChildren>(child)) {
            return (
                <RouterLink
                    to="."
                    hash={fragment}
                >
                    {child}
                </RouterLink>
            );
        }

        if (child.type === MdxLink) {
            return child;
        }

        return cloneElement(child, undefined, linkText(child.props.children, fragment));
    });
}

interface MdxHeadingProps extends ComponentProps<"h1"> {
    tag: "h1" | "h2" | "h3" | "h4" | "h5" | "h6";
}

function MdxHeading({ tag: Tag, id, children, ...props }: MdxHeadingProps) {
    return (
        <Tag
            id={id}
            {...props}
        >
            {linkText(children, id)}
            <span
                className={styles.headingAnchor}
                aria-hidden
                tabIndex={-1}
            >
                ¶
            </span>
        </Tag>
    );
}

function MdxCodeblock({ children, ...props }: ComponentProps<"pre">) {
    if (!isValidElement<ComponentProps<"code">>(children) || typeof children.props.children !== "string") {
        return <pre {...props}>{children}</pre>;
    }

    const { className = "", children: code } = children.props;
    const fence = /language-(\S+)/.exec(className)?.[1] ?? "";

    return (
        <Codeblock
            lang={tagToLanguageMap[fence] ?? Language.PLAINTEXT}
            className="border border-fg-700"
        >
            {code.trimEnd()}
        </Codeblock>
    );
}

const components: MDXComponents = {
    a: MdxLink,
    h1(props) {
        return (
            <MdxHeading
                tag="h1"
                {...props}
            />
        );
    },
    h2(props) {
        return (
            <MdxHeading
                tag="h2"
                {...props}
            />
        );
    },
    h3(props) {
        return (
            <MdxHeading
                tag="h3"
                {...props}
            />
        );
    },
    h4(props) {
        return (
            <MdxHeading
                tag="h4"
                {...props}
            />
        );
    },
    h5(props) {
        return (
            <MdxHeading
                tag="h6"
                {...props}
            />
        );
    },
    h6(props) {
        return (
            <MdxHeading
                tag="h6"
                {...props}
            />
        );
    },
    pre: MdxCodeblock,
};

export interface MdxProps {
    content: MDXContent;
}

export function Mdx({ content: Content }: MdxProps) {
    return (
        <div className={styles.prose}>
            <Content components={components} />
        </div>
    );
}
