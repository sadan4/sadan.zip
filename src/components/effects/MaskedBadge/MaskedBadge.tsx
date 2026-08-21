import { useForceUpdater } from "@/hooks/forceUpdater";
import { useResizeObserverFromRef } from "@/hooks/resizeObserver";
import { single } from "@/utils/array";
import { cn } from "@/utils/cn";
import { blackBox } from "@/utils/constants";
import { compilePath, makeBorderPath } from "@/utils/dom/path";
import { offsetPath } from "@/utils/dom/path/transform";
import { measureRect } from "@/utils/dom/rect";

import { BadgePosition } from "./enums";
import * as styles from "./styles.module.scss";

import { type PropsWithChildren, type ReactNode, type Ref, useId, useImperativeHandle, useLayoutEffect, useRef, useState } from "react";

export interface MaskedBadgeProps extends PropsWithChildren {
    renderMask(): ReactNode;
    /**
     * The position of the badge.
     * @default BadgePosition.BOTTOM_RIGHT
     */
    position?: BadgePosition;
    ref?: Ref<MaskedBadge.Handle>;
    /**
     * the gap cut around the badge, in pixels.
     *
     * @default 6px
     */
    borderWidth?: number;
}

const positionClassMap: Record<BadgePosition, string> = {
    [BadgePosition.BOTTOM_RIGHT]: styles.bottomRight,
    [BadgePosition.TOP_RIGHT]: styles.topRight,
    [BadgePosition.BOTTOM_LEFT]: styles.bottomLeft,
    [BadgePosition.TOP_LEFT]: styles.topLeft,
};

export namespace MaskedBadge {
    export interface Handle {
        recalculatePath(): void;
    }
}

export function MaskedBadge({
    children,
    renderMask,
    position = BadgePosition.BOTTOM_RIGHT,
    borderWidth = 6,
    ref: _ref,
}: MaskedBadgeProps) {
    const contentRef = useRef<HTMLDivElement>(null);
    const maskElRef = useRef<HTMLDivElement>(null);
    const [pathDep, forceUpdatePath] = useForceUpdater();
    const [contentPath, setContentPath] = useState("");
    const [holePath, setHolePath] = useState("");
    const maskId = useId();


    useImperativeHandle(_ref, () => ({
        recalculatePath() {
            forceUpdatePath();
        },
    }), []);

    useResizeObserverFromRef(maskElRef, forceUpdatePath);
    useResizeObserverFromRef(contentRef, forceUpdatePath);

    useLayoutEffect(() => {
        const maskEl = maskElRef.current;
        const contentEl = contentRef.current;

        if (maskEl && contentEl) {
            const badgeEl = single(maskEl.children);
            const badgeRect = measureRect(badgeEl);
            const contentRect = measureRect(contentEl);
            // the mask is applied to the content element, so its border box is the mask's user space
            const [, contentIR] = makeBorderPath(contentEl);
            // the gap around the badge is baked into the geometry instead of stroked,
            // so borderWidth is the real width of the gap and square corners stay square
            const [, badgeIR] = makeBorderPath(badgeEl, borderWidth);
            const dx = badgeRect.x - contentRect.x;
            const dy = badgeRect.y - contentRect.y;

            setContentPath(compilePath(contentIR));
            setHolePath(compilePath(offsetPath(badgeIR, dx, dy)));
        }
        blackBox(pathDep);
    }, [pathDep, borderWidth]);


    return (
        <div className="relative size-fit">
            <svg
                aria-hidden
                className={styles.maskHost}
            >
                <mask
                    id={maskId}
                    className={styles.mask}
                >
                    <path
                        d={contentPath}
                        className={styles.body}
                    />
                    <path
                        className={styles.hole}
                        d={holePath}
                    />
                </mask>
            </svg>
            <div
                ref={contentRef}
                className="h-max w-max"
                style={{
                    mask: `url("#${maskId}")`,
                }}
            >
                {children}
            </div>
            <div
                className={cn(styles.badge, positionClassMap[position])}
                ref={maskElRef}
            >
                {renderMask()}
            </div>
        </div>
    );
}
