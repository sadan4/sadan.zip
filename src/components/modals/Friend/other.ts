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
        top: 0,
        left: 0,
        width: window.innerWidth,
        height: window.innerHeight,
    };
}

export const FriendModalContext = namedContext<FriendModalContext>(proxyLazy(defaultPosition), "FriendModalContext");

export const NORMAL_MAIN_CIRCLE_DIAMETER = 500;
export const FRIEND_CARD_CIRCLE_DIAMETER = 192;
