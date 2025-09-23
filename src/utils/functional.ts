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

export function snap(a: number, b: number, num: number) {
    const da = Math.abs(a - num);
    const db = Math.abs(b - num);

    return da < db ? a : b;
}

export function clamp(min: number, max: number, num: number) {
    return Math.min(Math.max(num, min), max);
}

export function range(min, max) {
    return ((Math.random() * (max - min)) + min) | 0;
}

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

export function prop<O extends object, K extends keyof O>(key: K): (obj: O) => O[K] {
    return (obj) => obj[key];
}
