import { truthy } from "@/utils/types";

import { useRecent } from "./recent";

import { useEffect, useState } from "react";

export const enum StyleSource {
    CLASSNAME = 1 << 0,
    INLINE = 1 << 1,
    ALL = CLASSNAME | INLINE,
}

export function useCSSObserver(
    el: Element | null | undefined,
    callback: () => void,
    source: StyleSource = StyleSource.ALL,
) {
    const cb = useRecent(callback);

    const [observer] = useState(() => new MutationObserver((records: MutationRecord[]) => {
        records.forEach(() => cb.current());
    }));

    useEffect(() => {
        if (el) {
            observer.observe(el, {
                attributes: true,
                attributeFilter: [
                    source & StyleSource.CLASSNAME && "class",
                    source & StyleSource.INLINE && "style",
                ].filter(truthy),
            });
            return () => {
                // this is what i want
                // eslint-disable-next-line react-hooks/exhaustive-deps
                observer.takeRecords().forEach(() => cb.current());
                observer.disconnect();
            };
        }
    }, [el, observer, source, cb]);
}
