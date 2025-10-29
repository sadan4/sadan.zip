import { Boilerplate } from "@/components/Boilerplate";
import { DefaultFooter, FooterContainer, FooterContent, FooterFooter } from "@/components/Footer";
import { Input } from "@/components/Input";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines/HorizontalLine";
import { Select, type SelectOption } from "@/components/Select";
import { Text } from "@/components/Text";
import { LabeledTextArea, TextArea } from "@/components/TextArea";
import { textSize, textWeight } from "@/utils/cn";

import { useState } from "react";

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

export default function Components() {
    return (
        <>
            <Boilerplate />
            <FooterContainer>
                <FooterContent>
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
                        </Box>
                    </div>
                </FooterContent>
                <FooterFooter>
                    <DefaultFooter />
                </FooterFooter>
            </FooterContainer>
        </>
    );
}
