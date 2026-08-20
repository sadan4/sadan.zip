import { ellipseCircumference } from "@/utils/math";

import * as ir from "./ir";
import { parseCSSValue, PercentReference } from "../css";

export function compilePath(path: ir.PathNode[]): string {
    return path.flat().join(" ");
}

export function makeBorderPath(element: Element): [length: number, path: ir.PathNode[]] {
    const { width, height } = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    const [topLeftA, topLeftB] = normalizeRadius(style.borderTopLeftRadius);
    const [topRightA, topRightB] = normalizeRadius(style.borderTopRightRadius);
    const [bottomRightA, bottomRightB] = normalizeRadius(style.borderBottomRightRadius);
    const [bottomLeftA, bottomLeftB] = normalizeRadius(style.borderBottomLeftRadius);
    let isSquare = true;
    let rectLength = 2 * (width + height);

    rectLength += calcRadiusDelta(topLeftA, topLeftB);
    rectLength += calcRadiusDelta(topRightA, topRightB);
    rectLength += calcRadiusDelta(bottomRightA, bottomRightB);
    rectLength += calcRadiusDelta(bottomLeftA, bottomLeftB);

    const path = makePath();

    return [rectLength, path];

    function makePath(): ir.PathNode[] {
        if (isSquare) {
            return [
                ir.moveAbs(width / 2, 0),
                ir.hLineAbs(width),
                ir.vLineAbs(height),
                ir.hLineAbs(0),
                ir.vLineAbs(0),
                ir.closePath(),
            ];
        }
        return [
            ir.moveAbs(width / 2, 0),
            ir.hLineAbs(width - topRightA),
            ir.arcAbs(topRightA, topRightB, 0, false, true, width, topRightB),
            ir.vLineAbs(height - bottomRightB),
            ir.arcAbs(bottomRightA, bottomRightB, 0, false, true, width - bottomRightA, height),
            ir.hLineAbs(bottomLeftA),
            ir.arcAbs(bottomLeftA, bottomLeftB, 0, false, true, 0, height - bottomLeftB),
            ir.vLineAbs(topLeftB),
            ir.arcAbs(topLeftA, topLeftB, 0, false, true, topLeftA, 0),
            ir.closePath(),
        ];
    }

    /**
     * return the difference between the length of the curve and the sum of the radii
     */
    function calcRadiusDelta(a: number, b: number): number {
        if (!a && !b) {
            return 0;
        }
        isSquare = false;

        const curveLen = ellipseCircumference(a, b) / 4;
        const delta = curveLen - (a + b);

        return delta;
    }

    function normalizeRadius(radius: string): [a: number, b: number] {
        if (!radius) {
            return [0, 0];
        }

        let a: string,
            b = a = radius;

        if (radius.includes(" ")) {
            [a, b] = radius.split(" ");
        }

        const parsedA: number = Math.min(parseCSSValue(a, element, PercentReference.WIDTH), width / 2);
        const parsedB: number = Math.min(parseCSSValue(b, element, PercentReference.HEIGHT), height / 2);

        return [parsedA, parsedB];
    }
}
