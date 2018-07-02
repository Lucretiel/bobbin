/*
promiseRunner is a helper to ensure that your browser isn't overloaded with
things. It returns a function with the signature (() => Promise) => Promise. When
called, it adds the callable to the task queue. The callables will be called,
in order; each callable should initiate some work and return a Promise
representing that work. The returned Promise waits for the callable to start,
then resolves with the result of the callable.
*/

const ignoreErr = error => null

export default function promiseRunner(maxConcurrent) {
	let runningTasks = 0
	const waitingTasks = []

	const launchNextTask = () => {
		if(waitingTasks.length > 0 && runningTasks <= maxConcurrent) {
			const runner = waitingTasks.shift()
			runner()
		} else {
			runningTasks -= 1
		}
	}

	return userRunner => new Promise(resolve => {
		const runner = () => {
			const task = new Promise(resolve => resolve(userRunner()))
			resolve(task.finally(() => launchNextTask()))
		}

		if(runningTasks >= maxConcurrent) {
			waitingTasks.push(runner)
		} else {
			runningTasks += 1
			runner()
		}
	})
}
