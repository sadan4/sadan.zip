import { createContext } from "react";


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

export const FriendModalContext = createContext<FriendModalContext | null>(null);
FriendModalContext.displayName = "FriendModalContext";

export const NORMAL_MAIN_CIRCLE_DIAMETER = 500;
export const FRIEND_CARD_CIRCLE_DIAMETER = 192;
