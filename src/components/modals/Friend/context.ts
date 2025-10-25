import { namedContext } from "@/utils/devtools";

import { createRef, type RefObject } from "react";

export interface FriendModalContext {
    /**
     * should never be null
     */
    centerElement: RefObject<HTMLElement | null>;
}

export const FriendModalContext = namedContext<FriendModalContext>({ centerElement: createRef() }, "FriendModalContext");
