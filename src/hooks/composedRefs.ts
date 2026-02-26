import { type Ref, type RefCallback, useCallback } from "react";

export function useComposedRefs<T>(...refs: (Ref<T> | undefined)[]): RefCallback<T> {
    return useCallback<RefCallback<T>>((instance) => {
        const cleanups: (() => void)[] = [];

        for (const ref of refs) {
            if (!ref)
                continue;
            if (typeof ref === "function") {
                const maybeCleanup = ref(instance);

                if (typeof maybeCleanup === "function") {
                    cleanups.push(maybeCleanup);
                }
            } else {
                ref.current = instance;
            }
        }

        return () => {
            const errs: Error[] = [];

            for (const cleanup of cleanups) {
                try {
                    cleanup();
                } catch (e: any) {
                    errs.push(e);
                    console.error(new Error("Ref cleanup failed", { cause: e }));
                }
            }
            if (errs.length) {
                throw new AggregateError(errs, "Ref cleanup failed");
            }
        };
    // eslint-disable-next-line @eslint-react/exhaustive-deps -- this is correct
    }, refs);
}
