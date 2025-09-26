import { border } from "@/styles";
import cn, { textColors, textSize, textWeight } from "@/utils/cn";
import error from "@/utils/error";
import { updateRef } from "@/utils/ref";
import { useCursorContextStore } from "@components/Cursor/cursorContextStore";
import { AnimateHeight } from "@effects/AnimateHeight";
import { useDebouncedFn } from "@hooks/debouncedFn";

import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";
import Close from "../icons/Close";
import { ErrorIcon } from "../icons/ErrorIcon";
import { Text } from "../Text";

import { type ChangeEvent, type ChangeEventHandler, type ComponentProps, type PropsWithChildren, type ReactNode, useEffect, useRef, useState } from "react";

const inputSizes = {
    sm: "text-sm",
    md: "text-md",
    lg: "text-lg",
    xl: "text-xl",
    "2xl": "text-2xl",
    "3xl": "text-3xl",
} as const;

export interface InputProps extends ComponentProps<"input"> {
    textSize?: keyof typeof inputSizes;
    initialValue?: string;
    onChange: ChangeEventHandler<HTMLInputElement>;
    clearButton?: boolean;
    onClear?: () => void;
}

export function Input({
    className,
    textSize,
    onMouseOver,
    onMouseOut,
    ref: _ref,
    initialValue,
    value,
    onClear = () => { },
    onChange,
    clearButton = false,
    ...props
}: InputProps) {
    const shouldNullOnDemount = useRef(false);
    const ref = useRef<HTMLInputElement>(null);
    const isManaged = value !== undefined;
    const [hasValue, setHasValue] = useState(Boolean(initialValue ?? value));

    useEffect(() => {
        if (shouldNullOnDemount.current) {
            useCursorContextStore
                .getState()
                .updateTextElement(null);
        }
    }, []);
    useEffect(() => {
        if (!isManaged && ref.current && initialValue) {
            ref.current.value = initialValue;
        }
    }, [initialValue, isManaged]);
    return (
        <div className="relative">
            <input
                type="text"
                name="Text Input"
                className={cn(styles.input, inputSizes[textSize ?? "md"], border.interactive, border.autofocus, border.animate, className)}
                onMouseOver={(e) => {
                    if (onMouseOver) {
                        try {
                            onMouseOver(e);
                        } catch (e) {
                            console.error("Error occurred in onMouseOver:", e);
                        }
                    }
                    shouldNullOnDemount.current = true;
                    useCursorContextStore
                        .getState()
                        .updateTextElement(e.nativeEvent.target as Element);
                }}
                onMouseOut={(e) => {
                    if (onMouseOut) {
                        try {
                            onMouseOut(e);
                        } catch (e) {
                            console.error("Error occurred in onMouseOut:", e);
                        }
                    }
                    shouldNullOnDemount.current = false;
                    useCursorContextStore
                        .getState()
                        .updateTextElement(null);
                }}
                onChange={(e) => {
                    onChange(e);
                    setHasValue(Boolean(e.target.value));
                }}
                value={value}
                {...props}
                ref={(e) => {
                    updateRef(ref, e);
                    updateRef(_ref, e);
                }}
            />
            {
                clearButton && hasValue && (
                    <div className="pointer-events-none absolute top-0 left-0 flex h-full w-full flex-row-reverse items-center pr-2" >
                        <Clickable
                            className="pointer-events-auto -mr-2 p-2"
                            onClick={() => {
                                if (isManaged) {
                                    onClear();
                                } else if (ref.current) {
                                    ref.current.value = "";
                                    ref.current.focus();
                                    onClear();
                                }
                                setHasValue(false);
                            }}
                        >
                            <Close
                                height={16}
                                width={16}
                                className="fill-fg-600"
                            />
                        </Clickable>
                    </div>
                )
            }
        </div>
    );
}
export interface LabeledInputProps extends InputProps, PropsWithChildren {
    labelColor?: keyof typeof textColors;
    labelSize?: keyof typeof textSize;
    labelWeight?: keyof typeof textWeight;
    wrapperClassName?: string;
}

export function LabeledInput({
    labelColor,
    labelSize = "md",
    labelWeight,
    children,
    wrapperClassName,
    ...props
}: LabeledInputProps) {
    return (
        <div className={cn("flex flex-col gap-1", wrapperClassName)}>
            {children && (
                <Text
                    color={labelColor}
                    size={labelSize}
                    weight={labelWeight}
                    className="w-fit"
                >
                    {children}
                </Text>
            ) }
            <Input {...props} />
        </div>
    );
}

export interface ErrorMessageProps {
    badValue: string;
    origCheck: CheckedInputProps["check"];
}

function throwIfInvalidLenCheck(lenCheck: LenCheck) {
    // both undefined
    if (!(lenCheck.min || lenCheck.max)) {
        error("Invalid length check");
    }
    // min < 0
    if (lenCheck.min != null && lenCheck.min < 0) {
        error("Invalid minimum length");
    }
    // max <= 0
    if (lenCheck.max != null && lenCheck.max <= 0) {
        error("Invalid maximum length");
    }
    // min >= max
    if (lenCheck.min != null && lenCheck.max != null && lenCheck.min >= lenCheck.max) {
        error("Invalid length check");
    }
}

