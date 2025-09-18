import { nextStateValue } from "@/utils/react";

import { useRecent } from "./recent";

import { type Dispatch, type SetStateAction, useCallback, useEffect, useRef, useState } from "react";

export interface ControlledStateOptions<T> {
    managedValue: T | undefined;
    handleChange?: (newValue: NoInfer<T>) => void;
    initialValue: T;
    debugName?: string;
}

// credit to radix
export function useControlledState<T>({ managedValue, initialValue, handleChange = () => {}, debugName = "UNKNOWN_COMPONENT" }: ControlledStateOptions<T>) {
    const [uncontrolledState, setUncontrolledState] = useState(initialValue);
    const isControlled = managedValue !== undefined;
    const value = isControlled ? managedValue : uncontrolledState;
    const latestChangeFunc = useRecent(handleChange);

    // OK to disable conditionally calling hooks here because they will always run
    // consistently in the same environment. Bundlers should be able to remove the
    // code block entirely in production.
    // eslint-disable-next-line react-hooks/react-compiler
    /* eslint-disable react-hooks/rules-of-hooks */
    if (process.env.NODE_ENV !== "production") {
        const isControlledRef = useRef(managedValue !== undefined);

        useEffect(() => {
            const wasControlled = isControlledRef.current;

            if (wasControlled !== isControlled) {
                const from = wasControlled ? "controlled" : "uncontrolled";
                const to = isControlled ? "controlled" : "uncontrolled";

                console.warn(`${debugName} is changing from ${from} to ${to}. Components should not switch from controlled to uncontrolled (or vice versa). Decide between using a controlled or uncontrolled value for the lifetime of the component.`);
            }
            isControlledRef.current = isControlled;
        }, [isControlled, debugName]);
    }

    /* eslint-enable react-hooks/rules-of-hooks */

    const setValue = useCallback<SetStateFunc<T>>((nextValue) => {
        if (isControlled) {
            const value = nextStateValue(managedValue, nextValue);

            if (value !== managedValue) {
                latestChangeFunc.current(value);
            }
        } else {
            // guh??
            const value = nextStateValue(managedValue, nextValue as any) as T;

            if (value !== uncontrolledState) {
                latestChangeFunc.current(value);
            }
            setUncontrolledState(nextValue);
        }
    }, [isControlled, latestChangeFunc, managedValue, uncontrolledState]);

    return [value, setValue] as const;
}

type SetStateFunc<T> = Dispatch<SetStateAction<T>>;
