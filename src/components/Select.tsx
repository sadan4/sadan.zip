// import { createContext, type PropsWithChildren } from "react";

import { useRecent } from "@/hooks/recent";
import { joinWithKey } from "@/utils/array";
import cn from "@/utils/cn";
import { prop } from "@/utils/functional";
import { animated, useSpringValue } from "@react-spring/web";

import CheckCircle from "./icons/CheckCircle";
import { border } from "./border";
import { Clickable } from "./Clickable";
import { HorizontalLine } from "./HorizontalLine";
import z from "./z";

import invariant from "invariant";
import { type Dispatch, type Key, type PropsWithChildren, type ReactNode, type SetStateAction, useCallback, useEffect, useRef, useState } from "react";

// interface SelectContext {

// }

// const SelectContext = createContext<SelectContext | null>(null);

// export interface SelectProps extends PropsWithChildren {
//     initialVisibility?: boolean;
//     visible?: boolean;
// }

// export function Select({}: SelectProps) {
//     return null;
// }

// export function SelectContent() {
//     return null;
// }

export interface SelectOption<T> {
    disabled?: boolean;
    value: T;
    label: ReactNode;
    /**
     * The value to search by when using the keyboard to select an item
     */
    typedValue: string | null;
    key?: Key;
    default?: boolean;
}


interface ControlledStateOptions<T> {
    managedValue: T | undefined;
    handleChange?: (newValue: NoInfer<T>) => void;
    initialValue: T;
    debugName?: string;
}

function isFunction(func: any): func is ((...a: any[]) => any) {
    return typeof func === "function";
}

// credit to radix
function useControlledState<T>({ managedValue, initialValue, handleChange = () => {}, debugName = "UNKNOWN_COMPONENT" }: ControlledStateOptions<T>) {
    const [uncontrolledState, setUncontrolledState] = useState(initialValue);
    const isControlled = managedValue !== undefined;
    const value = isControlled ? managedValue : uncontrolledState;
    const latestChangeFunc = useRecent(handleChange);

    // OK to disable conditionally calling hooks here because they will always run
    // consistently in the same environment. Bundlers should be able to remove the
    // code block entirely in production.
    // eslint-disable-next-line react-hooks/react-compiler
    /* eslint-disable react-hooks/rules-of-hooks */
    if (process.env.NODE_ENV !== "production") {
        const isControlledRef = useRef(managedValue !== undefined);

        useEffect(() => {
            const wasControlled = isControlledRef.current;

            if (wasControlled !== isControlled) {
                const from = wasControlled ? "controlled" : "uncontrolled";
                const to = isControlled ? "controlled" : "uncontrolled";

                console.warn(`${debugName} is changing from ${from} to ${to}. Components should not switch from controlled to uncontrolled (or vice versa). Decide between using a controlled or uncontrolled value for the lifetime of the component.`);
            }
            isControlledRef.current = isControlled;
        }, [isControlled, debugName]);
    }

    /* eslint-enable react-hooks/rules-of-hooks */

    const setValue = useCallback<SetStateFunc<T>>((nextValue) => {
        if (isControlled) {
            const value = isFunction(nextValue) ? nextValue(managedValue) : nextValue;

            if (value !== managedValue) {
                latestChangeFunc.current(value);
            }
        } else {
            setUncontrolledState(nextValue);
        }
    }, [isControlled, latestChangeFunc, managedValue]);

    return [value, setValue] as const;
}

type SetStateFunc<T> = Dispatch<SetStateAction<T>>;

export interface SelectProps<T extends PropertyKey> extends PropsWithChildren {
    items: SelectOption<T>[];
    selectedValue?: NoInfer<T>;
    defaultValue?: NoInfer<T>;
    customChildren?: boolean;
    onChange?: (selection: NoInfer<T>) => void;
    closeOnSelect?: boolean;
}

interface SelectItemProps<T> {
    item: SelectOption<T>;
    isSelected: boolean;
    onChange: (item: NoInfer<T>) => void;
}

function SelectItem<T>({ item: { label, value, disabled }, isSelected, onChange }: SelectItemProps<T>) {
    return (
        <Clickable
            className={cn("flex items-center p-2", disabled && "brightness-50")}
            onClick={() => onChange(value)}
        >
            <div className="flex-1/1">{label}</div>
            {isSelected && <CheckCircle className="fill-info"/>}
        </Clickable>
    );
}

interface SelectMenuProps<T> {
    items: SelectOption<T>[];
    selectedItem: NoInfer<T>;
    onChange: (item: NoInfer<T>) => void;
}

function SelectMenu<T>({ items, onChange, selectedItem }: SelectMenuProps<T>) {
    return (
        <div className="bg-bg-300 rounded-md border-bg-fg-700 border-3 flex flex-col">
            {joinWithKey(items.map((item) => {
                return (
                    <SelectItem
                        key={item.key}
                        onChange={onChange}
                        isSelected={selectedItem === item.value}
                        item={item}
                    />
                );
            }), (i) => (
                <HorizontalLine
                    key={i}
                    className="w-3/4 h-0.5 self-center"
                />
            ))}
        </div>
    );
}


function getDefaultItem<T>(items: SelectOption<T>[]): SelectOption<T> {
    return items.find(prop("value")) ?? items[0];
}

export function Select<T extends PropertyKey>({
    items,
    customChildren = false,
    defaultValue = getDefaultItem(items).value,
    onChange,
    selectedValue,
    children,
    closeOnSelect = true,
}: SelectProps<T>) {
    const [selectedItem, setSelectedItem] = useControlledState({
        initialValue: defaultValue,
        debugName: "Select",
        managedValue: selectedValue,
        handleChange: onChange,
    });

    const [open, setOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (open && menuRef.current) {
            menuRef.current.focus();
        }
    }, [open]);

    function getItem(value: T) {
        return items.find((i) => i.value === value);
    }

    const rotation = useSpringValue(0);

    useEffect(() => {
        rotation.start(open ? 180 : 0);
    }, [open, rotation]);

    return (
        <>
            <div
                ref={ref}
                className="relative"
            >
                <Clickable
                    className={cn("items-center w-full rounded-md p-2 flex", border.compose("interactive", "autofocus", "animate"), open && border.focused)}
                    onClick={() => {
                        setOpen((o) => !o);
                    }}
                >
                    <div className="flex-1/1">
                        {customChildren ? children : getItem(selectedItem)?.label}
                    </div>
                    <animated.svg
                        viewBox="-2.4 -2.4 28.8 28.8"
                        className="fill-none stroke-bg-fg w-8 h-8"
                        style={{
                            transform: rotation.to((r) => `rotate(${r}deg)`),
                        }}
                    >
                        <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth="2"
                            d="m6 9 6 6 6-6"
                        />
                    </animated.svg>
                </Clickable>
                <div className={cn("absolute left-0 w-full", z.select)}>
                    {open && (
                        <div
                            onBlur={({ relatedTarget }) => {
                                invariant(ref.current, "how are we running this without ref.current being set");
                                if (ref.current.contains(relatedTarget)) {
                                    return;
                                }
                                setOpen(false);
                            }}
                            ref={menuRef}
                            tabIndex={-1}
                        >
                            <SelectMenu
                                items={items}
                                selectedItem={selectedItem}
                                onChange={(label) => {
                                    setSelectedItem(label);
                                    if (closeOnSelect) {
                                        setOpen(false);
                                    }
                                }}
                            />
                        </div>
                    )}
                </div>
            </div>
        </>
    );
}
