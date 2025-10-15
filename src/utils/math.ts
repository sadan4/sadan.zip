export function clamp(min: number, max: number, value: number): number {
    return Math.min(Math.max(value, min), max);
}

export function isNaN(value: number): boolean {
    return Number.isNaN(value);
}

export function range(min, max) {
    return Math.floor((Math.random() * (max - min)) + min);
}
