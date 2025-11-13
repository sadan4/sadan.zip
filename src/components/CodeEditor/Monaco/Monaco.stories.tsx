import type { Meta, StoryObj } from "@storybook/react-vite";

import sample from "./index.tsx?raw";
import { MonacoCodeEditor } from ".";

import { Uri } from "monaco-editor";
import { Language } from "@/utils/textmate";

const meta = {
    component: MonacoCodeEditor,
    render(args) {
        return (
            <div className="h-[80vh] w-[90vw]">
                <MonacoCodeEditor {...args} />
            </div>
        );
    },
} satisfies Meta<typeof MonacoCodeEditor>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        initialCode: sample,
        language: Language.TYPESCRIPT,
        uri: Uri.file("sample.tsx"),
    },
};
