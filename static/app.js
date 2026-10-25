// HTMX does not swap 4xx/5xx responses by default. Show a plain-text,
// accessible error in the existing target without interpreting server HTML.
document.addEventListener('htmx:responseError', (event) => {
  const target = event.detail.target;
  if (!target) return;
  target.textContent = event.detail.xhr.responseText || 'Request failed. Please try again.';
  target.setAttribute('role', 'alert');
});
