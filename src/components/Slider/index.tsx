import { useControlledState } from "@/hooks/controlledState";
import { z } from "@/styles";
import cn from "@/utils/cn";
import { parseCSSValue, rangeInputDefaultValue } from "@/utils/dom";
import { clamp } from "@/utils/functional";

import styles from "./styles.module.scss";
import { VerticalLine } from "../VerticalLine";

import { type ComponentProps, type ReactNode, type RefObject, useCallback, useEffect, useRef, useState } from "react";

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
    markers?: number[];
    /**
     * If false, don't show the markers provided
     */
    showMarkers?: boolean;
    /**
     * If true, only allow selecting values that match the markers
     */
    stickToMarkers?: "stick" | "snap" | "no";
    renderMarkers?(props: RenderMarkersProps): ReactNode;
    renderMarker?(props: RenderMarkerProps): ReactNode;
}

export function Slider(props: SliderProps) {
    const {
        min = 0,
        max = 100,
        value,
        size = "sm",
        vertical = false,
        reverseVertical = false,
        initialValue = rangeInputDefaultValue(min, max),
        onChange,
        markers = [],
        stickToMarkers = "no",
        showMarkers = true,
        disabled = false,
        renderMarkers: RenderMarkers = DefaultRenderMarkers,
        renderMarker = DefaultRenderMarker,
    } = props;

    const containerRef = useRef<HTMLDivElement>(null);

    const [currentValue, setCurrentValue] = useControlledState({
        initialValue,
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
            ref={containerRef}
            className={cn(styles.slider, z.slider, sliderSizes[size], shouldShowMarkers && "my-3", {
                [styles.vertical]: vertical,
                [styles.horizontal]: !vertical,
                [styles.reverse]: reverseVertical,
            })}
            style={{
                ["--progress" as any]: valueToPercent(currentValue),
            }}
        >
            <input
                ref={inputRef}
                disabled={disabled}
                className={z.thumb}
                type="range"
                min={min}
                max={max}
                value={currentValue}
                onBlur={() => {
                    if (stickToMarkers === "stick") {
                        setCurrentValue((cur) => snapToMarker(cur));
                    }
                }}
                onPointerUp={(e) => {
                    if (e.isPrimary && stickToMarkers === "stick") {
                        setCurrentValue((cur) => snapToMarker(cur));
                    }
                }}
                onChange={(e) => {
                    const _num = +e.target.value;
                    let num = _num;

                    if (stickToMarkers === "snap") {
                        num = snapToMarker(_num);
                    }

                    setCurrentValue(num);
                }}
            />
            <span className={cn(styles.progress, z.track)} />
            <span className={cn(styles.remainder, z.track)} />
            {shouldShowMarkers && (
                <RenderMarkers
                    markers={markers}
                    containerRef={containerRef}
                    min={min}
                    max={max}
                    valueToPercent={valueToPercent}
                    clampToRange={clampToRange}
                    renderMarker={renderMarker}
                />
            )}
        </div>
    );
}

export interface RenderMarkersProps {
    markers: number[];
    containerRef: RefObject<HTMLDivElement | null>;
    min: number;
    max: number;
    valueToPercent: (value: number) => number;
    clampToRange: (num: number) => number;
    renderMarker?(props: RenderMarkerProps): ReactNode;
}

function DefaultRenderMarkers({
    containerRef,
    renderMarker: RenderMarker = DefaultRenderMarker,
    clampToRange,
    valueToPercent,
    markers,
}: RenderMarkersProps) {
    const [containerWidth, setContainerWidth] = useState(0);
    const [thumbWidth, setThumbWidth] = useState(0);

    useEffect(() => {
        if (containerRef.current) {
            const { width } = containerRef.current.getBoundingClientRect();

            const thumbWidth = parseCSSValue(
                getComputedStyle(containerRef.current)
                    .getPropertyValue("--thumb-width"),
                containerRef.current,
            );

            setContainerWidth(width);
            setThumbWidth(thumbWidth);
        } else {
            setContainerWidth(0);
            setThumbWidth(0);
        }
    }, [containerRef]);

    return (
        <div className={cn("absolute top-0 left-0 h-full w-full pointer-events-none", z.markers)}>
            <div className="relative w-full h-full">
                {markers.map((marker) => {
                    const progress = valueToPercent(clampToRange(marker));

                    return (
                        <RenderMarker
                            key={marker}
                            marker={marker}
                            progress={progress}
                            containerWidth={containerWidth}
                            thumbWidth={thumbWidth}
                            containerRef={containerRef}
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
    thumbWidth: number;
    containerRef: RefObject<HTMLDivElement | null>;
}

function DefaultRenderMarker({ marker, progress, thumbWidth, containerWidth }: RenderMarkerProps) {
    return (
        <div
            key={marker}
            className="absolute top-0 flex flex-col justify-center h-full"
            style={{
                left: `${(progress * containerWidth) + ((progress - 0.5) * -1 * thumbWidth)}px`,
            }}
        >
            <div
                className="absolute left-0 -top-[1lh] -translate-x-1/2"
            >
                {marker}
            </div>
            <VerticalLine className="h-8/10"/>
        </div>
    );
}
