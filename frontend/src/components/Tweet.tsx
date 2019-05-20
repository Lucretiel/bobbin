/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";

import twttr from "../twitter";

const EmbeddedTweet: React.FC<{
	tweetId: string;
	rendered: (isRendered: boolean) => void;
}> = ({ tweetId, rendered }) => {
	const [node, setNode] = React.useState<HTMLDivElement | null>(null);
	const [isRendered, setIsRendered] = React.useState(false);

	React.useEffect(() => {
		if (node === null) {
			return;
		}

		const tweetTask = twttr.widgets
			.createTweet(tweetId, node, {
				conversation: "none",
				align: "center",
			})
			.then(tweet => {
				setIsRendered(true);
				return tweet;
			});

		tweetTask.catch(error => {
			console.error(error);
		});

		return () => {
			tweetTask.then(tweet => {
				setIsRendered(false);
			});
		};
	}, [node, tweetId, setIsRendered]);

	// Use a separate effect to call `rendered` so that if the callback changes we
	// don't have to completely reload the tweet.
	React.useEffect(() => {
		rendered(isRendered);
	}, [rendered, isRendered]);

	// if the tweetId changes, the key here will force the old tweet to unmount
	// and a new div to be created as a target for the new
	return <div key={tweetId} className="tweet-container" ref={setNode} />;
};

export default EmbeddedTweet;
