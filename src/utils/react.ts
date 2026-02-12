import { isFunction } from "./types";

import type { Activity, ActivityProps, SetStateAction } from "react";

export function nextStateValue<T>(prevState: T, nextState: SetStateAction<T>): T {
    return isFunction(nextState) ? nextState(prevState) : nextState;
}

/**
 * helper for {@link Activity}'s {@link ActivityProps.mode|mode} prop
 *
 * @see {@link https://react.dev/reference/react/Activity `<Activity>` documentation}
 */
export function visibleIf(condition: boolean): "visible" | "hidden" {
    return condition ? "visible" : "hidden";
}
