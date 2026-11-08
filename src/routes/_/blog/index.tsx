import { Link } from "@/components/Links";
import { Text } from "@/components/Text";
import { createFileRoute } from "@tanstack/react-router";

import { formatPostDate, loadPostSummaries } from "./-posts";

export const Route = createFileRoute("/_/blog/")({
    loader: loadPostSummaries,
    component: BlogIndex,
    staticData: {
        description: "random ramblings",
        pageTitle: "blog",
    },
});

function BlogIndex() {
    const posts = Route.useLoaderData();

    return (
        <>
            <h1 className="text-4xl font-bold text-primary-400">Blog</h1>
            <ul className="mt-8 flex flex-col gap-6">
                {posts.map(({ slug, title, description, date }) => (
                    <li key={slug}>
                        <Link
                            to="/blog/$slug"
                            params={{ slug }}
                        >
                            <Text
                                tag="span"
                                size="xl"
                            >
                                {title}
                            </Text>
                        </Link>
                        <Text
                            size="md"
                            color="white-600"
                        >
                            {description}
                        </Text>
                        <Text color="white-700">
                            <time dateTime={date}>{formatPostDate(date)}</time>
                        </Text>
                    </li>
                ))}
            </ul>
        </>
    );
}
