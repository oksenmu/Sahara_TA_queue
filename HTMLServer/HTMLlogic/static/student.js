var chosen = 0;

function setFavicon(num) {
    if ((num < 0 || num > 30) && num !== "Q") {
        console.error("Number must be between 0 and 30.");
        return;
    }

    const faviconLink = document.querySelector("link[rel='icon']") || document.createElement("link");
    faviconLink.rel = "icon";
    faviconLink.href = num === 0 ? "../static/favicons/0.gif" : `../static/favicons/${num}.png`;
    faviconLink.type = num === 0 ? "image/gif" : "image/png";

    // Append to <head> if not already in the document
    if (!document.querySelector("link[rel='icon']")) {
        document.head.appendChild(faviconLink);
    }
}

function fetchTableNumber() {
    fetch("/api/table_number", {
        method: "GET",
        headers: { "Content-Type": "application/json" },
    })
        .then((res) => res.json())
        .then((data) => {
            if (data["error"] === "") {
                const table_number = data["data"];
                if (table_number !== 0) {
                    chosen = table_number;
                    document.getElementById(chosen).classList.add("chosen-table");
                    document.getElementById("button").innerText = "Call TA " + chosen;
                }
            }
        })
        .catch((err) => console.error("Error fetching user info:", err));
}

document.querySelectorAll(".table").forEach((table) => {
    table.innerText = table.id;
    table.addEventListener("click", () => {
        if (chosen != 0) {
            document.getElementById(chosen).classList.remove("chosen-table");
        }
        fetch("/api/table_number", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: table.getAttribute("data-table-number"),
        })
            .then((res) => res.json())
            .then((data) => {
                if (data["error"] === "") {
                    table.classList.add("chosen-table");
                    chosen = table.getAttribute("data-table-number");
                    document.getElementById("button").innerText = "Call TA " + chosen;
                    startSocket("poll");
                }
            })
            .catch((err) => console.error("Error fetching user info:", err));
    });
});

document.getElementById("leave").addEventListener("click", () => {
    startSocket("leave");
});

document.getElementById("button").addEventListener("click", () => {
    requestHelp();
});

document.getElementById("oppgave").addEventListener("keypress", (event) => {
    if (event.key === "Enter") requestHelp();
});

function requestHelp() {
    if (chosen != 0) {
        fetch("/api/task", {
            method: "POST",
            headers: { "Content-Type": "application/text" },
            body: document.getElementById("oppgave").value,
        })
            .then((res) => res.json())
            .then((data) => {
                if (data["error"] != "") {
                    document.getElementById("warning").innerText = data["error"];
                    setTimeout(() => {
                        document.getElementById("warning").innerHTML = "";
                    }, 7000);
                } else {
                    startSocket("join");
                }
            });
    } else {
        document.getElementById("warning").innerText = "Please select your table";
        setTimeout(() => {
            document.getElementById("warning").innerHTML = "";
        }, 10000);
    }
}

function startSocket(command) {
    let socket = new WebSocket("ws://" + window.location.hostname + ":27890/student");
    if (command === "join") {
        data = { command: "join" };
        socket.onopen = function() {
            socket.send(JSON.stringify(data));
        };
    } else if (command === "poll") {
        data = { command: "poll" };
        socket.onopen = function() {
            socket.send(JSON.stringify(data));
        };
    } else if (command === "leave") {
        data = { command: "leave" };
        socket.onopen = function() {
            socket.send(JSON.stringify(data));
        };
    }
    socket.onclose = function() {
        document.getElementById("queue-info").innerText = "";
        document.getElementById("title").innerText = `TA queue`;
        console.log("Socket closed");
    };
    socket.onmessage = function(event) {
        console.log(event.data);
        const data = JSON.parse(event.data);
        console.log(data);
        if (data["message_type"] == "Error") {
            console.error(data["error"]);
            document.getElementById("warning").innerText = data["error"];
            setTimeout(() => {
                document.getElementById("warning").innerText = "";
            }, 7000);
        } else if (data["message_type"] == "NotQueue") {
            document.getElementById("queue-info").innerText = "";
            document.getElementById("title").innerText = `TA queue`;
            setFavicon("Q");
        } else if (data["message_type"] == "Index") {
            document.getElementById("queue-info").innerText = `You are number ${data["data"]} in the queue`;
            setFavicon(data["data"]);
            document.getElementById("title").innerText = `TA queue q=${data["data"]}`;
        } else if (data["message_type"] == "Helping") {
            document.getElementById("queue-info").innerText = data["data"];
            setFavicon(0);
            document.getElementById("title").innerText = `TA queue q=0`;
        }
    };
}

fetchTableNumber();
startSocket("poll");
