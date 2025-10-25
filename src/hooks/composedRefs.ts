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
            let err: Error | undefined;

            for (const cleanup of cleanups) {
                try {
                    cleanup();
                } catch (e: any) {
                    err ??= new Error("Ref cleanup failed", { cause: e });
                    console.error(err);
                }
            }
            if (err) {
                throw err;
            }
        };
    }, refs);
}
