import { createContext, type RefObject } from "react";

export interface ScrollAreaContext {
    ref: RefObject<HTMLDivElement | null>;
}

export const ScrollAreaContext = createContext<ScrollAreaContext>({ ref: { current: null } });
