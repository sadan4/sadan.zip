import { assert } from "./error";
import { isFunction } from "./types";

import { type Activity, type ActivityProps, Fragment, type ReactNode, type SetStateAction } from "react";

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

export function areChildrenEmpty(children: ReactNode): boolean {
    if (children == null
      || children === true
      || children === false
      || children === ""
    ) {
        return true;
    }
    if (Array.isArray(children)) {
        if (!children.length) {
            return true;
        }
        return children.every(areChildrenEmpty);
    }
    if (typeof children === "object" && "type" in children) {
        if (children.type === Fragment) {
            return areChildrenEmpty((children.props as any).children as ReactNode);
        }
    }
    return false;
}
