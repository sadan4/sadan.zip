type Rect = DOMRectReadOnly;

export interface RectOffset {
    readonly top: number;
    readonly left: number;
}

export function compareRectOffsets(a: RectOffset, b: RectOffset): boolean {
    return a === b || (a.top === b.top && a.left === b.left);
}

export function mergeRectOffsets(a: RectOffset, b: RectOffset): RectOffset {
    return {
        top: a.top + b.top,
        left: a.left + b.left,
    };
}

export function omitRectOffset(rect: Rect, offset: RectOffset): Rect {
    return new DOMRectReadOnly(
        rect.x - offset.left,
        rect.y - offset.top,
        rect.width,
        rect.height,
    );
}

export const NO_OFFSET = Object.freeze({
    top: 0,
    left: 0,
} satisfies RectOffset);

export function makeEmptyDomRect(): DOMRect {
    return new DOMRect(0, 0, 0, 0);
}

export function mergeDOMRects(a: Rect, b: Rect): Rect {
    const x = Math.min(a.left, b.left);
    const y = Math.min(a.top, b.top);
    const width = Math.max(a.width, b.width);
    const height = Math.max(a.height, b.height);

    return new DOMRectReadOnly(x, y, width, height);
}

export function mergeAllDOMRects(rects: Rect[]): Rect {
    if (!rects.length) {
        return makeEmptyDomRect();
    }
    return rects.reduce(mergeDOMRects);
}

export function cloneRect(rect: Rect): DOMRect {
    return new DOMRect(rect.x, rect.y, rect.width, rect.height);
}

/**
 * removes a margin from each edge of the rect, making it's area smaller
 */
export function removeMarginFromRect(rect: Rect, margin: number): Rect {
    const m2 = margin * 2;

    return new DOMRectReadOnly(
        rect.x + margin,
        rect.y + margin,
        rect.width - m2,
        rect.height - m2,
    );
}

/**
 * adds a margin to each edge of the rect, making it's area larger
 */
export function addMarginToRect(rect: Rect, margin: number): Rect {
    return removeMarginFromRect(rect, -margin);
}

export function rectWidthContainedBy(inner: Rect, outer: Rect): boolean {
    return inner.left >= outer.left && inner.right <= outer.right;
}

export function rectHeightContainedBy(inner: Rect, outer: Rect): boolean {
    return inner.top >= outer.top && inner.bottom <= outer.bottom;
}

export function rectFullyContainedBy(inner: Rect, outer: Rect): boolean {
    return rectWidthContainedBy(inner, outer) && rectHeightContainedBy(inner, outer);
}

export function rectWidthCanBeContainedBy(inner: Rect, outer: Rect): boolean {
    return inner.width <= outer.width;
}

export function rectHeightCanBeContainedBy(inner: Rect, outer: Rect): boolean {
    return inner.height <= outer.height;
}

export function rectCanBeFullyContainedBy(inner: Rect, outer: Rect): boolean {
    return rectWidthCanBeContainedBy(inner, outer) && rectHeightCanBeContainedBy(inner, outer);
}

export function computeRectClipOffsets(el: Rect, bounds: Rect): RectOffset {
    if (rectFullyContainedBy(el, bounds)) {
        return NO_OFFSET;
    }

    let top = 0;

    if (rectHeightCanBeContainedBy(el, bounds)) {
        // too far up
        if (el.top < bounds.top) {
            top = bounds.top - el.top;
        // too far down
        } else if (el.bottom > bounds.bottom) {
            top = bounds.bottom - el.bottom;
        }
    }

    let left = 0;

    if (rectWidthCanBeContainedBy(el, bounds)) {
        // too far left
        if (el.left < bounds.left) {
            left = bounds.left - el.left;
        // too far right
        } else if (el.right > bounds.right) {
            left = bounds.right - el.right;
        }
    }

    return {
        top,
        left,
    };
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

export function compareDomRects(a: Rect, b: Rect): boolean {
    return a === b || (
        a.x === b.x
        && a.y === b.y
        && a.width === b.width
        && a.height === b.height
    );
}
