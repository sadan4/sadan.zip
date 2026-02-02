import { type RefObject, useRef } from "react";

export function useRecent<T>(value: T): RefObject<T>;
export function useRecent<T>(value: T | null): RefObject<T | null>;
export function useRecent<T>(value: T | undefined): RefObject<T | undefined>;
export function useRecent<T>(value: T): RefObject<T> {
    const ref = useRef(value);

    // eslint-disable-next-line react-hooks/refs -- valid use case, memoing here doesnt matter anyway
    ref.current = value;

    return ref;
}
