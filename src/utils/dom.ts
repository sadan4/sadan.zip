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
