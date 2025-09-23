import { namedContext } from "@/utils/devtools";

import { type RefObject } from "react";

export interface ScrollAreaContext {
    ref: RefObject<HTMLDivElement | null>;
}

export const ScrollAreaContext = namedContext<ScrollAreaContext>({ ref: { current: null } }, "ScrollAreaContext");
