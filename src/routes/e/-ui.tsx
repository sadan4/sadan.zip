import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { Box } from "@/components/layout/Box";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { HorizontalLine } from "@/components/Lines";
import { Modal, ModalContext } from "@/components/modal";
import { Select, type SelectOption } from "@/components/Select";
import { Slider } from "@/components/Slider";
import { LabeledSwitch } from "@/components/Switch/index";
import { Text } from "@/components/Text";
import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { type GeneratedGraph, useModuleGraph2 } from "@/hooks/moduleGraph2";
import { makeRange } from "@/utils/array";
import cn from "@/utils/cn";
import { BUNDLE_TARBALL_FILENAME, BUNDLE_TARBALL_URL, GITHUB_REPO_URL, NBSP } from "@/utils/constants";
import { debug_assert } from "@/utils/error";
import { isNumber } from "@/utils/functional";
import type { Monaco } from "@/utils/monaco";
import { visibleIf } from "@/utils/react";
import { Language } from "@/utils/textmate";
import { TextmateTheme, themeDisplayNames } from "@/utils/textmate/theme";
import type { TModuleId } from "@/utils/types";
import { useQuery } from "@tanstack/react-query";
import { createLink } from "@tanstack/react-router";
import { Background, Controls, MiniMap, ReactFlow, ReactFlowProvider } from "@xyflow/react";

import {
    ModuleViewerSettingsStore,
    ModuleViewerStore,
    placeholderModel,
    placeholderURI,
    useModuleViewerSettingsStore,
    useModuleViewerStore,
    ViewMode,
} from "./-data";
import { Route } from "./view.{-$buildHash}.{-$moduleId}";

import "@xyflow/react/dist/style.css";
import {
    ArrowBigRight,
    BadgeInfoIcon,
    ChevronFirstIcon,
    ChevronLastIcon,
    DownloadIcon,
    FileCodeIcon,
    GithubIcon,
    NetworkIcon,
    SettingsIcon,
    TriangleAlertIcon,
    Undo2Icon,
    XIcon,
} from "lucide-react";
import { Activity, type PropsWithChildren, use, useCallback, useEffect, useRef, useState } from "react";


interface ModuleListItemProps {
    moduleId: number;
    onSelectModule(moduleId: number): void;
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
    modules: Uint32Array;
    onSelectModule: (module: number) => void;
}

