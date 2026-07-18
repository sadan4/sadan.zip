import { TreeAccordion } from "@/components/ASTViewer/TreeAccordion";
import { Clickable } from "@/components/Clickable";
import { Text } from "@/components/Text";
import type { TModuleId } from "@/utils/types";
import type { ExportTreeNode } from "@sadan4/libsadancore";
import { useQuery } from "@tanstack/react-query";

import { useModuleViewerStore } from "../../-data";
import type { ModuleDeps } from "../../-data/worker/sharedWorker";
import { Route } from "../../view.{-$buildHash}.{-$moduleId}";

import { type PropsWithChildren, type ReactNode, useState } from "react";

export function ModuleExports() {
    const selectedModule = useModuleViewerStore(({ selectedModule }) => selectedModule);

    if (selectedModule === null) {
        return (
            <Text className="p-2">
                Select a module to view its exports
            </Text>
        );
    }

    return (
        <div className="flex min-h-0 grow flex-col overflow-auto p-2">
            <ExportsSection moduleId={selectedModule as TModuleId} />
            <ModuleDependenciesSection moduleId={selectedModule as TModuleId} />
            <ModuleDependentsSection moduleId={selectedModule as TModuleId} />
        </div>
    );
}

function SectionHeading({ children }: { children: ReactNode; }) {
    return (
        <Text
            size="sm"
            color="neutral-content"
        >
            {children}
        </Text>
    );
}

interface CollapsibleSectionProps extends PropsWithChildren {
    title: string;
}

function CollapsibleSection({ title, children }: CollapsibleSectionProps) {
    const [open, setOpen] = useState(true);

    return (
        <section className="mt-4 first:mt-0">
            <TreeAccordion
                open={open}
                onArrowClick={() => setOpen((v) => !v)}
                contents={children}
            >
                <SectionHeading>
                    {title}
                </SectionHeading>
            </TreeAccordion>
        </section>
    );
}

interface ExportsSectionProps {
    moduleId: TModuleId;
}

function ExportsSection({ moduleId }: ExportsSectionProps) {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);

    const { data, status, error } = useQuery({
        queryKey: ["ExplorerSidebarExportMap", buildHash, moduleId],
        queryFn() {
            return buildService.getModuleExportMap(moduleId);
        },
    });

    return (
        <CollapsibleSection title="Exports">
            {status === "pending" && (
                <Text className="p-2">
                    Loading exports...
                </Text>
            )}
            {status === "error" && (
                <Text
                    className="p-2"
                    color="error"
                >
                    Failed to load export map:
                    {" "}
                    {error instanceof Error ? error.message : String(error)}
                </Text>
            )}
            {status === "success" && (
                data.length
                    ? data.map((node) => (
                        <ExportTreeItem
                            key={node.name}
                            node={node}
                            moduleId={moduleId}
                        />
                    ))
                    : (
                        <Text className="p-2">
                            No exports found for this module.
                        </Text>
                    )
            )}
        </CollapsibleSection>
    );
}

interface ExportTreeItemProps {
    node: ExportTreeNode;
    moduleId: number;
}

function ExportTreeItem({ node, moduleId }: ExportTreeItemProps) {
    const [open, setOpen] = useState(false);
    const navigate = Route.useNavigate();
    const hasChildren = node.children.length > 0;

    function goToRange(range: ExportTreeNode["ranges"][number]) {
        navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                moduleId,
            },
            search: {
                sl: range.start.line,
                sc: range.start.column,
            },
        });
    }

    const label = (
        <div
            className="flex w-fit flex-col"
            title={node.hover}
        >
            <Text size="sm">
                {node.name}
                {node.hover && (
                    <Text
                        tag="span"
                        size="xs"
                        color="neutral-content"
                        className="ml-1"
                    >
                        {node.hover}
                    </Text>
                )}
            </Text>
            {node.ranges.map((range, i) => (
                <Clickable
                    // eslint-disable-next-line @eslint-react/no-array-index-key -- the index is stable for this data
                    key={i}
                    onClick={() => goToRange(range)}
                    className="ml-4"
                >
                    <Text
                        size="xs"
                        color="accent"
                    >
                        {range.start.line}
                        :
                        {range.start.column}
                    </Text>
                </Clickable>
            ))}
        </div>
    );

    if (!hasChildren) {
        return label;
    }

    return (
        <TreeAccordion
            open={open}
            onArrowClick={() => setOpen((v) => !v)}
            contents={(
                <div className="flex h-fit">
                    <div className="w-4 shrink-0" />
                    <div className="grow">
                        {node.children.map((child) => (
                            <ExportTreeItem
                                key={child.name}
                                node={child}
                                moduleId={moduleId}
                            />
                        ))}
                    </div>
                </div>
            )}
        >
            {label}
        </TreeAccordion>
    );
}

