import { type RefObject, useRef } from "react";

export function useRecent<T>(value: T): RefObject<T>;
export function useRecent<T>(value: T | null): RefObject<T | null>;
export function useRecent<T>(value: T | undefined): RefObject<T | undefined>;
export function useRecent<T>(value: T): RefObject<T> {
    const ref = useRef(value);

    ref.current = value;

    return ref;
}
