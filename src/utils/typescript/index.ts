import { error, unreachable } from "../error";
import { Language } from "../textmate";

import {
    type Node,
    ScriptKind,
    ScriptTarget,
    type SourceFile,
    SyntaxKind,
    type TextChangeRange,
} from "typescript";

export function scriptKindForLanguage(language: Language): ScriptKind {
    switch (language) {
        case Language.JSON:
            return ScriptKind.JSON;
        case Language.TYPESCRIPT:
            return ScriptKind.TS;
        case Language.JAVASCRIPT:
            return ScriptKind.JS;
        case Language.TYPESCRIPT_REACT:
            return ScriptKind.TSX;
        case Language.JAVASCRIPT_REACT:
            return ScriptKind.JSX;
        default:
            error(`unsupported language: ${language}`);
    }
}

export function defaultScriptTarget(language: Language): ScriptTarget {
    switch (language) {
        case Language.JSON:
            return ScriptTarget.JSON;
        case Language.TYPESCRIPT:
        case Language.JAVASCRIPT:
        case Language.TYPESCRIPT_REACT:
        case Language.JAVASCRIPT_REACT:
            return ScriptTarget.ESNext;
        default:
            error(`unsupported language: ${language}`);
    }
}

export function getNodeKey({ pos, end, kind }: Node): string {
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

export function getChildrenWithMode(node: Node, mode: TreeMode): readonly Node[] {
    switch (mode) {
        case TreeMode.GET_CHILDREN:
            return node.getChildren();
        case TreeMode.FOR_EACH_CHILD: {
            const children: Node[] = [];

            node.forEachChild((child) => {
                children.push(child);
            });
            return children;
        }
        default:
            unreachable();
    }
}

export function getNodeName({ kind }: Node): string {
    return SyntaxKind[kind] ?? "<ERROR>";
}

export function getTextChanges(oldText: string, newText: string): TextChangeRange {
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

export function getVisibleNodeRange(node: Node, sourceFile: SourceFile): NodeRange {
    return [node.getStart(sourceFile, true), node.end];
}

export function isNode(n: any): n is Node {
    if (!n || typeof n !== "object") {
        return false;
    }

    return typeof n.kind === "number" && typeof n.pos === "number" && typeof n.end === "number";
}

export * from "./publicApi";
