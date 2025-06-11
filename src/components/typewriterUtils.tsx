import type { TypewriterFrame } from "./Typewriter";

import type { ReactNode } from "react";

export function *defaultEraser(prev: ReactNode, delay = 50): Generator<TypewriterFrame, void, ReactNode> {
    if (typeof prev !== "string") {
        yield {
            component: prev,
            nextDelay: 0,
        };
        return;
    }

    let cur = prev;

    while (cur.length > 0) {
        yield {
            component: `${cur = cur.substring(0, cur.length - 1)}|`,
            nextDelay: delay,
        };
    }
    yield {
        component: "",
        nextDelay: delay * 4,
    };
    return;
}
