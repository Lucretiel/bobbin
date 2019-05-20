/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import classNames from "classnames";
import { Helmet } from "react-helmet";

const tweetRegex = /^\s*(?:(?:https?:\/\/)?(?:(?:www|mobile)\.)?twitter\.com\/\w{1,15}\/status\/)?(\d{1,24})(?:[?#]\S*)?\s*$/;

/*
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
									<p>
										<small>
											To view a twitter thread, find the last tweet in the
											thread you want to view, and copy-paste a link to the
											tweet above. Bobbin will automatically follow the
											reply-chain backwards to the beginning of the thread, and
											display the whole thread. The thread view link can be
											shared with other people.
										</small>
									</p>
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
 */

const TweetEntryForm: React.FC<{
	navigateToThread: (tweetdId: string) => void;
}> = ({ navigateToThread }) => {
	const [tweetLink, setTweetLink] = React.useState("");

	const tweetId = React.useMemo(() => {
		const match = tweetRegex.exec(tweetLink);
		return match === null ? null : match[1];
	}, [tweetLink]);

	const isEmpty = tweetLink === "";
	const isValid = tweetId !== null;

	const submitTweet = React.useCallback(() => {
		if (tweetId !== null) {
			navigateToThread(tweetId);
		}
	}, [navigateToThread, tweetId]);

	const setTweetLinkEvent = React.useCallback(
		(event: React.ChangeEvent<HTMLInputElement>) =>
			setTweetLink(event.target.value),
		[setTweetLink],
	);

	const controlClass = classNames("control", {
		"has-icons-right": !isEmpty,
	});

	const submitClass = classNames(
		"button",
		"is-link",
		"transition",
		"is-primary",
		{
			"is-outlined": !isValid,
			disabled: !isValid,
		},
	);

	const textInputClass = classNames("input", "transition", {
		"is-success": !isEmpty && isValid,
		"is-danger": !isEmpty && !isValid,
	});

	const textInputIconClass = classNames("fas", {
		"fa-check": isValid,
		"fa-times": !isValid,
	});

	return (
		<section className="section">
			<div className="container">
				<form id="tweet-entry-form">
					<div className="field">
						<div className={controlClass}>
							<input
								type="text"
								className={textInputClass}
								placeholder="Link to last tweet in thread"
								value={tweetLink}
								onChange={setTweetLinkEvent}
							/>
							{isEmpty ? null : (
								<span className="icon is-small is-right">
									<i className={textInputIconClass} />
								</span>
							)}
						</div>
					</div>
					<div className="field is-grouped is-grouped-centered">
						<div className="control">
							<button
								type="button"
								className="button is-info is-outlined transition"
							>
								Help
							</button>
						</div>
						<div className="control">
							<button
								className={submitClass}
								type="submit"
								onClick={submitTweet}
								disabled={!isValid}
							>
								View Thread
							</button>
						</div>
					</div>
				</form>
			</div>
		</section>
	);
};

const MainPage: React.FC<{
	navigateToThread(tweetId: string): void;
}> = React.memo(({ navigateToThread }) => (
	<div id="homepage">
		<Helmet>
			<title>Bobbin</title>
		</Helmet>
		<section className="hero">
			<div className="hero-body">
				<div className="container">
					<h1 className="title has-text-centered">
						Share threads with <strong>Bobbin</strong>
					</h1>
				</div>
			</div>
		</section>
		<TweetEntryForm navigateToThread={navigateToThread} />
	</div>
));

export default MainPage;
