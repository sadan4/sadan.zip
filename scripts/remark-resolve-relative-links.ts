import type { Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";

const MDX_EXT_RE = /\.mdx?(?=$|[?#])/i;
/**
 * scheme, absolute path, or fragment
 */
const EXTERNAL_LINK_RE = /^([a-z][a-z0-9+.-]*:|\/|#)/i;

/**
 * strip `.md` or `.mdx` extensions from relative links
 */
// eslint-disable-next-line func-style
export const resolveMdxRelativeLinks: Plugin<[], Root> = () => {
    return function (tree) {
        visit(tree, "link", (node) => {
            if (!EXTERNAL_LINK_RE.test(node.url) && MDX_EXT_RE.test(node.url)) {
                node.url = node.url.replace(MDX_EXT_RE, "");
            }
        });
    };
};
