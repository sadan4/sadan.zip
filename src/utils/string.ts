export function dedent(literals: string): string;
export function dedent(strings: TemplateStringsArray, ...values: unknown[]): string;
export function dedent(
    strings: TemplateStringsArray | string,
    ...values: unknown[]
) {
    const raw = typeof strings === "string" ? [strings] : strings.raw;
    const escapeSpecialCharacters = Array.isArray(strings);
    // first, perform interpolation
    let result = "";

    for (let i = 0; i < raw.length; i++) {
        let next = raw[i];

        if (escapeSpecialCharacters) {
            // handle escaped newlines, backticks, and interpolation characters
            next = next
                .replace(/\\\n[ \t]*/g, "")
                .replace(/\\`/g, "`")
                .replace(/\\\$/g, "$")
                .replace(/\\\{/g, "{");
        }

        result += next;

        if (i < values.length) {
            const value = alignValue(values[i], result);


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

    // handle escaped newlines at the end to ensure they don't get stripped too
    if (escapeSpecialCharacters) {
        result = result.replace(/\\n/g, "\n");
    }

    return result;
}

/**
 * Adjusts the indentation of a multi-line interpolated value to match the current line.
 */
function alignValue(value: unknown, precedingText: string): string | unknown {
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
