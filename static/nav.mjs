import { whenReady, classNames } from "./common.mjs"

whenReady(() => {
    const burger = document.getElementById("nav-burger");
    const menu = document.getElementById("navbar-links");

    let isOpen = false;

    burger.addEventListener("click", () => {
        isOpen = !isOpen;

        const activeClass = { "is-active": isOpen };
        const burgerClass = classNames("navbar-burger", activeClass);
        const menuClass = classNames("navbar-menu", activeClass);

        burger.className = burgerClass;
        menu.className = menuClass;
    });
});
