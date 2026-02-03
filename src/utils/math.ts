export function clamp(min: number, max: number, value: number): number {
    return Math.min(Math.max(value, min), max);
}

export function isNaN(value: number): boolean {
    return Number.isNaN(value);
}

export function randInt(min: number, max: number) {
    return Math.floor((Math.random() * (max - min)) + min);
}

export function ellipseCircumference(a: number, b: number): number {
    a = Math.abs(a);
    b = Math.abs(b);
    if (Number.isNaN(a) || Number.isNaN(b))
        return NaN;
    if (a === 0 && b === 0)
        return 0;
    if (a === b)
        return 2 * Math.PI * a;

    const h = ((a - b) / (a + b)) ** 2;

    // Ramanujan's second approximation (accurate and simple)
    return Math.PI * (a + b) * (1 + ((3 * h) / (10 + Math.sqrt(4 - (3 * h)))));
}

export const PI2 = Math.PI * 2;