function ModuleSelector({ modules, onSelectModule }: ModuleSelectorProps) {
    const scrollerRef = useRef<BufferedScroller.Handle<number>>(null);
    const selectedModule = useModuleViewerStore(({ selectedModule }) => selectedModule);

    useEffect(() => {
        if (modules.length && selectedModule) {
            scrollerRef.current?.scrollItemIntoView((e) => e === selectedModule);
        }
    }, [modules.length, selectedModule]);

    return (
        <BufferedScroller
            handle={scrollerRef}
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
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);

    const { data, isPlaceholderData } = useQuery({
        queryKey: ["ModuleViewerURI", buildHash, moduleId],
        async queryFn() {
            if (moduleId == null) {
                return pendingUri("// Select a Module");
            }

            try {
                const model = await ModuleViewerStore.getState().getModuleModel(moduleId as TModuleId);

                return model.uri;
            } catch (e) {
                console.error("Error loading model for module", moduleId);
                console.error(e);
                throw e;
            }
        },
        placeholderData() {
            return pendingUri("// Select a Module");
        },
    });

    const { sl, sc, el, ec } = Route.useSearch();
    const [codeEditor, setCodeEditor] = useState<MonacoCodeEditor.Handle | null>(null);
    const editorTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);


    useEffect(() => {
        if (!codeEditor || isPlaceholderData) {
            return;
        }
        if (sl == null || sc == null) {
            debug_assert(el == null && ec == null, "when sl and sc are null, el and ec should be null");
            return;
        }
        if (el == null || ec == null || (sl === el && sc === ec)) {
            const pos = {
                lineNumber: sl,
                column: sc,
            } satisfies Monaco.IPosition;

            codeEditor.editor.setPosition(pos);
            codeEditor.editor.revealPositionInCenter(pos);
        } else {
            const range = {
                startLineNumber: sl,
                startColumn: sc,
                endLineNumber: el,
                endColumn: ec,
            } satisfies Monaco.IRange;

            codeEditor.editor.setSelection(range);
            codeEditor.editor.revealRangeInCenter(range);
        }
    }, [isPlaceholderData, moduleId, sl, sc, el, ec, codeEditor]);

    return (
        <MonacoCodeEditor
            ref={setCodeEditor}
            language={Language.JAVASCRIPT}
            theme={editorTheme}
            uri={data}
        />
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
                nodesDraggable
                onlyRenderVisibleElements
                nodesConnectable={false}
                minZoom={0}
                onNodeClick={(_e, node) => {
                    const moduleId = +node.id;
                    const { selectedModule } = ModuleViewerStore.getState();

                    if (moduleId === selectedModule) {
                        return;
                    }

                    navigate({
                        to: "/e/view/{-$buildHash}/{-$moduleId}",
                        params: {
                            moduleId,
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

const IconButtonInternalLink = createLink(IconButton);

function ModuleGraphWrapper() {
    const moduleId = useModuleViewerStore(({ selectedModule }) => selectedModule);
    const graph = useModuleGraph2();

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

interface SettingWithWarningProps extends PropsWithChildren {
    warning: string;
}

function SettingWithWarning({ warning, children }: SettingWithWarningProps) {
    return (
        <div className="flex w-full flex-col rounded-md border-2 border-warning-300/50 p-2">
            <Text
                color="warning"
                className="mb-2 flex items-center gap-2"
            >
                <TriangleAlertIcon className="inline" />{warning}
            </Text>
            {children}
        </div>
    );
}

interface ExperimentalSettingProps extends PropsWithChildren {
}

function ExperimentalSetting({ children }: ExperimentalSettingProps) {
    return (
        <SettingWithWarning warning="This Setting is Experimental. Expect and report any bugs!">
            {children}
        </SettingWithWarning>
    );
}

function SettingsModal() {
    const openModulesInNewTab = useModuleViewerSettingsStore(({ openModulesInNewTab }) => openModulesInNewTab);
    const selectedTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);
    const graphDepth = useModuleViewerSettingsStore(({ graphDepth }) => graphDepth);
    const ctx = use(ModalContext)!;

    return (
        <div className="inset-fill flex items-center justify-center">
            <Box className="h-fit w-[95vw] sm:w-[75vw] md:w-[50vw] lg:w-[35vw]">
                <div className="flex items-center justify-between">
                    <div />
                    <Text
                        size="2xl"
                        color="primary"
                    >
                        Settings
                    </Text>
                    <div>
                        <IconButton
                            label="Close"
                            colorType="text"
                            color="error"
                            onClick={() => {
                                ctx.close();
                                // TODO: Weird behavior with state not resetting when we return true;
                                return null;
                            }}
                        >
                            <XIcon />
                        </IconButton>
                    </div>
                </div>
                <HorizontalLine />
                <div className="flex flex-col gap-6">
                    <ExperimentalSetting>
                        <LabeledSwitch
                            value={openModulesInNewTab}
                            onChange={(value) => {
                                useModuleViewerSettingsStore.setState({ openModulesInNewTab: value });
                            }}
                        >
                            Open modules in new tab
                        </LabeledSwitch>
                        <Text
                            size="sm"
                            color="white-600"
                        >
                            When enabled, modules opened via jump to definition(ctrl-click)
                            inside of the editor will be opened in a new tab instead of the current tab
                        </Text>
                    </ExperimentalSetting>
                    <div className="flex flex-col gap-2">
                        <div className="flex items-center justify-between">
                            <Text size="md">
                                Editor Theme
                            </Text>
                            <Select
                                selectedValue={selectedTheme}
                                onChange={(editorTheme) => {
                                    ModuleViewerSettingsStore.setState({ editorTheme });
                                }}
                                items={Object
                                    .values(TextmateTheme)
                                    .filter(isNumber)
                                    .map((theme: TextmateTheme) => {
                                        const name = themeDisplayNames[theme];

                                        return {
                                            label: name,
                                            typedValue: name,
                                            value: theme,
                                            key: theme,
                                        } satisfies SelectOption<TextmateTheme>;
                                    })}
                                className="w-48"
                                scrollAreaClassName={cn("max-h-[25vh]")}
                            />
                        </div>
                        <Text
                            color="white-600"
                            size="sm"
                        >
                            The color theme for the code editor.
                            This does not change the theme of the rest of the app. Sorry :(
                        </Text>
                        <Text
                            color="info"
                            size="sm"
                            // FIXME: don't require a reload, this is a bug
                        >
                            <BadgeInfoIcon className="mr-1 inline size-4" />You must reload the page for the theme to take effect.
                        </Text>
                    </div>
                    <SettingWithWarning warning="High values (>1) can result in massive lag/hanging.">
                        <Text size="md">
                            Graph Depth
                        </Text>
                        <Text
                            size="sm"
                            color="white-600"
                            className="py-2"
                        >
                            The depth of the module graph.
                        </Text>
                        <Slider
                            min={0}
                            max={10}
                            markers={makeRange(0, 10)}
                            value={graphDepth}
                            onChange={(value) => {
                                // don't update the graph depth if we're currently viewing the graph
                                // as it would cause heavy CPU usage while the user is changing the setting
                                if (ModuleViewerStore.getState().activePanel === ViewMode.MODULE_GRAPH) {
                                    ModuleViewerStore.setState({ activePanel: ViewMode.CODE });
                                }
                                ModuleViewerSettingsStore.setState({ graphDepth: value });
                            }}
                            stickToMarkers
                        />
                    </SettingWithWarning>
                </div>
            </Box>
        </div>
    );
}

function ExplorerHeader() {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const settingsModalRef = useRef<ModalContext>(null);
    const moduleSidebarOpen = useModuleViewerStore(({ moduleSidebarOpen }) => moduleSidebarOpen);
    const activePanel = useModuleViewerStore(({ activePanel }) => activePanel);

    return (
        <>
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
            <div className="flex gap-2">
                <IconButton
                    tag="a"
                    label={`Download${NBSP}Bundle`}
                    colorType="outline"
                    tooltipClassName="z-6"
                    tooltipPosition={TooltipPosition.BOTTOM}
                    loadingAnimation
                    onClick={undefined}
                    href={BUNDLE_TARBALL_URL(buildHash)}
                    download={BUNDLE_TARBALL_FILENAME(buildHash)}
                >
                    <DownloadIcon />
                </IconButton>
                <IconButton
                    label={`Open${NBSP}Settings`}
                    colorType="outline"
                    onClick={() => {
                        if (settingsModalRef.current) {
                            settingsModalRef.current.open();
                            return true;
                        }
                        return false;
                    }}
                    tooltipClassName="z-6"
                    tooltipPosition={TooltipPosition.BOTTOM}
                >
                    <SettingsIcon />
                </IconButton>
                <Modal ref={settingsModalRef}>
                    <SettingsModal />
                </Modal>
                <IconButtonInternalLink
                    tooltipPosition={TooltipPosition.BOTTOM}
                    label={`Return${NBSP}to${NBSP}Bundle${NBSP}Selector`}
                    onClick={undefined}
                    colorType="outline"
                    // Monaco has a sidebar with z-5, which blocks our tooltip sometimes
                    tooltipClassName="z-6"
                    tag="a"
                    to="/e"
                >
                    <Undo2Icon />
                </IconButtonInternalLink>
                <IconButton
                    tooltipPosition={TooltipPosition.BOTTOM}
                    label={`Source${NBSP}Code.${NBSP}Star${NBSP}Me!`}
                    onClick={undefined}
                    color="secondary"
                    colorType="outline"
                    href={GITHUB_REPO_URL}
                    // Monaco has a sidebar with z-5, which blocks our tooltip sometimes
                    tooltipClassName="z-6"
                    target="_blank"
                    tag="a"
                >
                    <GithubIcon />
                </IconButton>
            </div>
        </>
    );
}

function ExplorerSidebar() {
    const navigate = Route.useNavigate();
    const inputRef = useRef<HTMLInputElement>(null);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);

    const { data: moduleIds, status } = useQuery({
        queryKey: ["allModuleIds", buildHash],
        queryFn() {
            return buildService.getAllModuleIds();
        },
    });

    const setSelectedModule = useCallback((moduleId: number) => {
        navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                moduleId,
            },
        });
    }, [navigate]);

    return (
        <div className="flex shrink-0 flex-col">
            <div className="flex items-center justify-between">
                <Input
                    ref={inputRef}
                    placeholder="Enter a Module ID"
                    className="m-2"
                />
                <IconButton
                    onClick={async () => {
                        const v = inputRef.current?.value;

                        if (!v) {
                            return false;
                        }

                        const { selectedModule, hasId } = ModuleViewerStore.getState();
                        const inputModuleId = +v;

                        if (selectedModule === inputModuleId) {
                            return null;
                        }

                        if (await hasId(inputModuleId)) {
                            setSelectedModule(inputModuleId);

                            return true;
                        }

                        return false;
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
                {status === "success" && (
                    <ModuleSelector
                        modules={moduleIds}
                        onSelectModule={setSelectedModule}
                    />
                )}
                {status === "pending" && (
                    <Text
                        size="lg"
                        color="accent"
                        center
                    >
                        Loading Modules...
                    </Text>
                )}
                {status === "error" && (
                    <Text
                        size="lg"
                        color="error"
                        center
                    >
                        An error occurred while loading the module list.
                    </Text>
                )}
            </div>
        </div>
    );
}

export function Explorer() {
    const { moduleId } = Route.useParams();
    const activePanel = useModuleViewerStore(({ activePanel }) => activePanel);
    const moduleSidebarOpen = useModuleViewerStore(({ moduleSidebarOpen }) => moduleSidebarOpen);

    useEffect(() => {
        ModuleViewerStore.setState({ selectedModule: moduleId });
    }, [moduleId]);

    return (
        <>
            <Boilerplate solidBg />
            <div className="flex h-full flex-col">
                <div className="flex items-center justify-between">
                    <ExplorerHeader />
                </div>
                <div
                    className="relative flex min-h-0 grow"
                >
                    <Activity mode={visibleIf(moduleSidebarOpen)}>
                        <ExplorerSidebar />
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
