export function dedent(literals: string): string;
export function dedent(strings: TemplateStringsArray, ...values: unknown[]): string;
export function dedent(
    strings: TemplateStringsArray | string,
    ...values: unknown[]
) {
    /**
     * https://github.com/dmnd/dedent
     * @license MIT
     */
    const raw = typeof strings === "string" ? [strings] : strings.raw;
    // first, perform interpolation
    let result = "";

    for (let i = 0; i < raw.length; i++) {
        result += raw[i];

        if (i < values.length) {
            const value = alignValue(values[i], result);

            // oxlint-disable-next-line typescript/restrict-plus-operands
            result += value;
        }
    }

    // now strip indentation
    const lines = result.split("\n");
    let mindent: null | number = null;

    for (const l of lines) {
        const m = l.match(/^(\s+)\S+/);

        if (m) {
            const indent = m[1].length;

            if (!mindent) {
                // this is the first indented line
                mindent = indent;
            } else {
                mindent = Math.min(mindent, indent);
            }
        }
    }

    if (mindent !== null) {
        const m = mindent; // appease TypeScript

        result = lines
        // https://github.com/typescript-eslint/typescript-eslint/issues/7140

            .map((l) => (l[0] === " " || l[0] === "\t" ? l.slice(m) : l))
            .join("\n");
    }

    // dedent eats leading and trailing whitespace too
    result = result.trim();

    return result;
}

/**
 * Adjusts the indentation of a multi-line interpolated value to match the current line.
 */
function alignValue(value: string, precedingText: string): string;
/**
 * Adjusts the indentation of a multi-line interpolated value to match the current line.
 */
function alignValue<T>(value: T, precedingText: string): T;
function alignValue<T>(value: T, precedingText: string): string | T {
    if (typeof value !== "string" || !value.includes("\n")) {
        return value;
    }

    const currentLine = precedingText.slice(precedingText.lastIndexOf("\n") + 1);
    const indentMatch = currentLine.match(/^(\s+)/);

    if (indentMatch) {
        const [indent] = indentMatch;

        return value.replace(/\n/g, `\n${indent}`);
    }

    return value;
}

export function popcnt(value: number): number {
    let i = 0;
    while (value) { 
        value &= value - 1;
        i++;
    }
    return i;
}