import type { Coord } from "@/utils/types";

import { create } from "zustand";

export interface FriendModalCenterStore {
    pos: Coord | null;
    updateFromElement(el: Element): void;
    updateFromPosition(x: number, y: number): void;
    resetPosition(): void;
}

export function defaultPosition(): Coord {
    return {
        x: window.innerWidth / 2,
        y: window.innerHeight / 2,
    };
}

export const useFriendModalCenterStore = create<FriendModalCenterStore>((set) => ({
    pos: null,
    updateFromElement(el: Element) {
        const rect = el.getBoundingClientRect();

        set(() => ({
            pos: {
                x: rect.left + (rect.width / 2),
                y: rect.top + (rect.height / 2),
            },
        }));
    },
    updateFromPosition(x: number, y: number) {
        set(() => ({
            pos: {
                x,
                y,
            },
        }));
    },
    resetPosition() {
        set(() => ({
            pos: null,
        }));
    },
}));
