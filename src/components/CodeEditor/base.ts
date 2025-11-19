import type { Language } from "@/utils/textmate";

import type { Ref } from "react";

export interface CodeEditorProps<THandle> {
    initialCode?: string;
    onChange?(newCode: string): void;
    language?: Language;
    width?: string;
    height?: string;
    className?: string;
    ref?: Ref<THandle>;
}

