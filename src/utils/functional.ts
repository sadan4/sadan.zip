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

export function prop<O extends object, K extends keyof O>(key: K): (obj: O) => O[K] {
    return (obj) => obj[key];
}

export function debounce<
    F extends (...args: any) => any,
>(func: F, delay = 300): (...args: Parameters<F>) => undefined {
    let timeout: number | NodeJS.Timeout;

    return function (...args: Parameters<F>): undefined {
        clearTimeout(timeout);
        timeout = setTimeout(() => func(...args), delay);
    };
}
