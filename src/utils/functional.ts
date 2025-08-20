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

export function clamp(min: number, max: number, num: number) {
    return Math.min(Math.max(num, min), max);
}

export function range(min, max) {
    return ((Math.random() * (max - min)) + min) | 0;
}
