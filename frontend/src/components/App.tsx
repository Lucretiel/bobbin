/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from "react";
import { BrowserRouter as Router, Route, Link } from "react-router-dom";
import classNames from "classnames";

import HomePage from "./HomePage";
import ThreadPage from "./ThreadPage";
import FAQPage from "./FAQPage";

const Nav: React.FC = React.memo(() => {
	const [isOpen, setOpen] = React.useState(false);
	const toggleOpen = React.useCallback(() => setOpen(open => !open), [setOpen]);

	const activeClass = { "is-active": isOpen };
	const burgerClass = classNames("navbar-burger", activeClass);
	const menuClass = classNames("navbar-menu", "is-boxed", activeClass);

	return (
		<nav className="navbar" role="navigation" aria-label="main navigation">
			<div className="container">
				<div className="navbar-brand">
					<Link className="navbar-item has-text-black is-size-5" to="/">
						Bobbin<span className="beta-label"> Beta</span>
					</Link>

					<a role="button" className={burgerClass} onClick={toggleOpen}>
						<span aria-hidden="true" />
						<span aria-hidden="true" />
						<span aria-hidden="true" />
					</a>
				</div>
				<div className={menuClass} id="navbar-links">
					<div className="navbar-start">
						<Link className="navbar-item" to="/">
							Home
						</Link>
						<Link className="navbar-item" to="/faq">
							FAQ
						</Link>
					</div>
				</div>
			</div>
		</nav>
	);
});

const Footer: React.FC = React.memo(() => (
	<footer className="footer">
		<div className="container">
			<span className="footer-item">
				<a href="https://github.com/Lucretiel/bobbin">Github</a>
			</span>
			<span className="footer-item">
				<a href="https://github.com/Lucretiel/bobbin/issues">
					{"Issues & Feedback"}
				</a>
			</span>
		</div>
	</footer>
));

const App: React.FC = () => (
	<Router>
		<div className="grow-main">
			<Nav />
			<main className="section">
				<div className="container">
					<Route
						exact
						path="/"
						render={({ history }) => (
							<HomePage
								navigateToThread={(tweetId: string) =>
									history.push(`/thread/${tweetId}`)
								}
							/>
						)}
					/>
					<Route
						exact
						path="/thread/:id"
						render={({ match }) => <ThreadPage tail={match.params.id} />}
					/>
					<Route exact path="/faq" render={() => <FAQPage />} />
				</div>
			</main>
			<Footer />
		</div>
	</Router>
);

export default App;
