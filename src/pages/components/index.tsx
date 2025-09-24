import { Boilerplate } from "@/components/Boilerplate";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { CheckedInput, Input, LabeledInput } from "@/components/Input";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines/HorizontalLine";
import { Marquee } from "@/components/Marquee";
import { SearchableSelect, Select, type SelectOption } from "@/components/Select";
import { Text } from "@/components/Text";
import { LabeledTextArea, TextArea } from "@/components/TextArea";
import { textSize, textWeight } from "@/utils/cn";

import { useState } from "react";

function SelectExample() {
    const selectionItems = [
        {
            label: "foo",
            value: "foo",
            key: "foo",
            typedValue: "foo",
        },
        {
            label: "bar",
            value: "bar",
            key: "bar",
            typedValue: "bar",
        },
        {
            label: "baz",
            value: "baz",
            key: "baz",
            typedValue: "baz",
        },
    ] as const satisfies SelectOption<PropertyKey>[];

    const manyItems = Array.from({ length: 100 }, (_, i) => ({
        label: `Item ${i + 1}`,
        value: `item${i + 1}`,
        key: `item${i + 1}`,
        typedValue: `item ${i + 1}`,
    })) satisfies SelectOption<PropertyKey>[];

    return (
        <>
            <Text
                size="xl"
                center
            >
                Selection Menu
            </Text>
            <Select
                items={selectionItems}
                defaultValue="bar"
            />
            <Text size="lg">
                Stay Open After Selection
            </Text>
            <Select
                items={selectionItems}
                defaultValue="bar"
                closeOnSelect={false}
            />
            <Text size="lg">
                Custom Label
            </Text>
            <Select
                items={selectionItems}
                defaultValue="bar"
                customChildren
            >
                <div className="flex items-center">
                    <Marquee className="w-fit">Custom Label</Marquee>
                </div>
            </Select>
            <Text size="lg">
                Custom Entries
            </Text>
            <Select
                items={[
                    {
                        label: (
                            <Text
                                size="sm"
                                color="accent"
                            >
                                Entry 1
                            </Text>
                        ),
                        value: "entry1",
                        typedValue: "entry 1",

                    },
                    {
                        label: (
                            <Text
                                size="lg"
                                color="error"
                            >
                                Entry 2
                            </Text>
                        ),
                        value: "entry2",
                        typedValue: "entry 2",
                    },
                    {
                        label: (
                            <Text
                                size="md"
                                weight="extraBold"
                                color="info"
                            >
                                Entry 3
                            </Text>
                        ),
                        value: "entry3",
                        typedValue: "entry 3",
                    },
                ]}
                defaultValue="entry2"
            />
            <Text size="lg">
                Many Items
            </Text>
            <Select
                items={manyItems}
                defaultValue="item50"
            />
            <Text size="lg">
                Many Items
            </Text>
            <SearchableSelect
                items={manyItems}
                defaultValue="item50"
            />
        </>
    );
}

function InputExample() {
    return (
        <>
            <Text
                size="xl"
                center
            >
                Input Field
            </Text>
            <div className="flex flex-col gap-2">
                <LabeledInput
                    onChange={() => {}}
                >
                    Input
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                >
                    With Placeholder
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    initialValue="Initial Value"
                >
                    With Initial Value
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    initialValue="Initial Value"
                    clearButton
                >
                    With Clear Button
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    disabled
                >
                    Disabled
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    initialValue="Read only Input"
                    readOnly
                >
                    Read Only
                </LabeledInput>
                <CheckedInput
                    check={() => false}
                    onValidChange={() => {}}
                >
                    Checked Input
                </CheckedInput>
                <CheckedInput
                    check={/fooba[rz]/}
                    onValidChange={() => {}}
                >
                    Regex Checked Input
                </CheckedInput>
                <CheckedInput
                    check={{
                        type: "len",
                        min: 3,
                        max: 10,
                    }}
                    onValidChange={() => {}}
                >
                    Length Checked Input
                </CheckedInput>
                <CheckedInput
                    check={/fooba[rz]/}
                    debounce={2000}
                    onValidChange={() => {}}
                >
                    2000s Debounced Check
                </CheckedInput>
                <CheckedInput
                    check={() => false}
                    errorMessage={() => (
                        <Text
                            size="sm"
                            color="error"
                            noselect
                        >
                            <Marquee>
                                Custom Error Message
                            </Marquee>
                        </Text>
                    )}
                    onValidChange={() => {}}
                >
                    Custom Error Message
                </CheckedInput>
                <CheckedInput
                    check={() => false}
                    onValidChange={() => { }}
                    disabled
                >
                    Disabled
                </CheckedInput>
            </div>
        </>
    );
}

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
            <div className="flex flex-wrap items-center justify-between mb-4 gap-y-4">
                <Text
                    size="xl"
                    center
                >
                    TextComponents
                </Text>
                <div className="flex items-center justify-between gap-3 w-min">
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
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="mt-4 flex flex-col items-center">
                    <Text
                        size="4xl"
                        weight="extraBold"
                    >
                        Component Testing
                    </Text>
                    <Box className="mt-6 w-[40vw]">
                        <SelectExample />
                        <HorizontalLine className="my-4" />
                        <InputExample />
                        <HorizontalLine className="my-4" />
                        <TextExample />
                        <HorizontalLine className="my-4" />
                        <TextAreaExample />
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
