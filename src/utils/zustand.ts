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


type StoreSelectorImpl<T> = <U>(state: T) => U;

export type StoreSelector<S> = S extends StoreApi<infer T>
    ? StoreSelectorImpl<T>
    : S extends UseBoundStore<infer T>
        ? StoreSelectorImpl<ExtractState<T>>
        : StoreSelectorImpl<S>;

export function createSelectors<S extends UseBoundStore<StoreApi<object>>>(_store: S) {
    const store = _store as WithSelectors<typeof _store>;

    store.use = {};
    for (const key of Object.keys(store.getState())) {
        (store.use as any)[key] = () => store((state) => (state as any)[key]);
    }
    // eslint-disable-next-line @eslint-react/component-hook-factories -- called once per store at top level
    store.useShallow = function useShallowStore(...args: Parameters<typeof store.useShallow>) {
        return store(useShallow(...args));
    } as any;

    return store;
}
