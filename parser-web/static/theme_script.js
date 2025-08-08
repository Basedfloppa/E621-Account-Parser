document.addEventListener("DOMContentLoaded", () => {
  const theme = localStorage.getItem("theme");
  const body = document.querySelector("body");
  body.setAttribute("data-bs-theme", theme || "light");
});
