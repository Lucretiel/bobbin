// Return a future that resolves when DOMContentLoaded fires, or if the

import { settings } from "cluster";

// page is already loaded
export const pageReady: Promise<void> = new Promise((resolve) => {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => resolve());
  } else {
    resolve();
  }
});

// Decorator for a single-argument function that causes
// it to only be called if the argument changed.
export const requireChanged = <T, R>(func: (arg: T) => R): ((arg: T) => R) => {
  let cache: { arg: T; ret: R } | null = null;

  return (arg) => {
    if (cache === null || cache.arg !== arg) {
      const ret = func(arg);
      cache = { ret, arg };
      return ret;
    } else {
      return cache.ret;
    }
  };
};

// Returns a future that waits for the document to be loaded, then fetches
// an element by ID, or throws an error if it doesn't exist
export const fetchElementById = (id: string) =>
  pageReady.then(() => {
    const element = document.getElementById(id);
    if (element !== null) {
      return element;
    } else {
      throw new Error(`No element with id ${id}`);
    }
  });

// Returns a future that waits for the document to be loaded, then fetches
// a list of elements by ID, returning them as an array.
export const fetchElementsByIds = (...ids: string[]) =>
  Promise.all(ids.map((id) => fetchElementById(id)));

export const fetchElementsByClass = (className: string) =>
  pageReady.then(
    () =>
      Array.from(document.getElementsByClassName(className)) as HTMLElement[]
  );

export const promiseChain = <T>(maxConcurrent: number) => {
  let runningTasks = 0;
  let queue: Array<() => void> = [];

  const taskComplete = () => {
    if (queue.length > 0 && runningTasks <= maxConcurrent) {
      const runner = queue.shift()!;
      runner();
    } else {
      runningTasks -= 1;
    }
  };

  return (userRunner: () => PromiseLike<T>) =>
    new Promise((resolve) => {
      const runner = () => {
        // Promisify the userRunner, handle thrown exceptions
        resolve(
          new Promise((resolveTask) => resolveTask(userRunner())).finally(
            taskComplete
          )
        );
      };

      if (runningTasks >= maxConcurrent) {
        queue.push(runner);
      } else {
        runningTasks += 1;
        runner();
      }
    });
};
