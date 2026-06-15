// Tiny progressive enhancement: copy-to-clipboard on command blocks.
document.querySelectorAll(".cmd .copy").forEach((btn) => {
  btn.addEventListener("click", () => {
    const pre = btn.parentElement.querySelector("pre");
    const text = pre ? pre.innerText : "";
    navigator.clipboard.writeText(text).then(
      () => {
        const old = btn.textContent;
        btn.textContent = "ok!";
        btn.classList.add("ok");
        setTimeout(() => {
          btn.textContent = old;
          btn.classList.remove("ok");
        }, 1200);
      },
      () => {
        btn.textContent = "err";
      }
    );
  });
});
