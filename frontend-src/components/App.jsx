import React from 'react'
import { BrowserRouter as Router, Route, Link } from 'react-router-dom'

import HomePage from 'components/HomePage.jsx'
import ThreadPage from 'components/ThreadPage.jsx'
import FAQPage from 'components/FAQPage.jsx'

export default class App extends React.PureComponent {
	render() {
		return <Router>
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
						<HomePage navigate={path => history.push(path)}/>
					}/>
					<Route exact path="/thread/:id" render={({ match }) =>
						<ThreadPage tail={match.params.id} />
					}/>
					<Route exact path="/faq" render={props =>
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
	}
}
