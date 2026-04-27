import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { Input } from "@/components/Input";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines/HorizontalLine";
import { Select, type SelectOption } from "@/components/Select";
import { Text } from "@/components/Text";
import { LabeledTextArea, TextArea } from "@/components/TextArea";
import { useToaster } from "@/hooks/toaster";
import { ToastPosition, ToastType } from "@/stores/ToastStore";
import { textSize, textWeight } from "@/utils/cn";
import { createFileRoute } from "@tanstack/react-router";

import { useState } from "react";

export const Route = createFileRoute("/_/components")({
    component: Components,
});


const textSizeSelectOptions: SelectOption<keyof typeof textSize>[] = Object.keys(textSize)
    .map((size) => {
        return {
            label: size,
            value: size,
            typedValue: size,
            key: size,
        } satisfies SelectOption<string>;
    }) as any;

const textWeightSelectOptions: SelectOption<keyof typeof textWeight>[] = Object.keys(textWeight)
    .map((weight) => {
        return {
            label: weight,
            value: weight,
            typedValue: weight,
            key: weight,
        } satisfies SelectOption<string>;
    }) as any;

function TextExample() {
    const [previewText, setPreviewText] = useState("");
    const [size, setSize] = useState<keyof typeof textSize>("md");
    const [weight, setWeight] = useState<keyof typeof textWeight>("normal");

    return (
        <>
            <div className="mb-4 flex flex-wrap items-center justify-between gap-y-4">
                <Text
                    size="xl"
                    center
                >
                    TextComponents
                </Text>
                <div className="flex w-min items-center justify-between gap-3">
                    <Select
                        className="w-20"
                        items={textSizeSelectOptions}
                        defaultValue={size}
                        onChange={(size) => setSize(size)}
                    />
                    <Select
                        className="w-30"
                        items={textWeightSelectOptions}
                        defaultValue={weight}
                        onChange={setWeight}
                    />
                </div>
                <Input
                    initialValue={previewText}
                    onChange={(e) => {
                        setPreviewText(e.target.value);
                    }}
                    placeholder="Preview Text"
                    className="w-fit"
                />
            </div>
            <Text
                weight={weight}
                size={size}
                key={`${weight}-${size}`}
                tag="span"
            >
                {previewText || `${size}-${weight}`}
                {previewText && (
                    <Text
                        size="md"
                        tag="span"
                    >
                        {` (${size}-${weight})`}
                    </Text>
                )}
            </Text>
        </>
    );
}

function TextAreaExample() {
    return (
        <>
            <Text
                size="xl"
                center
            >
                Text Area
            </Text>
            <TextArea />
            <LabeledTextArea resize="both">
                Resizable
            </LabeledTextArea>
        </>
    );
}

function ToastExample() {
    const api = useToaster();
    const [type, setType] = useState(ToastType.UNKNOWN);
    const [pos, setPos] = useState(ToastPosition.TOP);

    return (
        <>
            <Text
                size="xl"
                center
            >
                Toasts
            </Text>
            <div className="flex justify-between gap-4">
                <Select
                    items={[
                        {
                            label: "Unknown",
                            value: ToastType.UNKNOWN,
                            typedValue: "unknown",
                        },
                        {
                            label: "Info",
                            value: ToastType.INFO,
                            typedValue: "info",
                        },
                        {
                            label: "Success",
                            value: ToastType.SUCCESS,
                            typedValue: "success",
                        },
                        {
                            label: "Warning",
                            value: ToastType.WARNING,
                            typedValue: "warning",
                        },
                        {
                            label: "Error",
                            value: ToastType.ERROR,
                            typedValue: "error",
                        },
                    ]}
                    defaultValue={ToastType.UNKNOWN}
                    onChange={setType}
                />
                <Select
                    items={[
                        {
                            label: "Top",
                            value: ToastPosition.TOP,
                            typedValue: "top",
                        },
                        {
                            label: "Bottom",
                            value: ToastPosition.BOTTOM,
                            typedValue: "bottom",
                        },
                    ]}
                    defaultValue={ToastPosition.TOP}
                    onChange={setPos}
                />
                <Button onClick={() => {
                    const s = api.getState();

                    s.pushToast({
                        id: s.genId(),
                        duration: 3000,
                        type,
                        pos,
                        render: () => <div>Hello World!</div>,
                    });
                }}
                >
                    Push Toast
                </Button>
                <Button
                    onClick={() => {
                        api.getState().popToast();
                    }}
                    color="error"
                    colorType="outline"
                >
                    Pop Toast
                </Button>
            </div>
        </>
    );
}

function Components() {
    return (
        <>
            <Boilerplate />
            <div className="mt-4 flex flex-col items-center">
                <Text
                    size="4xl"
                    weight="extraBold"
                >
                    Component Testing
                </Text>
                <Box className="mt-6 w-[40vw]">
                    <TextExample />
                    <HorizontalLine className="my-4" />
                    <TextAreaExample />
                    <HorizontalLine className="my-4" />
                    <ToastExample />
                </Box>
            </div>
        </>
    );
}

