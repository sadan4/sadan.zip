export function sleep(ms: number): Promise<void> {
    return new Promise((res) => setTimeout(res, ms));
}

export function animatedSleep(ms: number): Promise<void> {
    const now = performance.now();

    return new Promise((resolve) => {
        function frame(newNow: number) {
            if (newNow - now >= ms) {
                resolve();
            } else {
                requestAnimationFrame(frame);
            }
        }
        requestAnimationFrame(frame);
    });
}

export function withResolvers<T>(): {
    promise: Promise<T>;
    resolve(value: T | PromiseLike<T>): void;
    reject(reason?: any): void;
} {
    let resolve!: (value: T | PromiseLike<T>) => void;
    let reject!: (reason?: any) => void;

    const promise = new Promise<T>((res, rej) => {
        resolve = res;
        reject = rej;
    });

    return {
        promise,
        resolve,
        reject,
    };
}
