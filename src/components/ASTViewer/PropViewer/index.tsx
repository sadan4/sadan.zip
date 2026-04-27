import { Button } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Text, type TextProps } from "@/components/Text";
import { useRecent } from "@/hooks/recent";
import { NBSP } from "@/utils/constants";
import { todo } from "@/utils/error";
import { getPropertyDescriptor } from "@/utils/obj";
import { type Primitive } from "@/utils/types";
import { getNodeName, getPublicKeys, isNode } from "@/utils/typescript";

import { PropViewerFlags, SYM_NOT_COMPUTED } from "./constants";
import { PropViewerContext, SourceFileContext } from "./context";
import { TreeAccordion } from "../TreeAccordion";

import { ExternalLinkIcon, RedoDotIcon } from "lucide-react";
import { use, useEffect, useMemo, useState } from "react";
import type { Node, SourceFile } from "typescript";

interface ObjectProp<T> {
    node: T;
}

function NonNodeObjectProp({ node }: ObjectProp<object>) {
    return (
        <div>
            {Object.keys(node).map((prop) => {
                return (
                    <SingleProp
                        key={prop}
                        node={node}
                        prop={prop}
                        flags={PropViewerFlags.NONE}
                    />
                );
            })}
        </div>
    );
}

const NODE_IGNORED_KEYS = Object.freeze(new Set<keyof Node>([
    "getSourceFile",
    "getChildAt",
    "getChildren",
    "parent",
    // TODO: support these
    "getFirstToken",
    "getLastToken",
    "forEachChild",
    // not public
    "isUnterminated" as any,
    "rawText" as any,
    "tagName" as any,
    "comment" as any,
    "typeExpression" as any,
    "isNameFirst" as any,
    "isBracketed" as any,
    "escapedText" as any,
]));

const FUNCTION_NODE_KEYS = Object.freeze(new Set([
    "getChildCount",
    "getStart",
    "getEnd",
    "getFullStart",
    "getWidth",
    "getFullWidth",
    "getLeadingTriviaWidth",
    "getFullText",
    "getText",
] satisfies (keyof Node)[]));

type NodeMethodName = typeof FUNCTION_NODE_KEYS extends Set<infer U> ? U : never;

function invokeNodeMethod(node: Node, methodName: NodeMethodName, sf: SourceFile): any {
    switch (methodName) {
        case "getChildCount":
        case "getFullStart":
        case "getStart":
        case "getEnd":
        case "getWidth":
        case "getFullWidth":
        case "getLeadingTriviaWidth":
        case "getFullText":
        case "getText":
            return node[methodName](sf);
    }
}

const CUSTOM_NODE_KEYS = Object.freeze(new Set([
    "kind",
    // "pos",
    // "end",
    // "flags",
] satisfies (keyof Node)[]));

const SPECIAL_KEYS = Object.freeze(new Set<never>()
    .union(CUSTOM_NODE_KEYS)
    .union(FUNCTION_NODE_KEYS));

function NodeObjectProp({ node }: ObjectProp<Node>) {
    const props = getPublicKeys(node).difference(NODE_IGNORED_KEYS);
    const sourceFile = use(SourceFileContext);
    const { kind } = node;

    return (
        <>
            <Text>
                kind:{NBSP}
                <SingleValueText value={kind} />
                <Text tag="span">
                    {NBSP}({getNodeName(node)})
                </Text>
            </Text>
            {
                Array.from(props.difference(SPECIAL_KEYS).values()).map((prop) => {
                    return (
                        <SingleProp
                            key={prop}
                            node={node}
                            prop={prop}
                            flags={PropViewerFlags.NONE}
                        />
                    );
                })
            }
            {
                Array.from(props.intersection(FUNCTION_NODE_KEYS).values()).map((prop) => {
                    return (
                        <SingleProp
                            key={prop}
                            node={{
                                // NOTE: bug in react compiler
                                // facebook/react#35203
                                [prop]: () => {
                                    return invokeNodeMethod(node, prop, sourceFile);
                                },
                            }}
                            prop={prop}
                            flags={PropViewerFlags.INVOKE}
                        />
                    );
                })
            }
        </>
    );
}

