import { Boilerplate } from "@/components/Boilerplate";
import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { Input } from "@/components/Input";
import { Box } from "@/components/layout/Box";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { HorizontalLine } from "@/components/Lines";
import { TextLink } from "@/components/Links";
import { Modal, ModalContext } from "@/components/modal";
import { Select, type SelectOption } from "@/components/Select";
import { LabeledSwitch } from "@/components/Switch/index";
import { Text } from "@/components/Text";
import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { type GeneratedGraph, useModuleGraph } from "@/hooks/moduleGraph";
import { dedupe } from "@/utils/array";
import { sleep } from "@/utils/async";
import cn from "@/utils/cn";
import { GITHUB_REPO_URL, NBSP } from "@/utils/constants";
import { sendMessage } from "@/utils/e/socket";
import { debug_assert } from "@/utils/error";
import { isNumber } from "@/utils/functional";
import type { Monaco } from "@/utils/monaco";
import { visibleIf } from "@/utils/react";
import { Language } from "@/utils/textmate";
import { TextmateTheme, themeDisplayNames } from "@/utils/textmate/theme";
import { useQuery } from "@tanstack/react-query";
import { createLink } from "@tanstack/react-router";
import { TAssert } from "@vencord-companion/webpack-ast-parser/util";
import type { WebpackAstParser } from "@vencord-companion/webpack-ast-parser/WebpackAstParser";
import { Background, Controls, MiniMap, ReactFlow, ReactFlowProvider } from "@xyflow/react";

import { downloadBundle, ModuleViewerSettingsStore, ModuleViewerStore, placeholderModel, placeholderURI, useModuleViewerSettingsStore, useModuleViewerStore, ViewMode } from "./-data";
import { Route } from "./view.{-$buildHash}.{-$moduleId}";
import { TModuleId } from "../../../server/types";

import "@xyflow/react/dist/style.css";
import { AppWindowIcon, ArrowBigRight, BadgeInfoIcon, ChevronFirstIcon, ChevronLastIcon, DownloadIcon, FileCodeIcon, GithubIcon, NetworkIcon, SettingsIcon, TriangleAlertIcon, Undo2Icon } from "lucide-react";
import { Activity, type PropsWithChildren, useCallback, useEffect, useMemo, useRef, useState } from "react";


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
    const { sl, sc, el, ec } = Route.useSearch();
    const [codeEditor, setCodeEditor] = useState<MonacoCodeEditor.Handle | null>(null);
    const editorTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);

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

    useEffect(() => {
        if (!codeEditor || !moduleId) {
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
    }, [moduleId, uri, sl, sc, el, ec, codeEditor]);

    return (
        <MonacoCodeEditor
            ref={setCodeEditor}
            language={Language.JAVASCRIPT}
            theme={editorTheme}
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

const IconButtonInternalLink = createLink(IconButton);

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

interface ExperimentalSettingProps extends PropsWithChildren {
}

function ExperimentalSetting({ children }: ExperimentalSettingProps) {
    return (
        <>
            <div className="flex w-full flex-col rounded-md border-2 border-warning-300/50 p-2">
                <Text
                    color="warning"
                    className="mb-2 flex items-center gap-2"
                >
                    <TriangleAlertIcon className="inline" />This Setting is Experimental. Expect and report any bugs!
                </Text>
                {children}
            </div>
        </>
    );
}

function SettingsModal() {
    const openModulesInNewTab = useModuleViewerSettingsStore(({ openModulesInNewTab }) => openModulesInNewTab);
    const selectedTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);

    return (
        <Box className="w-[95vw] sm:w-[75vw] md:w-[50vw] lg:w-[35vw]">
            <Text
                center
                size="2xl"
                color="primary"
            >
                Settings
            </Text>
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
            </div>
        </Box>
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
    const settingsModal = useRef<ModalContext>(null);

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

    const origModules = status === "success" && data.moduleInfo;

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
                    <div className="flex gap-2">
                        <IconButton
                            label="Download Bundle"
                            colorType="outline"
                            tooltipClassName="z-5"
                            tooltipPosition={TooltipPosition.BOTTOM}
                            loadingAnimation
                            onClick={() => {
                                return downloadBundle(buildHash);
                            }}
                        >
                            <DownloadIcon />
                        </IconButton>
                        <>
                            <IconButton
                                label={`Open${NBSP}Settings`}
                                colorType="outline"
                                onClick={() => {
                                    if (settingsModal.current) {
                                        settingsModal.current.open();
                                        return true;
                                    }
                                    return false;
                                }}
                                tooltipClassName="z-5"
                                tooltipPosition={TooltipPosition.BOTTOM}
                            >
                                <SettingsIcon />
                            </IconButton>
                            <Modal ref={settingsModal}>
                                <SettingsModal />
                            </Modal>
                        </>
                        <IconButtonInternalLink
                            tooltipPosition={TooltipPosition.BOTTOM}
                            label="Return to Bundle Selector"
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
                            // FIXME: this should be on the bottom, but it clips off the screen
                            tooltipPosition={TooltipPosition.LEFT}
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
