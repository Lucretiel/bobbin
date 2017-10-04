import ReactDOM from "react-dom"
import React from "react"

import App from "components/App.jsx"

console.log('Note: "sandbox not initialized" is a well-known bug in the twitter api')

document.addEventListener("DOMContentLoaded", event =>
	ReactDOM.render(<App />, document.getElementById("react-container"))
)
