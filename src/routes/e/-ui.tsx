import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { Text } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { sendMessage } from "@/utils/e/socket";
import { makeLazy } from "@/utils/lazy";
import { monaco } from "@/utils/monaco";
import { Language } from "@/utils/textmate";
import { useQuery } from "@tanstack/react-query";

import { useModuleViewerStore } from "./-data";
import { Route } from "./view.{-$buildHash}.{-$moduleId}";

import { ArrowBigRight } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";


interface ModuleListItemProps {
    moduleId: string;
    onSelectModule(moduleId: string): void;
}

function ModuleListItem({ moduleId, onSelectModule }: ModuleListItemProps) {
    const isSelectedModule = useModuleViewerStore(({ selectedModule }) => selectedModule === moduleId);

    return (
        <Clickable
            tag="li"
            onClick={() => onSelectModule(moduleId)}
        >
            <Text
                tag="span"
                color={isSelectedModule ? "primary" : "white"}
            >
                {moduleId}
            </Text>
        </Clickable>
    );
}

interface ModuleSelectorProps {
    modules: string[];
    onSelectModule: (module: string) => void;
}


function ModuleSelector({ modules, onSelectModule }: ModuleSelectorProps) {
    return (
        <BufferedScroller
            items={modules}
            batchSize={75}
            bufferSize={2}
            renderItem={({ item }) => {
                return (
                    <ModuleListItem
                        key={item}
                        moduleId={item}
                        onSelectModule={onSelectModule}
                    />
                );
            }}
        />
    );
}

const placeholderURI = makeLazy(() => monaco.Uri.parse("file:///placeholder.js"));
const placeholderModel = makeLazy(() => monaco.editor.createModel("", "javascript", placeholderURI()));

function pendingUri(str: string) {
    const model = placeholderModel();

    model.setValue(str);

    return placeholderURI();
}

function ModuleViewer() {
    const moduleId = useModuleViewerStore(({ selectedModule }) => selectedModule);

    const [uri, setUri] = useState(() => {
        if (moduleId) {
            return pendingUri("// Loading...");
        }
        return pendingUri("// Select a Module");
    });

    useEffect(() => {
        !async function () {
            if (!moduleId) {
                return;
            }

            const { uri } = await useModuleViewerStore.getState().getModuleModel(moduleId);

            setUri(uri);
        }();
    }, [moduleId]);

    return (
        <MonacoCodeEditor
            language={Language.JAVASCRIPT}
            uri={uri}
        />
    );
}

export function Explorer() {
    const navigate = Route.useNavigate();

    const setSelectedModule = useCallback((moduleId: string) => {
        navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                moduleId,
            },
        });
    }, [navigate]);

    const { buildHash } = Route.useParams();
    const inputRef = useRef<HTMLInputElement>(null);

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

                                    const { selectedModule } = useModuleViewerStore.getState();
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
                            modules={moduleIds}
                            onSelectModule={setSelectedModule}
                        />
                    </div>
                    <div className="grow">
                        <ModuleViewer />
                    </div>
                </div>
            </div>
        </>
    );
}
