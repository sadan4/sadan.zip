import { useComposedRefs } from "@/hooks/composedRefs";
import { useControlledState } from "@/hooks/controlledState";
import { getNewestEntry, useIntersection } from "@/hooks/intersection";
import { useResizeObserverFromRef } from "@/hooks/resizeObserver";
import cn from "@/utils/cn";
import { assert, debug_assert } from "@/utils/error";
import { clamp, inRange } from "@/utils/math";
import { mapObject } from "@/utils/obj";
import { defer } from "@/utils/scope";

import { ScrollArea, type ScrollAreaProps } from "../ScrollArea";
import { ScrollAreaContext } from "../ScrollArea/context";

import { type PropsWithChildren, type ReactNode, type Ref, type UIEvent, useCallback, useContext, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";

export interface LazyScrollerRenderItemProps<T> {
    item: T;
    index: number;
    array: readonly T[];
}

export interface BufferedScrollerScrollOptions extends ScrollOptions {
    /**
     * don't scroll if the item is already in view
     * 
     * false -> always scroll the item to the center
     * true -> only scroll if the item is not currently in view
     * 
     * @default true
     */
    ifNeeded?: boolean;
}

export interface BufferedScrollerHandle<T> {
    scrollItemIntoView(idx: number, options?: BufferedScrollerScrollOptions): void;
    scrollItemIntoView(predicate: Parameters<ReadonlyArray<T>["findIndex"]>[0], options?: BufferedScrollerScrollOptions): void;
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
    /**
     * Used to scroll an item into view.
     */
    handle?: Ref<BufferedScrollerHandle<T>>;
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
            aria-hidden
            className="pointer-events-none h-px w-px bg-transparent after:h-[1] after:w-[1] after:content-['']"
            data-flag
        />
    );
}

interface ScrollerChunkProps extends PropsWithChildren {
    idx: number;
    onHeightChange(idx: number, height: number): void;
}

function ScrollerChunk({ children, idx, onHeightChange }: ScrollerChunkProps) {
    const ref = useRef<HTMLDivElement>(null);

    useResizeObserverFromRef(ref, ({ contentRect: { height } }) => {
        onHeightChange(idx, height);
    });

    return (
        <div
            data-scroller-chunk={idx}
            ref={ref}
        >
            {children}
        </div>
    );
}

interface ScrollerPaddingProps {
    height: number;
}

function ScrollerPadding({ height }: ScrollerPaddingProps) {
    return (
        <div
            style={{ height }}
            className="pointer-events-none"
            aria-hidden
        />
    );
}

