import { Codeblock } from "@/components/Codeblock";
import { ExternalLink, Link } from "@/components/Links";
import { Language } from "@/utils/textmate";

import * as styles from "./styles.module.scss";

import type { MDXComponents, MDXContent } from "mdx/types";
import { type ComponentProps, isValidElement } from "react";

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

function MdxLink({ href = "", children }: ComponentProps<"a">) {
    return href.startsWith("/")
        ? <Link to={href}>{children}</Link>
        : <ExternalLink to={href}>{children}</ExternalLink>;
}

function MdxCodeblock({ children, ...props }: ComponentProps<"pre">) {
    if (!isValidElement<ComponentProps<"code">>(children) || typeof children.props.children !== "string") {
        return <pre {...props}>{children}</pre>;
    }

    const { className = "", children: code } = children.props;
    const fence = /language-(\S+)/.exec(className)?.[1] ?? "";

    return (
        <Codeblock lang={tagToLanguageMap[fence] ?? Language.PLAINTEXT}>
            {code.trimEnd()}
        </Codeblock>
    );
}

const components: MDXComponents = {
    a: MdxLink,
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
