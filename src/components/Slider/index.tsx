import { useControlledState } from "@/hooks/controlledState";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useResizeObserver } from "@/hooks/resizeObserver";
import cn from "@/utils/cn";
import { EMPTY_ARRAY } from "@/utils/constants";
import { makeDefaultForInputRange } from "@/utils/dom";
import { parseCSSValue, PercentReference } from "@/utils/dom/css";
import { assert } from "@/utils/error";
import { clamp } from "@/utils/math";

import * as styles from "./styles.module.scss";
import { HorizontalLine } from "../Lines";
import { VerticalLine } from "../Lines/VerticalLine";

import { type ComponentProps, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

const sliderSizes = {
    xs: styles.xs,
    sm: styles.sm,
    md: styles.md,
    lg: styles.lg,
};

export interface SliderProps extends Omit<ComponentProps<"input">, "onChange" | "size"> {
    /**
     * Defaults to 0
     */
    min?: number;
    /**
     * Defaults to 100
     */
    max?: number;
    /**
     * calculated with below if not provided
     * ```js
     * max < min
     *   ? min
     *   : min + (max - min) / 2;
     * ```
     */
    initialValue?: number;
    value?: number;
    disabled?: boolean;
    /**
     * the size of the slider, defaults to {@link sliderSizes.sm | sm}
     */
    size?: keyof typeof sliderSizes;
    vertical?: boolean;
    reverseVertical?: boolean;
    /**
     * called when the value changes
     */
    onChange?(value: number): void;
    markers?: readonly number[];
    /**
     * If false, don't show the markers provided
     */
    showMarkers?: boolean;
    /**
     * If true, only allow selecting values that match the markers
     */
    stickToMarkers?: boolean;
    renderMarkers?(props: RenderMarkersProps): ReactNode;
    renderMarker?(props: RenderMarkerProps): ReactNode;
}

declare module "react" {
    interface CSSProperties {
        /**
         * Custom property for slider progress
         */
        "--progress"?: number;
    }
}

export function Slider({
    min = 0,
    max = 100,
    value,
    size = "sm",
    vertical = false,
    reverseVertical = false,
    initialValue: _initialValue,
    onChange,
    markers = EMPTY_ARRAY,
    stickToMarkers = false,
    showMarkers = true,
    disabled = false,
    renderMarkers: RenderMarkers = DefaultRenderMarkers,
    renderMarker = DefaultRenderMarker,
}: SliderProps) {
    if (stickToMarkers) {
        assert(markers.length, "markers must be non-empty when stickToMarkers is true");
    }

    const [containerRef, setContainerRef] = useState<HTMLDivElement | null>(null);

    const [currentValue, setCurrentValue] = useControlledState({
        initialValue: _initialValue ?? makeDefaultForInputRange(min, max),
        managedValue: value,
        debugName: "Slider",
        handleChange: onChange,
    });

    const snapToMarker = useCallback(function (num: number): number {
        return markers.reduce((prev, cur) => {
            return Math.abs(cur - num) < Math.abs(prev - num) ? cur : prev;
        });
    }, [markers]);

    const clampToRange = useCallback((num: number) => {
        return clamp(min, max, num);
    }, [max, min]);

    const valueToPercent = useCallback((_value: number): number => {
        const value = clampToRange(_value);

        return (value - min) / (max - min);
    }, [clampToRange, max, min]);

    const shouldShowMarkers = markers.length > 0 && showMarkers;
    const inputRef = useRef<HTMLInputElement>(null);

    return (
        <div
            ref={setContainerRef}
            className={cn(styles.slider, sliderSizes[size], shouldShowMarkers && "my-3", {
                [styles.vertical]: vertical,
                [styles.horizontal]: !vertical,
                [styles.reverse]: reverseVertical,
            })}
            style={{
                "--progress": valueToPercent(currentValue),
            }}
        >
            {shouldShowMarkers && (
                <RenderMarkers
                    markers={markers}
                    container={containerRef}
                    min={min}
                    max={max}
                    valueToPercent={valueToPercent}
                    clampToRange={clampToRange}
                    renderMarker={renderMarker}
                    vertical={vertical}
                />
            )}
            <input
                ref={inputRef}
                disabled={disabled}
                type="range"
                min={min}
                max={max}
                value={currentValue}
                onKeyDown={() => {
                    if (stickToMarkers) {
                        setCurrentValue(snapToMarker);
                    }
                }}
                onChange={(e) => {
                    const _num = +e.target.value;
                    let num = _num;

                    if (stickToMarkers) {
                        num = snapToMarker(_num);
                    }

                    setCurrentValue(num);
                }}
            />
            <span className={cn(styles.progress)} />
            <span className={cn(styles.remainder)} />
        </div>
    );
}

export interface RenderMarkersProps {
    markers: readonly number[];
    container: HTMLDivElement | null;
    min: number;
    max: number;
    valueToPercent: (value: number) => number;
    clampToRange: (num: number) => number;
    renderMarker?(props: RenderMarkerProps): ReactNode;
    vertical: boolean;
}

function DefaultRenderMarkers({
    container,
    renderMarker: RenderMarker = DefaultRenderMarker,
    clampToRange,
    valueToPercent,
    markers,
    vertical,
}: RenderMarkersProps) {
    const [containerWidth, setContainerWidth] = useState(0);
    const [containerHeight, setContainerHeight] = useState(0);
    const [dep, updateSize] = useForceUpdater();
    const [thumbWidth, setThumbWidth] = useState(0);

    useResizeObserver(container, updateSize);

    useEffect(() => {
        dep;
        if (container) {
            const { width, height } = container.getBoundingClientRect();

            const thumbWidth = parseCSSValue(
                getComputedStyle(container)
                    .getPropertyValue("--thumb-width"),
                container,
                PercentReference.WIDTH,
            );

            setContainerWidth(width);
            setContainerHeight(height);
            setThumbWidth(thumbWidth);
        } else {
            setContainerWidth(0);
            setThumbWidth(0);
        }
    }, [container, dep]);

    return (
        <div className={cn("pointer-events-none absolute top-0 left-0 h-full w-full")}>
            <div className="relative h-full w-full">
                {markers.map((marker) => {
                    const progress = valueToPercent(clampToRange(marker));

                    return (
                        <RenderMarker
                            key={marker}
                            marker={marker}
                            progress={progress}
                            containerWidth={containerWidth}
                            containerHeight={containerHeight}
                            thumbWidth={thumbWidth}
                            containerRef={container}
                            vertical={vertical}
                        />
                    );
                })}
            </div>
        </div>
    );
}

export interface RenderMarkerProps {
    marker: number;
    progress: number;
    containerWidth: number;
    containerHeight: number;
    thumbWidth: number;
    containerRef: HTMLDivElement | null;
    vertical: boolean;
}

function DefaultRenderMarker({ vertical, ...props }: RenderMarkerProps) {
    if (vertical) {
        return (
            <DefaultRenderVerticalMarker
                vertical={vertical}
                {...props}
            />
        );
    }
    return (
        <DefaultRenderHorizontalMarker
            vertical={vertical}
            {...props}
        />
    );
}

function DefaultRenderVerticalMarker({ marker, progress, thumbWidth, containerHeight }: RenderMarkerProps) {
    return (
        <div
            key={marker}
            className="align-center absolute left-0 flex w-full flex-col"
            style={{
                top: `${(progress * containerHeight) + ((progress - 0.5) * -1 * thumbWidth)}px`,
            }}
        >
            <div
                className="absolute top-0 left-[-1lh] -translate-y-1/2 rotate-90"
            >
                {marker}
            </div>
            <HorizontalLine className="w-8/10" />
        </div>
    );
}

function DefaultRenderHorizontalMarker({ marker, progress, thumbWidth, containerWidth }: RenderMarkerProps) {
    return (
        <div
            key={marker}
            className="absolute top-0 flex h-full flex-col justify-center"
            style={{
                left: `${(progress * containerWidth) + ((progress - 0.5) * -1 * thumbWidth)}px`,
            }}
        >
            <div
                className="absolute top-[-1lh] left-0 -translate-x-1/2"
            >
                {marker}
            </div>
            <VerticalLine className="h-8/10"/>
        </div>
    );
}
