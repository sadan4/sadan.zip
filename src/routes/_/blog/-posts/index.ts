import type { MDXContent } from "mdx/types";
import { lazy } from "react";

export interface PostMeta {
    title: string;
    description: string;
    /**
     * `YYYY-MM-DD`
     */
    date: string;
}

export interface PostSummary extends PostMeta {
    slug: string;
}

interface PostModule {
    default: MDXContent;
    meta: PostMeta;
}

const posts = import.meta.glob<PostModule>("./*.mdx");
const postContent = new Map(Object.entries(posts).map(([path, load]) => [path, lazy(load)]));

function pathForSlug(slug: string) {
    return `./${slug}.mdx`;
}

export function loadPostMeta(slug: string): Promise<PostMeta> | undefined {
    return posts[pathForSlug(slug)]?.().then(({ meta }) => meta);
}

export function getPostContent(slug: string): MDXContent {
    return postContent.get(pathForSlug(slug))!;
}

export async function loadPostSummaries(): Promise<PostSummary[]> {
    const summaries = await Promise.all(Object.entries(posts).map(async ([path, load]) => ({
        ...(await load()).meta,
        slug: path.slice("./".length, -".mdx".length),
    })));

    return summaries.toSorted((a, b) => b.date.localeCompare(a.date));
}

export function formatPostDate(date: string): string {
    return new Date(date).toLocaleDateString("en-US", {
        dateStyle: "long",
        timeZone: "UTC",
    });
}
