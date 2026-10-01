import { Button, type ButtonProps } from "@/components/Button";
import { Codeblock } from "@/components/Codeblock";
import { Text, type TextProps } from "@/components/Text";
import { Language } from "@/utils/textmate";

import { useModuleViewerSettingsStore } from "../-data";

interface FilterPillsProps<K extends string> {
    keys: readonly K[];
    labels: Record<K, string>;
    counts: Record<K, number>;
    enabled: ReadonlySet<K>;
    color: ButtonProps["color"] | Record<K, ButtonProps["color"]>;
    onToggle(key: K): void;
}

export function FilterPills<K extends string>({ keys, labels, counts, enabled, color, onToggle }: FilterPillsProps<K>) {
    return (
        <div className="flex gap-2">
            {keys.map((key) => (
                <Button
                    key={key}
                    size="sm"
                    color={typeof color === "object" ? color[key] : color}
                    colorType={enabled.has(key) ? "filled" : "outline"}
                    className="rounded-full"
                    aria-pressed={enabled.has(key)}
                    onClick={() => {
                        onToggle(key);
                    }}
                >
                    {labels[key]} ({counts[key]})
                </Button>
            ))}
        </div>
    );
}

function formatConfig(config: unknown): string {
    return JSON.stringify(config, null, 4);
}

interface BadgeProps {
    children: string;
    textColor: TextProps["color"];
}

export function Badge({ children, textColor }: BadgeProps) {
    return (
        <Text
            tag="span"
            size="xs"
            color={textColor}
            className="rounded-sm border border-fg-700 px-1"
        >
            {children}
        </Text>
    );
}

interface ConfigBlockProps {
    title: string;
    config: unknown;
}

export function ConfigBlock({ title, config }: ConfigBlockProps) {
    const editorTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);

    return (
        <div className="flex min-w-0 flex-col gap-1">
            <Text
                size="sm"
                weight="bold"
            >
                {title}
            </Text>
            <Codeblock
                lang={Language.JSON}
                theme={editorTheme}
            >
                {formatConfig(config)}
            </Codeblock>
        </div>
    );
}
