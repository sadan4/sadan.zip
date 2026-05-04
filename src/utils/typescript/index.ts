import { markerMap } from "./generated/markerMap";
import { error, unavailableImport } from "../error";
import { Language } from "../textmate";

import type * as TS from "typescript";

const ts: typeof import("typescript") = import.meta.env.SSR ? unavailableImport<never>("typescript") : await import("typescript");

export {
    type TS,
    ts,
};

export function scriptKindForLanguage(language: Language): TS.ScriptKind {
    switch (language) {
        case Language.JSON:
            return ts.ScriptKind.JSON;
        case Language.TYPESCRIPT:
            return ts.ScriptKind.TS;
        case Language.JAVASCRIPT:
            return ts.ScriptKind.JS;
        case Language.TYPESCRIPT_REACT:
            return ts.ScriptKind.TSX;
        case Language.JAVASCRIPT_REACT:
            return ts.ScriptKind.JSX;
        case Language.PLAINTEXT:
        case Language.UNKNOWN:
        case Language.HTML:
        case Language.CSS:
            error(`unsupported language: ${language}`);
    }
}

export function defaultScriptTarget(language: Language): TS.ScriptTarget {
    switch (language) {
        case Language.JSON:
            return ts.ScriptTarget.JSON;
        case Language.TYPESCRIPT:
        case Language.JAVASCRIPT:
        case Language.TYPESCRIPT_REACT:
        case Language.JAVASCRIPT_REACT:
            return ts.ScriptTarget.ESNext;
        case Language.PLAINTEXT:
        case Language.UNKNOWN:
        case Language.HTML:
        case Language.CSS:
            error(`unsupported language: ${language}`);
    }
}

export function getNodeKey({ pos, end, kind }: TS.Node): string {
    return `${pos}-${end}-${kind}`;
}

export enum TreeMode {
    GET_CHILDREN = "getChildren",
    FOR_EACH_CHILD = "forEachChild",
}

export const treeModeStringMap = Object.freeze({
    [TreeMode.GET_CHILDREN]: "node.getChildren()",
    [TreeMode.FOR_EACH_CHILD]: "node.forEachChild(child => /* ... */)",
} satisfies Record<TreeMode, string>);

export function getChildrenWithMode(node: TS.Node, mode: TreeMode): readonly TS.Node[] {
    switch (mode) {
        case TreeMode.GET_CHILDREN:
            return node.getChildren();
        case TreeMode.FOR_EACH_CHILD: {
            const children: TS.Node[] = [];

            node.forEachChild((child) => {
                children.push(child);
            });
            return children;
        }
    }
}

export function getNodeName({ kind }: TS.Node): string {
    const ret = ts.SyntaxKind[kind];

    if (markerMap.has(ret)) {
        return markerMap.get(ret)!;
    }
    return ret ?? "<ERROR>";
}

export function getTextChanges(oldText: string, newText: string): TS.TextChangeRange {
    const { length: oldLen } = oldText;
    const { length: newLen } = newText;

    return {
        span: {
            start: 0,
            length: oldLen,
        },
        newLength: newLen,
    };
}

export type NodeRange = readonly [pos: number, end: number];

export function getVisibleNodeRange(node: TS.Node, sourceFile: TS.SourceFile): NodeRange {
    return [node.getStart(sourceFile, true), node.end];
}

export function isNode(n: any): n is TS.Node {
    if (!n || typeof n !== "object") {
        return false;
    }

    return typeof n.kind === "number" && typeof n.pos === "number" && typeof n.end === "number";
}

export function nodeFromPosition(node: TS.Node, position: number): TS.Node | undefined {
    const { pos, end } = node;

    if (position < pos || position >= end) {
        return;
    }

    const children = node.getChildren();

    for (const child of children) {
        const res = nodeFromPosition(child, position);

        if (res) {
            return res;
        }
    }
    return node;
}

export function isSyntaxList(node: TS.Node): node is TS.SyntaxList {
    return node.kind === ts.SyntaxKind.SyntaxList;
}

export function getParent(node: TS.Node): TS.Node | undefined {
    if (!node.parent) {
        return;
    }

    const { parent } = node;
    const children = parent.getChildren();

    if (children.includes(node)) {
        return parent;
    }
    for (const child of children) {
        if (isSyntaxList(child) && child.getChildren().includes(node)) {
            return child;
        }
    }
}

export * from "./publicApi";
