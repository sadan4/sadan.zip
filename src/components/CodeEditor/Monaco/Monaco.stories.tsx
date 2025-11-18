import { Language } from "@/utils/textmate";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { MonacoCodeEditor } from "./CodeEditorMonaco";
import sample from "./CodeEditorMonaco.tsx?raw";

import { Uri } from "monaco-editor";

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
        language: Language.TYPESCRIPT_REACT,
        uri: Uri.file("sample.tsx"),
    },
};
