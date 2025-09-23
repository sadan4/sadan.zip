import { isFunction } from "./types";

import type { SetStateAction } from "react";

export function nextStateValue<T>(prevState: T, nextState: SetStateAction<T>): T {
    return isFunction(nextState) ? nextState(prevState) : nextState;
}
