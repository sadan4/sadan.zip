import type { ExtractState, StoreApi, UseBoundStore } from "zustand";
import { useShallow } from "zustand/react/shallow";

type WithSelectors<S> = S extends { getState: () => infer T; }
    // eslint-disable-next-line @stylistic/indent-binary-ops
    ? S & {
        use: { [K in keyof T]: () => T[K] };
        useShallow: S extends UseBoundStore<infer A> ? {
            (): ExtractState<A>;
            <U>(selector: (state: ExtractState<A>) => U): U;
        } : never;
    }
    : never;

export function createSelectors<S extends UseBoundStore<StoreApi<object>>>(_store: S) {
    const store = _store as WithSelectors<typeof _store>;

    store.use = {};
    for (const key of Object.keys(store.getState())) {
        (store.use as any)[key] = () => store((state) => (state as any)[key]);
    }
    store.useShallow = function useShallowStore(...args: Parameters<typeof store.useShallow>) {
        return store(useShallow(...args));
    } as any;

    return store;
}
