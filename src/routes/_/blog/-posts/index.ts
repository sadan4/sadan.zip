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
    frontmatter: PostMeta;
}

const posts = import.meta.glob<PostModule>("./*.mdx");
const postContent = new Map<string, MDXContent>();

function pathForSlug(slug: string) {
    return `./${slug}.mdx`;
}

export function loadPostMeta(slug: string): Promise<PostMeta> | undefined {
    const path = pathForSlug(slug);

    return posts[path]?.().then(({ default: content, frontmatter }) => {
        if (!postContent.has(path)) {
            postContent.set(path, content);
        }
        return frontmatter;
    });
}

export function getPostContent(slug: string): MDXContent {
    const path = pathForSlug(slug);
    let content = postContent.get(path);

    if (!content) {
        content = lazy(posts[path]);
        postContent.set(path, content);
    }
    return content;
}

export async function loadPostSummaries(): Promise<PostSummary[]> {
    const summaries = await Promise.all(Object.entries(posts).map(async ([path, load]) => ({
        ...(await load()).frontmatter,
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