function validateLength(check: LenCheck, value: string): boolean {
    throwIfInvalidLenCheck(check);

    const len = value.length;

    if (check.min != null && len < check.min) {
        return false;
    }
    if (check.max != null && len > check.max) {
        return false;
    }
    return true;
}

function formatInvalidLenMessage(check: LenCheck): string {
    if (check.min != null && check.max != null) {
        return `Input must be between ${check.min} and ${check.max} characters long`;
    }
    if (check.min != null) {
        return `Input must be at least ${check.min} characters long`;
    }
    if (check.max != null) {
        return `Input must be at most ${check.max} characters long`;
    }
    error("invalid state");
}

function DefaultErrorMessage({ origCheck }: ErrorMessageProps) {
    let msg: string;

    if (typeof origCheck === "function") {
        msg = "Invalid value";
    } else if (origCheck instanceof RegExp) {
        msg = `Input must match /${origCheck.source}/`;
    } else if (origCheck.type === "len") {
        msg = formatInvalidLenMessage(origCheck);
    } else {
        throw new Error("invalid check type");
    }

    return (
        <Text
            color="error"
            tag="span"
            size="sm"
            className="flex items-center gap-1"
            noselect
        >
            <ErrorIcon height={18} />
            {msg}
        </Text>
    );
}

interface LenCheck {
    type: "len";
    /**
     * inclusive
     */
    min?: number;
    /**
     * inclusive
     */
    max?: number;
}

export interface CheckedInputProps extends Omit<InputProps, "onChange">, PropsWithChildren {
    labelColor?: keyof typeof textColors;
    labelSize?: keyof typeof textSize;
    labelWeight?: keyof typeof textWeight;
    check: RegExp | ((value: string) => boolean) | LenCheck;
    errorMessage?: (props: ErrorMessageProps) => ReactNode;
    onValidChange: (e: ChangeEvent<HTMLInputElement> | undefined, value: string) => void;
    onInvalidChange?: (e: ChangeEvent<HTMLInputElement> | undefined, value: string) => void;
    debounce?: number;
    wrapperClassName?: string;
    checkInitialRender?: boolean;
}


function validate(msg: string, check: CheckedInputProps["check"]): boolean {
    if (typeof check === "function") {
        return check(msg);
    } else if (check instanceof RegExp) {
        return check.test(msg);
    } else if (check.type === "len") {
        return validateLength(check, msg);
    }
    throw new Error("invalid check type");
}
export function CheckedInput({
    labelColor,
    labelSize = "md",
    labelWeight,
    check,
    children,
    errorMessage: ErrorMessage = DefaultErrorMessage,
    wrapperClassName,
    className,
    onValidChange,
    onInvalidChange,
    disabled = false,
    debounce = 100,
    checkInitialRender: _checkInitialRender = true,
    ref: _ref,
    ...props
}: CheckedInputProps) {
    const [error, setError] = useState<ReactNode>(null);
    const ref = useRef<HTMLInputElement>(null);
    const checkInitialRender = useRef(_checkInitialRender);
    const hasError = !!error;

    // validate on initial render
    useEffect(() => {
        if (ref.current) {
            checkInitialRender.current = false;

            const valid = validate(ref.current.value, check);

            if (checkInitialRender.current) {
                if (valid) {
                    onValidChange(undefined, ref.current.value);
                } else {
                    onInvalidChange?.(undefined, ref.current.value);
                }
            }
            if (!valid) {
                setError((
                    <ErrorMessage
                        badValue={ref.current.value}
                        origCheck={check}
                    />
                ));
            }
        }
    }, [ErrorMessage, check, onInvalidChange, onValidChange]);

    const handleChange = useDebouncedFn((e) => {
        const valid = validate(e.target.value, check);

        if (valid) {
            setError(null);
            onValidChange(e, e.target.value);
        } else {
            setError((
                <ErrorMessage
                    badValue={e.target.value}
                    origCheck={check}
                />
            ));
            onInvalidChange?.(e, e.target.value);
        }
    }, debounce);


    return (
        <div className={cn("flex flex-col gap-1", wrapperClassName)}>
            {children && (
                <Text
                    color={labelColor}
                    size={labelSize}
                    weight={labelWeight}
                >
                    {children}
                </Text>
            )}
            <Input
                {...props}
                ref={(e) => {
                    updateRef(ref, e);
                    updateRef(_ref, e);
                }}
                className={cn(hasError && "ring-error-400/65 focus:ring-error-400", className)}
                onChange={handleChange}
                disabled={disabled}
            />
            <AnimateHeight>
                <div
                    className={cn(disabled && "opacity-50")}
                >
                    {error}
                </div>
            </AnimateHeight>
        </div>
    );
}
