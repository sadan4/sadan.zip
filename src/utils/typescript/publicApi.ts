import { publicNodeProperties } from "./generated/publicApi";
import { type TS, ts } from ".";
import { error } from "../error";

export function getPublicKeys<T extends TS.Node>(node: T): ReadonlySet<keyof T> {
    const nk = ts.SyntaxKind[node.kind];

    return publicNodeProperties.get(nk) as ReadonlySet<keyof T> | undefined ?? error(`invalid node: ${nk}`);
}
