import type { ReactNode, RefObject } from "react";

export * from "./Typewriter.tsrx";
export * from "./utils";

export interface TypewriterFrame {
    component: ReactNode;
    nextDelay: number;
}

export interface TypewriterSource {
    type(): Generator<TypewriterFrame, void, ReactNode>;
    erase(prev: ReactNode): Generator<TypewriterFrame, void, ReactNode>;
}

export interface TypewriterRef {
    sendWord(source: TypewriterSource, dontDeleteOld?: boolean): void;
    isTyping: RefObject<boolean>;
}
