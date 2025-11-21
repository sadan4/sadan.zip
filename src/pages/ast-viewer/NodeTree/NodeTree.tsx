import { Clickable } from "@/components/Clickable";
import { Accordion, ArrowPosition, ClickableArea } from "@/components/layout/Accordion";
import { AccordionAnimation } from "@/components/layout/Accordion/utils";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { ScrollAreaDirection } from "@/components/layout/ScrollArea/types";
import { Text } from "@/components/Text";
import { useRecent } from "@/hooks/recent";
import { useShallowMemo } from "@/hooks/shallowMemo";
import cn from "@/utils/cn";
import { EMPTY_ARRAY, EMPTY_SET, NOOP } from "@/utils/constants";
import { namedContext } from "@/utils/devtools";
import { error } from "@/utils/error";
import { toggleSetItem } from "@/utils/set";
import { getChildrenWithMode, getNodeKey, getNodeName, TreeMode } from "@/utils/typescript";

import { type RefObject, use, useEffect, useState } from "react";
import type { Node, SourceFile } from "typescript";
import { createStore, type StoreApi, useStore } from "zustand";
import { useShallow } from "zustand/shallow";

export interface NodeTreeProps {
    onSelectNode(node: Node): void;
    root: SourceFile;
    treeMode: TreeMode;
    reparseCount: number;
    highlightedNodes?: Node[];
    selectedNode?: Node;
}

interface NodeTreeStore {
    highlightedNodeKeys: ReadonlySet<string>;
    selectedNodeKey: string | null;
    collapsedNodeKeys: ReadonlySet<string>;
    treeMode: TreeMode;
    reparseCount: number;
}

function createNodeTreeStore(initialTreeMode: TreeMode): StoreApi<NodeTreeStore> {
    return createStore<NodeTreeStore>((set, get) => ({
        highlightedNodeKeys: EMPTY_SET,
        selectedNodeKey: null,
        collapsedNodeKeys: EMPTY_SET,
        treeMode: initialTreeMode,
        reparseCount: 0,
    } satisfies NodeTreeStore));
}

function useCreateNodeTreeStore(initialTreeMode: TreeMode): StoreApi<NodeTreeStore> {
    const [store] = useState(() => createNodeTreeStore(initialTreeMode));

    return store;
}

const NodeTreeStoreContext = namedContext<StoreApi<NodeTreeStore> | null>(null, "NodeTreeStoreContext");

function useNodeTreeStore<T extends (state: NodeTreeStore) => any>(selector: T): ReturnType<T> {
    const store = use(NodeTreeStoreContext);

    if (import.meta.env.DEV && !store) {
        error("useNodeTreeStore must be used within a NodeTreeStoreProvider");
    }

    return useStore(store!, useShallow(selector));
}

export function NodeTree({
    onSelectNode: _onSelectNode = NOOP,
    root,
    highlightedNodes,
    selectedNode,
    treeMode,
    reparseCount,
}: NodeTreeProps) {
    const store = useCreateNodeTreeStore(treeMode);
    const reqHlNodes = useShallowMemo(highlightedNodes);

    useEffect(() => {
        const highlightedNodeKeys = new Set(reqHlNodes?.map((node) => getNodeKey(node)));

        store.setState({ highlightedNodeKeys });
    }, [reqHlNodes, store]);

    useEffect(() => {
        const selectedNodeKey = selectedNode ? getNodeKey(selectedNode) : null;

        store.setState({ selectedNodeKey });
    }, [selectedNode, store]);

    useEffect(() => {
        store.setState({
            treeMode,
            reparseCount,
        });
    }, [treeMode, reparseCount, store]);

    const onNodeArrowClick = useRecent((nodeKey: string) => {
        const collapsedNodeKeys = new Set(store.getState().collapsedNodeKeys);

        toggleSetItem(collapsedNodeKeys, nodeKey);

        store.setState({ collapsedNodeKeys });
    });

    const onSelectNode = useRecent(_onSelectNode);

    return (
        <div className="flex size-full flex-col">
            <div className="bg-cyan-600">
                Input Box
            </div>
            <ScrollArea dir={ScrollAreaDirection.BOTH}>
                <NodeTreeStoreContext value={store}>
                    <NodeTreeNode
                        node={root}
                        nodeKey={getNodeKey(root)}
                        onNodeArrowClick={onNodeArrowClick}
                        onSelectNode={onSelectNode}
                    />
                </NodeTreeStoreContext>
            </ScrollArea>
        </div>
    );
}

interface NodeTreeNodeProps {
    node: Node;
    nodeKey: string;
    onNodeArrowClick: RefObject<(nodeKey: string) => void>;
    onSelectNode: RefObject<NodeTreeProps["onSelectNode"] & {}>;
}

function NodeTreeNode({ node, nodeKey: key, onNodeArrowClick, onSelectNode }: NodeTreeNodeProps) {
    const name = getNodeName(node);

    const {
        childNodes,
        isHighlighted,
        isCollapsed,
        isSelected,
        reparseCount,
    } = useNodeTreeStore(({
        treeMode,
        highlightedNodeKeys,
        collapsedNodeKeys,
        selectedNodeKey,
        reparseCount,
    }) => {
        const isHighlighted = highlightedNodeKeys.has(key);
        const isCollapsed = collapsedNodeKeys.has(key);
        const isSelected = selectedNodeKey === key;
        let childNodes = getChildrenWithMode(node, treeMode);

        if (!childNodes.length) {
            childNodes = EMPTY_ARRAY;
        }

        return {
            childNodes,
            isHighlighted,
            isCollapsed,
            isSelected,
            reparseCount,
        };
    });

    const children = (

        <Clickable
            onClick={(e) => {
                e.stopPropagation();
                onSelectNode.current(node);
            }}
            className="w-fit"
        >
            <Text
                className={cn(isHighlighted && "bg-warning-400/50")}
                color={isSelected ? "accent" : undefined}
            >
                {name}
            </Text>
        </Clickable>

    );

    if (childNodes.length) {
        return (
            <Accordion
                item={{
                    id: key,
                    contents: (
                        <div className="flex h-fit">
                            <div className="w-5 shrink-0" />
                            <div className="grow">
                                {childNodes.map((child) => {
                                    const key = getNodeKey(child);

                                    return (
                                        <NodeTreeNode
                                            key={`${key}-${reparseCount}`}
                                            node={child}
                                            nodeKey={key}
                                            onNodeArrowClick={onNodeArrowClick}
                                            onSelectNode={onSelectNode}
                                        />
                                    );
                                })}
                            </div>
                        </div>
                    ),
                }}
                onToggle={() => {
                    onNodeArrowClick.current(key);
                }}
                open={!isCollapsed}
                clicableArea={ClickableArea.ARROW}
                arrowPosition={ArrowPosition.LEFT}
                animation={AccordionAnimation.NONE}
                arrowClassName={cn("size-5")}
            >
                {children}
            </Accordion>
        );
    }
    return children;
}
