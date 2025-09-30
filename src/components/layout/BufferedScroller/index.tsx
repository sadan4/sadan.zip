import { useControlledState } from "@/hooks/controlledState";
import { getNewestEntry, useIntersection } from "@/hooks/intersection";
import cn from "@/utils/cn";
import { mapObject } from "@/utils/functional";

import { ScrollArea, type ScrollAreaProps } from "../ScrollArea";
import { ScrollAreaContext } from "../ScrollArea/context";

import invariant from "invariant";
import { Fragment, type ReactNode, useContext, useEffect, useMemo, useRef, useState } from "react";

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
            className="pointer-events-none h-0 w-0 bg-transparent after:h-[1] after:w-[1] after:content-['']"
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
    ...props
}: BufferedScrollProps<T>) {
    const [batchSize] = useControlledState({
        initialValue: Math.min(Math.floor(items.length / 20), items.length),
        managedValue: _batchSize && Math.floor(_batchSize),
    });

    invariant(batchSize === Math.floor(batchSize) && batchSize > 0, "batchSize must be a positive integer");
    Object.freeze(items);

    type VisibleChunks = Partial<Record<number, Partial<Record<"top" | "bottom", boolean>>>>;

    const totalChunks = Math.ceil(items.length / batchSize);
    const [visibleChunks, setVisibleChunks] = useState<VisibleChunks>({});
    const [firstChunk, setFirstChunk] = useState(0);
    const [numChunks, setNumChunks] = useState(1);

    const chunks = useMemo(
        () => makeChunks(items, batchSize, numChunks, firstChunk),
        [items, batchSize, numChunks, firstChunk],
    );

    function setChunkVisibility(chunkIdx: number, direction: "top" | "bottom", isVisible: boolean) {
        setVisibleChunks((prev) => {
            const chunk = prev[chunkIdx] ??= {};

            chunk[direction] = isVisible;
            return { ...prev };
        });
    }

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
            if (firstVisibleChunk - firstChunk > bufferSize) {
                first = Math.max(0, firstVisibleChunk - bufferSize);
            }
            if (lastVisibleChunk - firstVisibleChunk > bufferSize) {
                num = Math.max(0, lastVisibleChunk + bufferSize);
            }
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

    return (
        <ScrollArea
            className={cn(className)}
            {...props}
        >
            <Fragment key="bufferedscroller-header">{renderHeader?.()}</Fragment>
            {chunks.map(({ chunkIdx, startIdx, size }) => {
                return (
                    <Fragment key={`chunk-${startIdx}`}>
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
                    </Fragment>
                );
            })}
            {
                (alwaysRenderFooter || firstChunk + numChunks >= totalChunks) && <Fragment key="bufferedscroller-footer">{renderFooter?.()}</Fragment>
            }
        </ScrollArea>
    );
}
