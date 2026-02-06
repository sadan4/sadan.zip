import { publicNodeProperties } from "./publicApi.gen&gen";
import { type TS, ts } from ".";
import { error } from "../error";

export function getPublicKeys<T extends TS.Node>(node: T): ReadonlySet<keyof T> {
    const nk = ts.SyntaxKind[node.kind];

    return publicNodeProperties.get(nk) as ReadonlySet<keyof T> ?? error(`invalid node: ${nk}`);
}
