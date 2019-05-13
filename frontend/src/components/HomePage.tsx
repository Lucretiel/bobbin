/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from 'react'
import classNames from 'classnames'

import Title from './Title'

const tweetRegex = /^\s*(?:(?:https?:\/\/)?(?:(?:www|mobile)\.)?twitter\.com\/\w{1,15}\/status\/)?(\d{1,24})(?:[?#]\S*)?\s*$/

interface EntryFormProps {
	submit(tweetId: string): void
}

interface EntryFormState {
	tweetLink: string,
}

const TweetEntryForm: React.FC<EntryFormProps> = ({submit}) => {
	const [tweetLink, setTweetLink] = React.useState("");

	const tweetId = React.useMemo((): string | null => {
		const match = tweetRegex.exec(tweetLink)
		return match === null ? null : match[1]
	}, [tweetLink]);

	const isEmpty = tweetLink === ""
	const isValid = tweetId !== null

	const submitTweet = () => {
		if(tweetId) {
			submit(tweetId);
		}
	}

	const submitClass = classNames({
		"btn": true,
		"btn-primary": isValid,
		"btn-outline-primary": !isValid,
		"disabled": !isValid,
	})

	const textInputClass = classNames(
		"form-control-lg",
		"form-control", {
			"is-valid": !isEmpty && isValid,
			"is-invalid": !isEmpty && !isValid
	})

	return <form id="tweet-entry-form">
		<div className="form-row">
			<div className="col">
				<div className="form-group">
					<input
						type="text"
						className={textInputClass}
						placeholder="Link to last tweet in thread"
						value={tweetLink}
						onChange={event => setTweetLink(event.target.value)}
					/>
				</div>
			</div>
		</div>
		<div className="form-row justify-content-center">
			<div className="col col-md-7 col-sm-9">
				<div className="collapse" id="submit-help-block">
					<p><small>
						To view a twitter thread, find the last tweet in the thread you
						want to view, and copy-paste a link to the tweet above. Bobbin
						will automatically follow the reply-chain backwards to the
						beginning of the thread, and display the whole thread. The thread
						view link can be shared with other people.
					</small></p>
				</div>
			</div>
		</div>
		<div className="form-row justify-content-center">
			<div className="col-auto">
				<div className="form-group">
					<button
						type="button"
						className="btn btn-info"
						data-toggle="collapse"
						data-target="#submit-help-block"
					>
						Help
					</button>
				</div>
			</div>
			<div className="col-auto">
				<div className="form-group">
					<button
						type="submit"
						className={submitClass}
						onClick={submitTweet}
						disabled={!isValid}
					>
						Submit
					</button>
				</div>
			</div>
		</div>
	</form>
}

const MainPage: React.FC<{navigate(path: string): void}> = ({navigate}) => {
	const submit = React.useCallback(tweetId => {
		navigate(tweetId)
	}, [navigate]);

	return <div className="container" id="homepage">
		<Title>Bobbin</Title>
		<div className="row">
			<div className="col text-center">
				<h2>Share threads with <strong>Bobbin</strong></h2>
			</div>
		</div>
		<div className="row">
			<div className="col">
				<TweetEntryForm submit={submit}/>
			</div>
		</div>
	</div>
}

export default MainPage;
