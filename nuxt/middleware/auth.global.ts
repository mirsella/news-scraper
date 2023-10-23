export default defineNuxtRouteMiddleware((to, from) => {
  if (!process.client || to.path === "/login") return;
  const jwt = localStorage.getItem("jwt");
  if (!jwt) {
    console.log("No JWT found, redirecting to login");
    return navigateTo("/login");
  }
});
