import { namedContext } from "@/utils/devtools";
import { proxyLazy } from "@/utils/lazy";

export interface FriendModalContext {
    x: number;
    y: number;
    width: number;
    height: number;
}

export function defaultPosition() {
    return {
        x: 0,
        y: 0,
        width: window.innerWidth,
        height: window.innerHeight,
    };
}

export const FriendModalContext = namedContext<FriendModalContext>(proxyLazy(defaultPosition), "FriendModalContext");
