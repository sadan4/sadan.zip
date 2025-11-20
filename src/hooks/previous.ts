import type { InitialState } from "@/utils/types";

import { useRecent } from "./recent";

import { useState } from "react";

export function usePrevious<T>(value: T, initialValue: InitialState<T>): T;
export function usePrevious<T extends {} | null>(value: T): T | undefined;
export function usePrevious<T extends {} | null>(value: T, initialValue?: InitialState<T>): T | undefined {
    const current = useRecent(value);
    const [prev, setPrev] = useState<T | undefined>(initialValue);

    if (current.current !== prev) {
        setPrev(current.current);
    }

    return prev;
}
