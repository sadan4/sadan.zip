import { ellipseCircumference } from "@/utils/math";
import { dedent } from "@/utils/string";

import { parseCSSValue, PercentReference } from "./css";

export function makeBorderPath(element: Element): [length: number, path: string] {
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

    function makePath(): string {
        if (isSquare) {
            return dedent`
                M ${width / 2} 0
                H ${width}
                V ${height}
                H 0
                V 0
                Z
            `;
        }

        return dedent`
            M ${width / 2} 0
            H ${width - topRightA}
            A ${topRightA} ${topRightB} 0 0 1 ${width} ${topRightB}
            V ${height - bottomRightB}
            A ${bottomRightA} ${bottomRightB} 0 0 1 ${width - bottomRightA} ${height}
            H ${bottomLeftA}
            A ${bottomLeftA} ${bottomLeftB} 0 0 1 0 ${height - bottomLeftB}
            V ${topLeftB}
            A ${topLeftA} ${topLeftB} 0 0 1 ${topLeftA} 0
            Z
        `;
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
