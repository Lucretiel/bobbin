/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

import React from 'react'
import { BrowserRouter as Router, Route, Link } from 'react-router-dom'

import HomePage from './HomePage'
import ThreadPage from './ThreadPage'
import FAQPage from './FAQPage'

const App: React.FC = () => <Router>
	<div id="page-wrapper">
		<nav className="navbar navbar-light navbar-expand-sm">
			<div className="container">
				<Link className="navbar-brand" to="/">Bobbin <span className="beta-label">Beta</span></Link>
				<button className="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbar-links">
					<span className="navbar-toggler-icon"></span>
				</button>
				<div className="collapse navbar-collapse" id="navbar-links">
					{/*<Link className="nav-item nav-link disabled" to="#">About</Link>*/}
					<Link className="nav-item nav-link" to="/faq">FAQ</Link>
				</div>
			</div>
		</nav>
		<main>
			<Route exact path="/" render={({ history }) =>
				<HomePage navigate={tweetId => history.push(`/thread/${tweetId}`)}/>
			}/>
			<Route exact path="/thread/:id" render={({ match }) =>
				<ThreadPage tail={match.params.id} />
			}/>
			<Route exact path="/faq" render={() =>
				<FAQPage />
			}/>
		</main>
		<footer>
			<div className="container">
				<div className="row">
					<div className="col d-flex justify-content-end">
						<span className="footer-item">
							<a href="https://github.com/Lucretiel/bobbin">Github</a>
						</span>
						<span className="footer-item">
							<a href="https://github.com/Lucretiel/bobbin/issues">Issues & Feedback</a>
						</span>
					</div>
				</div>
			</div>
		</footer>
	</div>
</Router>

export default App;
