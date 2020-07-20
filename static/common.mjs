export const classNames = (...items) => {
    const result = [];

    const handleItem = item => {
        if (typeof item === "string") {
            if (item) result.push(item);
        } else if (Array.isArray(item)) {
            item.forEach(handleItem);
        } else {
            for (const key in item) {
                if (item[key]) {
                    result.push(key);
                }
            }
        }
    }

    items.forEach(handleItem);

    return result.join(" ");
}

export const whenReady = func => {
    const run = () => func();
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", run);
    } else {
        setTimeout(run, 0);
    }
}

// Decorator for a single-argument function that causes
// it to only be called if the argument changed.
export const requireChanged = func => {
    let lastSeen = Symbol();
    let lastReturn = null;

    return arg => {
        if (lastSeen !== arg) {
            lastSeen = arg;
            lastReturn = func(arg);
            return lastReturn;
        } else {
            return lastReturn;
        }
    }
}
