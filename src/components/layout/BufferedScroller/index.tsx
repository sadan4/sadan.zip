import { useControlledState } from "@/hooks/controlledState";
import { getNewestEntry, useIntersection } from "@/hooks/intersection";
import { useReiszeObserverFromRef } from "@/hooks/resizeObserver";
import cn from "@/utils/cn";
import { assert } from "@/utils/error";
import { mapObject } from "@/utils/obj";

import { ScrollArea, type ScrollAreaProps } from "../ScrollArea";
import { ScrollAreaContext } from "../ScrollArea/context";

import { Fragment, type ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";

export interface LazyScrollerRenderItemProps<T> {
    item: T;
    index: number;
    array: readonly T[];
}

export interface BufferedScrollProps<T> extends ScrollAreaProps {
    renderHeader?(): ReactNode;
    renderItem(props: LazyScrollerRenderItemProps<T>): ReactNode;
    renderFooter?(): ReactNode;
    alwaysRenderFooter?: boolean;
    items: Readonly<T[]>;
    /**
     * Number of items to render in each batch. Default is min(floor({@link items}.length / 20), {@link items}.length).
     */
    batchSize?: number;
    /**
     * The number of batches to keep rendered above and below the viewport.
     */
    bufferSize?: number;
}

interface FlagProps {
    onEnter?(): void;
    onExit?(): void;
}

function Flag({ onEnter, onExit }: FlagProps) {
    const scrollAreaHandle = useContext(ScrollAreaContext);
    const lastState = useRef<boolean>(null);

    const setIntersectionRef = useIntersection((entries) => {
        const entry = getNewestEntry(entries);
        const { isIntersecting } = entry;

        // no change
        if (lastState.current === isIntersecting)
            return;
        else if ((lastState.current = isIntersecting)) {
            onEnter?.();
        } else {
            onExit?.();
        }
    }, {
        rootRef: scrollAreaHandle.ref,
    });

    return (
        <div
            ref={setIntersectionRef}
            className="pointer-events-none h-px w-px bg-transparent after:h-[1] after:w-[1] after:content-['']"
        />
    );
}

interface Chunk {
    chunkIdx: number;
    startIdx: number;
    size: number;
}

interface MeasuredChunkProps {
    chunkIdx: number;
    onHeightChange(chunkIdx: number, height: number): void;
    children: ReactNode;
}

function MeasuredChunk({ chunkIdx, onHeightChange, children }: MeasuredChunkProps) {
    const ref = useRef<HTMLDivElement | null>(null);

    useReiszeObserverFromRef(ref, (entry) => {
        const { height } = entry.contentRect;

        if (Number.isFinite(height) && height >= 0) {
            onHeightChange(chunkIdx, height);
        }
    });

    return (
        <div ref={ref}>
            {children}
        </div>
    );
}

interface PaddingSentinelProps {
    height: number;
    onVisible(): void;
}

/**
 * Replaces a plain padding div with one observed by IntersectionObserver.
 * When any part of the padding becomes visible (e.g. the user drags the
 * scrollbar past all rendered chunks), {@link onVisible} fires exactly once.
 * The callback resets when the sentinel leaves the viewport, so it can
 * fire again on the next fast-scroll.
 */
function PaddingSentinel({ height, onVisible }: PaddingSentinelProps) {
    const scrollAreaHandle = useContext(ScrollAreaContext);
    const wasVisible = useRef(false);

    const setIntersectionRef = useIntersection((entries) => {
        const entry = getNewestEntry(entries);
        const { isIntersecting } = entry;

        if (isIntersecting && !wasVisible.current) {
            wasVisible.current = true;
            onVisible();
        } else if (!isIntersecting) {
            wasVisible.current = false;
        }
    }, {
        rootRef: scrollAreaHandle.ref,
    });

    return (
        <div
            ref={height > 0 ? setIntersectionRef : undefined}
            aria-hidden
            style={{ height }}
            className="pointer-events-none"
        />
    );
}

function makeChunks(items: readonly unknown[], batchSize: number, maxChunks: number, firstChunkIdx: number): Chunk[] {
    if (items.length === 0 || maxChunks === 0 || batchSize === 0) {
        return [];
    }

    const chunks: Chunk[] = [];
    const totalChunks = Math.ceil(items.length / batchSize);
    const lastChunkIdx = Math.min(totalChunks - 1, firstChunkIdx + maxChunks - 1);

    for (let i = firstChunkIdx; i <= lastChunkIdx; i++) {
        chunks.push({
            chunkIdx: i,
            startIdx: i * batchSize,
            size: Math.min(batchSize, items.length - (i * batchSize)),
        });
    }

    return chunks;
}

export function BufferedScroller<T>({
    renderHeader,
    renderItem,
    renderFooter,
    items,
    batchSize: _batchSize,
    bufferSize = Infinity,
    alwaysRenderFooter = false,
    className,
    ...props
}: BufferedScrollProps<T>) {
    const scrollRef = useRef<HTMLDivElement | null>(null);

    const [batchSize] = useControlledState({
        initialValue: Math.min(Math.floor(items.length / 20), items.length),
        managedValue: _batchSize && Math.floor(_batchSize),
    });

    assert(batchSize === Math.floor(batchSize) && batchSize > 0, "batchSize must be a positive integer");
    Object.freeze(items);

    type VisibleChunks = Partial<Record<number, Partial<Record<"top" | "bottom", boolean>>>>;

    type ChunkHeights = Partial<Record<number, number>>;

    const totalChunks = Math.ceil(items.length / batchSize);
    const [visibleChunks, setVisibleChunks] = useState<VisibleChunks>({});
    const [chunkHeights, setChunkHeights] = useState<ChunkHeights>({});
    const [firstChunk, setFirstChunk] = useState(0);
    const [numChunks, setNumChunks] = useState(1);

    const chunks = useMemo(
        () => makeChunks(items, batchSize, numChunks, firstChunk),
        [items, batchSize, numChunks, firstChunk],
    );

    const avgItemHeight = useMemo(() => {
        let totalMeasuredHeight = 0;
        let totalMeasuredItems = 0;

        for (const [chunkIdxStr, height] of Object.entries(chunkHeights)) {
            const chunkIdx = +chunkIdxStr;

            if (typeof height !== "number" || !Number.isFinite(height) || height <= 0) {
                continue;
            }

            const startIdx = chunkIdx * batchSize;
            const size = Math.min(batchSize, Math.max(0, items.length - startIdx));

            if (size === 0) {
                continue;
            }

            totalMeasuredHeight += height;
            totalMeasuredItems += size;
        }

        if (totalMeasuredItems === 0) {
            return 0;
        }

        return totalMeasuredHeight / totalMeasuredItems;
    }, [batchSize, chunkHeights, items.length]);

    const estimateChunkHeight = useCallback((chunkIdx: number) => {
        const measured = chunkHeights[chunkIdx];

        if (typeof measured === "number" && Number.isFinite(measured) && measured > 0) {
            return measured;
        }

        if (avgItemHeight <= 0) {
            return 0;
        }

        const startIdx = chunkIdx * batchSize;
        const size = Math.min(batchSize, Math.max(0, items.length - startIdx));

        return avgItemHeight * size;
    }, [chunkHeights, avgItemHeight, batchSize, items.length]);

    const { topPaddingPx, bottomPaddingPx } = useMemo(() => {
        let top = 0;

        for (let i = 0; i < firstChunk; i++) {
            top += estimateChunkHeight(i);
        }

        const lastRenderedExclusive = Math.min(totalChunks, firstChunk + numChunks);
        let bottom = 0;

        for (let i = lastRenderedExclusive; i < totalChunks; i++) {
            bottom += estimateChunkHeight(i);
        }

        return {
            topPaddingPx: Math.max(0, top),
            bottomPaddingPx: Math.max(0, bottom),
        };
    }, [estimateChunkHeight, firstChunk, numChunks, totalChunks]);

    function setChunkVisibility(chunkIdx: number, direction: "top" | "bottom", isVisible: boolean) {
        setVisibleChunks((prev) => {
            // eslint-disable-next-line logical-assignment-operators -- React compiler doesn't like ??=
            const chunk = prev[chunkIdx] = prev[chunkIdx] ?? {};

            chunk[direction] = isVisible;
            return { ...prev };
        });
    }

    function setChunkHeight(chunkIdx: number, height: number) {
        setChunkHeights((prev) => {
            const nextHeight = Math.ceil(height);

            if (prev[chunkIdx] === nextHeight) {
                return prev;
            }
            return {
                ...prev,
                [chunkIdx]: nextHeight,
            };
        });
    }

    /**
     * Called when a padding sentinel becomes visible, meaning the viewport
     * has jumped past all rendered chunks (fast scrollbar drag). Reads the
     * current scroll position, estimates which chunk should be visible, and
     * repositions the rendered window accordingly.
     */
    const jumpToScrollPosition = useCallback(() => {
        const el = scrollRef.current;

        if (!el || avgItemHeight <= 0) {
            return;
        }

        const { scrollTop, clientHeight } = el;
        let acc = 0;
        let viewStartChunk = 0;

        // Walk cumulative estimated chunk heights to find the chunk at scrollTop
        for (let i = 0; i < totalChunks; i++) {
            const h = estimateChunkHeight(i);

            if (acc + h > scrollTop) {
                viewStartChunk = i;
                break;
            }

            acc += h;
            viewStartChunk = i;
        }

        // If the estimated chunk is already rendered, let the normal flags handle it
        if (viewStartChunk >= firstChunk && viewStartChunk < firstChunk + numChunks) {
            return;
        }

        // Estimate how many chunks fill the viewport
        const avgChunkHeight = avgItemHeight * batchSize;

        const viewportChunks = avgChunkHeight > 0
            ? Math.ceil(clientHeight / avgChunkHeight) + 1
            : 1;

        const buffer = bufferSize === Infinity ? totalChunks : bufferSize;
        const newFirst = Math.max(0, viewStartChunk - buffer);
        const needed = viewportChunks + (2 * buffer);
        const newNum = Math.min(needed, totalChunks - newFirst);

        // Clear stale visibility data from the old chunk positions
        setVisibleChunks({});
        setFirstChunk(newFirst);
        setNumChunks(newNum);
    }, [avgItemHeight, totalChunks, estimateChunkHeight, firstChunk, numChunks, batchSize, bufferSize]);

    useEffect(() => {
        let first = firstChunk;
        let num = numChunks;

        const visibleChunkIndices = mapObject(visibleChunks, (directions) => {
            return directions?.bottom || directions?.top;
        });

        let firstVisibleChunk = Infinity;
        let lastVisibleChunk = -Infinity;

        for (const [chunkId, isVisible] of Object.entries(visibleChunkIndices)) {
            if (isVisible) {
                firstVisibleChunk = Math.min(firstVisibleChunk, +chunkId);
                lastVisibleChunk = Math.max(lastVisibleChunk, +chunkId);
            }
        }

        // handle buffering
        if (
            bufferSize !== Infinity
            && firstVisibleChunk !== Infinity
            && lastVisibleChunk !== -Infinity
        ) {
            // Update first chunk if we've scrolled too far from the beginning
            if (firstVisibleChunk - firstChunk > bufferSize) {
                first = Math.max(0, firstVisibleChunk - bufferSize);
            }

            // Calculate numChunks relative to first, not as an absolute index
            // We want enough chunks to cover from first to (lastVisible + buffer)
            const desiredLastChunk = lastVisibleChunk + bufferSize;

            num = Math.min(desiredLastChunk - first + 1, totalChunks - first);
        }

        // handle when a new chunk is added to the bottom and we are at the end of the list
        if (lastVisibleChunk === totalChunks - 2) {
            num = Math.min(totalChunks, num + 1);
        }

        setFirstChunk(first);
        setNumChunks(num);
    }, [visibleChunks, bufferSize, firstChunk, numChunks, totalChunks]);

    useEffect(() => {
        setVisibleChunks({});
    }, [totalChunks]);

    useEffect(() => {
        setChunkHeights({});
    }, [batchSize, items.length, totalChunks]);

    // Scroll-event fallback: IntersectionObserver may not fire when the user
    // drags the scrollbar so fast that the padding sentinel goes from "above
    // viewport" to "below viewport" in a single frame. This listener checks if
    // the viewport has no overlap with rendered content and triggers recovery.
    useEffect(() => {
        const el = scrollRef.current;

        if (!el || avgItemHeight <= 0 || totalChunks === 0) {
            return;
        }

        function handleScroll() {
            const scrollEl = scrollRef.current;

            if (!scrollEl) {
                return;
            }

            const { scrollTop, clientHeight } = scrollEl;
            // Calculate the scroll range covered by rendered chunks
            let renderedStart = 0;

            for (let i = 0; i < firstChunk; i++) {
                renderedStart += estimateChunkHeight(i);
            }

            let renderedEnd = renderedStart;

            for (let i = firstChunk; i < Math.min(firstChunk + numChunks, totalChunks); i++) {
                renderedEnd += estimateChunkHeight(i);
            }

            const viewportTop = scrollTop;
            const viewportBottom = scrollTop + clientHeight;

            // If viewport overlaps with rendered content, no intervention needed
            if (viewportBottom > renderedStart && viewportTop < renderedEnd) {
                return;
            }

            // Viewport does not overlap rendered content; trigger recovery
            jumpToScrollPosition();
        }

        el.addEventListener("scroll", handleScroll, { passive: true });

        return () => {
            el.removeEventListener("scroll", handleScroll);
        };
    }, [avgItemHeight, totalChunks, firstChunk, numChunks, estimateChunkHeight, jumpToScrollPosition]);

    return (
        <ScrollArea
            ref={scrollRef}
            className={cn(className)}
            {...props}
        >
            <Fragment key="bufferedscroller-header">{renderHeader?.()}</Fragment>
            <PaddingSentinel
                height={topPaddingPx}
                onVisible={jumpToScrollPosition}
            />
            {chunks.map(({ chunkIdx, startIdx, size }) => {
                return (
                    <MeasuredChunk
                        key={`chunk-${startIdx}`}
                        chunkIdx={chunkIdx}
                        onHeightChange={setChunkHeight}
                    >
                        <Flag
                            key="chunk-start"
                            onEnter={() => {
                                if (bufferSize !== Infinity) {
                                    if (chunkIdx === firstChunk || chunkIdx === firstChunk + bufferSize) {
                                        setFirstChunk((prev) => Math.max(0, prev - 1));
                                    }
                                }
                                setChunkVisibility(chunkIdx, "top", true);
                            }}
                            onExit={() => {
                                setChunkVisibility(chunkIdx, "top", false);
                            }}
                        />
                        {
                            items
                                .slice(startIdx, startIdx + size)
                                .map((item, i, array) => {
                                    return renderItem({
                                        item,
                                        index: startIdx + i,
                                        array,
                                    });
                                })
                        }
                        <Flag
                            key="chunk-end"
                            onEnter={() => {
                                setChunkVisibility(chunkIdx, "bottom", true);
                                if (chunkIdx === numChunks - 1) {
                                    setNumChunks((prev) => {
                                        return Math.min(prev + 1, totalChunks);
                                    });
                                }
                            }}
                            onExit={() => {
                                setChunkVisibility(chunkIdx, "bottom", false);
                            }}
                        />
                    </MeasuredChunk>
                );
            })}
            <PaddingSentinel
                height={bottomPaddingPx}
                onVisible={jumpToScrollPosition}
            />
            {
                (alwaysRenderFooter || firstChunk + numChunks >= totalChunks) && <Fragment key="bufferedscroller-footer">{renderFooter?.()}</Fragment>
            }
        </ScrollArea>
    );
}
