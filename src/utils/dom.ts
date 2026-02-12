import { error, unreachable } from "./error";
import { ellipseCircumference } from "./math";
import { dedent } from "./string";

export function getLineHeight(element: Element) {
    // Get computed style
    const computedStyle = window.getComputedStyle(element);
    // Get the line-height property
    const { lineHeight } = computedStyle;

    // If line-height is 'normal', calculate it as 1.2 * font-size (typical default)
    if (lineHeight === "normal") {
        const fontSize = parseFloat(computedStyle.fontSize);

        return fontSize * 1.2;
    }

    // If line-height is a number (unitless), multiply by font-size
    if (/^\d+(\.\d+)?$/.test(lineHeight)) {
        const fontSize = parseFloat(computedStyle.fontSize);

        return fontSize * parseFloat(lineHeight);
    }

    // Otherwise, line-height is in px, em, rem, etc.
    return parseFloat(lineHeight);
}

export function isMobileDevice(): boolean {
    // i guess people with laptops are fucked (myself included)
    return navigator.maxTouchPoints > 0;
}


/**
 * Gives the default value for an <input type="range" /> element when the default value is not provided
 */
export function rangeInputDefaultValue(min = 0, max = 100) {
    // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/range#value
    return max < min
        ? min
        : min + ((max - min) / 2);
}
function isDigit(char: string): boolean {
    return /^\d$/.test(char);
}

const CSS_VALUE_REGEX = /^(\d*(?:\.\d+)?(?:[eE][+-]?\d+)?)(%|r?em|px)$/;

export const enum PercentReference {
    WIDTH,
    HEIGHT,
}

function getPercentReferenceValue(element: Element, reference: PercentReference): number {
    switch (reference) {
        case PercentReference.WIDTH:
            return element.clientWidth;
        case PercentReference.HEIGHT:
            return element.clientHeight;
        default:
            unreachable();
    }
}

export function parseCSSValue(value: string, element: Element, percentReference: PercentReference): number {
    // px
    if (isDigit(value.slice(-1))) {
        return parseFloat(value);
    }

    const [, num, unit] = value.match(CSS_VALUE_REGEX) ?? [];

    switch (unit) {
        case "%": {
            const referenceValue = getPercentReferenceValue(element, percentReference);

            return (parseFloat(num) / 100) * referenceValue;
        }
        case "em":
        case "rem": {
            const fontSize = parseFloat(getComputedStyle(unit === "rem" ? document.documentElement : element).fontSize);

            return fontSize * parseFloat(num);
        }
        case "px":
            return parseFloat(num);
        default:
            error(`unhandled css value: ${value}`);
    }
}

export function measureRect(element: Element): DOMRect {
    const { display } = getComputedStyle(element);

    if (display !== "contents") {
        return element.getBoundingClientRect();
    }

    if (element.children.length === 1) {
        const [child] = element.children;
        const { display } = getComputedStyle(child);

        if (display === "contents") {
            return measureRect(child);
        }
    }

    const range = document.createRange();

    range.selectNodeContents(element);
    return range.getBoundingClientRect();
}

/**
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons|MDN}
 */
export const enum MouseButtons {
    /**
     * No button or un-initialized
     */
    NONE = 0,
    /**
     * Primary button (usually the left button)
     */
    PRIMARY = 1,
    /**
     * Secondary button (usually the right button)
     */
    SECONDARY = 2,
    /**
     * Auxiliary button (usually the mouse wheel button or middle button)
     */
    AUXILIARY = 4,
    /**
     * 4th button (typically the "Browser Back" button)
     */
    BACK = 8,
    /**
     * 5th button (typically the "Browser Forward" button)
     */
    FORWARD = 16,
}

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
     * return the difference bewteen the length of the curve and the sum of the radii
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
