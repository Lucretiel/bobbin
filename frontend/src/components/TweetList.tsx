/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import Tweet from "./Tweet";

/**
 * Helper for passing a callback to each component in a loop. Given a callback
 * with the signature (key, ..args), returns a new factory with the signature
 * (key) => (...args). When called with a key, returns a wrapper function taking
 * (...args) wich calls the original callback. This wrapper function is guaranteed
 * to be the same for any given key.
 */
/*const useKeyedCallback = function<
	Ret,
	Key,
	Args extends Array<any>
>(
	callback: (key: Key, ...args: Args) => Ret,
):
	(key: Key) => (...args: Args) => Ret
{
	return React.useMemo(() => memoize(key => (...args) => callback(key, ...args)), [])
}
*/

type TweetsRendered = { [tweetId: string]: boolean };

const TweetList: React.FC<{
	tweetIds: string[];
	fullyRendered: (isFullyRendered: boolean) => void;
}> = ({ tweetIds, fullyRendered }) => {
	const [tweetsRendered, setTweetsRendered] = React.useState<{
		[tweetId: string]: boolean;
	}>({});

	const setRendered = React.useCallback(
		(tweetId: string, isRendered: boolean) =>
			setTweetsRendered(oldRendered => ({
				...oldRendered,
				[tweetId]: isRendered,
			})),
		[setTweetsRendered],
	);

	React.useEffect(() => {
		const renderCount = Object.values(tweetsRendered).filter(r => r).length;

		if (renderCount === tweetIds.length) {
			fullyRendered(true);
		} else {
			fullyRendered(false);
		}
	}, [fullyRendered, tweetsRendered, tweetIds]);

	return (
		<ul className="list-unstyled">
			{tweetIds.map((tweetId: string) => (
				<li key={tweetId}>
					<Tweet
						key={tweetId}
						tweetId={tweetId}
						// TODO: find a way to cache the setRendered function for each individual Tweet
						rendered={isRendered => setRendered(tweetId, isRendered)}
					/>
				</li>
			))}
		</ul>
	);
};

export default TweetList;