function ObjectProp({ node }: ObjectProp<object>) {
    if (isNode(node)) {
        return <NodeObjectProp node={node} />;
    }
    return <NonNodeObjectProp node={node as object} />;
}

function GetOrSetProp({ prop, node, flags, desc: { set, get } }: ComputedSinglePropProps) {
    const [value, setValue] = useState<any>(() => {
        if (flags & PropViewerFlags.EAGER_GETTERS) {
            return node[prop];
        }
        return SYM_NOT_COMPUTED;
    });

    useEffect(() => {
        setValue(() => {
            if (flags & PropViewerFlags.EAGER_GETTERS) {
                return node[prop];
            }
            return SYM_NOT_COMPUTED;
        });
    }, [node, prop, flags]);

    const [open, setOpen] = useState(true);

    return (
        <div>
            <TreeAccordion
                contents={(
                    <div>
                        {
                            get && (
                                <div>
                                    {
                                        value === SYM_NOT_COMPUTED
                                            ? (
                                                <>
                                                    <Text
                                                        tag="span"
                                                    >
                                                        get():{NBSP}
                                                    </Text>
                                                    <Button
                                                        onClick={() => {
                                                            setValue(() => node[prop]);
                                                        }}
                                                        size="sm"
                                                        className="p-0"
                                                        colorType="text"
                                                    >
                                                        (...)
                                                    </Button>
                                                </>
                                            )
                                            : (
                                                <>
                                                    <ValueProp
                                                        node={{ "get()": value }}
                                                        desc={{
                                                            value,
                                                        }}
                                                        prop="get()"
                                                        flags={flags}
                                                    />
                                                    {
                                                        !!(flags & PropViewerFlags.CAN_RECALCULATE_GETTERS) && (
                                                            <Button
                                                                onClick={() => {
                                                                    setValue(() => node[prop]);
                                                                }}
                                                                colorType="text"
                                                            >
                                                                <RedoDotIcon className="size-5" />
                                                            </Button>
                                                        )
                                                    }
                                                </>
                                            )
                                    }
                                </div>
                            )
                        }
                        {
                            set && (
                                <div>
                                    <ValueProp
                                        desc={{
                                            value: set,
                                        }}
                                        node={{
                                            "set()": set,
                                        }}
                                        flags={flags}
                                        prop="set()"
                                    />
                                </div>
                            )
                        }
                    </div>
                )}
                onArrowClick={() => {
                    setOpen((open) => !open);
                }}
                open={open}
            >
                <Clickable onClick={() => {
                    setOpen((open) => !open);
                }}
                >
                    <Text>
                        {String(prop)}:{NBSP}
                    </Text>
                </Clickable>
            </TreeAccordion>
        </div>
    );
}

interface SinglePropProps {
    node: any;
    prop: keyof any;
    flags: PropViewerFlags;
}

function SingleProp({ node, prop, flags }: SinglePropProps) {
    const desc = getPropertyDescriptor(node, prop);

    if (!desc) {
        if (flags & PropViewerFlags.SHOW_UNDEFINED) {
            return <NotDefinedProp prop={prop} />;
        }
        return null;
    } else if (desc.get || desc.set) {
        return (
            <GetOrSetProp
                node={node}
                prop={prop}
                flags={flags}
                desc={desc}
            />
        );
    }
    return (
        <ValueProp
            node={node}
            prop={prop}
            flags={flags}
            desc={desc}
        />
    );
}

interface ComputedSinglePropProps extends SinglePropProps {
    desc: PropertyDescriptor;
}


