import { Button } from "@/components/Button";
import { Accordion } from "@/components/layout/Accordion";
import { Text } from "@/components/Text";
import { EMPTY_SET, NBSP } from "@/utils/constants";
import { getPublicKeys, isNode } from "@/utils/typescript";

import { PropViewerFlags, SYM_NOT_COMPUTED } from "./constants";

import { useMemo, useState } from "react";
import type { Node } from "typescript";

interface ObjectProp<T> {
    node: T;
}

function ObjectProp<T>({ node }: ObjectProp<T>) {
    const props = useMemo(() => {
        if (isNode(node)) {
            return getPublicKeys(node) as Readonly<Set<keyof T>>;
        }
        return EMPTY_SET;
    }, [node]);

    return (
        <div>
            {
                Array.from(props.values()).map((prop) => {
                    return (
                        <SingleProp
                            node={node}
                            prop={prop}
                            flags={PropViewerFlags.NONE}
                        />
                    );
                })
            }
        </div>
    );
}

interface SinglePropProps {
    node: any;
    prop: keyof any;
    flags: PropViewerFlags;
}

function SingleProp({ node, prop, flags }: SinglePropProps) {
    const desc = Object.getOwnPropertyDescriptor(node, prop);

    if (!desc) {
        return <NotDefinedProp prop={prop} />;
    } else if (desc.get && desc.set) {
        return (
            <GetAndSetProp
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

function GetAndSetProp({ prop, node, flags, desc: { set, get } }: ComputedSinglePropProps) {
    const [value, setValue] = useState<any>(() => {
        if (flags & PropViewerFlags.EAGER_GETTERS) {
            return node[prop];
        }
        return SYM_NOT_COMPUTED;
    });

    return (
        <div>
            <Accordion item={{
                id: "",
                contents: (
                    <div>
                        {
                            get && (
                                <div>
                                    {
                                        value === SYM_NOT_COMPUTED
                                            ? (
                                                <>
                                                    <Text>
                                                        get():{NBSP}
                                                    </Text>
                                                    <Button onClick={() => {
                                                        setValue(() => node[prop]);
                                                    }}
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
                                                        flags={PropViewerFlags.EAGER_GETTERS}
                                                    />
                                                    {
                                                        !!(flags & PropViewerFlags.CAN_RECALCULATE_GETTERS) && (
                                                            <Button onClick={() => {
                                                                setValue(() => node[prop]);
                                                            }}
                                                            >
                                                                R
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
                ),
            }}
            >
                {String(prop)}
            </Accordion>
        </div>
    );
}

function ValueProp({ desc, prop }: ComputedSinglePropProps) {
    const { value } = desc;
    const isSingle = value !== null && (typeof value === "object");

    if (isSingle) {
        return (
            <div>
                <Text tag="span">
                    {String(prop)}:{NBSP}
                </Text>
                <Text tag="span">
                    {String(value)}
                </Text>
            </div>
        );
    }
    return <ObjectProp node={value} />;
}

function NotDefinedProp({ prop }: Pick<ComputedSinglePropProps, "prop">) {
    return (
        <div>
            <Text tag="span">
                {String(prop)}
            </Text>
            : {"<NOT_DEFINED>"}
        </div>
    );
}

export interface PropViewerProps {
    node: Node;
}

export function PropViewer({ node }: PropViewerProps) {
    return (
        <>
            <div>
                <Accordion item={{
                    id: "",
                    contents: <ObjectProp node={node} />,
                }}
                >
                    <Text>
                        Properties
                    </Text>
                </Accordion>
            </div>
        </>
    );
}
