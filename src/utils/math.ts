export function clamp(min: number, max: number, value: number): number {
    return Math.min(Math.max(value, min), max);
}

export function isNaN(value: number): boolean {
    return Number.isNaN(value);
}
