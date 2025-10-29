import { error } from "@/utils/error";

import type { RefObject } from "react";

export interface UseResizeObserverCallback {
    (entry: ResizeObserverEntry, observer: ResizeObserver): void;
}

export function useResizeObserver<T extends Element>(target: RefObject<T> | T | null, callback: UseResizeObserverCallback): ResizeObserver {
    error("todo");
}
