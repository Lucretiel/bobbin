/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import { Helmet } from "react-helmet";

type Entry = {
	slug: string;
	question: string;
	answer: React.ReactElement;
};
const entries: Entry[] = [
	{
		slug: "what-is-this",
		question: "What is this?",
		answer: (
			<span>
				Bobbin is a way to easily share Twitter threads with your friends.
			</span>
		),
	},
	{
		slug: "how-does-it-work",
		question: "How does it work?",
		answer: (
			<span>
				Bobbin threads are definied by the final tweet in the thread. When given
				the final tweet in a thread, Bobbin follows the reply chain backwards,
				towards the beginning of the thread, and displays the thread from the
				beginning. It ignores tweets <em>after</em> the final tweet, even if
				they were posted by the author of the thread.
			</span>
		),
	},
	{
		slug: "load-times",
		question: "Why does it take a while for my thread to load?",
		answer: (
			<span>
				The first time a user shares a thread, Bobbin must look up each
				individual tweet one-by-one, because Twitter doesn't currently provide a
				way to look up whole threads. Internally, Bobbin stores the reply chain,
				so subsequent loads of the thread should be faster.
			</span>
		),
	},
	{
		slug: "does-bobbin-store-tweets",
		question: "Does bobbin store my tweets?",
		answer: (
			<span>
				Nope! The only thing that bobbin stores is the 20+ digit tweet ID of
				each tweet in the thread, plus your own User ID. It uses Twitter's own
				"embedded tweet" widget to actually display the tweet. We don't store
				any of your content, and any tweets you delete will not appear in the
				thread.
			</span>
		),
	},
	{
		slug: "why-is-it-called-bobbin",
		question: "Why is it called Bobbin?",
		answer: (
			<span>
				Because a <a href="https://en.wikipedia.org/wiki/Bobbin">bobbin</a> is
				how you share thread.
			</span>
		),
	},
];

const FaqPage: React.FC = () => (
	<section className="section">
		<Helmet>
			<title>Bobbin FAQ</title>
		</Helmet>
		<div className="container" id="faq-content">
			<h2 className="title">Frequently Asked Questions</h2>
			<div className="content">
				<dl>
					{entries.map(({ question, answer, slug }) => (
						<React.Fragment key={slug}>
							<dt className="faq-question" id={slug}>
								<strong>{question}</strong>
								<a className="hoverlink" href={`#${slug}`}>
									<i className="fas fa-link" />
								</a>
							</dt>
							<dd className="faq-answer">{answer}</dd>
						</React.Fragment>
					))}
				</dl>
			</div>
		</div>
	</section>
);

export default FaqPage;
