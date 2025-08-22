/**
 * Inserts a separator between each pair of elements in the given array and returns the mutated array.
 *
 * @typeParam T - Type of the elements in the input array.
 * @typeParam I - Type of the separator to insert.
 * @param array - The input array to interleave with the separator. This array is mutated in-place.
 * @param separator - The value to insert between each pair of elements.
 * @returns The same array instance mutated to contain the original elements interleaved with the separator. The returned array has type (T | I)[]. For an input array of length n (n > 0) the resulting length will be 2n - 1.
 * @remarks
 * - If `array.length <= 1`, the function returns the input array unchanged.
 * - This function mutates the provided array using repeated `splice` calls. If you need to preserve the original array, pass a shallow copy (e.g. `joinWith(array.slice(), separator)`).
 * - If the separator type `I` is the same as `T`, separator values may be indistinguishable from original elements.
 * - Performance: repeated mid-array splices make this O(n^2) time in the worst case; it operates in-place with O(1) additional space.
 * @example
 * const arr = ['a', 'b', 'c'];
 * joinWith(arr, '-'); // => ['a', '-', 'b', '-', 'c'] and `arr` is mutated
 */
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
export function joinWithKey<T, I>(array: T[], separator: (i: number) => I): (T | I)[] {
    if (array.length <= 1) {
        return array;
    }

    const ret: (T | I)[] = array;

    for (let i = array.length - 1; i >= 1; i--) {
        ret.splice(i, 0, separator(i));
    }

    return ret;
}
/**
 * Rotates an array so that the element at the specified index becomes the first element
 * of the returned array.
 *
 * Behavior:
 * - Positive `idx` rotates left by `idx` (elements before `idx` are moved to the end).
 * - Negative `idx` rotates right by `|idx|` (last `|idx|` elements are moved to the front).
 *
 * Implementation notes:
 * - The effective offset is computed as `Math.abs(idx) % array.length`.
 * - For positive `idx`, the element at position `(idx % length)` becomes the first element.
 * - For negative `idx`, the element at position `(length - (|idx| % length))` becomes the first.
 * - The function does not mutate the input array; it returns a new array in the common case.
 * - If the input array is empty or `idx` is falsy (for example `0`, `NaN`, `undefined`), the original array reference is returned.
 * - Non-integer `idx` values will be coerced by the numeric operations; callers should prefer integers.
 *
 * Edge cases:
 * - When `idx` is a non-zero multiple of `array.length` the returned array will contain the same elements
 *   in the same order but may be a distinct array instance.
 *
 * @template T
 * @param {T[]} array - The array to rotate.
 * @param {number} idx - Starting index; positive to rotate left, negative to rotate right.
 * @returns {T[]} A rotated array where the normalized start index is the first element.
 *
 * @example
 * loopArrayStartingAt([1, 2, 3, 4], 1); // -> [2, 3, 4, 1]
 *
 * @example
 * loopArrayStartingAt([1, 2, 3, 4], -1); // -> [4, 1, 2, 3]
 */
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

/**
 * {@link Object.keys} but typed
 */
export function keys<T extends Object>(obj: T): (keyof T)[] {
    return Object.keys(obj) as (keyof T)[];
}
