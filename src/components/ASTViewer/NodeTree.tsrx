import { Clickable } from "@/components/Clickable";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { ScrollAreaContext } from "@/components/layout/ScrollArea/context";
import { ScrollAreaDirection } from "@/components/layout/ScrollArea/types";
import { Text } from "@/components/Text";
import { useChange } from "@/hooks/change";
import { useComposedRefs } from "@/hooks/composedRefs";
import { KeybindKeys, useKeybinds } from "@/hooks/keybind";
import { useRecent } from "@/hooks/recent";
import { useShallowMemo } from "@/hooks/shallowMemo";
import cn from "@/utils/cn";
import { EMPTY_ARRAY, EMPTY_SET, NOOP } from "@/utils/constants";
import { error } from "@/utils/error";
import { toggleSetItem } from "@/utils/set";
import { getChildrenWithMode, getNodeKey, getNodeName, getParent, TreeMode } from "@/utils/typescript";

import { TreeAccordion } from "./TreeAccordion";

import { createContext, Fragment, type RefObject, use, useEffect, useRef, useState } from "react";
import scrollIntoView from "scroll-into-view-if-needed";
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
    return createStore<NodeTreeStore>(() => ({
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

const NodeTreeStoreContext = createContext<StoreApi<NodeTreeStore> | null>(null);

NodeTreeStoreContext.displayName = "NodeTreeStoreContext";

function useNodeTreeStore<T extends (state: NodeTreeStore) => any>(selector: T): ReturnType<T> {
    const store = use(NodeTreeStoreContext);

    if (import.meta.env.DEV && !store) {
        error("useNodeTreeStore must be used within a NodeTreeStoreProvider");
    }

    return useStore(store!, useShallow(selector));
}

export component NodeTree({
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
        const needsToBeShown = new Set<string>();

        if (selectedNode) {
            let cur = selectedNode;

            do {
                needsToBeShown.add(getNodeKey(cur));
            } while ((cur = cur.parent));
        }


        store.setState(({ collapsedNodeKeys }) => ({
            selectedNodeKey,
            collapsedNodeKeys: collapsedNodeKeys.difference(needsToBeShown),
        }));
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
    const focusRef = useRef<HTMLDivElement>(null);

    function focusLater() {
        requestAnimationFrame(() => {
            if (focusRef.current) {
                focusRef.current.focus();
            }
        });
    }

    const installKeybinds = useKeybinds([
        {
            key: KeybindKeys.Up,
            timing: "down",
            handle(ev) {
                if (!selectedNode) {
                    return;
                }

                ev.preventDefault();
                focusLater();

                const parent = getParent(selectedNode);

                if (!parent) {
                    return;
                }

                const children = getChildrenWithMode(parent, treeMode);
                const idx = children.indexOf(selectedNode);

                if (idx > 0) {
                    onSelectNode.current(children[idx - 1]);
                } else {
                    onSelectNode.current(parent);
                }
            },
        },
        {
            key: KeybindKeys.Left,
            timing: "down",
            handle(ev) {
                if (!selectedNode) {
                    return;
                }

                ev.preventDefault();
                focusLater();

                const nodeKey = getNodeKey(selectedNode);

                if (store.getState().collapsedNodeKeys.has(nodeKey)) {
                    const parent = getParent(selectedNode);

                    if (parent) {
                        onSelectNode.current(parent);
                    }
                } else {
                    onNodeArrowClick.current(nodeKey);
                }
            },
        },
        {
            key: KeybindKeys.Right,
            timing: "down",
            handle(ev) {
                if (!selectedNode) {
                    return;
                }

                ev.preventDefault();
                focusLater();

                const nodeKey = getNodeKey(selectedNode);

                if (store.getState().collapsedNodeKeys.has(nodeKey)) {
                    onNodeArrowClick.current(nodeKey);
                } else {
                    const children = getChildrenWithMode(selectedNode, treeMode);

                    if (!children.length) {
                        return;
                    }

                    onSelectNode.current(children[0]);
                }
            },
        },
        {
            key: KeybindKeys.Down,
            timing: "down",
            handle(ev) {
                if (!selectedNode) {
                    return;
                }

                ev.preventDefault();
                focusLater();

                const parent = getParent(selectedNode);

                if (!parent) {
                    return;
                }


                const siblings = getChildrenWithMode(parent, treeMode);
                const idx = siblings.indexOf(selectedNode);

                if (idx === siblings.length - 1) {
                    onSelectNode.current(parent);
                } else if (idx >= 0) {
                    onSelectNode.current(siblings[idx + 1]);
                }
            },
        },
    ]);

    const refs = useComposedRefs(installKeybinds, focusRef);

    <div className="flex size-full flex-col">
        <div className="bg-cyan-600">
            {"Input Box"}
        </div>
        <ScrollArea dir={ScrollAreaDirection.BOTH}>
            <div
                {ref refs}
                tabIndex={0}
            >
                <NodeTreeStoreContext value={store}>
                    <NodeTreeNode
                        node={root}
                        nodeKey={getNodeKey(root)}
                        onNodeArrowClick={onNodeArrowClick}
                        onSelectNode={onSelectNode}
                    />
                </NodeTreeStoreContext>
            </div>
        </ScrollArea>
    </div>
}

interface NodeTreeNodeProps {
    node: Node;
    nodeKey: string;
    onNodeArrowClick: RefObject<(nodeKey: string) => void>;
    onSelectNode: RefObject<NodeTreeProps["onSelectNode"] & {}>;
}

component NodeTreeNode({ node, nodeKey: key, onNodeArrowClick, onSelectNode }: NodeTreeNodeProps) {
    const name = getNodeName(node);
    const ref = useRef<HTMLDivElement>(null);
    const scrollAreaCtx = use(ScrollAreaContext);

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

    useChange((_, cur) => {
        if (cur && ref.current) {
            scrollIntoView(ref.current, {
                boundary: scrollAreaCtx.ref.current,
                behavior: "instant",
                scrollMode: "if-needed",
                inline: "center",
            });
        }
    }, isSelected);

    const children = <tsx>
        <Clickable
            ref={ref}
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
    </tsx>;

    if (childNodes.length) {
            const contents = <tsx>
                <NodeTreeNodeContentsWrapper
                    reparseCount={reparseCount}
                    childNodes={childNodes}
                    onNodeArrowClick={onNodeArrowClick}
                    onSelectNode={onSelectNode}
                />
            </tsx>;
            <TreeAccordion
                {contents}
                onArrowClick={() => {
                    onNodeArrowClick.current(key);
                }}
                open={!isCollapsed}
            >
                {children}
            </TreeAccordion>
    } else {
        <Fragment>{children}</Fragment>
    }
}

interface NodeTreeNodeContentsWrapperProps {
    reparseCount: number;
    childNodes: readonly Node[];
    onNodeArrowClick: NodeTreeNodeProps["onNodeArrowClick"];
    onSelectNode: NodeTreeNodeProps["onSelectNode"];
}

component NodeTreeNodeContentsWrapper({reparseCount, childNodes, onNodeArrowClick, onSelectNode}: NodeTreeNodeContentsWrapperProps) {
    <div className="flex h-fit">
        <div className="w-5 shrink-0" />
        <div className="grow">
            for (const node of childNodes; key `${getNodeKey(node)}-${reparseCount}`) {
                const nodeKey = getNodeKey(node);
                <NodeTreeNode
                    {node}
                    {nodeKey}
                    {onNodeArrowClick}
                    {onSelectNode}
                />
            }
        </div>
    </div>
}