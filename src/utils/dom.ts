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

const CSS_VALUE_REGEX = /^(\d*(?:\.\d+)?)(%|rem|px)$/;

export function parseCSSValue(value: string, element: Element): number {
    if (isDigit(value.slice(-1))) {
        return parseFloat(value);
    }

    const [, num, unit] = value.match(CSS_VALUE_REGEX) ?? [];

    switch (unit) {
        case "%": {
            const parentHeight = element.clientHeight;

            return (parseFloat(num) / 100) * parentHeight;
        }
        case "em":
        case "rem": {
            const fontSize = parseFloat(getComputedStyle(unit === "rem" ? document.documentElement : element).fontSize);

            return fontSize * parseFloat(num);
        }
        case "px":
            return parseFloat(num);
        default:
            throw new Error(`unhandled css value: ${value}`);
    }
}
