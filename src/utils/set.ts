export function toggleSetItem<T>(set: Set<T>, item: T): Set<T> {
    if (set.has(item)) {
        set.delete(item);
    } else {
        set.add(item);
    }

    return set;
}
