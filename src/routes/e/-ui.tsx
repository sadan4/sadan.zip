import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { BufferedScroller, type BufferedScrollerHandle } from "@/components/layout/BufferedScroller";
import { Text } from "@/components/Text";
import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { dedupe } from "@/utils/array";
import { sendMessage } from "@/utils/e/socket";
import { Language } from "@/utils/textmate";
import { useQuery } from "@tanstack/react-query";

import { ModuleViewerStore, placeholderModel, placeholderURI, useModuleViewerStore, ViewMode } from "./-data";
import { Route } from "./view.{-$buildHash}.{-$moduleId}";

import { ArrowBigRight, ChevronFirstIcon, ChevronLastIcon, FileCodeIcon, NetworkIcon } from "lucide-react";
import { Activity, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { visibleIf } from "@/utils/react";


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
    const scrollerHandle = useRef<BufferedScrollerHandle<string>>(null);
    const selectedModule = useModuleViewerStore(({ selectedModule }) => selectedModule);

    useEffect(() => {
        if (modules.length && selectedModule) {
            scrollerHandle.current?.scrollItemIntoView((e) => e === selectedModule);
        }
    }, [modules.length, selectedModule]);

    return (
        <BufferedScroller
            handle={scrollerHandle}
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
        if (!moduleId) {
            setUri(pendingUri("// Select a Module"));

            return;
        }

        // When a new module is selected, show a loading placeholder immediately.
        setUri(pendingUri("// Loading..."));

        let cancelled = false;

        !async function () {
            const { uri } = await ModuleViewerStore.getState().getModuleModel(moduleId);

            // don't race if we are called while this is pending
            if (cancelled) {
                return;
            }

            setUri(uri);
        }();

        return () => {
            cancelled = true;
        };
    }, [moduleId]);

    return (
        <MonacoCodeEditor
            language={Language.JAVASCRIPT}
            uri={uri}
        />
    );
}

function ModuleGraph() {
    return (
        <Text
            size="3xl"
            weight="bold"
            center
        >
            TODO: Module Graph View
        </Text>
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
    const activePanel = useModuleViewerStore(({ activePanel }) => activePanel);
    const moduleSidebarOpen = useModuleViewerStore(({ moduleSidebarOpen }) => moduleSidebarOpen);

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

    const origModules = status === "success" && data.metadata.modules;

    const moduleIds = useMemo(() => (origModules
        // webpack will duplicate the same module across multiple chunks, so we need to dedupe them
        ? dedupe(Object.values(origModules)
            .flat()
            .toSorted((a, b) => +a - +b))
        : []), [origModules]);

    return (
        <>
            <Boilerplate solidBg />
            <div className="flex h-full flex-col">
                <div className="flex items-center justify-between">
                    <div className="pl-2">
                        <IconButton
                            label={`${moduleSidebarOpen ? "Hide" : "Show"} Module Sidebar`}
                            colorType="outline"
                            className="border-2 border-fg-700"
                            tooltipPosition={TooltipPosition.RIGHT}
                            color="neutral"
                            onClick={() => {
                                ModuleViewerStore.getState().updateModuleSidebarOpen(!moduleSidebarOpen);
                                return null;
                            }}
                        >
                            {moduleSidebarOpen ? <ChevronFirstIcon /> : <ChevronLastIcon />}
                        </IconButton>
                    </div>
                    <div className="">
                        <ToggleButtonGroup
                            tooltipPosition={TooltipPosition.BOTTOM}
                            className="m-2 rounded-lg border-2 border-fg-700 p-2"
                            selectedItem={activePanel}
                            onSelectItem={(panel) => {
                                ModuleViewerStore.getState().updateActivePanel(panel);
                            }}
                            items={[
                                {
                                    id: ViewMode.CODE,
                                    label: "Module Code",
                                    renderIcon() {
                                        return <FileCodeIcon />;
                                    },
                                },
                                {
                                    id: ViewMode.MODULE_GRAPH,
                                    label: "Module Graph",
                                    renderIcon() {
                                        return <NetworkIcon />;
                                    },
                                },
                            ]}
                        />
                    </div>
                    <div />
                </div>
                <div
                    className="relative flex min-h-0 grow"
                >
                    <Activity mode={visibleIf(moduleSidebarOpen)}>
                        <div className="flex shrink-0 flex-col">
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

                                        const { selectedModule } = ModuleViewerStore.getState();
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
                            <div className="min-h-0 grow">
                                <ModuleSelector
                                    modules={moduleIds}
                                    onSelectModule={setSelectedModule}
                                />
                            </div>
                        </div>
                    </Activity>
                    <div className="shrink grow">
                        <Activity mode={visibleIf(activePanel === ViewMode.CODE)}>
                            <div className="size-full bg-bg-100">
                                <ModuleViewer />
                            </div>
                        </Activity>
                        <Activity mode={visibleIf(activePanel === ViewMode.MODULE_GRAPH)}>
                            <ModuleGraph />
                        </Activity>
                    </div>
                </div>
            </div>
        </>
    );
}
