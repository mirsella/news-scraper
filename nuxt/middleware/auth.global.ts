export default defineNuxtRouteMiddleware((to, from) => {
  return;
  if (to.path === "/login") return;
  if (from.path === to.path) return;
  if (process.server) return;
  const jwt = localStorage.getItem("jwt");
  if (!jwt) {
    console.log("No JWT found, redirecting to login");
    return navigateTo("/login");
  }
});
