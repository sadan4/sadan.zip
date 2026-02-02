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
