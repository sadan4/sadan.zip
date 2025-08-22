class ToCSS {
    perspective(px: number): string;
    perspective(px: string): string;
    perspective(_px: number | string): string {
        const px = typeof _px === "number" ? this.px(_px) : _px;

        return `perspective(${px})`;
    }

    px(px: number): string;
    px(px: number): string {
        return `${px}px`;
    }

    url(url: string): string;
    url(url: string, fragment: string): string;
    url(url: string, fragment?: string): string {
        if (fragment) {
            return `url(${url}#${fragment})`;
        }
        return `url(${url})`;
    }

    translate(x: number | string, y: number | string): string;
    translate(_x: number | string, _y: number | string): string {
        const x = typeof _x === "number" ? this.px(_x) : _x;
        const y = typeof _y === "number" ? this.px(_y) : _y;

        return `${x} ${y}`;
    }

    rotate(deg: number | string): string;
    rotate(deg: number | string, x: number | string, y: number | string): string;
    rotate(deg: number | string, x?: number | string, y?: number | string): string {
        if (x === undefined) {
            return `rotate(${deg})`;
        }
        return `rotate(${deg} ${x} ${y})`;
    }
}
export default new ToCSS();
