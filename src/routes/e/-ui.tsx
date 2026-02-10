import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { Text } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { sendMessage } from "@/utils/e/socket";
import { Language } from "@/utils/textmate";
import { Format } from "@sadan4/devtools-pretty-printer";
import { useQuery } from "@tanstack/react-query";
import { formatModule } from "@vencord-companion/webpack-ast-parser/util";

import { Route } from "./view";

import { ArrowBigRight } from "lucide-react";
import { useRef, useState } from "react";


function useModuleCode(moduleId: string | null, bundleHash: string) {
    const { status, data } = useQuery({
        queryKey: [
            "getBundleFile",
            {
                moduleId,
                bundleHash,
            },
        ],
        async queryFn() {
            if (moduleId === null) {
                return "// Select a Module";
            }

            const { fileText } = await sendMessage<"getBundleFileResponse">({
                type: "getBundleFile",
                bundleHash,
                moduleNumber: moduleId,
            });

            return Format(formatModule(fileText, moduleId, false));
        },
    });

    if (status === "pending") {
        return formatModule("// Loading...", moduleId ?? -1, false);
    }
    if (status === "error") {
        return formatModule("// Error loading file", moduleId ?? -1, false);
    }
    return data;
}

interface ModuleSelectorProps {
    selectedModule: string | null;
    modules: string[];
    onSelectModule: (module: string) => void;
}

function ModuleSelector({ modules, onSelectModule, selectedModule }: ModuleSelectorProps) {
    return (
        <BufferedScroller
            items={modules}
            batchSize={75}
            bufferSize={2}
            renderItem={({ item }) => {
                return (
                    <Clickable
                        key={item}
                        tag="li"
                        onClick={() => onSelectModule(item)}
                    >
                        <Text
                            tag="span"
                            color={item === selectedModule ? "primary" : "white"}
                        >
                            {item}
                        </Text>
                    </Clickable>
                );
            }}
        />
    );
}

interface ModuleViewerProps {
    moduleId: string | null;
    bundleHash: string;
}

function ModuleViewer({ moduleId, bundleHash }: ModuleViewerProps) {
    const code = useModuleCode(moduleId, bundleHash);

    return (
        <MonacoCodeEditor
            language={Language.JAVASCRIPT}
            code={code}
        />
    );
}

export function Explorer() {
    const [selectedModule, setSelectedModule] = useState<string | null>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const { buildHash } = Route.useSearch();

    const { status, data } = useQuery({
        queryKey: ["getBundleMetadata", { buildHash }],
        async queryFn() {
            try {
                return await sendMessage<"getBundleMetadataResponse">({
                    type: "getBundleMetadata",
                    bundleHash: buildHash,
                });
            } catch (e) {
                console.error(e);
                throw e;
            }
        },
    });

    const moduleIds = status === "success" ? Object.values(data.metadata.modules).flat() : [];

    // <div className="flex w-fit flex-col gap-3">
    //     main body
    //     <Button onClick={() => {
    //         leftSidebarHidden
    //             ? leftSidebarStateStore.getState().show()
    //             : leftSidebarStateStore.getState().hide();
    //     }}
    //     >
    //         {leftSidebarHidden ? "Show" : "Hide"} Left Sidebar
    //     </Button>
    //     <Button onClick={() => {
    //         rightSidebarHidden
    //             ? rightSidebarStateStore.getState().show()
    //             : rightSidebarStateStore.getState().hide();
    //     }}
    //     >
    //         {rightSidebarHidden ? "Show" : "Hide"} Right Sidebar
    //     </Button>
    // </div>
    return (
        <>
            <Boilerplate solidBg />
            <div className="flex h-full flex-col">
                <div className="h-1/20 bg-primary-400/50">header</div>
                <div
                    className="relative flex max-h-19/20 grow"
                >
                    <div>
                        <div className="flex items-center justify-between">
                            <Input
                                ref={inputRef}
                                placeholder="Enter a Module ID"
                                className="m-2"
                            />
                            <IconButton
                                onClick={() => {
                                    const el: HTMLInputElement | null = inputRef.current;

                                    if (!el) {
                                        return false;
                                    }

                                    const moduleId = el.value;

                                    if (selectedModule === moduleId) {
                                        return null;
                                    }
                                    if (!moduleIds.includes(moduleId)) {
                                        return false;
                                    }
                                    setSelectedModule(moduleId);
                                    return true;
                                }}
                                className="mr-2 ml-4 size-10"
                                label="Jump To Module"
                                tooltipPosition={TooltipPosition.RIGHT}
                                colorType="outline"
                            >
                                <ArrowBigRight />
                            </IconButton>
                        </div>
                        <ModuleSelector
                            selectedModule={selectedModule}
                            modules={moduleIds}
                            onSelectModule={setSelectedModule}
                        />
                    </div>
                    <div className="grow">
                        <ModuleViewer
                            moduleId={selectedModule}
                            bundleHash={buildHash}
                        />
                    </div>
                </div>
            </div>
        </>
    );
}
