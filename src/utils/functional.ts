import _ from "lodash";

export function once<T extends (...args: any[]) => any>(fn: T): T {
    let called = false;
    let result: ReturnType<T>;

    return ((...args: Parameters<T>) => {
        if (called) {
            return result;
        }
        called = true;
        result = fn(...args);
        return result;
    }) as T;
}

export function pick<T extends object, K extends keyof T>(keys: K | K[]): (obj: T) => Pick<T, K> {
    return (obj: T): Pick<T, K> => {
        return _.pick(obj, keys);
    };
}
