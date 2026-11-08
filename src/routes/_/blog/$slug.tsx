import { Link } from "@/components/Links";
import { Mdx } from "@/components/Mdx";
import { Text } from "@/components/Text";
import { EM_DASH } from "@/utils/constants";
import { createFileRoute, notFound } from "@tanstack/react-router";

import { formatPostDate, getPostContent, loadPostMeta } from "./-posts";

import { Undo2Icon } from "lucide-react";

export const Route = createFileRoute("/_/blog/$slug")({
    async loader({ params: { slug } }) {
        const meta = await loadPostMeta(slug);

        if (!meta) {
            // eslint-disable-next-line typescript/only-throw-error
            throw notFound();
        }
        return meta;
    },
    head({ loaderData }) {
        if (!loaderData) {
            return {};
        }

        const { title, description } = loaderData;

        return {
            meta: [
                {
                    property: "og:title",
                    content: `sadan ${EM_DASH} ${title}`,
                },
                {
                    property: "og:description",
                    content: description,
                },
            ],
        };
    },
    component: Post,
});

function Post() {
    const { slug } = Route.useParams();
    const { title, date } = Route.useLoaderData();

    return (
        <article>
            <Link
                to="/blog"
                className="hover:*:text-info-500"
            >
                <Undo2Icon className="mr-1 inline size-6 text-info-400" />
                <Text tag="span">Back to blog</Text>
            </Link>
            <h1 className="mt-4 text-4xl font-bold text-primary-400">{title}</h1>
            <Text
                color="white-700"
                className="mb-8"
            >
                <time dateTime={date}>{formatPostDate(date)}</time>
            </Text>
            <Mdx content={getPostContent(slug)} />
        </article>
    );
}
