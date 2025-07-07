export function joinWith<T, I>(array: T[], separator: I): (T | I)[] {
    if (array.length <= 1) {
        return array;
    }

    const ret: (T | I)[] = array;

    for (let i = array.length - 1; i >= 1; i--) {
        ret.splice(i, 0, separator);
    }

    return ret;
}
export function loopArrayStartingAt<T>(array: T[], idx: number): T[] {
    if (!array.length || !idx)
        return array;

    const absIdx = Math.abs(idx) % array.length;

    if (idx > 0) {
        const firstHalf = array.slice(0, absIdx);
        const secondHalf = array.slice(absIdx);

        return [...secondHalf, ...firstHalf];
    }

    const revIdx = array.length - absIdx;
    const firstHalf = array.slice(revIdx);
    const secondHalf = array.slice(0, revIdx);

    return [...firstHalf, ...secondHalf];
}
