import { deepEqual } from "@/utils/obj";
import { nextStateValue } from "@/utils/react";
import { isFunction } from "@/utils/types";

import { type Dispatch, type SetStateAction, useCallback, useState } from "react";

export function useDeepState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>] {
    const [state, setState] = useState<S>(isFunction(initialState) ? initialState() : initialState);

    const updateState = useCallback((action: SetStateAction<S>) => setState((prevState) => {
        const nextState = nextStateValue(prevState, action);

        return deepEqual(prevState, nextState) ? prevState : nextState;
    }), []);

    return [state, updateState];
}
