import error from "@/utils/error";
import { clamp } from "@/utils/math";

export const enum Direction {
    HORIZONTAL,
    VERTICAL,
}

export interface Bounds {
    max: number;
    min: number;
    size: number;
    clampToBounds(num: number): number;
    toPercentage(num: number): number;
}

export function getBounds(dir: Direction, boundingElement: HTMLElement | undefined | null): Bounds {
    if (!boundingElement) {
        return {
            max: Infinity,
            min: -Infinity,
            size: Infinity,
            clampToBounds: (num: number) => num,
            toPercentage: () => 0,
        };
    }

    const { top, right, bottom, left, width, height } = boundingElement.getBoundingClientRect();
    let min: number;
    let max: number;
    let size: number;

    switch (dir) {
        case Direction.VERTICAL:
            min = left;
            max = right;
            size = width;
            break;
        case Direction.HORIZONTAL:
            min = top;
            max = bottom;
            size = height;
            break;
        default: {
            error("Invalid direction");
        }
    }

    return {
        max,
        min,
        size,
        clampToBounds(num: number) {
            return clamp(min, max, num);
        },
        toPercentage(num: number) {
            return (num - min) / (max - min);
        },
    };
}
