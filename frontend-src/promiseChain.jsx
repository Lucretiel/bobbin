/*
promiseRunner is a helper to ensure that your browser isn't overloaded with
things. It returns a function with the signature (() => Promise) => Promise. When
called, it adds the callable to the task queue. The callables will be called,
in order; each callable should initiate some work and return a Promise
representing that work. The returned Promise waits for the callable to start,
then resolves with the result of the callable.
*/

export default function promiseRunner(maxConcurrent) {
	let runningTasks = 0

	// List of functions that initate the task
	const waitingTasks = []

	return userRunner => new Promise(resolve => {
		const runner = () => {
			// Promisify the userRunner– handle thrown exceptions and non-promise return
			// values
			const task = new Promise(resolveTask => resolveTask(userRunner()))

			// Schedule the next task when the user task completes
			resolve(task.finally(() => {
				if(waitingTasks.length > 0 && runningTasks <= maxConcurrent) {
					const nextRunner = waitingTasks.shift()
					nextRunner()
				} else {
					runningTasks -= 1
				}
			}))
		}

		if(runningTasks >= maxConcurrent) {
			waitingTasks.push(runner)
		} else {
			runningTasks += 1
			runner()
		}
	})
}
