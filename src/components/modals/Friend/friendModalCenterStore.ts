import type { Coord } from "@/utils/types";

import { create } from "zustand";
import { devtools } from "zustand/middleware";

export interface FriendModalCenterStore {
    pos: Coord | null;
    updateFromElement(el: Element): void;
    updateFromPosition(x: number, y: number): void;
    useDefaultPosition(): void;
    resetPosition(): void;
}

export function defaultPosition(): Coord {
    return {
        x: window.innerWidth / 2,
        y: window.innerHeight / 2,
    };
}

export const useFriendModalCenterStore = create<FriendModalCenterStore>()(devtools((set) => ({
    pos: null,
    updateFromElement(el: Element) {
        const rect = el.getBoundingClientRect();

        set(() => ({
            pos: {
                x: rect.left + (rect.width / 2),
                y: rect.top + (rect.height / 2),
            },
        }), undefined, "friendModalCenter/updateFromElement");
    },
    updateFromPosition(x: number, y: number) {
        set(() => ({
            pos: {
                x,
                y,
            },
        }), undefined, "friendModalCenter/updateFromPosition");
    },
    resetPosition() {
        set(() => ({
            pos: null,
        }), undefined, "friendModalCenter/resetPosition");
    },
}), {
    store: "FriendModalCenterStore",
    name: "FriendModalCenterStore",
    enabled: import.meta.env.DEV,
    trace: true,
}));
