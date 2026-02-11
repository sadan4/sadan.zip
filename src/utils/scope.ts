export function defer(fn: () => void): Disposable {
    return {
        [Symbol.dispose]() {
            fn();
        },
    };
}
