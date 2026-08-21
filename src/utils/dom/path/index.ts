import { ellipseCircumference } from "@/utils/math";

import * as ir from "./ir";
import { parseCSSValue, PercentReference } from "../css";

export function compilePath(path: ir.PathNode[]): string {
    return path.flat().join(" ");
}

function filterNan(value: number): number {
    return isNaN(value) ? Infinity : value;
}

export function makeBorderPath(element: Element): [length: number, path: ir.PathNode[]] {
    const { width, height } = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    let [topLeftA, topLeftB] = parseRadius(style.borderTopLeftRadius);
    let [topRightA, topRightB] = parseRadius(style.borderTopRightRadius);
    let [bottomRightA, bottomRightB] = parseRadius(style.borderBottomRightRadius);
    let [bottomLeftA, bottomLeftB] = parseRadius(style.borderBottomLeftRadius);
    let isSquare = true;
    let rectLength = 2 * (width + height);

    normalizeRadii();

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

    /**
     * normalize the radii according to https://drafts.csswg.org/css-backgrounds/#corner-overlap
     * 
     * this prevents producing invalid paths when the radii are too large for the box size
     * eg: border-radius: 999999px; to fully round a small rectangle
     */
    function normalizeRadii(): void {
        // Let f = min(L_i/S_i)
        // where i ∈ {top, right, bottom, left}
        // S_i is the sum of the two corresponding radii of the corners on side i,
        const S_top = topLeftA + topRightA;
        const S_right = topRightB + bottomRightB;
        const S_bottom = bottomRightA + bottomLeftA;
        const S_left = bottomLeftB + topLeftB;
        // and L_top = L_bottom = the width of the box,
        const L_top = width;
        const L_bottom = width;
        // and Lleft = Lright = the height of the box
        const L_left = height;
        const L_right = height;

        const values = [
            L_top / S_top,
            L_right / S_right,
            L_bottom / S_bottom,
            L_left / S_left,
        ].map(filterNan);

        // Let f = min(L_i/S_i)
        const f = Math.min(1, ...values);

        // If f < 1, then all corner radii are reduced by multiplying them by f.
        if (f < 1) {
            topLeftA *= f;
            topLeftB *= f;
            topRightA *= f;
            topRightB *= f;
            bottomRightA *= f;
            bottomRightB *= f;
            bottomLeftA *= f;
            bottomLeftB *= f;
        }
    }

    function parseRadius(radius: string): [a: number, b: number] {
        if (!radius) {
            return [0, 0];
        }

        let a: string,
            b = a = radius;

        if (radius.includes(" ")) {
            [a, b] = radius.split(" ");
        }

        const parsedA: number = parseCSSValue(a, element, PercentReference.WIDTH);
        const parsedB: number = parseCSSValue(b, element, PercentReference.HEIGHT);

        return [parsedA, parsedB];
    }
}
