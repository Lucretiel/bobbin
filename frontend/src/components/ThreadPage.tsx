/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import { Helmet } from "react-helmet";

import TweetList from "./TweetList";

type Author = {
	name: string;
	handle: string;
};

const ThreadTitle: React.FC<{ author: Author | null }> = ({ author }) =>
	author ? (
		<h3 className="has-text-centered author-header">
			Thread by{" "}
			<a
				href={`https://twitter.com/${author.handle}`}
				target="_blank"
				rel="noopener noreferrer"
				className="hover-underline"
			>
				<span className="author">
					<span className="author-name">{author.name}</span>{" "}
					<span className="author-handle">@{author.handle}</span>
				</span>
			</a>
		</h3>
	) : (
		<h3 className="title has-text-centered">Conversation</h3>
	);

const ThreadJumper: React.FC<{
	fullyRendered: boolean;
	scrollTarget: HTMLElement | null;
}> = React.memo(({ fullyRendered, scrollTarget }) => {
	const [isHover, setIsHover] = React.useState(false);

	const setHover = React.useCallback(() => setIsHover(true), [setIsHover]);
	const setNoHover = React.useCallback(() => setIsHover(false), [setIsHover]);

	const jumpToTop = React.useCallback(() => {
		scrollTarget &&
			scrollTarget.scrollIntoView({ behavior: "smooth", block: "end" });
	}, [scrollTarget]);

	return (
		<div
			className="has-text-centered thread-end tweet-like"
			onMouseEnter={setHover}
			onMouseLeave={setNoHover}
		>
			{fullyRendered ? (
				<span className="strike">
					{isHover ? (
						<span className="hover-underline" onClick={jumpToTop}>
							Return to Top
						</span>
					) : (
						<span className="hover-underline">End of Thread</span>
					)}
				</span>
			) : (
				"Loading Tweets..."
			)}
		</div>
	);
});

const ThreadPage: React.FC<{
	head?: string;
	tail: string;
}> = ({ head, tail }) => {
	const [threadTweetIds, setThreadTweetIds] = React.useState<string[] | null>(
		null,
	);
	const [author, setAuthor] = React.useState<Author | null>(null);
	const [fullyRendered, setFullyRendered] = React.useState(false);
	const [headerRef, setHeaderRef] = React.useState<HTMLElement | null>(null);

	React.useEffect(() => {
		const controller = new AbortController();

		const query = head ? `head=${head}&tail=${tail}` : `tail=${tail}`;

		// TODO: Error Handling!!
		fetch(`/api/thread?${query}`, {
			signal: controller.signal,
			headers: {
				Accept: "application/json",
			},
		})
			.then(response => response.json())
			.then(content => {
				setThreadTweetIds(content.thread);
				setAuthor(content.author);
			});

		return () => {
			setThreadTweetIds(null);
			setAuthor(null);
			controller.abort();
		};
	}, [head, tail]);

	return (
		<section className="section">
			<Helmet>
				<title>
					{author
						? `Thread by @${author.handle}`
						: threadTweetIds
						? "Conversation"
						: "Thread"}
				</title>
			</Helmet>
			<div className="container">
				<div ref={setHeaderRef} />
				<ThreadTitle author={author} />
				{threadTweetIds === null ? null : (
					<TweetList
						tweetIds={threadTweetIds}
						fullyRendered={setFullyRendered}
					/>
				)}
				<ThreadJumper fullyRendered={fullyRendered} scrollTarget={headerRef} />
			</div>
		</section>
	);
};

export default ThreadPage;
