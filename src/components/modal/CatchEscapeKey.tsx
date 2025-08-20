import { matchesEvent } from "@/hooks/keybind";
import { useGesture } from "@use-gesture/react";

import { exitModalKeybinds } from ".";

import { type PropsWithChildren } from "react";

function shouldCancel(timing: "up" | "down", ev: KeyboardEvent): boolean {
    return exitModalKeybinds.filter((kb) => {
        return kb.timing === timing && matchesEvent(ev, kb);
    })
        .length > 0;
}


export function CatchEscapeKey({ children }: PropsWithChildren) {
    const bind = useGesture({
        onKeyDown({ event }) {
            if (shouldCancel("down", event)) {
                event.stopPropagation();
            }
        },
        onKeyUp({ event }) {
            if (shouldCancel("up", event)) {
                event.stopPropagation();
            }
        },
    });

    return (
        <div
            tabIndex={-1}
            {...bind()}
        >
            {children}
        </div>
    );
}