interface Chunk {
    chunkIdx: number;
    startIdx: number;
    size: number;
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
    ref,
    handle,
    onScrollEnd: _onScrollEnd,
    ...props
}: BufferedScrollProps<T>) {
    type VisibleChunks = Partial<Record<number, Partial<Record<"top" | "bottom", boolean>>>>;

    /**
     * chunkIdx -> height of chunk
     */
    type ChunkHeights = Partial<Record<number, number>>;

    const [batchSize] = useControlledState({
        initialValue: Math.min(Math.floor(items.length / 20), items.length),
        managedValue: _batchSize && Math.floor(_batchSize),
    });

    assert(batchSize === Math.floor(batchSize) && batchSize > 0, "batchSize must be a positive integer");
    Object.freeze(items);

    const scrollAreaRef = useRef<HTMLDivElement>(null);
    const totalChunks = Math.ceil(items.length / batchSize);
    const [visibleChunks, setVisibleChunks] = useState<VisibleChunks>({});
    const [chunkHeights, setChunkHeights] = useState<ChunkHeights>({});
    const [firstChunk, setFirstChunk] = useState(0);
    const [numChunks, setNumChunks] = useState(1);

    const chunks = useMemo(
        () => makeChunks(items, batchSize, numChunks, firstChunk),
        [items, batchSize, numChunks, firstChunk],
    );

    const getNumItemsInChunk = useCallback((chunkIdx: number): number => {
        const startIdx = chunkIdx * batchSize;
        const size = clamp(0, batchSize, items.length - startIdx);

        return size;
    }, [batchSize, items.length]);


    /**
     * NaN when no items set yet
     */
    const averageItemHeight = useMemo(() => {
        let totalHeight = 0;
        let totalNumItems = 0;

        for (const [chunkIdx, height] of Object.entries(chunkHeights)) {
            if (!height) {
                continue;
            }

            const numItems = getNumItemsInChunk(+chunkIdx);

            if (!numItems) {
                continue;
            }

            totalNumItems += numItems;
            totalHeight += height;
        }

        return totalHeight / totalNumItems;
    }, [chunkHeights, getNumItemsInChunk]);

    function setChunkVisibility(chunkIdx: number, direction: "top" | "bottom", isVisible: boolean) {
        setVisibleChunks((prev) => {
            // eslint-disable-next-line logical-assignment-operators -- React compiler doesn't like ??=
            const chunk = prev[chunkIdx] = prev[chunkIdx] ?? {};

            chunk[direction] = isVisible;
            return { ...prev };
        });
    }

    function onChunkHeight(chunkIdx: number, height: number) {
        setChunkHeights((prev) => {
            height = Math.ceil(height);
            if (prev[chunkIdx] === height) {
                return prev;
            }
            return {
                ...prev,
                [chunkIdx]: height,
            };
        });
    }

    // TODO: allow user to pass guess for element height
    const guessChunkHeight = useCallback((chunkIdx: number): number => {
        const exact = chunkHeights[chunkIdx];

        if (exact) {
            return exact;
        }
        // if we don't have an exact height, guess based on average item height
        if (!averageItemHeight) {
            return 0;
        }

        const numItems = getNumItemsInChunk(chunkIdx);

        return averageItemHeight * numItems;
    }, [averageItemHeight, chunkHeights, getNumItemsInChunk]);

    // calculate the padding needed above and below the rendered chunks to make the scrollbar accurate
    const [paddingTop, paddingBottom] = useMemo(() => {
        let top = 0;
        let bottom = 0;

        for (let i = 0; i < firstChunk; ++i) {
            top += guessChunkHeight(i);
        }

        for (let i = firstChunk + numChunks; i < totalChunks; ++i) {
            bottom += guessChunkHeight(i);
        }


        return [top, bottom];
    }, [firstChunk, guessChunkHeight, numChunks, totalChunks]);

    const hasNoPadding = paddingTop === 0 && paddingBottom === 0;

    function reCalcVisibleChunks(el: HTMLDivElement) {
        const { scrollTop, clientHeight } = el;
        let acc = 0;
        // guess the first chunk that should be visible
        let viewStartChunk = 0;

        // react compiler doesn't like for loops without init statements
        for (let _; viewStartChunk < totalChunks; ++viewStartChunk) {
            // typescript no unused vars
            _;
            acc += guessChunkHeight(viewStartChunk);

            if (acc > scrollTop) {
                break;
            }
        }

        // if we're already rendering the chunk we don't need to do anything
        if (inRange(firstChunk, firstChunk + numChunks, viewStartChunk)) {
            return;
        }

        const averageChunkHeight = averageItemHeight * batchSize;
        // guess how many chunks fill the viewport
        const maxChunksInViewAtOnce = Math.ceil(clientHeight / averageChunkHeight);
        // TODO: will this do the wrong thing when buffer === infinity?
        const buffer = bufferSize === Infinity ? totalChunks : bufferSize;
        const newFirstChunk = Math.max(0, viewStartChunk - buffer);
        const numNeededChunks = Math.min(maxChunksInViewAtOnce + (2 * buffer), totalChunks - newFirstChunk);

        setVisibleChunks({});
        setFirstChunk(newFirstChunk);
        setNumChunks(numNeededChunks);
    }

    // scrollTop: how far the user has scrolled
    // clientHeight: the height of the visible area
    function onScrollEnd(ev: UIEvent<HTMLDivElement>) {
        using _ = defer(() => {
            _onScrollEnd?.(ev);
        });

        const { currentTarget: el } = ev;
        const { scrollTop: viewportTop, clientHeight: viewportHeight } = el;
        // guess where we should be based on average chunk size
        // TODO: this might be janky if the footer/header is large        
        const viewportBottom = viewportTop + viewportHeight;
        const lastChunkIdx = Math.min(firstChunk + numChunks, totalChunks);
        let startOffset = 0;

        // guess the offset where we are currently rendering chunks
        for (let i = 0; i < firstChunk; ++i) {
            startOffset += guessChunkHeight(i);
        }

        if (startOffset > viewportBottom) {
            setTimeout(() => {
                reCalcVisibleChunks(el);
            });
            return;
        }

        let endOffset = startOffset;

        // guess the offset where we stop rendering chunks
        for (let i = firstChunk; i < lastChunkIdx; ++i) {
            endOffset += guessChunkHeight(i);
        }

        if (endOffset < viewportTop) {
            setTimeout(() => {
                reCalcVisibleChunks(el);
            });
            return;
        }
    }

    useImperativeHandle(handle, () => {
        const api = {
            scrollItemIntoView(arg, { ifNeeded = true, ...domOptions } = {}) {
                const scrollArea = scrollAreaRef.current;

                if (!scrollArea) {
                    return;
                }

                const idx = typeof arg === "number" ? arg : items.findIndex(arg);

                debug_assert(idx !== -1, "trying to scroll to item that does not exist");

                if (idx === -1) {
                    return;
                }

                const { clientHeight: viewportHeight, scrollTop, scrollHeight } = scrollArea;
                let itemOffset: number | undefined;

                // if average height is nan we haven't calculated the average height yet
                // so we need to do it by hand
                if (Number.isNaN(averageItemHeight)) {
                    // calculate it by hand
                    const renderedNodes = scrollArea.querySelectorAll("[data-scroller-chunk]>:not([data-flag])");
                    let height = 0;

                    for (let i = 0; i < renderedNodes.length; ++i) {
                        height += renderedNodes[i].clientHeight;
                    }

                    itemOffset = idx * (height / renderedNodes.length);
                } else {
                    itemOffset = idx * averageItemHeight;
                }

                // if the item is already in the viewport, then we don't need to scroll
                if (ifNeeded && inRange(scrollTop, scrollTop + viewportHeight, itemOffset)) {
                    return;
                }

                // If we are called by a parent before we have setup padding
                // scrolling will do nothing until the padding is setup
                if (hasNoPadding && itemOffset > scrollHeight) {
                    setTimeout(() => {
                        api.scrollItemIntoView(arg, {
                            ifNeeded,
                            ...domOptions,
                        });
                    });
                    return;
                }

                scrollArea.scrollTo({
                    ...domOptions,
                    top: clamp(0, scrollHeight, itemOffset - (viewportHeight / 2)),
                });
            },
        } satisfies BufferedScrollerHandle<T>;

        return api;
    }, [averageItemHeight, hasNoPadding, items]);

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

    // reset chunk heights when things that could change it update
    useEffect(() => {
        setChunkHeights({});
    }, [totalChunks, batchSize, items.length]);

    return (
        <ScrollArea
            ref={useComposedRefs(ref, scrollAreaRef)}
            onScrollEnd={onScrollEnd}
            className={cn(className)}
            {...props}
        >
            <>{renderHeader?.()}</>
            <ScrollerPadding height={paddingTop} />
            {chunks.map(({ chunkIdx, startIdx, size }) => {
                return (
                    <ScrollerChunk
                        key={`chunk-${startIdx}`}
                        idx={chunkIdx}
                        onHeightChange={onChunkHeight}
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
                    </ScrollerChunk>
                );
            })}
            <ScrollerPadding height={paddingBottom} />
            {
                (alwaysRenderFooter || firstChunk + numChunks >= totalChunks) && <>{renderFooter?.()}</>
            }
        </ScrollArea>
    );
}
