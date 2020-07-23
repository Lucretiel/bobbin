import { classNames, fetchElementsByIds } from "./common";

fetchElementsByIds("nav-burger", "navbar-links").then(([burger, menu]) => {
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