function ValueProp({ desc, node, prop, flags }: ComputedSinglePropProps) {
    let { value } = desc;
    const { onSelectNode } = use(PropViewerContext);
    const [open, setOpen] = useState(false);

    if (typeof value === "function") {
        if (flags & PropViewerFlags.INVOKE) {
            value = node[prop]();
        } else if (!(flags & PropViewerFlags.SHOW_FUNCTIONS)) {
            return null;
        }
    }

    const isSingle = (value !== null && (typeof value !== "object")) || (Array.isArray(value) && !value.length);

    if (isSingle) {
        return (
            <div>
                <Text tag="span">
                    {String(prop)}:{NBSP}
                </Text>
                <SingleValueText value={value} />
            </div>
        );
    }
    return (
        <div>
            <TreeAccordion
                contents={() => (
                    <div className="flex h-fit">
                        <div className="w-5 shrink-0" />
                        <div className="grow">
                            <ObjectProp node={value} />
                        </div>
                    </div>
                )}
                open={open}
                onArrowClick={() => {
                    setOpen((open) => !open);
                }}
            >
                <Clickable onClick={() => {
                    setOpen((open) => !open);
                }}
                >
                    <Text className="flex place-content-center">
                        {String(prop)}:{NBSP}
                        {
                            isNode(value)
                            && (
                                <Button
                                    onClick={() => {
                                        onSelectNode(value);
                                    }}
                                    colorType="text"
                                    className="p-0"
                                >
                                    <ExternalLinkIcon className="size-5" />
                                </Button>
                            )
                        }
                    </Text>
                </Clickable>
            </TreeAccordion>
        </div>
    );
}

function SingleValueText({ value }: { value: Primitive; }) {
    let color: TextProps["color"];

    switch (typeof value) {
        case "boolean":
            color = value ? "success" : "error";
            value = String(value);
            break;
        case "number":
            color = "info";
            break;
        case "symbol":
            color = "warning";
            value = String(value);
            break;
        case "undefined":
        case "object":
            color = "warning";
            if (Array.isArray(value)) {
                value = "[]";
            } else {
                value = String(value);
            }
            break;
        case "string":
            value = JSON.stringify(value.slice(0, 125));
            break;
        case "bigint":
            return (
                <span>
                    <Text color="info">
                        {String(value)}
                    </Text>
                    <Text color="warning">n</Text>
                </span>
            );
        case "function":
            todo("handle functions as classes (static) (not here)");
            break;
    }
    return (
        <Text
            tag="span"
            color={color}
            className="overflow-clip whitespace-nowrap"
        >
            {value}
        </Text>
    );
}

function NotDefinedProp({ prop }: Pick<ComputedSinglePropProps, "prop">) {
    return (
        <Text>
            {String(prop)}:{NBSP}
            <Text
                tag="span"
                color="warning"
            >
                {"<NOT_DEFINED>"}
            </Text>
        </Text>
    );
}

export interface PropViewerProps {
    node: Node;
    onSelectNode: (node: Node) => void;
}

export function PropViewer({ node, onSelectNode }: PropViewerProps) {
    const cbRef = useRecent(onSelectNode);
    const [open, setOpen] = useState(true);

    const contextValue = useMemo<PropViewerContext>(() => (
        {
            onSelectNode(node) {
                cbRef.current(node);
            },
        }
    ), [cbRef]);

    return (
        <PropViewerContext value={contextValue} >
            <ScrollArea className="pl-2">
                <TreeAccordion
                    contents={() => (
                        <div className="flex h-fit">
                            <div className="w-5 shrink-0" />
                            <div className="max-w-full grow">
                                <ObjectProp node={node} />
                            </div>
                        </div>
                    )}
                    open={open}
                    onArrowClick={() => {
                        setOpen((open) => !open);
                    }}
                >
                    <Clickable
                        onClick={() => {
                            setOpen((open) => !open);
                        }}
                    >
                        <Text>
                            Properties
                        </Text>
                    </Clickable>
                </TreeAccordion>
            </ScrollArea>
        </PropViewerContext>
    );
}
