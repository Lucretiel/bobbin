/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import { Set } from "immutable";
import { memoize, curry } from "lodash";
import Tweet from "./Tweet";

type TweetsRendered = { [tweetId: string]: boolean };

const TweetList: React.FC<{
	tweetIds: string[];
	fullyRendered: (isFullyRendered: boolean) => void;
}> = ({ tweetIds, fullyRendered }) => {
	const [tweetsRendered, setTweetsRendered] = React.useState<Set<string>>(Set);

	const setRenderedCb = React.useMemo(
		() =>
			memoize((tweetId: string) => (isRendered: boolean) =>
				setTweetsRendered(state =>
					isRendered ? state.add(tweetId) : state.remove(tweetId),
				),
			),
		[setTweetsRendered],
	);

	const isFullyRendered = tweetsRendered.size === tweetIds.length;

	React.useEffect(() => {
		fullyRendered(isFullyRendered);
	}, [fullyRendered, isFullyRendered]);

	return (
		<ul className="list-unstyled">
			{tweetIds.map((tweetId: string) => (
				<li key={tweetId}>
					<Tweet
						key={tweetId}
						tweetId={tweetId}
						rendered={setRenderedCb(tweetId)}
					/>
				</li>
			))}
		</ul>
	);
};

export default TweetList;
