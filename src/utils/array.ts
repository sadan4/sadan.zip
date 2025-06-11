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