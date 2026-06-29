import { IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { Input } from "@/components/Input";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { Text } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { Route } from "@/routes/e/view.{-$buildHash}.{-$moduleId}";
import { useQuery } from "@tanstack/react-query";

import { ModuleViewerStore, useModuleViewerStore } from "../../../-data";

import { ArrowBigRight } from "lucide-react";
import { useCallback, useEffect, useRef } from "react";

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


export function ModuleList() {
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