function ModuleDepsRow({ moduleId }: { moduleId: TModuleId; }) {
    const navigate = Route.useNavigate();

    return (
        <Clickable
            onClick={() => navigate({
                to: "/e/view/{-$buildHash}/{-$moduleId}",
                params: {
                    moduleId,
                },
            })}
            className="ml-4"
        >
            <Text
                size="xs"
                color="accent"
            >
                {moduleId}
            </Text>
        </Clickable>
    );
}

function ModuleDepsGroups({ deps }: { deps: ModuleDeps; }) {
    if (!deps.syncUses.length && !deps.lazyUses.length) {
        return null;
    }

    return (
        <>
            {deps.syncUses.length > 0 && (
                <div className="flex flex-col">
                    <Text
                        size="xs"
                        color="neutral-content"
                    >
                        Sync
                    </Text>
                    {deps.syncUses.map((moduleId) => (
                        <ModuleDepsRow
                            key={moduleId}
                            moduleId={moduleId}
                        />
                    ))}
                </div>
            )}
            {deps.lazyUses.length > 0 && (
                <div className="flex flex-col">
                    <Text
                        size="xs"
                        color="neutral-content"
                    >
                        Lazy
                    </Text>
                    {deps.lazyUses.map((moduleId) => (
                        <ModuleDepsRow
                            key={moduleId}
                            moduleId={moduleId}
                        />
                    ))}
                </div>
            )}
        </>
    );
}

interface ModuleDependenciesSectionProps {
    moduleId: TModuleId;
}

function ModuleDependenciesSection({ moduleId }: ModuleDependenciesSectionProps) {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);

    const { data, status, error } = useQuery({
        queryKey: ["ExplorerSidebarModuleDependencies", buildHash, moduleId],
        queryFn() {
            return buildService.getModuleDependencies(moduleId);
        },
    });

    return (
        <CollapsibleSection title="Dependencies">
            {status === "pending" && (
                <Text className="p-2">
                    Loading dependencies...
                </Text>
            )}
            {status === "error" && (
                <Text
                    className="p-2"
                    color="error"
                >
                    Failed to load dependencies:
                    {" "}
                    {error instanceof Error ? error.message : String(error)}
                </Text>
            )}
            {status === "success" && (
                data.syncUses.length || data.lazyUses.length
                    ? <ModuleDepsGroups deps={data} />
                    : (
                        <Text className="p-2">
                            This module has no dependencies.
                        </Text>
                    )
            )}
        </CollapsibleSection>
    );
}

interface ModuleDependentsSectionProps {
    moduleId: TModuleId;
}

function ModuleDependentsSection({ moduleId }: ModuleDependentsSectionProps) {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);

    const { data, status, error } = useQuery({
        queryKey: ["ExplorerSidebarModuleDependents", buildHash, moduleId],
        queryFn() {
            return buildService.getModuleDependents(moduleId);
        },
    });

    return (
        <CollapsibleSection title="Dependents">
            {status === "pending" && (
                <Text className="p-2">
                    Loading dependents...
                </Text>
            )}
            {status === "error" && (
                <Text
                    className="p-2"
                    color="error"
                >
                    Failed to load dependents:
                    {" "}
                    {error instanceof Error ? error.message : String(error)}
                </Text>
            )}
            {status === "success" && (
                data && (data.syncUses.length || data.lazyUses.length)
                    ? <ModuleDepsGroups deps={data} />
                    : (
                        <Text className="p-2">
                            No other modules depend on this module.
                        </Text>
                    )
            )}
        </CollapsibleSection>
    );
}
