import type { ExperimentInfo, ExperimentScope } from "@sadan4/libsadancore";

export type ExperimentType = ExperimentInfo["type"];

export const EXPERIMENT_TYPES: readonly ExperimentType[] = ["apex", "normal"];

export const experimentTypeLabels: Record<ExperimentType, string> = {
    apex: "Apex",
    normal: "Normal",
};

export const EXPERIMENT_SCOPES: readonly ExperimentScope[] = ["user", "guild", "installation"];

export const experimentScopeLabels: Record<ExperimentScope, string> = {
    user: "User",
    guild: "Guild",
    installation: "Installation",
};

/**
 * experiment names start with their creation date, e.g. `2026-04-foo` or `2025-09_bar`,
 * sometimes with a day, e.g. `2026-04-12-foo`
 */
const EXPERIMENT_DATE_REGEX = /^(\d{4})-(\d{2})(?:-(\d{2}))?(?=[-_])/;

/**
 * a sortable number for the date in an experiment's name, or `-1` if it has none
 */
function experimentDateKey(name: string): number {
    const match = EXPERIMENT_DATE_REGEX.exec(name);

    if (!match) {
        return -1;
    }

    const [, year, month, day = "0"] = match;

    return (((+year * 100) + +month) * 100) + +day;
}

/**
 * newest first, undated experiments last
 */
export function sortByDate<T extends { name: string; }>(experiments: readonly T[]): T[] {
    return experiments
        .map((experiment) => [experimentDateKey(experiment.name), experiment] as const)
        .toSorted(([a], [b]) => b - a)
        .map(([, experiment]) => experiment);
}

export function toggledSet<T>(set: ReadonlySet<T>, value: T): ReadonlySet<T> {
    const next = new Set(set);

    if (!next.delete(value)) {
        next.add(value);
    }

    return next;
}

export function countBy<T, K extends string>(items: readonly T[], keys: readonly K[], getKey: (item: T) => K) {
    const counts = Object.fromEntries(keys.map((key) => [key, 0])) as Record<K, number>;

    for (const item of items) {
        counts[getKey(item)]++;
    }

    return counts;
}
