import { AnimateHeight } from "@/components/effects/AnimateHeight";
import { useComposedRefs } from "@/hooks/composedRefs";
import { useDebouncedFn } from "@/hooks/debouncedFn";
import { border } from "@/styles";
import cn, { type textColors, type textSize, type textWeight } from "@/utils/cn";
import { error } from "@/utils/error";
import type { TOmit } from "@/utils/types";

import * as styles from "./styles.module.scss";
import { validateCheckedInput } from "./util";
import { Clickable } from "../Clickable";
import { ErrorIcon } from "../icons/ErrorIcon";
import { Text } from "../Text";

import { XIcon } from "lucide-react";
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
    onChange?: ChangeEventHandler<HTMLInputElement>;
    clearButton?: boolean;
    focusAfterClear?: boolean;
    onClear?(): void;
}

export function Input({
    className,
    textSize,
    ref: _ref,
    initialValue,
    value,
    onClear,
    onChange,
    clearButton = false,
    focusAfterClear = false,
    ...props
}: InputProps) {
    const ref = useRef<HTMLInputElement>(null);
    const refs = useComposedRefs(ref, _ref);
    const isManaged = value !== undefined;
    const [hasValue, setHasValue] = useState(Boolean(initialValue ?? value));

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
                onChange={(e) => {
                    onChange?.(e);
                    setHasValue(Boolean(e.target.value));
                }}
                value={value}
                {...props}
                ref={refs}
            />
            {
                clearButton && hasValue && (
                    <div className="pointer-events-none absolute top-0 left-0 flex h-full w-full flex-row-reverse items-center pr-2" >
                        <Clickable
                            className="pointer-events-auto -mr-2 p-2"
                            onClick={() => {
                                if (isManaged) {
                                    onClear?.();
                                } else if (ref.current) {
                                    ref.current.value = "";
                                    onClear?.();
                                }
                                if (focusAfterClear) {
                                    ref.current?.focus();
                                }
                                setHasValue(false);
                            }}
                        >
                            <XIcon
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
            <Text
                color={labelColor}
                size={labelSize}
                weight={labelWeight}
                className="w-fit"
            >
                {children}
            </Text>
            <Input {...props} />
        </div>
    );
}

export interface ErrorMessageProps {
    badValue: string;
    origCheck: CheckedInputProps["check"];
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
    } else {
        origCheck.type satisfies "len";
        msg = formatInvalidLenMessage(origCheck);
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

export interface LenCheck {
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

export interface CheckedInputProps extends TOmit<InputProps, "onChange">, PropsWithChildren {
    labelColor?: keyof typeof textColors;
    labelSize?: keyof typeof textSize;
    labelWeight?: keyof typeof textWeight;
    check: RegExp | ((value: string) => boolean) | LenCheck;
    errorMessage?(props: ErrorMessageProps): ReactNode;
    /**
     * e is undefined on initial render if checkInitialRender is true
     */
    onValidChange?(e: ChangeEvent<HTMLInputElement> | undefined, value: string): void;
    /**
     * e is undefined on initial render if checkInitialRender is true
     */
    onInvalidChange?(e: ChangeEvent<HTMLInputElement> | undefined, value: string): void;
    debounce?: number;
    wrapperClassName?: string;
    checkInitialRender?: boolean;
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
    const checkInitialRenderRef = useRef(_checkInitialRender);
    const hasError = !!error;

    // validate on initial render
    useEffect(() => {
        if (ref.current && checkInitialRenderRef.current) {
            const valid = validateCheckedInput(ref.current.value, check);

            if (valid) {
                onValidChange?.(undefined, ref.current.value);
            } else {
                onInvalidChange?.(undefined, ref.current.value);
            }

            if (!valid) {
                setError((
                    <ErrorMessage
                        badValue={ref.current.value}
                        origCheck={check}
                    />
                ));
            }
            checkInitialRenderRef.current = false;
        }
    }, [ErrorMessage, check, onInvalidChange, onValidChange]);

    const handleChange = useDebouncedFn((e) => {
        const valid = validateCheckedInput(e.target.value, check);

        if (valid) {
            setError(null);
            onValidChange?.(e, e.target.value);
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
            <Text
                color={labelColor}
                size={labelSize}
                weight={labelWeight}
            >
                {children}
            </Text>
            <Input
                {...props}
                ref={useComposedRefs(ref, _ref)}
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
