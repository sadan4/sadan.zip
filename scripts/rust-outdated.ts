#!/usr/bin/env node
/**
 * List out-of-date Rust dependencies and their latest crates.io versions.
 *
 * Reads the workspace's direct dependencies via `cargo metadata`, then queries
 * crates.io in parallel for each crate's latest stable version and compares it
 * against every locked version. A crate locked at several majors at once (e.g.
 * syn 1.x and 2.x) is checked per-version, so an outdated older major is not
 * hidden behind a current newer one.
 *
 * Run: `node scripts/rust-outdated.ts`
 */
import { execFileSync } from "node:child_process";

const USER_AGENT = "rust-outdated-script (https://sadan.zip)";
const CONCURRENCY = 6;

interface CargoDependency {
    name: string;
    req: string;
}

interface CargoPackage {
    id: string;
    name: string;
    version: string;
    source: string | null;
    dependencies: CargoDependency[];
}

interface CargoMetadata {
    packages: CargoPackage[];
    workspace_members: string[];
}

interface Outdated {
    crate: string;
    req: string;
    locked: string;
    latest: string;
}

/**
 * Print a table with column widths derived from the header and every cell, so
 * columns stay aligned regardless of content length. A two-space gutter
 * separates columns; the last column is not padded.
 */
function printTable(headers: string[], rows: string[][]): void {
    const gutter = "  ";
    const widths = headers.map((h, i) => Math.max(h.length, ...rows.map((r) => r[i].length)));

    function line(cells: string[]): string {
        const padded = cells.map((c, i) => c.padEnd(widths[i]));

        return padded.join(gutter).trimEnd();
    }

    console.log(line(headers));
    console.log(line(widths.map((w) => "-".repeat(w))));
    for (const row of rows)
        console.log(line(row));
}

/** Split a semver string into numeric [major, minor, patch], ignoring pre-release. */
function parseVersion(v: string): number[] {
    return v.split("-")[0].split(".").map((n) => Number.parseInt(n, 10) || 0);
}

/** Compare two semver strings numerically. Returns >0 if `a` is newer. */
function compareVersions(a: string, b: string): number {
    const pa = parseVersion(a);
    const pb = parseVersion(b);

    for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
        const diff = (pa[i] ?? 0) - (pb[i] ?? 0);

        if (diff !== 0)
            return diff;
    }
    return 0;
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Latest stable version on crates.io, or null if the crate isn't published.
 *
 * A 404 means unpublished (git/renamed crate) and yields null. A 429/5xx is
 * transient (rate limiting) and is retried with backoff; if it never clears we
 * throw rather than silently pretend the crate is up to date.
 */
async function fetchLatest(crate: string): Promise<string | null> {
    const maxRetries = 5;

    for (let attempt = 0; ; attempt++) {
        const res = await fetch(`https://crates.io/api/v1/crates/${crate}`, {
            headers: { "User-Agent": USER_AGENT },
        });

        if (res.status === 404)
            return null;

        if ((res.status === 429 || res.status >= 500) && attempt < maxRetries) {
            const retryAfter = Number.parseInt(res.headers.get("retry-after") ?? "", 10);

            const wait = Number.isFinite(retryAfter)
                ? retryAfter * 1000
                : (2 ** attempt) * 500;

            await sleep(wait);
            continue;
        }

        if (!res.ok)
            throw new Error(`crates.io ${res.status} for "${crate}"`);

        const body = (await res.json()) as {
            crate?: { max_stable_version?: string;
                max_version?: string; };
        };

        return body.crate?.max_stable_version ?? body.crate?.max_version ?? null;
    }
}

/** Run async `worker` over `items` with bounded concurrency. */
async function mapPool<T, R>(
    items: T[],
    limit: number,
    worker: (item: T) => Promise<R>,
): Promise<R[]> {
    const results: R[] = Array.from({ length: items.length });
    let next = 0;

    async function run(): Promise<void> {
        while (next < items.length) {
            const i = next++;

            results[i] = await worker(items[i]);
        }
    }
    await Promise.all(Array.from({ length: limit }, run));
    return results;
}

