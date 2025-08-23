import { useDebouncedFn } from "@/hooks/debouncedFn";
import cn, { textColors, textSize, textWeight } from "@/utils/cn";
import error from "@/utils/error";
import { updateRef } from "@/utils/ref";
import { animated, useSpringValue } from "@react-spring/web";

import { useCursorContextStore } from "./Cursor/cursorContextStore";
import { ErrorIcon } from "./icons/ErrorIcon";
import { Text } from "./Text";

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

}

export function Input({
    className,
    textSize,
    onMouseOver,
    onMouseOut,
    ref: _ref,
    initialValue,
    value,
    ...props
}: InputProps) {
    const shouldNullOnDemount = useRef(false);
    const ref = useRef<HTMLInputElement>(null);
    const isManaged = value !== undefined;

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
        <input
            type="text"
            name="Text Input"
            className={cn("bg-bg-300 rounded-md px-3 py-2 w-full disabled:pointer-events-none disabled:cursor-not-allowed disabled:select-none disabled:opacity-50 outline-none transition-[color,box-shadow] focus-visible:ring-2 ring-2 ring-bg-fg-600/25 focus:ring-bg-fg-600", inputSizes[textSize ?? "md"], className)}
            onMouseOver={(e) => {
                try {
                    onMouseOver?.(e);
                } catch (e) {
                    console.error("Error occurred in onMouseOver:", e);
                }
                shouldNullOnDemount.current = true;
                useCursorContextStore
                    .getState()
                    .updateTextElement(e.nativeEvent.target as Element);
            }}
            onMouseOut={(e) => {
                try {
                    onMouseOut?.(e);
                } catch (e) {
                    console.error("Error occurred in onMouseOut:", e);
                }
                shouldNullOnDemount.current = false;
                useCursorContextStore
                    .getState()
                    .updateTextElement(null);
            }}
            value={value}
            {...props}
            ref={(e) => {
                updateRef(ref, e);
                updateRef(_ref, e);
            }}
        />
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
            className="flex gap-1 items-center"
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
    ref: _ref,
    ...props
}: CheckedInputProps) {
    const [error, setError] = useState<ReactNode>(null);
    const ref = useRef<HTMLInputElement>(null);
    const checkInitialRender = useRef(true);
    const animateInitialRender = useRef(true);
    const errorRef = useRef<HTMLDivElement>(null);
    const hasError = !!error;
    const height = useSpringValue(0);

    useEffect(() => {
        if (errorRef.current) {
            const size = errorRef.current.getBoundingClientRect();

            if (animateInitialRender.current) {
                height.set(size.height);
                animateInitialRender.current = false;
            } else {
                height.start(size.height);
            }
        }
    });

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
                className={cn(hasError && "ring-error/65 focus:ring-error", className)}
                onChange={handleChange}
                disabled={disabled}
            />
            <animated.div style={{ height }}>
                <div
                    ref={errorRef}
                    className={cn(disabled && "opacity-50")}
                >
                    {error}
                </div>
            </animated.div>
        </div>
    );
}
