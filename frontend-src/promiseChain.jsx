/*
promiseChain is a helper to ensure that your browser isn't overloaded with
things
*/

export default function promiseRunner(maxConcurrent) {
	const runningTasks = new Set()
	const waitingTasks = []

	const launchTask = callable => {
		const task = callable()
		runningTasks.add(task)

		task.catch(error => null).then(result => {
			runningTasks.delete(task)

			if(waitingTasks.length > 0 && runningTasks.size < maxConcurrent) {
				launchTask(waitingTasks.shift())
			}
		})
	}

	return callable => new Promise(resolve => {
		const runner = () => {
			const task = callable()
			resolve(task)
			return task
		}

		if(runningTasks.size >= maxConcurrent) {
			waitingTasks.push(runner)
		} else {
			launchTask(runner)
		}
	})
}
