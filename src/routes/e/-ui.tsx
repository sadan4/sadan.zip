import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { type GeneratedGraph, useModuleGraph } from "@/hooks/moduleGraph";
import { dedupe } from "@/utils/array";
import { sendMessage } from "@/utils/e/socket";
import { visibleIf } from "@/utils/react";
import { Language } from "@/utils/textmate";
import { useQuery } from "@tanstack/react-query";
import { TAssert } from "@vencord-companion/webpack-ast-parser/util";
import type { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";
import { Background, Controls, MiniMap, ReactFlow, ReactFlowProvider } from "@xyflow/react";

import { ModuleViewerStore, placeholderModel, placeholderURI, useModuleViewerStore, ViewMode } from "./-data";
import { Route } from "./view.{-$buildHash}.{-$moduleId}";
import { TModuleId } from "../../../server/types";

import "@xyflow/react/dist/style.css";
import { ArrowBigRight, ChevronFirstIcon, ChevronLastIcon, FileCodeIcon, NetworkIcon } from "lucide-react";
import { Activity, useCallback, useEffect, useMemo, useRef, useState } from "react";


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
    modules: TModuleId[];
    onSelectModule: (module: TModuleId) => void;
}


function ModuleSelector({ modules, onSelectModule }: ModuleSelectorProps) {
    const scrollerHandle = useRef<BufferedScroller.Handle<TModuleId>>(null);
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

interface ModuleGraphProps {
    parser: WebpackAstParser;
}

// @ts-expect-error
// eslint-disable-next-line unused-imports/no-unused-vars
function ModuleGraph({ parser }: ModuleGraphProps) {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const outgoingModules = useMemo(() => parser.getModulesThatThisModuleRequires(), [parser]);

    return (
        <div className="flex h-full justify-evenly">
            <ReactFlow />
            <div className="flex flex-col items-center gap-2">
                <Text
                    size="md"
                    color="secondary"
                >
                    Modules that this module requires
                </Text>
                <Text size="lg">
                    Sync:
                </Text>
                {!outgoingModules && (
                    <Text
                        color="error"
                    >
                        This module does not require any other modules
                    </Text>
                )}
                {outgoingModules?.sync.map((moduleId) => {
                    TAssert<TModuleId>(moduleId);
                    return (
                        <TextLink
                            to="/e/view/{-$buildHash}/{-$moduleId}"
                            params={{
                                buildHash,
                                moduleId,
                            }}
                        >{moduleId}
                        </TextLink>
                    );
                })}
            </div>
            <div className="flex flex-col items-center gap-2">
                <Text
                    size="md"
                    color="secondary"
                >
                    Module that require this module
                </Text>
                <Text
                    size="2xl"
                    color="error"
                >
                    TODO
                </Text>
            </div>
        </div>
    );
}

interface ModuleGraph2Props {
    graph: GeneratedGraph;
}

function ModuleGraph2({ graph: { nodes, edges } }: ModuleGraph2Props) {
    const navigate = Route.useNavigate();

    return (
        <div className="size-full bg-black">
            <ReactFlow
                nodes={nodes}
                edges={edges}
                colorMode="dark"
                nodesDraggable={true}
                onlyRenderVisibleElements={true}
                nodesConnectable={false}
                minZoom={0}
                onNodeClick={(_e, node) => {
                    const moduleId = node.id;
                    const { selectedModule } = ModuleViewerStore.getState();

                    if (moduleId === selectedModule) {
                        return;
                    }

                    navigate({
                        to: "/e/view/{-$buildHash}/{-$moduleId}",
                        params: {
                            moduleId: moduleId as TModuleId,
                        },
                    });
                }}
            >
                <Controls />
                <Background />
                <MiniMap
                    pannable
                    zoomable
                />
            </ReactFlow>
        </div>
    );
}

function ModuleGraphWrapper() {
    const moduleId = useModuleViewerStore(({ selectedModule }) => selectedModule);
    const graph = useModuleGraph();

    if (!moduleId || !graph) {
        return (
            <Text
                size="3xl"
                weight="bold"
                center
            >
                {moduleId ? "Loading Module Graph..." : "Select a Module"}
            </Text>
        );
    }

    return (
        <ReactFlowProvider>
            <ModuleGraph2 graph={graph} />
        </ReactFlowProvider>
    );
}

export function Explorer() {
    const navigate = Route.useNavigate();

    const setSelectedModule = useCallback((moduleId: TModuleId) => {
        navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                moduleId,
            },
        });
    }, [navigate]);

    const { buildHash, moduleId } = Route.useParams();
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

    useEffect(() => {
        ModuleViewerStore.setState({ selectedModule: moduleId });
    }, [moduleId]);

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
                                        const inputModuleId = el.value;

                                        if (selectedModule === inputModuleId) {
                                            return null;
                                        }
                                        if (!moduleIds.includes(inputModuleId as TModuleId)) {
                                            return false;
                                        }

                                        setSelectedModule(inputModuleId as TModuleId);

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
                            <ModuleGraphWrapper />
                        </Activity>
                    </div>
                </div>
            </div>
        </>
    );
}
