import type { AssertedType0, AssertedType1 } from "./types";

export {
    default as deepEqual,
} from "fast-deep-equal";

export function mapObject<T extends Object, U>(
    obj: T,
    fn: (value: T[keyof T], key: keyof T) => U,
): { [K in keyof T]: U } {
    const result = {} as { [K in keyof T]: U };

    for (const key in obj) {
        result[key] = fn(obj[key], key);
    }
    return result;
}

export function filterObject<
    T extends Object,
    K extends keyof T,
    V extends T[K],
    U extends V,
    F extends (key: K, value: V) => value is U,
>(
    obj: T,
    fn: F,
): { -readonly [K in keyof T as T[K] extends AssertedType1<F> ? K : never]: T[K] };
export function filterObject<
    T extends Object,
    K extends keyof T,
    V extends T[K],
    U extends K,
    F extends (key: K, value: V) => key is U,
>(
    obj: T,
    fn: F,
): { -readonly [K in keyof T as K extends AssertedType0<F> ? K : never]: T[K] };
export function filterObject<T extends Object>(obj: T, fn: (key: keyof T, value: T[keyof T]) => boolean): Partial<T>;
export function filterObject<T extends Object>(obj: T, fn: (key: keyof T, value: T[keyof T]) => boolean): Partial<T> {
    return Object.fromEntries(Object.entries(obj).filter(([key, value]) => fn(key as keyof T, value))) as any;
}

export function pick<T extends Object, K extends keyof T>(obj: T, keys: K[]): Pick<T, K> {
    type R = Pick<T, K>;

    const result: Partial<R> = {};

    for (let i = 0; i < keys.length; i++) {
        const key = keys[i];

        result[key] = obj[key];
    }

    return result as R;
}
