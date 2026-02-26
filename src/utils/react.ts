import { mergeAllDOMRects } from "./dom/rect";
import { assert } from "./error";
import { isFunction } from "./types";

import { type Activity, type ActivityProps, Fragment, type FragmentInstance, type SetStateAction } from "react";

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

export const SYM_REACT_FRAGMENT = Symbol.for("react.fragment");

if (import.meta.env.DEV) {
    assert(SYM_REACT_FRAGMENT as any === Fragment, "SYM_REACT_FRAGMENT does not match React.Fragment");
}

export function measureFragmentRect(fragment: FragmentInstance): DOMRect {
    return mergeAllDOMRects(fragment.getClientRects());
}
