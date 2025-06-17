interface ToCSS {
    perspective(px: number): string;
    perspective(px: string): string;
    px(px: number): string;
}

export default {
    perspective(px: number | string): string {
        if (typeof px === "number") {
            return `perspective(${px}px)`;
        }
        return `perspective(${px})`;
    },
    px(px: number): string {
        return `${px}px`;
    },
} satisfies ToCSS as ToCSS;
