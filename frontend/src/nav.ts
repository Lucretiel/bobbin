import { fetchElementsByIds } from "./common";

fetchElementsByIds("nav-burger", "navbar-links").then(([burger, menu]) => {
  let isOpen = false;

  burger.addEventListener("click", () => {
    isOpen = !isOpen;

    burger.classList.toggle("is-active", isOpen);
    menu.classList.toggle("is-active", isOpen);
  });
});
