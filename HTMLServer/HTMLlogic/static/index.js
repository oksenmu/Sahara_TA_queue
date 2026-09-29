
function rotDiv() {
    const el = document.getElementById("tables");
    if (!el) return;
    el.classList.toggle('rot-y');
}

window.addEventListener("keypress", (event)=>event.key==="f"?rotDiv():undefined);
document.getElementById("flip").onclick = rotDiv

fetch("rooms")
  .then(response => response.json())
  .then(rooms => {
    // document.getElementById("tables").innerHTML = html;
    fetch(`rooms/${rooms[0]}`)
      .then(response => response.text())
      .then(html => {
        document.getElementById("tables").innerHTML = html;
        init();
      });
  });
