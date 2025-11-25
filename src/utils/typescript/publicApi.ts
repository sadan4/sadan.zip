import { publicNodeProperties } from "./publicApi.gen&gen";
import { error } from "../error";

import { type Node, SyntaxKind } from "typescript";

export function getPublicKeys<T extends Node>(node: T): ReadonlySet<keyof T> {
    const nk = SyntaxKind[node.kind];

    return publicNodeProperties.get(nk) as ReadonlySet<keyof T> ?? error(`invalid node: ${nk}`);
}
