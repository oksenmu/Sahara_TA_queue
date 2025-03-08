var chosen = 0

function fetchTableNumber() {
    fetch("/api/table_number", {
        method: "GET",
        headers: { 'Content-Type': 'application/json' }
    })
    .then(res => res.json())
    .then(data => {
        if (data["error"] === "") {
            const table_number = data["data"];
            if (table_number !== 0) {
                chosen = table_number
                document.getElementById(chosen).classList.add("chosen-table");
                document.getElementById("button").innerText = "Call TA " + chosen;
            }
        }
    })
    .catch(err => console.error("Error fetching user info:", err));
}

document.querySelectorAll('.table').forEach(table => {
    table.addEventListener('click', () => {
        if(chosen != 0){
            document.getElementById(chosen).classList.remove("chosen-table");
        }
        fetch("/api/table_number", {
            method: "POST",
            headers: { 'Content-Type': 'application/json' },
            body: table.getAttribute('data-table-number')
        })
        .then(res => res.json())
        .then(data => {
            if (data["error"] === "") {
                table.classList.add("chosen-table");
                chosen = table.getAttribute('data-table-number');
                document.getElementById("button").innerText = "Call TA " + chosen
            }
        })
        .catch(err => console.error("Error fetching user info:", err));
    });
});

document.getElementById('button').addEventListener('click', () => {
    requestHelp()
});

document.getElementById('oppgave').addEventListener('keypress', (event) => {
    if(event.key === "Enter") requestHelp()
});

function requestHelp(){
    if (chosen != 0){
        fetch("/request_help", {
            method: "POST",
            headers: {'Content-Type': 'application/json'}, 
            body: JSON.stringify({table_id:chosen, task:document.getElementById("oppgave").value})
        }).then(res => 
            res.json()
        ).then(data=>{
            if (data["error"] != ""){
                document.getElementById("warning").innerText = data["error"];
                setTimeout(() => {
                    document.getElementById("warning").innerHTML = ""
                }, 7000);
            }
            else{
                updatePosition()
            }
        });
    }
    else{
        document.getElementById("warning").innerText = "Please select your table";
        setTimeout(() => {
            document.getElementById("warning").innerHTML = ""
        }, 10000);
    }
}

function updatePosition() {
    fetch("/api/my_index", {
        method: "GET",
        headers: {'Content-Type': 'text/plain'}, 
    }).then(res => 
        res.json()
    ).then(data=>{
        if (data["error"] == "" && data["data"]["status"]=="In queue"){
            document.getElementById("queue-info").innerText = `You are number ${data["data"]["index"]} in the queue`;
            document.getElementById("title").innerText = `TA queue q=${data["data"]["index"]}`
        }
        else if(data["error"] == "" && data["data"]["status"]=="Getting help"){
            document.getElementById("queue-info").innerText = `You are now getting help from ${data["data"]["helped_by"]}`;
            document.getElementById("title").innerText = `TA queue q=0`
        }
        else{
            document.getElementById("queue-info").innerText = ""
            document.getElementById("title").innerText = `TA queue`
        }
    });
}

fetchTableNumber();

const socket = new WebSocket('ws://'+ window.location.hostname + ':27890');

socket.onmessage = function (event) {
    const data = JSON.parse(event.data);
    if (data["error"] == "" && data["data"]["status"]=="In queue"){
        document.getElementById("queue-info").innerText = `You are number ${data["data"]["index"]} in the queue`;
        document.getElementById("title").innerText = `TA queue q=${data["data"]["index"]}`
    }
    else if(data["error"] == "" && data["data"]["status"]=="Getting help"){
        document.getElementById("queue-info").innerText = `You are now getting help from ${data["data"]["helped_by"]}`;
        document.getElementById("title").innerText = `TA queue q=0`
    }
    else{
        document.getElementById("queue-info").innerText = ""
        document.getElementById("title").innerText = `TA queue`
    }
}


socket.onopen = function () {
    // Send a message to the WebSocket server
    socket.send(document.cookie);
};
