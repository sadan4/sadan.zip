import { useControlledState } from "@/hooks/controlledState";
import { getNewestEntry, useIntersection } from "@/hooks/intersection";
import cn from "@/utils/cn";

import { ScrollArea, type ScrollAreaProps } from "../ScrollArea";
import { ScrollAreaContext } from "../ScrollArea/context";

import invariant from "invariant";
import { Fragment, type ReactNode, useContext, useEffect, useMemo, useRef, useState } from "react";
import { mapObject } from "@/utils/functional";

export interface LazyScrollerRenderItemProps<T> {
    item: T;
    index: number;
    array: readonly T[];
}

export interface LazyScrollerProps<T> extends ScrollAreaProps {
    renderHeader?(): ReactNode;
    renderItem(props: LazyScrollerRenderItemProps<T>): ReactNode;
    renderFooter?(): ReactNode;
    items: Readonly<T[]>;
    /**
     * Number of items to render in each batch. Default is min(floor({@link items}.length / 20), {@link items}.length).
     */
    batchSize?: number;
}

const enum FlagPosition {
    START,
    END,
}

interface FlagProps {
    // FIXME: remove these after debugging
    idx?: number;
    onEnter?(): void;
    onExit?(): void;
}

function Flag({ idx, onEnter, onExit }: FlagProps) {
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
            data-flag-idx={idx}
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

export function LazyScroller<T>({
    renderHeader,
    renderItem,
    renderFooter,
    items,
    batchSize: _batchSize,
    className,
    ...props
}: LazyScrollerProps<T>) {
    const [batchSize] = useControlledState({
        initialValue: Math.min(Math.floor(items.length / 20), items.length),
        managedValue: _batchSize,
    });

    invariant(batchSize === Math.floor(batchSize), "batchSize must be an integer");
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
        const visibleChunkIndices = mapObject(visibleChunks, (directions, chunkId) => {
            if (!directions) {
                return false;
            }
            return !(directions.bottom || directions.top);
        });
        let firstVisibleChunk = Infinity;
        let lastVisibleChunk = -Infinity;

        for (const [chunkId, isVisible] of Object.entries(visibleChunkIndices)) {
            if (isVisible) {
                firstVisibleChunk = Math.min(firstVisibleChunk, +chunkId);
                lastVisibleChunk = Math.max(lastVisibleChunk, +chunkId);
            }
        }
    }, [visibleChunks]);

    useEffect(() => {
        setVisibleChunks({});
    }, [totalChunks]);

    return (
        <ScrollArea
            className={cn(className)}
            {...props}
        >
            <Fragment key="lazyscroller-header">{renderHeader?.()}</Fragment>
            {chunks.map(({ chunkIdx, startIdx, size }) => {
                return (
                    <Fragment key={`chunk-${startIdx}`}>
                        <Flag
                            key="chunk-start"
                            idx={chunkIdx}
                            onEnter={() => {
                                console.log("enter", "start", chunkIdx);
                                setChunkVisibility(chunkIdx, "top", true);
                            }}
                            onExit={() => {
                                console.log("exit", "start", chunkIdx);
                                setChunkVisibility(chunkIdx, "top", false);
                                console.log(visibleChunks);
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
                            idx={chunkIdx}
                            onEnter={() => {
                                console.log("enter", "end", chunkIdx, numChunks);
                                setChunkVisibility(chunkIdx, "bottom", true);
                                if (chunkIdx === numChunks - 1) {
                                    console.log("update");
                                    setNumChunks((prev) => {
                                        return Math.min(prev + 1, totalChunks);
                                    });
                                }
                            }}
                            onExit={() => {
                                setChunkVisibility(chunkIdx, "bottom", false);
                                console.log("exit", "end", chunkIdx);
                            }}
                        />
                    </Fragment>
                );
            })}
            <Fragment key="lazyscroller-footer">{renderFooter?.()}</Fragment>
        </ScrollArea>
    );
}
