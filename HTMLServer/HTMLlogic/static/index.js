function rotDiv() {
    const el = document.getElementById("tables");
    if (!el) return;
    el.classList.toggle('rot-y');
}

window.addEventListener("keypress", (event)=>event.key==="f"?rotDiv():undefined);
document.getElementById("flipp").onclick = rotDiv
