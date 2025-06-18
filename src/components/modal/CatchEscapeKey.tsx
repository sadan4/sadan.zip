import { matchesEvent } from "@/hooks/keybind";
import { useGesture } from "@use-gesture/react";

import { exitModalKeybinds } from ".";

import _ from "lodash";
import { type PropsWithChildren, useRef } from "react";

function shouldCancel(timing: "up" | "down", ev: KeyboardEvent): boolean {
    return _(exitModalKeybinds)
        .filter({ timing })
        .filter(matchesEvent.bind(null, ev))
        .isEmpty();
}


export function CatchEscapeKey({ children }: PropsWithChildren) {
    const ref = useRef<HTMLDivElement>(null);

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
        <div {...bind()}>
            {children}
        </div>
    );
}
