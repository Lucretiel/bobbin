/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from 'react'

import TweetList from './TweetList'
import Title from './Title'

interface Props {
	head?: string,
	tail: string,
}

const ThreadPage: React.FC<Props> = ({head, tail}) => {
	const [threadTweetIds, setThreadTweetIds] = React.useState<string[] | null>(null);
	const [author, setAuthor] = React.useState<{name: string, handle: string} | null>(null);
	const [fullyRendered, setFullyRendered] = React.useState(false);

	React.useEffect(() => {
		const controller = new AbortController();

		setThreadTweetIds(null);
		setAuthor(null);

		const query = head ?
			`head=${head}&tail=${tail}` :
			`tail=${tail}`

		// TODO: Error Handling!!
		fetch(`/api/thread?${query}`, {
			signal: controller.signal,
			headers: {
				Accept: "application/json",
			}
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

	const header = author ?
		<h3 className="author-header">Thread by <a
			href={`https://twitter.com/${author.handle}`}
			target="_blank"
			rel="noopener noreferrer">
			<span className="author">
				<span className="author-name">{author.name}</span>{' '}
				<span className="author-handle">@{author.handle}</span>
			</span>
		</a></h3>:
		<h3>Conversation</h3>;

	return <div className="container">
		<Title>{
			author ? `Thread by @${author.handle}` :
			threadTweetIds ? "Conversation" :
			"Thread"
		}</Title>
		<div className="row">
			<div className="col text-center">
				{header}
			</div>
		</div>
		<div className="row justify-content-center">
			<div className="col">
				{threadTweetIds === null ?
					null :
					<TweetList
						tweetIds={threadTweetIds}
						fullyRendered={setFullyRendered}
					/>
				}
			</div>
		</div>
		<div className="row">
			<div className="col">
				<div className="text-center thread-end tweet-like">
					{fullyRendered ?
						<span className="strike">
							<span>End of Thread</span>
						</span> :
						"Loading Tweets..."
					}
				</div>
			</div>
		</div>
	</div>
}

export default ThreadPage;