function loadMetadata(): CargoMetadata {
    const root = execFileSync("git", ["rev-parse", "--show-toplevel"], {
        encoding: "utf8",
    }).trim();

    const raw = execFileSync(
        "cargo",
        ["metadata", "--format-version", "1", "--all-features"],
        {
            cwd: root,
            encoding: "utf8",
            maxBuffer: 64 * 1024 * 1024,
        },
    );

    return JSON.parse(raw) as CargoMetadata;
}

interface DirectDeps {
    /** Unique (name, version) packages that direct deps resolve to. */
    packages: CargoPackage[];
    /** Cargo.toml version requirement(s) per crate name, joined for display. */
    reqByName: Map<string, string>;
}

/**
 * Direct workspace dependencies that resolve to a registry source, plus the
 * version requirement declared for each in Cargo.toml. Path/git deps have a
 * null source and would collide with unrelated same-named crates, so they're
 * excluded.
 */
function directRegistryDeps(meta: CargoMetadata): DirectDeps {
    const members = new Set(meta.workspace_members);
    const reqs = new Map<string, Set<string>>();

    for (const pkg of meta.packages) {
        if (!members.has(pkg.id))
            continue;
        for (const dep of pkg.dependencies) {
            const set = reqs.get(dep.name) ?? new Set<string>();

            set.add(dep.req);
            reqs.set(dep.name, set);
        }
    }

    const seen = new Set<string>();
    const packages: CargoPackage[] = [];

    for (const pkg of meta.packages) {
        if (!reqs.has(pkg.name))
            continue;
        if (!pkg.source?.startsWith("registry+"))
            continue;

        const key = `${pkg.name}\t${pkg.version}`;

        if (seen.has(key))
            continue;
        seen.add(key);
        packages.push(pkg);
    }

    const reqByName = new Map<string, string>();

    for (const [name, set] of reqs)
        reqByName.set(name, [...set].join(", "));

    return {
        packages: packages.toSorted((a, b) => a.name.localeCompare(b.name)),
        reqByName,
    };
}

async function main(): Promise<void> {
    const { packages, reqByName } = directRegistryDeps(loadMetadata());
    // Fetch each crate's latest version once, shared across its locked versions.
    const names = [...new Set(packages.map((p) => p.name))];
    const latestByName = new Map<string, string | null>();

    await mapPool(names, CONCURRENCY, async (name) => {
        latestByName.set(name, await fetchLatest(name));
    });

    // Collect every locked version per crate, so a crate pinned at several
    // majors at once (e.g. syn 2.x and 3.x) shows all of them together.
    const lockedByName = new Map<string, string[]>();

    for (const { name, version } of packages) {
        const list = lockedByName.get(name) ?? [];

        list.push(version);
        lockedByName.set(name, list);
    }

    const outdated: Outdated[] = [];

    for (const [name, versions] of lockedByName) {
        const latest = latestByName.get(name);

        // Flag the crate if any locked version trails the latest release.
        const behind = latest
          && versions.some((v) => compareVersions(latest, v) > 0);

        if (latest && behind) {
            outdated.push({
                crate: name,
                req: reqByName.get(name) ?? "?",
                locked: versions.toSorted(compareVersions).join(", "),
                latest,
            });
        }
    }
    outdated.sort((a, b) => a.crate.localeCompare(b.crate));

    printTable(
        ["CRATE", "CARGO.TOML", "LOCKED", "LATEST"],
        outdated.map((o) => [o.crate, o.req, o.locked, o.latest]),
    );
    console.error(`\n${outdated.length} outdated crate(s).`);
}

main().catch((err: unknown) => {
    console.error(err);
    process.exit(1);
});
