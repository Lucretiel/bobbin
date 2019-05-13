/*
 * Copyright 2019 Nathan West
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * promiseRunner is a helper to ensure that your browser isn't overloaded with
 * things. It returns a function with the signature (() => Promise) => Promise. When
 * called, it adds the callable to the task queue. The callables will be called,
 * in order; each callable should initiate some work and return a Promise
 * representing that work. The returned Promise waits for the callable to start,
 * then resolves with the result of the callable.
*/

export default function promiseRunner<T>(maxConcurrent: number):
	(userRunner: () => PromiseLike<T> | Promise<T> | T) => Promise<T>
{
	let runningTasks = 0

	// List of functions that initate the task
	const waitingTasks: Array<() => void> = []

	const taskComplete = () => {
		if(runningTasks <= maxConcurrent) {
			const runner = waitingTasks.shift();
			if(runner) {
				runner()
			}
		} else {
			runningTasks -= 1;
		}
	}

	return (userRunner: () => PromiseLike<T> | Promise<T> | T): Promise<T> => new Promise<T>(resolve => {
		const runner = () => {
			// Promisify the userRunner– handle thrown exceptions and non-promise return
			// values
			const task = new Promise<T>(resolveTask => resolveTask(userRunner()))

			// Schedule the next task when the user task completes
			resolve(task.finally(taskComplete));
		}

		if(runningTasks >= maxConcurrent) {
			waitingTasks.push(runner)
		} else {
			runningTasks += 1
			runner()
		}
	})
}
