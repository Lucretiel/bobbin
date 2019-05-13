/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from 'react'

import Tweet from './Tweet'
import promiseRunner from '../promiseChain'

interface TweetListProps {
	tweetIds: string[],
	fullyRendered(isFullyRendered: boolean): void,
}

const TweetList: React.FC<TweetListProps> = ({tweetIds, fullyRendered}) => {
	React.useEffect(() => {
		fullyRendered(true);
	});

	return <ul className="list-unstyled">{
		tweetIds.map((tweetId: string) =>
			<li key={tweetId}>
				<Tweet tweetId={tweetId} done={() => {}}/>
			</li>
		)
	}</ul>
}

export default TweetList;
