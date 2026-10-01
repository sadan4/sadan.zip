import { deepEqual } from "@/utils/obj";
import type { ExperimentInfo, ExperimentVariant } from "@sadan4/libsadancore";

export interface ValueChange<T> {
    before: T;
    after: T;
}

export interface VariantChange {
    key: string;
    before: ExperimentVariant;
    after: ExperimentVariant;
}

export interface VariantChanges {
    added: ExperimentVariant[];
    removed: ExperimentVariant[];
    changed: VariantChange[];
}

export interface ExperimentChanges {
    type?: ValueChange<ExperimentInfo["type"]>;
    scope?: ValueChange<ExperimentInfo["scope"]>;
    label?: ValueChange<ExperimentInfo["label"]>;
    defaultConfig?: ValueChange<unknown>;
    variants: VariantChanges;
}

export type DiffStatus = "added" | "removed" | "changed";

export interface AddedExperiment {
    status: "added";
    name: string;
    after: ExperimentInfo;
}

export interface RemovedExperiment {
    status: "removed";
    name: string;
    before: ExperimentInfo;
}

export interface ChangedExperiment {
    status: "changed";
    name: string;
    before: ExperimentInfo;
    after: ExperimentInfo;
    changes: ExperimentChanges;
}

export type ExperimentDiffEntry = AddedExperiment | RemovedExperiment | ChangedExperiment;

export interface ExperimentDiff {
    entries: ExperimentDiffEntry[];
    /**
     * experiments present in both builds with no changes
     */
    unchanged: number;
}

/**
 * the newer side of an entry, for display info shared by both sides (type, scope, label)
 */
export function latestOf(entry: ExperimentDiffEntry): ExperimentInfo {
    return entry.status === "removed" ? entry.before : entry.after;
}

function valueChange<T>(before: T, after: T): ValueChange<T> | undefined {
    if (deepEqual(before, after)) {
        return undefined;
    }
    return {
        before,
        after,
    };
}

function diffVariants(before: readonly ExperimentVariant[], after: readonly ExperimentVariant[]): VariantChanges {
    const beforeByKey = new Map(before.map((variant) => [variant.key, variant]));
    const afterKeys = new Set(after.map(({ key }) => key));

    const changes: VariantChanges = {
        added: [],
        removed: before.filter(({ key }) => !afterKeys.has(key)),
        changed: [],
    };

    for (const variant of after) {
        const old = beforeByKey.get(variant.key);

        if (!old) {
            changes.added.push(variant);
        } else if (!deepEqual(old, variant)) {
            changes.changed.push({
                key: variant.key,
                before: old,
                after: variant,
            });
        }
    }
    return changes;
}

/**
 * changes between two versions of the same experiment, or `undefined` if there are none.
 *
 * location (`moduleId` / `rawIndex`) is ignored, it changes every build
 */
function diffExperiment(before: ExperimentInfo, after: ExperimentInfo): ExperimentChanges | undefined {
    const changes: ExperimentChanges = {
        type: valueChange(before.type, after.type),
        scope: valueChange(before.scope, after.scope),
        label: valueChange(before.label, after.label),
        defaultConfig: valueChange(before.defaultConfig, after.defaultConfig),
        variants: diffVariants(before.variants, after.variants),
    };

    const { variants } = changes;

    const hasChanges = changes.type || changes.scope || changes.label || changes.defaultConfig
      || variants.added.length > 0 || variants.removed.length > 0 || variants.changed.length > 0;

    return hasChanges ? changes : undefined;
}

/**
 * diff experiments between two builds, matched by name
 */
export function diffExperiments(before: readonly ExperimentInfo[], after: readonly ExperimentInfo[]): ExperimentDiff {
    // names should be unique within a build, dedupe just in case so each name has one entry
    const beforeByName = new Map(before.map((experiment) => [experiment.name, experiment]));
    const afterByName = new Map(after.map((experiment) => [experiment.name, experiment]));
    const entries: ExperimentDiffEntry[] = [];
    let unchanged = 0;

    for (const experiment of afterByName.values()) {
        const { name } = experiment;
        const old = beforeByName.get(name);

        if (!old) {
            entries.push({
                status: "added",
                name,
                after: experiment,
            });
            continue;
        }

        const changes = diffExperiment(old, experiment);

        if (changes) {
            entries.push({
                status: "changed",
                name,
                before: old,
                after: experiment,
                changes,
            });
        } else {
            unchanged++;
        }
    }

    for (const experiment of beforeByName.values()) {
        if (!afterByName.has(experiment.name)) {
            entries.push({
                status: "removed",
                name: experiment.name,
                before: experiment,
            });
        }
    }

    return {
        entries,
        unchanged,
    };
}
