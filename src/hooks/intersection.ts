import { useDeepMemo } from "./deepMemo";
import { useRecent } from "./recent";

import invariant from "invariant";
import { type RefCallback, type RefObject, useEffect, useState } from "react";

interface UseIntersectionOpts extends IntersectionObserverInit {
    /**
     * Pass a ref object to be used as the root for the intersection observer.
     */
    rootRef?: RefObject<Element | null>;
}

export function getNewestEntry(entries: IntersectionObserverEntry[]): IntersectionObserverEntry {
    // should always have at least one entry
    invariant(entries.length > 0, "entries is empty");
    if (entries.length === 1)
        return entries[0];

    return entries.reduce((best, cur) => {
        return cur.time > best.time ? cur : best;
    });
}

export function useIntersection(
    _callback: IntersectionObserverCallback,
    _opts: UseIntersectionOpts,
): RefCallback<Element | null> {
    const [el, setEl] = useState<Element | null>(null);
    const callback = useRecent(_callback);
    const opts = useDeepMemo(_opts);

    useEffect(() => {
        if (!el)
            return;

        const { rootRef, root = rootRef?.current, ...rest } = opts;

        if (root == null) {
            console.error("useIntersection: root is null");
            return;
        }

        const observer = new IntersectionObserver((entries, observer) => {
            callback.current(entries.filter(({ target }) => target === el), observer);
        }, {
            root,
            ...rest,
        });

        observer.observe(el);

        return () => {
            observer.unobserve(el);
        };
    }, [callback, el, opts]);

    return setEl;
}
